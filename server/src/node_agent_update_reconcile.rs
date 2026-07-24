//! Update transaction checkpoints and restart-safe supervised task recovery.

use std::{path::Path, sync::Arc};

use anyhow::{Context, Result};
use homecli_proto::{CliCompletionProducerIdentity, CliProjectContext};
use tokio_tungstenite::tungstenite::Message;
use tracing::warn;

use crate::{
    node_agent_cli_done::CliCompletionContext,
    node_agent_cli_sidecar::{now_ms, CliSidecarSessionRecord},
    node_agent_cli_task_dispatch::{spawn_cli_task, CliTaskDispatchRequest},
    node_agent_local_task_store::{LocalTaskRecord, LocalTaskStart},
    node_agent_local_task_supervision::{
        load_supervision_contract, record_supervision_event, SupervisionContract,
        SUPERVISION_PROTOCOL,
    },
    node_agent_update_checkpoint::{
        file_sha256, fingerprint_workspace, git_output, incomplete_non_repeatable_action,
        same_path, stable_resume_task_id,
    },
    node_agent_update_recovery::{
        UpdateRecoveryReceipt, UpdateRecoveryState, UpdateRecoveryStore, WorkspaceGitFingerprint,
    },
    NodeRuntime,
};

#[path = "node_agent_update_reconcile_supersede.rs"]
mod supersede;
#[cfg(test)]
use crate::node_agent_update_recovery::ReleaseIdentity;
use supersede::{
    reconcile_superseded_history, record_superseded_recovery, superseding_release_evidence,
};

pub(crate) async fn reconcile_startup(runtime: Arc<NodeRuntime>) {
    let current_release = crate::node_agent_release_identity::current();
    let receipts = match runtime.update_recovery.active() {
        Ok(receipts) => receipts,
        Err(error) => {
            warn!(%error, "启动时读取更新恢复事务失败");
            return;
        }
    };
    let now = now_ms();
    let live_sidecar_task_ids = runtime
        .cli_sidecars
        .all_sessions()
        .unwrap_or_default()
        .into_iter()
        .filter(|session| {
            session.protects_startup_reconcile_at(now) || session.can_replay_after_restart_at(now)
        })
        .map(|session| session.task_id)
        .collect::<std::collections::HashSet<_>>();
    let (startup_critical, historical): (Vec<_>, Vec<_>) = receipts
        .into_iter()
        .filter(|receipt| {
            !matches!(
                receipt.state,
                UpdateRecoveryState::Paused
                    | UpdateRecoveryState::ApprovalRequired
                    | UpdateRecoveryState::Conflict
                    | UpdateRecoveryState::Timeout
            )
        })
        .partition(|receipt| {
            receipt_requires_startup_reconcile(receipt, &current_release, &live_sidecar_task_ids)
        });
    for receipt in startup_critical {
        if let Err(error) = reconcile_one(runtime.clone(), receipt).await {
            warn!(%error, "启动更新恢复事务失败");
        }
    }
    tokio::spawn(async move {
        if let Err(error) = reconcile_superseded_history(&runtime.update_recovery, &current_release)
        {
            warn!(%error, "后台收敛历史更新恢复票据失败");
        }
        for receipt in historical {
            if let Err(error) = reconcile_one(runtime.clone(), receipt).await {
                warn!(%error, "后台收敛历史更新恢复事务失败");
            }
        }
    });
}

fn receipt_requires_startup_reconcile(
    receipt: &UpdateRecoveryReceipt,
    current_release: &str,
    live_sidecar_task_ids: &std::collections::HashSet<String>,
) -> bool {
    matches!(
        release_relation(receipt, current_release),
        ReleaseRelation::Target
    ) || live_sidecar_task_ids.contains(receipt.active_task_id())
}

pub(crate) fn reconcile_superseded_history_for_current_release(
    store: &UpdateRecoveryStore,
) -> Result<usize> {
    reconcile_superseded_history(store, &crate::node_agent_release_identity::current())
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
        crate::node_agent_local_terminal_reconcile::LocalTerminalReconciler::from_runtime(&runtime)
            .reconcile(&completion)
            .await?;
        return Ok(());
    }
    let current_release = crate::node_agent_release_identity::current();
    match release_relation(&receipt, &current_release) {
        ReleaseRelation::Target => {}
        ReleaseRelation::From => {
            // The old process can briefly win the restart race before the
            // launcher installs the target. Keep the transaction recoverable;
            // it is neither target-online nor a target identity failure.
            return Ok(());
        }
        ReleaseRelation::Other => {
            if let Some(evidence) =
                superseding_release_evidence(&runtime.update_recovery, &receipt, &current_release)?
            {
                if let Err(error) = validate_superseded_recovery(&runtime, &task, &receipt).await {
                    let reason = format!("节点更新恢复已熔断：后继版本收敛证据不完整：{error}");
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
                let changed =
                    record_superseded_recovery(&runtime.update_recovery, &receipt, &evidence)?;
                if changed {
                    record_supervision_event(
                        &runtime.task_journal,
                        &active_task_id,
                        "supervision_update_recovery_superseded",
                        serde_json::json!({
                            "old_update_id": update_id,
                            "superseded_by_update_id": evidence.update_id,
                            "current_release": current_release,
                            "evidence": evidence.source,
                            "non_repeatable_actions_replayed": false,
                        }),
                    )?;
                }
                return Ok(());
            }
            let reason = "节点更新恢复已熔断：节点发布身份既不是 from_release 也不是目标 release";
            runtime
                .local_tasks
                .mark_recovery_blocked(&active_task_id, reason)?;
            return set_recovery_state(
                &runtime.update_recovery,
                &update_id,
                &original_task_id,
                UpdateRecoveryState::Failed,
                reason,
            );
        }
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
    let supervision = load_supervision_contract(&runtime.task_journal, &active_task_id)?
        .context("更新恢复任务缺少监督合同")?;
    let admission_base = crate::node_agent_supervision_terminal_lease_safety::admission_base(
        &task,
        &supervision,
        &active_task_id,
    )?;
    let admission = crate::node_agent_supervision_worktree_lease::ResumeAdmissionGuard::acquire(
        &admission_base,
    )?;
    if let Some(completion) = runtime
        .completion_outbox
        .latest_for_req_id(&active_task_id)?
    {
        drop(admission);
        crate::node_agent_local_terminal_reconcile::LocalTerminalReconciler::from_runtime(&runtime)
            .reconcile(&completion)
            .await?;
        return Ok(());
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
        crate::node_agent_sidecar_recovery::spawn_recovered_sidecar_monitor(
            runtime,
            task,
            sidecar,
            Some(receipt),
            Some(&admission),
        )
        .await?;
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
    spawn_resume_original(runtime, receipt, task, admission).await
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
    Ok(())
}

async fn validate_superseded_recovery(
    runtime: &NodeRuntime,
    task: &LocalTaskRecord,
    receipt: &UpdateRecoveryReceipt,
) -> Result<()> {
    anyhow::ensure!(
        receipt.state == UpdateRecoveryState::Resumed
            && receipt.resume_strategy.as_deref() == Some("sidecar_reattach"),
        "后继版本目标存在，但旧恢复票据尚未完成 sidecar reattach"
    );
    validate_local_recovery(runtime, task, receipt).await?;
    anyhow::ensure!(
        receipt.safety.evidence_complete,
        "后继版本目标存在，但旧恢复票据缺少完整安全证据"
    );
    anyhow::ensure!(
        fingerprint_workspace(Path::new(&receipt.workspace.workspace_path)) == receipt.workspace,
        "后继版本目标存在，但工作区或 Git 指纹已漂移"
    );
    anyhow::ensure!(
        recovery_sidecar(runtime, receipt, receipt.active_task_id())?.is_some(),
        "后继版本目标存在，但结构化 sidecar 已不可验证"
    );
    Ok(())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ReleaseRelation {
    Target,
    From,
    Other,
}

fn release_relation(receipt: &UpdateRecoveryReceipt, current: &str) -> ReleaseRelation {
    if release_identity_matches(&receipt.to_release, current) {
        ReleaseRelation::Target
    } else if release_identity_matches(&receipt.from_release, current) {
        ReleaseRelation::From
    } else {
        ReleaseRelation::Other
    }
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

async fn spawn_resume_original(
    runtime: Arc<NodeRuntime>,
    receipt: UpdateRecoveryReceipt,
    parent: LocalTaskRecord,
    admission: crate::node_agent_supervision_worktree_lease::ResumeAdmissionGuard,
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
            resume_admission: Some(admission),
            inherited_authorization_record: Some(parent.clone()),
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

pub(crate) fn recovered_workspace(
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
    if !session.can_replay_after_restart_at(now_ms())
        && !crate::node_agent_sidecar_recovery::output_contains_terminal_record(output_path)?
    {
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
