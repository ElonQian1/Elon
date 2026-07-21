//! Runtime reconciliation for the durable low-priority self-evolution queue.

use std::{sync::Arc, time::Duration};

use anyhow::{Context, Result};
use homecli_proto::{CancelRequestAudit, InterruptionSource};
use serde_json::Value;

use crate::{
    node_agent_local_task_supervision::{SupervisionContract, SUPERVISION_PROTOCOL},
    NodeRuntime,
};

use crate::node_agent_self_evolution::{now_ms, support::SelfEvolutionGates, SelfEvolutionItem};

const LOOP_INTERVAL: Duration = Duration::from_secs(3);

pub(crate) fn spawn_scheduler(runtime: Arc<NodeRuntime>) {
    tokio::spawn(async move {
        loop {
            if let Err(error) = reconcile(&runtime).await {
                tracing::warn!(%error, "self evolution scheduler reconcile failed safely");
            }
            tokio::time::sleep(LOOP_INTERVAL).await;
        }
    });
}

async fn reconcile(runtime: &Arc<NodeRuntime>) -> Result<()> {
    crate::node_agent_local_task_supervision::reconcile_pending_review_actions(runtime)?;
    apply_pending_actions(runtime).await?;
    runtime
        .self_evolution
        .reconcile_records(&runtime.local_tasks)?;
    let evolution_ids = runtime.self_evolution.active_task_ids();
    let active = runtime.active_cli_prompts.views_without_approvals().await;
    let foreground_task_ids = active
        .iter()
        .filter(|view| !evolution_ids.contains(&view.req_id))
        .map(|view| view.req_id.clone())
        .collect::<Vec<_>>();
    let publish = observe_publish(runtime).await;
    let update_phase = runtime.update_recovery.load()?.install_gate.phase;
    let update_active = matches!(
        update_phase.as_str(),
        "downloaded" | "checkpoint_saved" | "applying" | "deferred_active_foreground"
    );
    let data_paths = runtime.node_data_root.read().await.paths.clone();
    let resource_pressure = data_paths
        .as_ref()
        .is_some_and(|paths| crate::node_agent_build_runtime::active_leases(paths) > 1);
    runtime.self_evolution.update_gates(SelfEvolutionGates {
        foreground_task_ids,
        publish_active: publish.active,
        publish_status: publish.status,
        publish_owner: publish.owner,
        publish_waiter_count: publish.waiter_count,
        update_active,
        resource_pressure,
        checked_at_ms: now_ms(),
    })?;
    runtime.self_evolution.request_gate_pauses()?;
    apply_pending_actions(runtime).await?;
    if let Some(item) = runtime.self_evolution.reserve_next()? {
        if let Err(error) = dispatch_generation(runtime, &item).await {
            runtime
                .self_evolution
                .mark_dispatch_failed(&item.logical_id, &error.to_string())?;
        }
    }
    Ok(())
}

pub(super) async fn apply_pending_action(
    runtime: &Arc<NodeRuntime>,
    item: &SelfEvolutionItem,
) -> Result<SelfEvolutionItem> {
    let pending = item
        .pending_action
        .as_ref()
        .context("self evolution item has no pending action")?;
    if pending.action == "pause" {
        if let Some(task_id) = item.active_task_id.as_deref() {
            let reason = pending.note.as_deref().unwrap_or("manual_pause");
            let audit = CancelRequestAudit::now(&pending.actor, &pending.source, reason)
                .with_interruption_source(if item.yield_reason.as_deref() == Some("node_update") {
                    InterruptionSource::UpdaterApply
                } else {
                    InterruptionSource::SupervisorIntervention
                });
            if !runtime.cancel_cli_prompt_with_audit(task_id, &audit).await {
                let durable = runtime
                    .task_journal
                    .snapshot(task_id, 0, 1)?
                    .record
                    .is_some_and(|record| {
                        matches!(
                            record.status.as_str(),
                            "cancel_requested" | "canceled" | "failed" | "done"
                        )
                    })
                    || runtime.local_tasks.get(task_id)?.is_some_and(|record| {
                        matches!(
                            record.status.as_str(),
                            "cancel_requested" | "canceled" | "failed" | "done"
                        )
                    });
                anyhow::ensure!(
                    durable,
                    "self evolution durable pause/cancel failed closed for {task_id}"
                );
            }
        }
    } else if matches!(pending.action.as_str(), "approve" | "reject") {
        if let Some(task_id) = item.active_task_id.as_deref() {
            crate::node_agent_local_task_supervision::record_actor_review(
                runtime,
                task_id,
                if pending.action == "approve" {
                    "accepted"
                } else {
                    "rejected"
                },
                pending
                    .note
                    .as_deref()
                    .unwrap_or("self evolution queue review"),
                &pending.actor,
                &pending.source,
            )?;
        }
    }
    runtime
        .self_evolution
        .commit_action(&item.owner_user_id, &item.logical_id, &pending.action)
}

async fn apply_pending_actions(runtime: &Arc<NodeRuntime>) -> Result<()> {
    for item in runtime.self_evolution.pending_actions()? {
        apply_pending_action(runtime, &item).await?;
    }
    Ok(())
}

async fn dispatch_generation(runtime: &Arc<NodeRuntime>, item: &SelfEvolutionItem) -> Result<()> {
    let task_id = item
        .active_task_id
        .as_deref()
        .context("reserved evolution generation has no task id")?;
    let frozen =
        crate::node_agent_codex_child_env::FrozenCodexHome::capture_unmanaged_for_local_task()?;
    let contract = SupervisionContract {
        protocol: SUPERVISION_PROTOCOL.to_string(),
        supervisor: "codex_desktop".to_string(),
        task_role: "post_task_improvement".to_string(),
        parent_task_id: Some(item.parent_task_id.clone()),
        root_task_id: Some(item.root_task_id.clone()),
        acceptance_criteria: vec![
            "run independently after the user task releases its resources".to_string(),
            "yield to foreground, publish, update, and resource pressure and require review"
                .to_string(),
        ],
        improvement_policy: "after_task_only".to_string(),
    };
    let data_paths = runtime.node_data_root.read().await.paths.clone();
    let continuation = if item.generation > 1 {
        format!("Low-priority self evolution is automatically resuming generation {} after yielding. Inspect the existing isolated worktree and durable journal before continuing; do not redo completed or non-repeatable actions.\n\n{}", item.generation, item.prompt)
    } else {
        item.prompt.clone()
    };
    let (record, execution_workspace) =
        crate::node_agent_local_tasks::provision_record_and_dispatch_supervised_task(
            data_paths.as_ref(),
            &runtime.local_tasks,
            &runtime.task_journal,
            crate::node_agent_local_tasks::SupervisedLocalTaskProvision {
                task_id,
                owner_user_id: &item.owner_user_id,
                agent_id: &item.agent_id,
                install_id: &item.install_id,
                project_id: &item.project_id,
                channel_id: item.channel_id.as_deref(),
                conversation_id: &item.conversation_id,
                base_workspace_path: &item.workspace_path,
                prompt: &item.prompt,
                runtime_permission: &item.runtime_permission,
                root_task_id: &item.root_task_id,
                contract: &contract,
            },
            |record, workspace| {
                runtime
                    .self_evolution
                    .record_execution_worktree(&item.logical_id, workspace)?;
                crate::node_agent_local_tasks::dispatch_local_task_record(
                    runtime.clone(),
                    record,
                    crate::node_agent_local_task_supervision::executor_prompt(
                        &continuation,
                        Some(&contract),
                    ),
                    workspace.workspace_path.clone(),
                    Some(&contract),
                    Some(workspace.clone()),
                    None,
                    None,
                    frozen.clone(),
                );
                Ok(())
            },
        )?;
    anyhow::ensure!(record.workspace_path == execution_workspace.workspace_path);
    Ok(())
}

#[derive(Default)]
struct PublishObservation {
    active: bool,
    status: String,
    owner: Option<String>,
    waiter_count: usize,
}

async fn observe_publish(runtime: &NodeRuntime) -> PublishObservation {
    let url = format!(
        "{}/api/release/status",
        runtime.cloud_http_url().trim_end_matches('/')
    );
    let response = reqwest::Client::new()
        .get(url)
        .timeout(Duration::from_secs(2))
        .send()
        .await;
    let Ok(response) = response else {
        return PublishObservation {
            status: "unavailable".to_string(),
            ..Default::default()
        };
    };
    let Ok(body) = response.json::<Value>().await else {
        return PublishObservation {
            status: "invalid".to_string(),
            ..Default::default()
        };
    };
    let global = body.get("globalPublish").cloned().unwrap_or(Value::Null);
    let owner = global
        .get("owner")
        .filter(|value| !value.is_null())
        .map(|value| {
            let kind = value
                .get("kind")
                .and_then(Value::as_str)
                .unwrap_or("publish");
            let builder = value
                .get("builderLabel")
                .and_then(Value::as_str)
                .unwrap_or("unknown");
            format!("{kind}:{builder}")
        });
    let waiter_count = global
        .get("waiterCount")
        .and_then(Value::as_u64)
        .unwrap_or(0) as usize;
    PublishObservation {
        active: owner.is_some(),
        status: "observed".to_string(),
        owner,
        waiter_count,
    }
}
