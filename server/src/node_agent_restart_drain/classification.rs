use serde_json::json;

use crate::NodeRuntime;

const ACTIVE_HANDLE_STALE_AFTER_MS: u128 = 2 * 60 * 1_000;

#[derive(Default)]
pub(super) struct DrainClassification {
    pub(super) blocking: Vec<String>,
    pub(super) recoverable: Vec<String>,
    pub(super) stale: Vec<String>,
}

#[derive(Debug, PartialEq, Eq)]
pub(super) enum DrainTaskDisposition {
    Blocking,
    Recoverable,
    Stale,
}

pub(super) fn drain_task_disposition(
    durable_running: bool,
    cancel_requested: bool,
    handle_fresh: bool,
    replayable_sidecar: bool,
    safe_receipt: bool,
) -> DrainTaskDisposition {
    if !durable_running {
        DrainTaskDisposition::Stale
    } else if replayable_sidecar || safe_receipt {
        DrainTaskDisposition::Recoverable
    } else if cancel_requested {
        // Cancellation is its own durable state machine. Never rewrite it to
        // Resume merely because the executor heartbeat became stale.
        DrainTaskDisposition::Blocking
    } else if handle_fresh {
        DrainTaskDisposition::Blocking
    } else {
        DrainTaskDisposition::Stale
    }
}

pub(super) async fn classify_supervised_tasks(
    runtime: &NodeRuntime,
) -> anyhow::Result<DrainClassification> {
    let active = runtime.active_cli_prompts.views_without_approvals().await;
    let active = active
        .into_iter()
        .map(|task| (task.req_id.clone(), task))
        .collect::<std::collections::HashMap<_, _>>();
    let now = super::now_ms();
    let mut result = DrainClassification::default();
    let tasks = load_drain_candidates(&runtime.local_tasks)?;
    let mut seen = std::collections::HashSet::new();
    for task in tasks {
        seen.insert(task.task_id.clone());
        let supervised = crate::node_agent_local_task_supervision::load_supervision_state(
            &runtime.task_journal,
            &task.task_id,
        )?
        .enabled;
        if !supervised {
            continue;
        }
        let durable_running = matches!(
            task.status.as_str(),
            "running" | "recovering" | "reattaching" | "cancel_requested"
        );
        let handle_fresh = active.get(&task.task_id).is_some_and(|handle| {
            handle.control_handle_live
                && now.saturating_sub(handle.last_heartbeat_ms) <= ACTIVE_HANDLE_STALE_AFTER_MS
        });
        let sidecar = runtime.cli_sidecars.session_for_task(&task.task_id)?;
        let replayable_sidecar = sidecar
            .as_ref()
            .is_some_and(|sidecar| sidecar.can_replay_after_restart_at(now));
        let safe_receipt = runtime
            .update_recovery
            .receipt_for_task(&task.task_id)?
            .is_some_and(|receipt| {
                receipt.safety.evidence_complete
                    && matches!(
                        receipt.state,
                        crate::node_agent_update_recovery::UpdateRecoveryState::Reattaching
                            | crate::node_agent_update_recovery::UpdateRecoveryState::ResumeCreated
                            | crate::node_agent_update_recovery::UpdateRecoveryState::Resumed
                    )
            });
        match drain_task_disposition(
            durable_running,
            task.status == "cancel_requested",
            handle_fresh,
            replayable_sidecar,
            safe_receipt,
        ) {
            DrainTaskDisposition::Stale => {
                let context = json!({
                    "state": "resume_required",
                    "reason": "stale_runtime_and_sidecar",
                    "sidecar_session_id": sidecar.as_ref().map(|value| value.session_id.as_str()),
                    "sidecar_pid": sidecar.as_ref().and_then(|value| value.sidecar_pid),
                    "child_pid": sidecar.as_ref().and_then(|value| value.child_pid),
                    "journal_preserved": true,
                    "workspace_preserved": true,
                    "root_lease_preserved": true,
                });
                let reason = "监督任务没有活动运行句柄，且记录的 sidecar/CLI 进程均不存活；现场已保留并转入 Resume";
                let transitioned = runtime.local_tasks.mark_stale_sidecar_resume_required(
                    &task.task_id,
                    reason,
                    &context,
                )?;
                if transitioned {
                    runtime
                        .cli_sidecars
                        .mark_task_resume_required(&task.task_id)?;
                    runtime.active_cli_prompts.remove(&task.task_id).await;
                    crate::node_agent_local_task_supervision::record_supervision_event(
                        &runtime.task_journal,
                        &task.task_id,
                        "supervision_stale_runtime_resume_required",
                        context,
                    )?;
                    result.stale.push(task.task_id.clone());
                } else {
                    // A concurrent durable transition is not proof that the
                    // task is safe to drain. Re-read on the next pass.
                    result.blocking.push(task.task_id.clone());
                }
            }
            DrainTaskDisposition::Recoverable => result.recoverable.push(task.task_id),
            DrainTaskDisposition::Blocking => result.blocking.push(task.task_id),
        }
    }
    // A missing durable row must never turn an otherwise active supervised
    // handle into an empty blocking set. Journal read failures also propagate.
    for task in active.values().filter(|task| !seen.contains(&task.req_id)) {
        if crate::node_agent_local_task_supervision::load_supervision_state(
            &runtime.task_journal,
            &task.req_id,
        )?
        .enabled
        {
            result.blocking.push(task.req_id.clone());
        }
    }
    result.blocking.sort();
    result.recoverable.sort();
    result.stale.sort();
    Ok(result)
}

pub(super) fn load_drain_candidates(
    store: &crate::node_agent_local_task_store::LocalTaskStore,
) -> anyhow::Result<Vec<crate::node_agent_local_task_store::LocalTaskRecord>> {
    store
        .list_update_candidates()
        .map_err(|error| anyhow::anyhow!("durable supervised task query failed: {error:#}"))
}
