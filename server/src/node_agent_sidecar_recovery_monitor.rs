//! Shared live-sidecar ownership and terminal persistence after a node restart.

use std::{path::Path, sync::Arc};

use anyhow::{Context, Result};
use tokio::sync::watch;
use tokio_tungstenite::tungstenite::Message;
use tracing::warn;

use crate::{
    node_agent_active_task::ActiveCliPromptHandle,
    node_agent_cli_sidecar::CliSidecarSessionRecord,
    node_agent_cli_sidecar_runner::CliSidecarReplayCursor,
    node_agent_local_task_store::LocalTaskRecord,
    node_agent_local_task_supervision::load_supervision_contract,
    node_agent_update_recovery::{UpdateRecoveryReceipt, UpdateRecoveryState},
    NodeRuntime,
};

pub(crate) async fn spawn_recovered_sidecar_monitor(
    runtime: Arc<NodeRuntime>,
    task: LocalTaskRecord,
    sidecar: CliSidecarSessionRecord,
    update_receipt: Option<UpdateRecoveryReceipt>,
    admission: Option<&crate::node_agent_supervision_worktree_lease::ResumeAdmissionGuard>,
) -> Result<bool> {
    let (cancel_tx, mut cancel_rx) = watch::channel(false);
    let handle = ActiveCliPromptHandle::new(
        &task.task_id,
        &task.cli,
        &sidecar.route,
        sidecar.cwd.clone(),
        Some(task.runtime_permission.clone()),
        cancel_tx,
    )
    .with_exclusive_workspace(is_platform_supervised(&task));
    let registration = if is_platform_supervised(&task) {
        runtime
            .try_register_supervised_cli_prompt(handle, admission)
            .await?
    } else {
        runtime.try_register_cli_prompt(handle).await
    };
    if registration != crate::node_agent_active_task_registry::CliPromptRegistration::Inserted {
        return Ok(false);
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
    tokio::spawn(async move {
        let initial_cursor = CliSidecarReplayCursor {
            offset: update_receipt
                .as_ref()
                .map(|receipt| receipt.sidecar_output_offset)
                .unwrap_or(sidecar.output_offset),
            sequence: update_receipt
                .as_ref()
                .map(|receipt| receipt.sidecar_output_sequence)
                .unwrap_or(sidecar.output_sequence),
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
                        crate::node_agent_sidecar_recovery_replay::record_replayed_activity(
                            &runtime.task_journal,
                            &runtime.local_tasks,
                            &task.task_id,
                            records,
                        )?;
                        runtime.cli_sidecars.record_output_cursor(
                            &task.task_id,
                            &session_id,
                            cursor.offset,
                            cursor.sequence,
                        )?;
                        if let Some(receipt) = update_receipt.as_ref() {
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
        .await;
        match result {
            Ok(result) => {
                if let Err(error) = persist_recovered_terminal(
                    &runtime,
                    &task,
                    &sidecar,
                    &output_path,
                    result,
                    update_receipt.as_ref(),
                )
                .await
                {
                    warn!(%error, task_id = %task.task_id, "durable sidecar terminal reconciliation remains retryable");
                }
            }
            Err(error) => {
                if let Some(receipt) = update_receipt.as_ref() {
                    let _ = runtime.update_recovery.update(
                        &receipt.update_id,
                        &receipt.original_task_id,
                        |current| {
                            current.transition(
                                UpdateRecoveryState::Paused,
                                Some(&format!("sidecar replay failed: {error}")),
                            )?;
                            Ok(())
                        },
                    );
                }
                warn!(%error, task_id = %task.task_id, "sidecar recovery monitor stopped before a trusted terminal event");
            }
        }
        runtime.finish_cli_prompt(&task.task_id).await;
    });
    Ok(true)
}

async fn persist_recovered_terminal(
    runtime: &NodeRuntime,
    task: &LocalTaskRecord,
    sidecar: &CliSidecarSessionRecord,
    output_path: &Path,
    result: crate::node_agent_cli_sidecar_runner::CliSidecarRunResult,
    update_receipt: Option<&UpdateRecoveryReceipt>,
) -> Result<()> {
    if super::bind_existing_completion(runtime, task).await? {
        return Ok(());
    }
    let (stdout, stderr) = crate::node_agent_sidecar_recovery_replay::recovered_completion_output(
        &runtime.task_journal,
        &task.task_id,
        output_path,
        200_000,
    )
    .unwrap_or_else(|error| {
        warn!(%error, task_id = %task.task_id, "failed to merge recovery transcript");
        (result.stdout_text.clone(), result.stderr_text.clone())
    });
    let codex_terminal = super::codex_terminal_outcome(&stdout);
    if task.cli.eq_ignore_ascii_case("codex") && codex_terminal.is_none() {
        anyhow::bail!("sidecar exited without a Codex turn terminal event");
    }
    let success = codex_terminal
        .unwrap_or(result.exit_ok && !result.canceled && result.terminal_error.is_none());
    let error = if success {
        None
    } else {
        result.terminal_error.clone().or_else(|| {
            Some(if result.canceled {
                "任务在节点恢复期间被取消".to_string()
            } else {
                "CLI sidecar 恢复后返回失败".to_string()
            })
        })
    };
    let workspace = update_receipt.and_then(|receipt| {
        crate::node_agent_update_reconcile::recovered_workspace(
            task,
            &receipt.workspace,
            &receipt.root_task_id,
        )
    });
    let (success, error, workspace_status) =
        crate::node_agent_cli_runner::finalize_cli_prompt_workspace(success, error, workspace);
    let workspace_status =
        workspace_status.or_else(|| super::recovered_workspace_status(runtime, task, sidecar));
    let session_id = runtime
        .task_journal
        .snapshot(&task.task_id, 0, 1)
        .ok()
        .and_then(|snapshot| snapshot.record)
        .and_then(|record| record.codex_session_id);
    let (done, output) = crate::node_agent_cli_done::cli_done_message_from_output(
        task.task_id.clone(),
        success,
        error,
        &stdout,
        &stderr,
        None,
        workspace_status,
        session_id,
    );
    let (out_tx, _out_rx) = tokio::sync::mpsc::unbounded_channel::<Message>();
    crate::node_agent_cli_done::persist_and_send_cli_done(
        runtime,
        &super::completion_context(task),
        &task.cli,
        Some(&output),
        done,
        &out_tx,
    )
    .await?;
    Ok(())
}

pub(super) fn supervised_admission(
    runtime: &NodeRuntime,
    task: &LocalTaskRecord,
) -> Result<Option<crate::node_agent_supervision_worktree_lease::ResumeAdmissionGuard>> {
    if !is_platform_supervised(task) {
        return Ok(None);
    }
    let contract = load_supervision_contract(&runtime.task_journal, &task.task_id)?
        .context("surviving supervised sidecar has no durable contract")?;
    let base = crate::node_agent_supervision_terminal_lease_safety::admission_base(
        task,
        &contract,
        &task.task_id,
    )?;
    crate::node_agent_supervision_worktree_lease::ResumeAdmissionGuard::acquire(&base).map(Some)
}

fn is_platform_supervised(task: &LocalTaskRecord) -> bool {
    task.workspace_status
        .as_ref()
        .and_then(|status| status.get("platform_provenance"))
        .and_then(serde_json::Value::as_str)
        == Some("elon.conversation_worktree.v1")
}
