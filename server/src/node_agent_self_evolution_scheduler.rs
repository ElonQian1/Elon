//! Runtime reconciliation for the durable low-priority self-evolution queue.

use std::{sync::Arc, time::Duration};

use anyhow::{Context, Result};
use homecli_proto::CancelRequestAudit;
use serde_json::Value;

use crate::{
    node_agent_local_task_store::LocalTaskStart,
    node_agent_local_task_supervision::{SupervisionContract, SUPERVISION_PROTOCOL},
    NodeRuntime,
};

use crate::node_agent_self_evolution::{now_ms, SelfEvolutionGates, SelfEvolutionItem};

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
    for (task_id, reason) in runtime.self_evolution.request_gate_pauses()? {
        let audit = CancelRequestAudit::now(
            "node_agent",
            "self_evolution_scheduler",
            format!("yield_for_{reason}"),
        );
        let _ = runtime.cancel_cli_prompt_with_audit(&task_id, &audit).await;
    }
    if let Some(item) = runtime.self_evolution.reserve_next()? {
        if let Err(error) = dispatch_generation(runtime, &item).await {
            runtime
                .self_evolution
                .mark_dispatch_failed(&item.logical_id, &error.to_string())?;
        }
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
    crate::node_agent_local_task_supervision::record_supervision_event(
        &runtime.task_journal,
        task_id,
        "supervision_contract",
        crate::node_agent_local_task_supervision::contract_payload(&contract),
    )?;
    let record = runtime.local_tasks.create(LocalTaskStart {
        task_id,
        owner_user_id: &item.owner_user_id,
        agent_id: &item.agent_id,
        install_id: &item.install_id,
        project_id: &item.project_id,
        channel_id: item.channel_id.as_deref(),
        conversation_id: &item.conversation_id,
        workspace_path: &item.workspace_path,
        prompt: &item.prompt,
        cli: "codex",
        runtime_permission: &item.runtime_permission,
    })?;
    let continuation = if item.generation > 1 {
        format!("Low-priority self evolution is automatically resuming generation {} after yielding. Inspect the existing isolated worktree and durable journal before continuing; do not redo completed or non-repeatable actions.\n\n{}", item.generation, item.prompt)
    } else {
        item.prompt.clone()
    };
    crate::node_agent_local_tasks::dispatch_local_task_record(
        runtime.clone(),
        &record,
        crate::node_agent_local_task_supervision::executor_prompt(&continuation, Some(&contract)),
        item.workspace_path.clone(),
        Some(&contract),
        None,
        frozen,
    );
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
