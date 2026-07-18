//! Startup repair for sidecars that outlive the node runtime.
//!
//! The immutable worker owns CLI output capture. This module consumes its
//! durable JSONL after a runtime restart and commits exactly one completion
//! envelope before repairing the local task row and journal terminal state.

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
    node_agent_task_journal_events::completion_terminal_status,
    node_agent_update_checkpoint::{fingerprint_workspace, incomplete_non_repeatable_action},
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
    let Some(task) = runtime.local_tasks.get(&session.task_id)? else {
        return Ok(());
    };
    if task.completion_event_id.is_some() {
        bind_existing_completion(&runtime, &task).await?;
        return Ok(());
    }
    if !matches!(
        task.status.as_str(),
        "running" | "recovering" | "interrupted"
    ) {
        return Ok(());
    }
    let Some(output_path) = session.endpoint.as_deref().map(Path::new) else {
        return Ok(());
    };
    if !output_path.is_file() {
        return Ok(());
    }
    if runtime
        .update_recovery
        .receipt_for_task(&task.task_id)?
        .is_some_and(|receipt| !receipt.allows_local_reconcile())
    {
        return Ok(());
    }

    let terminal_output = output_contains_terminal_record(output_path)?;
    let receipt = ensure_recovery_receipt(&runtime, &task, &session)?;
    if !terminal_output {
        if session.is_live_at(now_ms()) && receipt.is_some() {
            info!(task_id = %task.task_id, "发现更新后仍存活的 sidecar，交由恢复事务重接");
        }
        return Ok(());
    }

    if bind_existing_completion(&runtime, &task).await? {
        return Ok(());
    }
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
    let (done, combined_output) = cli_done_message_from_output(
        task.task_id.clone(),
        exit_ok,
        error,
        &stdout,
        &stderr,
        None,
        recovered_workspace_status(&task, &session),
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
    ) {
        Ok(completion) => {
            record_recovery_terminal(&runtime, &task.task_id, &completion)?;
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

fn output_contains_terminal_record(path: &Path) -> Result<bool> {
    let mut offset = 0;
    Ok(read_new_output_records(path, &mut offset)?
        .iter()
        .any(|record| record.record_type == "exit"))
}

fn ensure_recovery_receipt(
    runtime: &NodeRuntime,
    task: &LocalTaskRecord,
    session: &CliSidecarSessionRecord,
) -> Result<Option<UpdateRecoveryReceipt>> {
    if let Some(receipt) = runtime.update_recovery.receipt_for_task(&task.task_id)? {
        return Ok(Some(receipt));
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
    let mut receipt = UpdateRecoveryReceipt::planned(
        format!("legacy-sidecar-{}", session.session_id),
        root_task_id,
        original_task_id,
    );
    if contract.task_role == "resume_original" {
        receipt.resume_task_id = Some(task.task_id.clone());
    }
    receipt.parent_task_id = contract.parent_task_id;
    receipt.from_release = ReleaseIdentity {
        version: crate::node_agent_release_identity::current(),
        git_sha: String::new(),
    };
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
        Some("runtime restarted before parent completion commit"),
    )?;
    runtime.update_recovery.upsert(receipt.clone())?;
    Ok(Some(receipt))
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
    runtime.local_tasks.reconcile_completion(&completion)?;
    runtime.task_journal.record_finished_with_outcome(
        &task.task_id,
        completion_terminal_status(completion.exit_ok, completion.error.as_deref()),
        completion.error.as_deref(),
    )?;
    record_recovery_terminal(runtime, &task.task_id, &completion)?;
    Ok(true)
}

fn record_recovery_terminal(
    runtime: &NodeRuntime,
    task_id: &str,
    completion: &homecli_proto::CliCompletionEnvelope,
) -> Result<()> {
    if let Some(receipt) = runtime.update_recovery.receipt_for_task(task_id)? {
        runtime.update_recovery.record_terminal_binding(
            task_id,
            &completion.event_id,
            completion_terminal_status(completion.exit_ok, completion.error.as_deref()),
            completion.created_at_ms as u128,
        )?;
        runtime.update_recovery.update(
            &receipt.update_id,
            &receipt.original_task_id,
            |current| {
                if !current.state.is_terminal() {
                    current.transition(
                        if completion.exit_ok {
                            UpdateRecoveryState::Verified
                        } else {
                            UpdateRecoveryState::Failed
                        },
                        Some("durable sidecar completion bound to local task"),
                    )?;
                }
                Ok(())
            },
        )?;
    }
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

fn recovered_workspace_status(
    task: &LocalTaskRecord,
    session: &CliSidecarSessionRecord,
) -> Option<CliWorkspaceStatus> {
    let active = session.cwd.as_deref()?.trim();
    if active.is_empty() {
        return None;
    }
    let isolated = !crate::node_agent_update_checkpoint::same_path(
        Path::new(active),
        Path::new(&task.workspace_path),
    );
    Some(CliWorkspaceStatus {
        base_workspace_path: isolated.then(|| task.workspace_path.clone()),
        active_workspace_path: active.to_string(),
        isolated,
        branch: crate::node_agent_update_checkpoint::git_output(
            Path::new(active),
            &["branch", "--show-current"],
        ),
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
