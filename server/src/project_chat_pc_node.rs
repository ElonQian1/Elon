use std::{sync::Arc, time::Duration};

use homecli_proto::NodeHardwareProfile;
use tokio::sync::mpsc::UnboundedSender;

use crate::{
    node_runtime::node_runtime_by_id,
    pc_agent_runtime_choice::PcRuntimeRoutePreference,
    project_task_scheduler::ProjectTaskPermit,
    types::{AppState, WsMessage},
};

const DEFAULT_PC_NODE_CLI_PARALLEL_LIMIT: usize = 1;
const DEFAULT_PC_NODE_CLI_HARD_LIMIT: usize = 4;
const GIB: u64 = 1024 * 1024 * 1024;

pub(crate) struct PcNodeCliPermit {
    pub(crate) permit: ProjectTaskPermit,
    pub(crate) parallel_limit: usize,
}

pub(crate) fn pc_node_fast_path_route(
    pc_runtime_route: Option<PcRuntimeRoutePreference>,
    direct_pc_cli: bool,
) -> Option<PcRuntimeRoutePreference> {
    if direct_pc_cli {
        return Some(PcRuntimeRoutePreference::RouteA);
    }
    pc_runtime_route
}

pub(crate) async fn acquire_pc_node_cli_permit(
    state: &Arc<AppState>,
    tx: &UnboundedSender<String>,
    trace_id: Option<&str>,
    project_id: &str,
    conversation_id: &str,
    node_id: &str,
) -> PcNodeCliPermit {
    let node_queue_key = pc_node_cli_execution_key(node_id);
    let parallel_limit = pc_node_cli_parallel_limit(state, node_id).await;
    if let Some(permit) = try_acquire_pc_node_cli_slot(state, &node_queue_key, parallel_limit).await
    {
        return PcNodeCliPermit {
            permit,
            parallel_limit,
        };
    }

    let queued_node_label = pc_node_cli_queue_label(node_id);
    record_pc_node_cli_queue_wait(
        state,
        trace_id,
        project_id,
        conversation_id,
        node_id,
        parallel_limit,
    );
    let _ = tx.send(
        WsMessage::progress(format!(
            "当前 PC 节点 {} 的本机 CLI 并发槽位已满（{} 个），本次消息已进入节点队列；其他 PC 节点仍可并行。",
            queued_node_label, parallel_limit
        ))
        .to_json(),
    );

    let mut waited_secs = 0u64;
    loop {
        tokio::time::sleep(Duration::from_secs(5)).await;
        waited_secs += 5;
        if let Some(permit) =
            try_acquire_pc_node_cli_slot(state, &node_queue_key, parallel_limit).await
        {
            return PcNodeCliPermit {
                permit: permit.mark_queued(),
                parallel_limit,
            };
        }
        if waited_secs % 30 == 0 {
            let _ = tx.send(
                WsMessage::progress(format!(
                    "仍在等待 PC 节点 {} 的本机 CLI 并发槽位（已等待 {}s，当前上限 {} 个）；这是容量限流，不是节点断线。",
                    queued_node_label, waited_secs, parallel_limit
                ))
                .to_json(),
            );
        }
    }
}

pub(crate) fn record_pc_node_cli_execution_granted(
    state: &Arc<AppState>,
    trace_id: Option<&str>,
    project_id: &str,
    conversation_id: &str,
    node_id: &str,
    was_queued: bool,
    parallel_limit: usize,
) {
    if let Some(trace_id) = trace_id {
        state.server_traces.record(
            trace_id,
            "server_pc_node_cli_execution_granted",
            serde_json::json!({
                "project_id": project_id,
                "conversation_id": conversation_id,
                "node_id": node_id,
                "was_queued": was_queued,
                "parallel_limit": parallel_limit,
            }),
        );
    }
}

pub(crate) fn pc_node_cli_execution_progress_message(
    was_queued: bool,
    parallel_limit: usize,
) -> String {
    let prefix = if was_queued {
        "已轮到 PC 节点本机 CLI，开始交给节点执行"
    } else {
        "已获得 PC 节点本机 CLI 执行权，开始交给节点执行"
    };
    format!("{prefix}（当前节点并发槽位 {parallel_limit} 个）。")
}

fn record_pc_node_cli_queue_wait(
    state: &Arc<AppState>,
    trace_id: Option<&str>,
    project_id: &str,
    conversation_id: &str,
    node_id: &str,
    parallel_limit: usize,
) {
    if let Some(trace_id) = trace_id {
        state.server_traces.record(
            trace_id,
            "server_pc_node_cli_queue_wait",
            serde_json::json!({
                "project_id": project_id,
                "conversation_id": conversation_id,
                "node_id": node_id,
                "parallel_limit": parallel_limit,
            }),
        );
    }
}

async fn try_acquire_pc_node_cli_slot(
    state: &Arc<AppState>,
    base_key: &str,
    parallel_limit: usize,
) -> Option<ProjectTaskPermit> {
    for slot in 0..parallel_limit.max(1) {
        if let Some(permit) = state
            .project_task_scheduler
            .try_acquire(&pc_node_cli_slot_key(base_key, slot))
            .await
        {
            return Some(permit);
        }
    }
    None
}

fn pc_node_cli_execution_key(agent_id: &str) -> String {
    format!("pc-node-cli:{}", agent_id.trim())
}

fn pc_node_cli_slot_key(base_key: &str, slot: usize) -> String {
    format!("{base_key}:slot:{slot}")
}

fn pc_node_cli_queue_label(agent_id: &str) -> String {
    let agent_id = agent_id.trim();
    if agent_id.is_empty() {
        "当前节点".to_string()
    } else {
        agent_id.to_string()
    }
}

async fn pc_node_cli_parallel_limit(state: &Arc<AppState>, node_id: &str) -> usize {
    let hard_limit = pc_node_cli_hard_limit();
    if let Some(limit) = env_usize("ELON_PC_NODE_CLI_MAX_PARALLEL") {
        return limit.clamp(1, hard_limit);
    }
    let hardware = node_runtime_by_id(state, node_id)
        .await
        .ok()
        .flatten()
        .and_then(|node| node.hardware);
    hardware
        .as_ref()
        .map(pc_node_cli_parallel_limit_from_hardware)
        .unwrap_or(DEFAULT_PC_NODE_CLI_PARALLEL_LIMIT)
        .clamp(1, hard_limit)
}

fn pc_node_cli_hard_limit() -> usize {
    env_usize("ELON_PC_NODE_CLI_HARD_MAX_PARALLEL")
        .unwrap_or(DEFAULT_PC_NODE_CLI_HARD_LIMIT)
        .clamp(1, 8)
}

fn pc_node_cli_parallel_limit_from_hardware(hardware: &NodeHardwareProfile) -> usize {
    let cores = hardware.cpu_cores.unwrap_or(0) as usize;
    let memory = hardware.memory_total_bytes.unwrap_or(0);
    let gpu_memory = hardware.gpu_memory_total_bytes.unwrap_or(0);
    let has_gpu = gpu_memory >= 8 * GIB || !hardware.gpu_names.is_empty();

    if cores >= 16 && memory >= 96 * GIB && gpu_memory >= 12 * GIB {
        4
    } else if cores >= 12 && memory >= 48 * GIB && has_gpu {
        3
    } else if cores >= 8 && memory >= 24 * GIB {
        2
    } else {
        1
    }
}

fn env_usize(name: &str) -> Option<usize> {
    std::env::var(name)
        .ok()
        .and_then(|value| value.trim().parse::<usize>().ok())
        .filter(|value| *value > 0)
}

#[cfg(test)]
mod tests {
    use super::{
        pc_node_cli_execution_key, pc_node_cli_parallel_limit_from_hardware,
        pc_node_fast_path_route, GIB,
    };
    use crate::pc_agent_runtime_choice::PcRuntimeRoutePreference;
    use homecli_proto::NodeHardwareProfile;

    #[test]
    fn pc_node_fast_path_does_not_default_lightweight_chat_to_route_a() {
        assert_eq!(pc_node_fast_path_route(None, false), None);
        assert_eq!(
            pc_node_fast_path_route(Some(PcRuntimeRoutePreference::RouteC3), false),
            Some(PcRuntimeRoutePreference::RouteC3)
        );
    }

    #[test]
    fn pc_node_direct_cli_switch_selects_route_a() {
        assert_eq!(
            pc_node_fast_path_route(None, true),
            Some(PcRuntimeRoutePreference::RouteA)
        );
        assert_eq!(
            pc_node_fast_path_route(Some(PcRuntimeRoutePreference::RouteC), true),
            Some(PcRuntimeRoutePreference::RouteA)
        );
    }

    #[test]
    fn pc_node_cli_execution_key_uses_trimmed_node_id() {
        assert_eq!(
            pc_node_cli_execution_key(" node-usr_5c-dd33ed36 "),
            "pc-node-cli:node-usr_5c-dd33ed36"
        );
    }

    #[test]
    fn hardware_limit_allows_more_slots_for_strong_pc() {
        let hardware = NodeHardwareProfile {
            cpu_cores: Some(16),
            memory_total_bytes: Some(96 * GIB),
            gpu_names: vec!["RTX 4060".to_string()],
            gpu_memory_total_bytes: Some(12 * GIB),
            ..Default::default()
        };

        assert_eq!(pc_node_cli_parallel_limit_from_hardware(&hardware), 4);
    }

    #[test]
    fn hardware_limit_stays_conservative_for_unknown_pc() {
        assert_eq!(
            pc_node_cli_parallel_limit_from_hardware(&NodeHardwareProfile::default()),
            1
        );
    }
}
