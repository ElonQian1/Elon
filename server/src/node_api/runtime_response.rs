use homecli_proto::{NodeDevRuntimeProfile, NodeHardwareProfile};
use std::collections::HashMap;

use crate::{
    node_runtime::{short_node_id, NodeRuntime},
    pc_node_capacity::{assess_pc_node_capacity, PcNodeCapacity},
    types::AppState,
};

pub(super) fn runtime_route_flags(
    runtime: Option<&NodeDevRuntimeProfile>,
    legacy_cli_ready: bool,
) -> (bool, bool, bool) {
    runtime
        .map(|runtime| {
            (
                runtime.route_a_ready,
                runtime.api_runtime_ready,
                runtime.server_runtime_ready,
            )
        })
        .unwrap_or((legacy_cli_ready, false, false))
}

pub(super) fn project_counts_for_user(state: &AppState, user_id: &str) -> HashMap<String, i64> {
    match state.store.list_projects_for_user(user_id) {
        Ok(projects) => projects
            .into_iter()
            .filter_map(|project| project.node_id)
            .fold(HashMap::<String, i64>::new(), |mut counts, node_id| {
                *counts.entry(node_id).or_insert(0) += 1;
                counts
            }),
        Err(e) => {
            tracing::warn!(user_id = %user_id, error = %e, "failed to count user projects per node");
            HashMap::new()
        }
    }
}

pub(super) fn capacity_for_response(
    state: &AppState,
    node_id: &str,
    owner_user_id: &str,
    label: &str,
    device_name: Option<&str>,
    display_name: &str,
    online: bool,
    cli_connected: bool,
    allowed_clis: &[String],
    dev_runtime: Option<NodeDevRuntimeProfile>,
    project_count: i64,
) -> PcNodeCapacity {
    let latest_snapshot = state
        .store
        .latest_workspace_health_snapshot_for_node(node_id)
        .ok()
        .flatten();
    let runtime = NodeRuntime {
        node_id: node_id.to_string(),
        owner_user_id: owner_user_id.to_string(),
        label: label.to_string(),
        device_name: device_name.map(ToOwned::to_owned),
        install_id: None,
        public_dev_enabled: false,
        public_dev_allowed_clis: Vec::new(),
        public_dev_permission_level: "project_write".to_string(),
        last_handshake_at: None,
        last_handshake_agent_version: None,
        last_handshake_allowed_clis: Vec::new(),
        last_handshake_route_a_ready: false,
        last_handshake_api_runtime_ready: false,
        last_handshake_server_runtime_ready: false,
        last_handshake_ai_cli_ready: false,
        hardware: None,
        storage: None,
        dev_runtime,
        lifecycle: None,
        display_name: display_name.to_string(),
        short_id: short_node_id(node_id),
        models: Vec::new(),
        allowed_clis: allowed_clis.to_vec(),
        allowed_cwds: Vec::new(),
        agent_version: None,
        connected_at: 0,
        created_at: String::new(),
        online,
        registry_online: online,
        cli_connected,
        project_count,
    };
    assess_pc_node_capacity(&runtime, latest_snapshot.as_ref())
}

pub(super) fn hardware_for_response(
    state: &AppState,
    node_id: &str,
    live: Option<NodeHardwareProfile>,
) -> Option<NodeHardwareProfile> {
    live.or_else(|| {
        state
            .store
            .get_node_hardware_snapshot(node_id)
            .ok()
            .flatten()
            .map(|snapshot| snapshot.hardware)
    })
}

pub(super) fn hardware_summary(profile: Option<&NodeHardwareProfile>) -> String {
    let Some(profile) = profile else {
        return "硬件未知".to_string();
    };
    let mut parts = Vec::new();
    if !profile.gpu_names.is_empty() {
        parts.push(format!("GPU {}", profile.gpu_names.join(" / ")));
    }
    if let Some(bytes) = profile.gpu_memory_total_bytes.and_then(format_bytes) {
        parts.push(format!("显存 {bytes}"));
    }
    if let Some(bytes) = profile.memory_total_bytes.and_then(format_bytes) {
        parts.push(format!("内存 {bytes}"));
    }
    if let Some(cores) = profile.cpu_cores.filter(|cores| *cores > 0) {
        parts.push(format!("CPU {cores} 核"));
    } else if let Some(cpu) = profile
        .cpu_brand
        .as_deref()
        .filter(|value| !value.trim().is_empty())
    {
        parts.push(cpu.trim().to_string());
    }
    if parts.is_empty() {
        "硬件未知".to_string()
    } else {
        parts.join(" · ")
    }
}

fn format_bytes(bytes: u64) -> Option<String> {
    if bytes == 0 {
        return None;
    }
    let units = ["B", "KB", "MB", "GB", "TB"];
    let mut value = bytes as f64;
    let mut idx = 0usize;
    while value >= 1024.0 && idx < units.len() - 1 {
        value /= 1024.0;
        idx += 1;
    }
    Some(if idx >= 3 {
        format!("{value:.1} {}", units[idx])
    } else {
        format!("{} {}", value.round() as u64, units[idx])
    })
}
