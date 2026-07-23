//! Startup repair for sidecars that outlive the node runtime.
//!
//! The immutable worker owns CLI output capture. This module consumes its
//! durable JSONL after a runtime restart and commits exactly one completion
//! envelope before repairing the local task row and journal terminal state.

#[path = "node_agent_sidecar_recovery_monitor.rs"]
mod monitor;
pub(crate) use monitor::spawn_recovered_sidecar_monitor;
use monitor::supervised_admission;

use std::{path::Path, sync::Arc};

use anyhow::{Context, Result};
use homecli_proto::{CliCompletionProducerIdentity, CliProjectContext, CliWorkspaceStatus};
use tokio::sync::watch;
use tokio_tungstenite::tungstenite::Message;
use tracing::{info, warn};

use crate::{
    node_agent_cli_done::{
        cli_done_message_from_output, persist_and_send_cli_done, CliCompletionContext,
    },
    node_agent_cli_sidecar::{now_ms, CliSidecarSessionRecord},
    node_agent_cli_sidecar_io::read_new_output_records,
    node_agent_cli_sidecar_runner::CliSidecarReplayCursor,
    node_agent_local_task_store::LocalTaskRecord,
    node_agent_local_task_supervision::{load_supervision_contract, SUPERVISION_PROTOCOL},
    node_agent_update_checkpoint::{
        fingerprint_workspace, incomplete_non_repeatable_action,
        preserve_platform_workspace_identity,
    },
    node_agent_update_recovery::{ReleaseIdentity, UpdateRecoveryReceipt, UpdateRecoveryState},
    NodeRuntime,
};

pub(crate) async fn reconcile_surviving_sidecars(runtime: Arc<NodeRuntime>) {
    let sessions = match runtime.cli_sidecars.all_sessions() {
        Ok(sessions) => sessions,
        Err(error) => {
            warn!(%error, "启动时读取 sidecar 注册表失败");
            return;
        }
    };
    for session in sessions {
        if let Err(error) = reconcile_one(runtime.clone(), session).await {
            warn!(%error, "启动时补账 sidecar 终态失败");
        }
    }
}

async fn reconcile_one(runtime: Arc<NodeRuntime>, session: CliSidecarSessionRecord) -> Result<()> {
    let Some(task) =
        crate::node_agent_local_task_durable_reconcile::reconcile_missing_sidecar_task(
            &runtime, &session,
        )
        .await?
    else {
        return Ok(());
    };
    if task.completion_event_id.is_some() {
        bind_existing_completion(&runtime, &task).await?;
        return Ok(());
    }
    if !recoverable_sidecar_task_status(&task.status) {
        return Ok(());
    }
    let output_path = session
        .endpoint
        .as_deref()
        .map(Path::new)
        .filter(|path| path.is_file());
    if runtime
        .update_recovery
        .receipt_for_task(&task.task_id)?
        .is_some_and(|receipt| !receipt.allows_local_reconcile())
    {
        return Ok(());
    }

    let terminal_output = output_path
        .map(output_contains_terminal_record)
        .transpose()?
        .unwrap_or(false);
    if !terminal_output {
        let update_receipt = runtime.update_recovery.receipt_for_task(&task.task_id)?;
        if update_receipt.is_some() {
            // update_reconcile owns the receipt transition and attaches the
            // same shared monitor after it establishes runtime-online state.
            return Ok(());
        }
        let admission = supervised_admission(&runtime, &task)?;
        let live_runtime_handle = runtime
            .active_cli_prompt_view(&task.task_id)
            .await
            .is_some_and(|handle| handle.control_handle_live);
        let now = now_ms();
        let current = runtime
            .cli_sidecars
            .session_for_task(&task.task_id)?
            .context("surviving sidecar disappeared during startup admission")?;
        if live_runtime_handle {
            info!(task_id = %task.task_id, "sidecar 仍在启动宽限、心跳或 live handle 下，保持运行/恢复状态");
            return Ok(());
        }
        let terminal_arrived = current
            .endpoint
            .as_deref()
            .map(Path::new)
            .filter(|path| path.is_file())
            .map(output_contains_terminal_record)
            .transpose()?
            .unwrap_or(false);
        if terminal_arrived || current.can_replay_after_restart_at(now) {
            spawn_recovered_sidecar_monitor(runtime, task, current, None, admission.as_ref())
                .await?;
            return Ok(());
        }
        if current.protects_startup_reconcile_at(now) {
            info!(task_id = %task.task_id, "sidecar 仍在启动宽限或心跳下，但进程身份尚不可重接；保持 recoverable");
            return Ok(());
        }
        let receipt = ensure_recovery_receipt(&runtime, &task, &current)?;
        if !stale_transition_evidence_complete(false, false, receipt.as_ref()) {
            warn!(task_id = %task.task_id, "stale sidecar evidence is incomplete; preserving recoverable state");
            return Ok(());
        }
        move_stale_sidecar_to_resume_required(&runtime, &task, &current, receipt.as_ref())?;
        return Ok(());
    }

    let _receipt = ensure_recovery_receipt(&runtime, &task, &session)?;

    if bind_existing_completion(&runtime, &task).await? {
        return Ok(());
    }
    let output_path = output_path.context("sidecar 终态输出文件缺失")?;
    prepare_receipt_for_terminal_replay(&runtime, &task.task_id)?;
    let (cancel_tx, mut cancel_rx) = watch::channel(false);
    let _cancel_tx = cancel_tx;
    let receipt_for_cursor = runtime.update_recovery.receipt_for_task(&task.task_id)?;
    let initial_cursor = CliSidecarReplayCursor::default();
    let session_id = session.session_id.clone();
    let result = crate::node_agent_cli_sidecar_runner::follow_sidecar_output_from_with_batch(
        &runtime.cli_sidecars,
        &task.task_id,
        output_path,
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
                    if let Some(receipt) = receipt_for_cursor.as_ref() {
                        runtime.update_recovery.record_sidecar_cursor(
                            &receipt.update_id,
                            &receipt.original_task_id,
                            cursor.offset,
                            cursor.sequence,
                        )?;
                    }
                    Ok(())
                },
            )
        },
    )
    .await?;

    let exit_ok = result.exit_ok && !result.canceled && result.terminal_error.is_none();
    let error = if exit_ok {
        None
    } else {
        result.terminal_error.clone().or_else(|| {
            Some(if result.canceled {
                "任务在节点更新恢复期间被取消".to_string()
            } else {
                "CLI sidecar 已退出并返回失败".to_string()
            })
        })
    };
    let (stdout, stderr) = crate::node_agent_sidecar_recovery_replay::recovered_completion_output(
        &runtime.task_journal,
        &task.task_id,
        output_path,
        200_000,
    )?;
    let codex_terminal = codex_terminal_outcome(&stdout);
    if task.cli.eq_ignore_ascii_case("codex") && codex_terminal.is_none() {
        let reason = "sidecar 已退出，但缺少 Codex turn.completed/turn.failed 可靠终态；保留隔离工作区并等待 Resume";
        let _ = runtime
            .local_tasks
            .mark_recovery_blocked(&task.task_id, reason)?;
        if let Some(receipt) = runtime.update_recovery.receipt_for_task(&task.task_id)? {
            runtime.update_recovery.update(
                &receipt.update_id,
                &receipt.original_task_id,
                |current| {
                    current.transition(UpdateRecoveryState::Paused, Some(reason))?;
                    current.resume_strategy =
                        Some("resume_required_after_non_terminal_replay".to_string());
                    Ok(())
                },
            )?;
        }
        return Ok(());
    }
    let exit_ok = match codex_terminal {
        Some(false) => false,
        _ => exit_ok,
    };
    let (done, combined_output) = cli_done_message_from_output(
        task.task_id.clone(),
        exit_ok,
        error,
        &stdout,
        &stderr,
        None,
        recovered_workspace_status(&runtime, &task, &session),
        runtime
            .task_journal
            .snapshot(&task.task_id, 0, 1)?
            .record
            .and_then(|record| record.codex_session_id),
    );
    let (out_tx, _out_rx) = tokio::sync::mpsc::unbounded_channel::<Message>();
    match persist_and_send_cli_done(
        &runtime,
        &completion_context(&task),
        &task.cli,
        Some(&combined_output),
        done,
        &out_tx,
    )
    .await
    {
        Ok(completion) => {
            info!(
                task_id = %task.task_id,
                event_id = %completion.event_id,
                "已从 surviving sidecar 原子补写本机任务终态"
            );
        }
        Err(first_error) => {
            if !bind_existing_completion(&runtime, &task).await? {
                return Err(first_error).context("持久化 sidecar 恢复终态");
            }
        }
    }
    Ok(())
}

fn stale_transition_evidence_complete(
    live_runtime_handle: bool,
    protected_sidecar: bool,
    receipt: Option<&UpdateRecoveryReceipt>,
) -> bool {
    !live_runtime_handle
        && !protected_sidecar
        && receipt.is_some_and(|receipt| receipt.safety.evidence_complete)
}

fn move_stale_sidecar_to_resume_required(
    runtime: &NodeRuntime,
    task: &LocalTaskRecord,
    session: &CliSidecarSessionRecord,
    receipt: Option<&UpdateRecoveryReceipt>,
) -> Result<()> {
    let reason = "sidecar 与 CLI 子进程均已退出且没有可靠 exit 记录；工作区、journal、root lease 与恢复游标已保留，请使用 Resume 继续";
    let root_task_id = receipt
        .map(|value| value.root_task_id.clone())
        .or_else(|| {
            load_supervision_contract(&runtime.task_journal, &task.task_id)
                .ok()
                .flatten()
                .and_then(|contract| contract.root_task_id)
        })
        .unwrap_or_else(|| task.task_id.clone());
    let context = serde_json::json!({
        "state": "resume_required",
        "reason": "stale_sidecar_without_exit_record",
        "root_task_id": root_task_id,
        "sidecar_session_id": session.session_id,
        "sidecar_pid": session.sidecar_pid,
        "child_pid": session.child_pid,
        "sidecar_last_seen_at_ms": session.last_seen_at_ms,
        "journal_preserved": true,
        "workspace_preserved": true,
        "root_lease_preserved": true,
        "sidecar_output_offset": session.output_offset,
        "sidecar_output_sequence": session.output_sequence,
    });
    let transitioned =
        runtime
            .local_tasks
            .mark_stale_sidecar_resume_required(&task.task_id, reason, &context)?;
    if !transitioned {
        return Ok(());
    }
    runtime
        .cli_sidecars
        .mark_task_resume_required(&task.task_id)?;
    crate::node_agent_local_task_supervision::record_supervision_event(
        &runtime.task_journal,
        &task.task_id,
        "supervision_stale_sidecar_resume_required",
        context,
    )?;
    if let Some(receipt) = receipt {
        runtime.update_recovery.update(
            &receipt.update_id,
            &receipt.original_task_id,
            |current| {
                if !current.state.is_terminal() {
                    current.transition(UpdateRecoveryState::Paused, Some(reason))?;
                    current.resume_strategy = Some("resume_required_stale_sidecar".to_string());
                }
                Ok(())
            },
        )?;
    }
    info!(task_id = %task.task_id, "moved stale persisted sidecar task to resume_required");
    Ok(())
}

pub(crate) fn output_contains_terminal_record(path: &Path) -> Result<bool> {
    let mut offset = 0;
    Ok(read_new_output_records(path, &mut offset)?
        .iter()
        .any(|record| record.record_type == "exit"))
}

fn codex_terminal_outcome(output: &str) -> Option<bool> {
    match crate::node_agent_cli_sidecar_runner::codex_completion_disposition(output) {
        crate::node_agent_cli_sidecar_runner::CodexCompletionDisposition::Complete { .. } => {
            Some(true)
        }
        crate::node_agent_cli_sidecar_runner::CodexCompletionDisposition::Failed => Some(false),
        crate::node_agent_cli_sidecar_runner::CodexCompletionDisposition::ResumeRequired => None,
    }
}

fn ensure_recovery_receipt(
    runtime: &NodeRuntime,
    task: &LocalTaskRecord,
    session: &CliSidecarSessionRecord,
) -> Result<Option<UpdateRecoveryReceipt>> {
    if let Some(receipt) = runtime.update_recovery.receipt_for_task(&task.task_id)? {
        if receipt_targets_release(&receipt, &crate::node_agent_release_identity::current())
            || task.status != "resume_required"
        {
            return Ok(Some(receipt));
        }
    }
    let Some(contract) = load_supervision_contract(&runtime.task_journal, &task.task_id)? else {
        return Ok(None);
    };
    if contract.protocol != SUPERVISION_PROTOCOL {
        return Ok(None);
    }
    let snapshot = runtime.task_journal.snapshot(&task.task_id, 0, 200)?;
    let original_task_id = contract
        .parent_task_id
        .as_deref()
        .filter(|_| contract.task_role == "resume_original")
        .unwrap_or(&task.task_id);
    let root_task_id = contract.root_task_id.as_deref().unwrap_or(original_task_id);
    let current_release = crate::node_agent_release_identity::current();
    let mut receipt = UpdateRecoveryReceipt::planned(
        format!(
            "legacy-sidecar-{}-{}",
            session.session_id,
            current_release.replace(['+', ':', '/', '\\'], "_")
        ),
        root_task_id,
        original_task_id,
    );
    if contract.task_role == "resume_original" {
        receipt.resume_task_id = Some(task.task_id.clone());
    }
    receipt.parent_task_id = contract.parent_task_id;
    receipt.from_release = release_identity(&current_release);
    receipt.to_release = receipt.from_release.clone();
    receipt.codex_session_id = snapshot
        .record
        .as_ref()
        .and_then(|record| record.codex_session_id.clone());
    receipt.sidecar_session_id = Some(session.session_id.clone());
    receipt.sidecar_output_offset = session.output_offset;
    receipt.sidecar_output_sequence = session.output_sequence;
    receipt.journal_cursor = snapshot.last_event_seq as u64;
    let cwd = session
        .cwd
        .as_deref()
        .or_else(|| {
            snapshot
                .record
                .as_ref()
                .and_then(|record| record.cwd.as_deref())
        })
        .unwrap_or(&task.workspace_path);
    receipt.workspace = fingerprint_workspace(Path::new(cwd));
    preserve_platform_workspace_identity(&mut receipt.workspace, task.workspace_status.as_ref());
    receipt.safety.pending_approval_ids = snapshot.approvals.pending_approval_ids();
    receipt.safety.non_repeatable_action = incomplete_non_repeatable_action(&snapshot.events);
    receipt.safety.journal_event_count = snapshot.last_event_seq;
    receipt.safety.evidence_complete = snapshot.record.is_some()
        && receipt.workspace.has_sufficient_identity()
        && receipt.safety.pending_approval_ids.is_empty()
        && receipt.safety.non_repeatable_action.is_none();
    receipt.transition(
        UpdateRecoveryState::Downloaded,
        Some("legacy sidecar discovered"),
    )?;
    receipt.transition(
        UpdateRecoveryState::CheckpointSaved,
        Some("legacy sidecar output and journal persisted"),
    )?;
    receipt.transition(
        UpdateRecoveryState::Applying,
        Some(crate::node_agent_update_recovery::LEGACY_SNAPSHOT_APPLYING_REASON),
    )?;
    runtime.update_recovery.upsert(receipt.clone())?;
    Ok(Some(receipt))
}

fn recoverable_sidecar_task_status(status: &str) -> bool {
    matches!(
        status,
        "running" | "recovering" | "interrupted" | "resume_required" | "cancel_requested"
    )
}

fn receipt_targets_release(receipt: &UpdateRecoveryReceipt, current: &str) -> bool {
    let target = &receipt.to_release;
    let current = release_identity(current);
    (target.version.trim().is_empty() || target.version == current.version)
        && (target.git_sha.trim().is_empty() || target.git_sha == current.git_sha)
}

fn release_identity(current: &str) -> ReleaseIdentity {
    let current = current.trim();
    let (version, git_sha) = current.rsplit_once('+').unwrap_or((current, ""));
    ReleaseIdentity {
        version: version.to_string(),
        git_sha: git_sha.to_string(),
    }
}

fn prepare_receipt_for_terminal_replay(runtime: &NodeRuntime, task_id: &str) -> Result<()> {
    let Some(receipt) = runtime.update_recovery.receipt_for_task(task_id)? else {
        return Ok(());
    };
    runtime
        .update_recovery
        .update(&receipt.update_id, &receipt.original_task_id, |current| {
            loop {
                let next = match current.state {
                    UpdateRecoveryState::Planned => UpdateRecoveryState::Downloaded,
                    UpdateRecoveryState::Downloaded => UpdateRecoveryState::CheckpointSaved,
                    UpdateRecoveryState::CheckpointSaved => UpdateRecoveryState::Applying,
                    UpdateRecoveryState::Applying => UpdateRecoveryState::RuntimeOnline,
                    UpdateRecoveryState::Paused
                    | UpdateRecoveryState::ApprovalRequired
                    | UpdateRecoveryState::Conflict
                    | UpdateRecoveryState::Timeout => UpdateRecoveryState::RuntimeOnline,
                    UpdateRecoveryState::RuntimeOnline => UpdateRecoveryState::Reattaching,
                    UpdateRecoveryState::Reattaching | UpdateRecoveryState::ResumeCreated => {
                        UpdateRecoveryState::Resumed
                    }
                    _ => break,
                };
                current.transition(next, Some("sidecar terminal replay"))?;
            }
            current.resume_strategy = Some("sidecar_terminal_replay".to_string());
            Ok(())
        })
}

async fn bind_existing_completion(runtime: &NodeRuntime, task: &LocalTaskRecord) -> Result<bool> {
    let Some(completion) = runtime.completion_outbox.latest_for_req_id(&task.task_id)? else {
        return Ok(false);
    };
    crate::node_agent_local_terminal_reconcile::LocalTerminalReconciler::from_runtime(runtime)
        .reconcile(&completion)
        .await?;
    Ok(true)
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

fn recovered_workspace_status(
    runtime: &NodeRuntime,
    task: &LocalTaskRecord,
    session: &CliSidecarSessionRecord,
) -> Option<CliWorkspaceStatus> {
    let active = session.cwd.as_deref()?.trim();
    if active.is_empty() {
        return None;
    }
    let receipt_workspace = runtime
        .update_recovery
        .receipt_for_task(&task.task_id)
        .ok()
        .flatten()
        .map(|receipt| receipt.workspace);
    let preserved = receipt_workspace
        .as_ref()
        .filter(|workspace| workspace.isolated);
    let isolated = preserved.is_some()
        || !crate::node_agent_update_checkpoint::same_path(
            Path::new(active),
            Path::new(&task.workspace_path),
        );
    Some(CliWorkspaceStatus {
        base_workspace_path: preserved
            .and_then(|workspace| workspace.base_workspace_path.clone())
            .or_else(|| isolated.then(|| task.workspace_path.clone())),
        active_workspace_path: active.to_string(),
        isolated,
        branch: preserved
            .and_then(|workspace| workspace.branch.clone())
            .or_else(|| {
                crate::node_agent_update_checkpoint::git_output(
                    Path::new(active),
                    &["branch", "--show-current"],
                )
            }),
        git_head: crate::node_agent_update_checkpoint::git_output(
            Path::new(active),
            &["rev-parse", "--verify", "HEAD^{commit}"],
        ),
        prepare_status: "recovered_from_sidecar".to_string(),
        merge_status: Some("preserved".to_string()),
        merge_message: Some("节点更新后保留原工作区并补写终态".to_string()),
    })
}

#[cfg(test)]
#[path = "node_agent_sidecar_recovery_tests.rs"]
mod tests;
