//! Update transaction checkpoints and restart-safe supervised task recovery.

use std::{path::Path, sync::Arc};

use anyhow::{Context, Result};
use homecli_proto::{CliCompletionProducerIdentity, CliProjectContext};
use tokio::sync::watch;
use tokio_tungstenite::tungstenite::Message;
use tracing::warn;

use crate::{
    node_agent_active_task::ActiveCliPromptHandle,
    node_agent_cli_done::{
        cli_done_message_from_output, persist_and_send_cli_done, CliCompletionContext,
    },
    node_agent_cli_sidecar::{now_ms, CliSidecarSessionRecord},
    node_agent_cli_task_dispatch::{spawn_cli_task, CliTaskDispatchRequest},
    node_agent_local_task_store::{LocalTaskRecord, LocalTaskStart},
    node_agent_local_task_supervision::{
        load_supervision_contract, record_supervision_event, SupervisionContract,
        SUPERVISION_PROTOCOL,
    },
    node_agent_task_journal_events::completion_terminal_status,
    node_agent_update_checkpoint::{
        file_sha256, fingerprint_workspace, git_output, incomplete_non_repeatable_action,
        same_path, stable_resume_task_id,
    },
    node_agent_update_recovery::{
        UpdateRecoveryReceipt, UpdateRecoveryState, UpdateRecoveryStore, WorkspaceGitFingerprint,
    },
    NodeRuntime,
};

pub(crate) async fn reconcile_startup(runtime: Arc<NodeRuntime>) {
    let receipts = match runtime.update_recovery.active() {
        Ok(receipts) => receipts,
        Err(error) => {
            warn!(%error, "启动时读取更新恢复事务失败");
            return;
        }
    };
    for receipt in receipts {
        if matches!(
            receipt.state,
            UpdateRecoveryState::Paused
                | UpdateRecoveryState::ApprovalRequired
                | UpdateRecoveryState::Conflict
                | UpdateRecoveryState::Timeout
        ) {
            continue;
        }
        if let Err(error) = reconcile_one(runtime.clone(), receipt).await {
            warn!(%error, "启动更新恢复事务失败");
        }
    }
}

async fn reconcile_one(runtime: Arc<NodeRuntime>, receipt: UpdateRecoveryReceipt) -> Result<()> {
    let update_id = receipt.update_id.clone();
    let original_task_id = receipt.original_task_id.clone();
    let active_task_id = receipt.active_task_id().to_string();
    let task = runtime
        .local_tasks
        .get(&active_task_id)?
        .context("更新恢复目标本机任务不存在")?;
    if let Some(completion) = runtime
        .completion_outbox
        .latest_for_req_id(&active_task_id)?
    {
        runtime.update_recovery.reconcile_terminal_completion(
            &active_task_id,
            &completion.event_id,
            completion_terminal_status(completion.exit_ok, completion.error.as_deref()),
            completion.created_at_ms as u128,
            completion.exit_ok,
        )?;
        return Ok(());
    }
    if let Err(error) = validate_local_recovery(&runtime, &task, &receipt).await {
        let reason = format!("节点更新恢复已熔断：{error}");
        runtime
            .local_tasks
            .mark_recovery_blocked(&active_task_id, &reason)?;
        return set_recovery_state(
            &runtime.update_recovery,
            &update_id,
            &original_task_id,
            UpdateRecoveryState::Failed,
            &reason,
        );
    }
    let snapshot = runtime.task_journal.snapshot(&active_task_id, 0, 200)?;
    let sidecar = recovery_sidecar(&runtime, &receipt, &active_task_id)?;

    if !receipt.safety.evidence_complete && sidecar.is_none() {
        return set_recovery_state(
            &runtime.update_recovery,
            &update_id,
            &original_task_id,
            UpdateRecoveryState::Failed,
            "insufficient recovery evidence",
        );
    }
    advance_runtime_online(&runtime.update_recovery, &update_id, &original_task_id)?;

    if let Some(sidecar) = sidecar {
        let resumed = if snapshot.approvals.pending_count > 0 {
            set_recovery_state(
                &runtime.update_recovery,
                &update_id,
                &original_task_id,
                UpdateRecoveryState::ApprovalRequired,
                "approval pending after runtime restart",
            )?;
            false
        } else {
            runtime
                .update_recovery
                .update(&update_id, &original_task_id, |receipt| {
                    receipt.transition(
                        UpdateRecoveryState::Reattaching,
                        Some("versioned sidecar survived update"),
                    )?;
                    receipt.resume_strategy = Some("sidecar_reattach".to_string());
                    receipt.transition(
                        UpdateRecoveryState::Resumed,
                        Some("sidecar output replay reattached"),
                    )?;
                    Ok(())
                })?;
            true
        };
        if resumed {
            crate::node_agent_sidecar_recovery_replay::record_receipt_resumed(
                &runtime.task_journal,
                &runtime.local_tasks,
                &active_task_id,
            )?;
        } else {
            runtime
                .local_tasks
                .mark_recovering(&active_task_id, "节点更新后仍有审批等待处理")?;
        }
        spawn_sidecar_monitor(runtime, receipt, task, sidecar).await?;
        return Ok(());
    }

    if receipt
        .recovery_policy
        .deadline_ms
        .is_some_and(|deadline| now_ms() > deadline)
    {
        return set_recovery_state(
            &runtime.update_recovery,
            &update_id,
            &original_task_id,
            UpdateRecoveryState::Paused,
            "recovery deadline expired",
        );
    }
    if snapshot.approvals.pending_count > 0 || !receipt.safety.pending_approval_ids.is_empty() {
        return set_recovery_state(
            &runtime.update_recovery,
            &update_id,
            &original_task_id,
            UpdateRecoveryState::ApprovalRequired,
            "approval pending; automatic replay is forbidden",
        );
    }
    if receipt.safety.non_repeatable_action.is_some()
        || incomplete_non_repeatable_action(&snapshot.events).is_some()
    {
        return set_recovery_state(
            &runtime.update_recovery,
            &update_id,
            &original_task_id,
            UpdateRecoveryState::Paused,
            "non-repeatable action requires review",
        );
    }
    if fingerprint_workspace(Path::new(&receipt.workspace.workspace_path)) != receipt.workspace {
        return set_recovery_state(
            &runtime.update_recovery,
            &update_id,
            &original_task_id,
            UpdateRecoveryState::Conflict,
            "workspace or git fingerprint drift",
        );
    }
    spawn_resume_original(runtime, receipt, task).await
}

async fn validate_local_recovery(
    runtime: &NodeRuntime,
    task: &LocalTaskRecord,
    receipt: &UpdateRecoveryReceipt,
) -> Result<()> {
    anyhow::ensure!(
        receipt.allows_local_reconcile(),
        "remote v1 或恢复能力声明不匹配，保持 fail-closed"
    );
    let creds = runtime.creds().await.context("节点当前没有已绑定身份")?;
    anyhow::ensure!(
        recovery_task_identity_matches(
            &task.owner_user_id,
            &task.agent_id,
            &task.install_id,
            &creds.owner_user_id,
            &creds.agent_id,
            &runtime.install_id,
        ),
        "任务 owner/agent/install 身份不匹配"
    );
    anyhow::ensure!(
        release_identity_matches(
            &receipt.to_release,
            &crate::node_agent_release_identity::current(),
        ),
        "节点发布身份与恢复回执目标不匹配"
    );
    Ok(())
}

fn recovery_task_identity_matches(
    task_owner: &str,
    task_agent: &str,
    task_install: &str,
    current_owner: &str,
    current_agent: &str,
    current_install: &str,
) -> bool {
    task_owner == current_owner && task_agent == current_agent && task_install == current_install
}

fn release_identity_matches(
    expected: &crate::node_agent_update_recovery::ReleaseIdentity,
    current: &str,
) -> bool {
    let current = current.trim();
    let version_matches = expected.version.trim().is_empty()
        || current == expected.version.trim()
        || current.starts_with(&format!("{}+", expected.version.trim()));
    let git_sha_matches = expected.git_sha.trim().is_empty()
        || current
            .rsplit_once('+')
            .is_some_and(|(_, sha)| sha == expected.git_sha.trim());
    version_matches && git_sha_matches
}

async fn spawn_sidecar_monitor(
    runtime: Arc<NodeRuntime>,
    receipt: UpdateRecoveryReceipt,
    task: LocalTaskRecord,
    sidecar: CliSidecarSessionRecord,
) -> Result<()> {
    let (cancel_tx, mut cancel_rx) = watch::channel(false);
    let handle = ActiveCliPromptHandle::new(
        &task.task_id,
        &task.cli,
        &sidecar.route,
        sidecar.cwd.clone(),
        Some(task.runtime_permission.clone()),
        cancel_tx,
    )
    .with_exclusive_workspace(true);
    if runtime.try_register_cli_prompt(handle).await
        != crate::node_agent_active_task_registry::CliPromptRegistration::Inserted
    {
        return Ok(());
    }
    let output_path = sidecar
        .endpoint
        .as_deref()
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            runtime
                .cli_sidecars
                .output_path(&task.task_id, &sidecar.session_id)
        });
    let update_id = receipt.update_id.clone();
    let original_task_id = receipt.original_task_id.clone();
    tokio::spawn(async move {
        let initial_cursor = crate::node_agent_cli_sidecar_runner::CliSidecarReplayCursor {
            offset: receipt.sidecar_output_offset,
            sequence: receipt.sidecar_output_sequence,
        };
        let session_id = sidecar.session_id.clone();
        let result = crate::node_agent_cli_sidecar_runner::follow_sidecar_output_from_with_batch(
            &runtime.cli_sidecars,
            &task.task_id,
            &output_path,
            initial_cursor,
            &mut cancel_rx,
            |_| {},
            |records, cursor| {
                crate::node_agent_sidecar_recovery_replay::persist_batch_before_cursor(
                    &runtime.task_journal,
                    &task.task_id,
                    &session_id,
                    records,
                    cursor,
                    |cursor| {
                        if runtime
                            .update_recovery
                            .receipt_for_task(&task.task_id)?
                            .is_some_and(|receipt| {
                                matches!(
                                    receipt.state,
                                    UpdateRecoveryState::Reattaching | UpdateRecoveryState::Resumed
                                )
                            })
                        {
                            crate::node_agent_sidecar_recovery_replay::record_replayed_activity(
                                &runtime.task_journal,
                                &runtime.local_tasks,
                                &task.task_id,
                                records,
                            )?;
                        }
                        runtime.cli_sidecars.record_output_cursor(
                            &task.task_id,
                            &session_id,
                            cursor.offset,
                            cursor.sequence,
                        )?;
                        runtime.update_recovery.record_sidecar_cursor(
                            &update_id,
                            &original_task_id,
                            cursor.offset,
                            cursor.sequence,
                        )
                    },
                )
            },
        )
        .await;
        match result {
            Ok(result) => {
                let success = result.exit_ok && !result.canceled && result.terminal_error.is_none();
                let error = if success {
                    None
                } else {
                    result.terminal_error.clone().or_else(|| {
                        Some(if result.canceled {
                            "任务在更新恢复期间被取消".to_string()
                        } else {
                            "CLI sidecar 恢复后返回失败".to_string()
                        })
                    })
                };
                let workspace =
                    recovered_workspace(&task, &receipt.workspace, &receipt.root_task_id);
                let (success, error, workspace_status) =
                    crate::node_agent_cli_runner::finalize_cli_prompt_workspace(
                        success, error, workspace,
                    );
                let session_id = runtime
                    .task_journal
                    .snapshot(&task.task_id, 0, 1)
                    .ok()
                    .and_then(|snapshot| snapshot.record)
                    .and_then(|record| record.codex_session_id);
                let (stdout, stderr) =
                    crate::node_agent_sidecar_recovery_replay::recovered_completion_output(
                        &runtime.task_journal,
                        &task.task_id,
                        &output_path,
                        200_000,
                    )
                    .unwrap_or_else(|error| {
                        warn!(%error, task_id = %task.task_id, "failed to merge recovery transcript");
                        (result.stdout_text.clone(), result.stderr_text.clone())
                    });
                let (done, output) = cli_done_message_from_output(
                    task.task_id.clone(),
                    success,
                    error,
                    &stdout,
                    &stderr,
                    None,
                    workspace_status,
                    session_id,
                );
                let context = completion_context(&task);
                let (out_tx, _out_rx) = tokio::sync::mpsc::unbounded_channel::<Message>();
                let persisted = persist_and_send_cli_done(
                    &runtime,
                    &context,
                    &task.cli,
                    Some(&output),
                    done,
                    &out_tx,
                );
                if let Ok(completion) = persisted.as_ref() {
                    let _ = runtime.update_recovery.record_terminal_binding(
                        &task.task_id,
                        &completion.event_id,
                        completion_terminal_status(completion.exit_ok, completion.error.as_deref()),
                        completion.created_at_ms as u128,
                    );
                }
                let state = if persisted.is_ok() && success {
                    UpdateRecoveryState::Verified
                } else {
                    UpdateRecoveryState::Failed
                };
                let reason = if state == UpdateRecoveryState::Verified {
                    "recovered task completed and durable result persisted"
                } else {
                    "recovered task failed or durable completion was rejected"
                };
                let _ = set_recovery_state(
                    &runtime.update_recovery,
                    &update_id,
                    &original_task_id,
                    state,
                    reason,
                );
            }
            Err(error) => {
                let _ = set_recovery_state(
                    &runtime.update_recovery,
                    &update_id,
                    &original_task_id,
                    UpdateRecoveryState::Failed,
                    &format!("sidecar replay failed: {error}"),
                );
            }
        }
        runtime.finish_cli_prompt(&task.task_id).await;
    });
    Ok(())
}

async fn spawn_resume_original(
    runtime: Arc<NodeRuntime>,
    receipt: UpdateRecoveryReceipt,
    parent: LocalTaskRecord,
) -> Result<()> {
    let resume_task_id = receipt
        .resume_task_id
        .clone()
        .unwrap_or_else(|| stable_resume_task_id(&receipt.update_id, &receipt.original_task_id));
    let contract = SupervisionContract {
        protocol: SUPERVISION_PROTOCOL.to_string(),
        supervisor: "codex_desktop".to_string(),
        task_role: "resume_original".to_string(),
        parent_task_id: Some(receipt.active_task_id().to_string()),
        root_task_id: Some(receipt.root_task_id.clone()),
        acceptance_criteria: vec![
            "检查更新前 journal、Git 现场和未完成步骤后安全续跑原需求。".to_string(),
            "完成原任务验证、发布、复核和项目规定收尾。".to_string(),
        ],
        improvement_policy: "after_task_or_unblock".to_string(),
    };
    let resume = runtime.local_tasks.create(LocalTaskStart {
        task_id: &resume_task_id,
        owner_user_id: &parent.owner_user_id,
        agent_id: &parent.agent_id,
        install_id: &parent.install_id,
        project_id: &parent.project_id,
        channel_id: parent.channel_id.as_deref(),
        conversation_id: &parent.conversation_id,
        workspace_path: &parent.workspace_path,
        prompt: &parent.prompt,
        cli: "codex",
        runtime_permission: &parent.runtime_permission,
    })?;
    if load_supervision_contract(&runtime.task_journal, &resume_task_id)?.is_none() {
        record_supervision_event(
            &runtime.task_journal,
            &resume_task_id,
            "supervision_contract",
            crate::node_agent_local_task_supervision::contract_payload(&contract),
        )?;
    }
    runtime.local_tasks.mark_recovering(
        &parent.task_id,
        "节点更新后已创建幂等 resume_original 续跑任务",
    )?;
    runtime
        .update_recovery
        .update(&receipt.update_id, &receipt.original_task_id, |current| {
            current.resume_task_id = Some(resume_task_id.clone());
            current.resume_strategy = Some(if current.codex_session_id.is_some() {
                "codex_session_resume".to_string()
            } else {
                "snapshot_continue".to_string()
            });
            if current.state != UpdateRecoveryState::ResumeCreated {
                current.transition(
                    UpdateRecoveryState::ResumeCreated,
                    Some("idempotent resume_original child created"),
                )?;
            }
            current.transition(
                UpdateRecoveryState::Resumed,
                Some("resume_original dispatched"),
            )?;
            Ok(())
        })?;

    let frozen =
        crate::node_agent_codex_child_env::FrozenCodexHome::capture_unmanaged_for_local_task()?;
    let (out_tx, out_rx) = tokio::sync::mpsc::unbounded_channel::<Message>();
    crate::node_agent_local_tasks::spawn_local_output_consumer(
        runtime.clone(),
        resume.owner_user_id.clone(),
        resume_task_id.clone(),
        out_rx,
    );
    let inherited_workspace =
        recovered_workspace(&parent, &receipt.workspace, &receipt.root_task_id);
    spawn_cli_task(
        runtime,
        out_tx,
        CliTaskDispatchRequest {
            req_id: resume_task_id,
            cli: "codex".to_string(),
            extra_args: Vec::new(),
            cwd: Some(parent.workspace_path.clone()),
            project_context: Some(CliProjectContext {
                project_id: parent.project_id.clone(),
                conversation_id: parent.conversation_id.clone(),
                runtime_permission: Some(parent.runtime_permission.clone()),
            }),
            codex_credential_binding: None,
            requires_cloud_control: false,
            cloud_control_deadline: None,
            cloud_control_issued_at: None,
            cloud_control_ttl_ms: None,
            prompt: crate::node_agent_local_task_supervision::executor_prompt(
                &format!(
                    "节点更新后自动恢复原任务。先从 journal cursor {} 检查现场；不要重复已完成或不可重复动作。\n\n{}",
                    receipt.journal_cursor, parent.prompt
                ),
                Some(&contract),
            ),
            completion_context: completion_context(&resume),
            inherited_workspace,
            resume_admission: None,
            allow_codex_auth_switch: false,
            frozen_codex_home: Some(frozen),
        },
    );
    Ok(())
}

fn completion_context(task: &LocalTaskRecord) -> CliCompletionContext {
    CliCompletionContext::local_offline(
        CliCompletionProducerIdentity {
            owner_user_id: task.owner_user_id.clone(),
            agent_id: task.agent_id.clone(),
            install_id: task.install_id.clone(),
        },
        CliProjectContext {
            project_id: task.project_id.clone(),
            conversation_id: task.conversation_id.clone(),
            runtime_permission: Some(task.runtime_permission.clone()),
        },
        task.channel_id.clone(),
        task.prompt.clone(),
        Some(SUPERVISION_PROTOCOL.to_string()),
    )
}

fn recovered_workspace(
    task: &LocalTaskRecord,
    fingerprint: &WorkspaceGitFingerprint,
    root_task_id: &str,
) -> Option<crate::pc_workspace_provisioner::ConversationWorkspaceResult> {
    let active = fingerprint.workspace_path.trim();
    if active.is_empty() || same_path(Path::new(active), Path::new(&task.workspace_path)) {
        return None;
    }
    Some(
        crate::pc_workspace_provisioner::ConversationWorkspaceResult {
            base_workspace_path: Some(task.workspace_path.clone()),
            workspace_path: active.to_string(),
            isolated: true,
            branch: git_output(Path::new(active), &["branch", "--show-current"]),
            supervision_root_task_id: Some(root_task_id.to_string()),
        },
    )
}

fn recovery_sidecar(
    runtime: &NodeRuntime,
    receipt: &UpdateRecoveryReceipt,
    task_id: &str,
) -> Result<Option<CliSidecarSessionRecord>> {
    let Some(session) = runtime.cli_sidecars.session_for_task(task_id)? else {
        return Ok(None);
    };
    if !session.can_replay_output_at(now_ms()) {
        return Ok(None);
    }
    if receipt
        .sidecar_session_id
        .as_deref()
        .is_some_and(|expected| expected != session.session_id)
        && receipt.resume_task_id.as_deref() != Some(task_id)
    {
        return Ok(None);
    }
    let output_path = session
        .endpoint
        .as_deref()
        .map(Path::new)
        .unwrap_or_else(|| Path::new(""));
    if !output_path.is_file() {
        return Ok(None);
    }
    if let (Some(worker), Some(expected)) = (
        session.worker_path.as_deref(),
        session.worker_sha256.as_deref(),
    ) {
        let path = Path::new(worker);
        if path.exists() && file_sha256(path).as_deref() != Some(expected) {
            return Ok(None);
        }
    }
    Ok(Some(session))
}

fn advance_runtime_online(
    store: &UpdateRecoveryStore,
    update_id: &str,
    original: &str,
) -> Result<()> {
    store.update(update_id, original, |receipt| {
        loop {
            let next = match receipt.state {
                UpdateRecoveryState::Planned => UpdateRecoveryState::Downloaded,
                UpdateRecoveryState::Downloaded => UpdateRecoveryState::CheckpointSaved,
                UpdateRecoveryState::CheckpointSaved => UpdateRecoveryState::Applying,
                UpdateRecoveryState::Applying => UpdateRecoveryState::RuntimeOnline,
                _ => break,
            };
            receipt.transition(next, Some("startup reconcile"))?;
        }
        Ok(())
    })
}

fn set_recovery_state(
    store: &UpdateRecoveryStore,
    update_id: &str,
    original: &str,
    state: UpdateRecoveryState,
    reason: &str,
) -> Result<()> {
    store.update(update_id, original, |receipt| {
        receipt.transition(state, Some(reason)).map(|_| ())
    })
}

#[cfg(test)]
#[path = "node_agent_update_reconcile_tests.rs"]
mod tests;
