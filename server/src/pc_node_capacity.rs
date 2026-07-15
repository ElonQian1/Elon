//! PC node capacity policy for code-project workspaces.
//!
//! Servers are not the default place for new code projects anymore, so PC nodes
//! need an availability check plus advisory capacity guidance before
//! provisioning another workspace. Disk and project-count guidance must not
//! turn an otherwise usable node into an outage.

use serde::Serialize;

use crate::{node_runtime::NodeRuntime, store::ProjectWorkspaceHealthSnapshot};

const DEFAULT_MAX_PROJECTS_PER_NODE: i64 = 12;
const DEFAULT_MIN_FREE_BYTES: u64 = 4 * 1024 * 1024 * 1024;

#[derive(Debug, Clone, Serialize)]
pub struct PcNodeCapacity {
    pub project_count: i64,
    pub project_limit: i64,
    pub project_slots_remaining: i64,
    pub disk_free_bytes: Option<u64>,
    pub min_free_bytes: u64,
    pub can_accept_project: bool,
    pub label: String,
    pub tone: String,
    pub warnings: Vec<String>,
}

pub fn assess_pc_node_capacity(
    node: &NodeRuntime,
    latest_snapshot: Option<&ProjectWorkspaceHealthSnapshot>,
) -> PcNodeCapacity {
    let project_limit = max_projects_per_node();
    let min_free_bytes = min_free_bytes();
    let project_slots_remaining = (project_limit - node.project_count).max(0);
    let disk_free_bytes = latest_snapshot.and_then(|snapshot| snapshot.disk_free_bytes);
    let mut warnings = Vec::new();

    if !node.online {
        warnings.push("PC 节点离线，不能创建新项目目录".to_string());
    } else if !node.cli_connected {
        warnings.push("PC 开发运行时通道未连接，不能创建新项目目录".to_string());
    } else if !node.workspace_provision_ready() {
        warnings.push("PC 节点未上报可创建项目工作区的开发运行时能力".to_string());
    }
    if node.project_count >= project_limit {
        warnings.push(format!(
            "PC 节点项目数已达到建议整理线 {project_limit} 个；仍可继续使用"
        ));
    }
    match disk_free_bytes {
        Some(bytes) if bytes < min_free_bytes => warnings.push(format!(
            "PC 节点最近巡检磁盘剩余低于 {}",
            human_bytes(min_free_bytes)
        )),
        None => warnings.push("暂无 PC 磁盘健康快照，创建后会在健康页补充巡检".to_string()),
        _ => {}
    }

    let hard_blocked = !node.online || !node.cli_connected || !node.workspace_provision_ready();
    let (label, tone) = if !node.online {
        ("离线", "bad")
    } else if !node.cli_connected || !node.workspace_provision_ready() {
        ("开发运行时不可用", "bad")
    } else if node.project_count >= project_limit {
        ("建议整理项目", "warn")
    } else if disk_free_bytes.is_some_and(|bytes| bytes < min_free_bytes) {
        ("建议整理磁盘", "warn")
    } else if disk_free_bytes.is_none() {
        ("容量未知", "warn")
    } else {
        ("可创建项目", "ok")
    };

    PcNodeCapacity {
        project_count: node.project_count,
        project_limit,
        project_slots_remaining,
        disk_free_bytes,
        min_free_bytes,
        can_accept_project: !hard_blocked,
        label: label.to_string(),
        tone: tone.to_string(),
        warnings,
    }
}

pub fn capacity_block_message(node: &NodeRuntime, capacity: &PcNodeCapacity) -> String {
    let reason = capacity
        .warnings
        .first()
        .cloned()
        .unwrap_or_else(|| "节点开发运行时暂不可用".to_string());
    format!(
        "PC 节点 {} 暂不能连接开发运行时：{reason}",
        node.display_name
    )
}

pub fn max_projects_per_node() -> i64 {
    std::env::var("PC_NODE_MAX_PROJECTS")
        .ok()
        .and_then(|value| value.parse::<i64>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(DEFAULT_MAX_PROJECTS_PER_NODE)
}

pub fn min_free_bytes() -> u64 {
    std::env::var("PC_NODE_MIN_FREE_BYTES")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(DEFAULT_MIN_FREE_BYTES)
}

fn human_bytes(bytes: u64) -> String {
    const GIB: u64 = 1024 * 1024 * 1024;
    if bytes >= GIB && bytes % GIB == 0 {
        format!("{}GB", bytes / GIB)
    } else if bytes >= GIB {
        format!("{:.1}GB", bytes as f64 / GIB as f64)
    } else {
        format!("{}MB", bytes / (1024 * 1024))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_limit_is_advisory_for_an_available_node() {
        let mut node = test_node();
        node.project_count = max_projects_per_node();

        let capacity = assess_pc_node_capacity(&node, None);

        assert!(capacity.can_accept_project);
        assert_eq!(capacity.label, "建议整理项目");
        assert_eq!(capacity.tone, "warn");
    }

    #[test]
    fn low_disk_is_advisory_for_an_available_node() {
        let node = test_node();
        let snapshot = ProjectWorkspaceHealthSnapshot {
            project_id: "project-a".to_string(),
            node_id: Some("node-a".to_string()),
            workspace_path: Some("D:/repo".to_string()),
            can_run_on_pc: true,
            verified_can_run_on_pc: Some(true),
            health_label: "可运行".to_string(),
            health_tone: "ok".to_string(),
            recommended_action: String::new(),
            warning_count: 0,
            warnings: Vec::new(),
            live_inspect: None,
            inspect_error: None,
            disk_free_bytes: Some(1),
            path_exists: Some(true),
            is_dir: Some(true),
            is_git_worktree: Some(true),
            cli_available: Some(true),
            captured_at: String::new(),
        };

        let capacity = assess_pc_node_capacity(&node, Some(&snapshot));

        assert!(capacity.can_accept_project);
        assert_eq!(capacity.label, "建议整理磁盘");
        assert_eq!(capacity.tone, "warn");
    }

    #[test]
    fn capacity_allows_unknown_disk_with_warning() {
        let node = test_node();

        let capacity = assess_pc_node_capacity(&node, None);

        assert!(capacity.can_accept_project);
        assert_eq!(capacity.label, "容量未知");
        assert!(!capacity.warnings.is_empty());
    }

    fn test_node() -> NodeRuntime {
        NodeRuntime {
            node_id: "node-a".to_string(),
            owner_user_id: "user-a".to_string(),
            label: "PC-A".to_string(),
            device_name: Some("PC-A".to_string()),
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
            dev_runtime: None,
            lifecycle: None,
            display_name: "PC-A".to_string(),
            short_id: "node-a".to_string(),
            models: Vec::new(),
            allowed_clis: vec!["codex".to_string()],
            allowed_cwds: Vec::new(),
            agent_version: None,
            connected_at: 1,
            created_at: String::new(),
            online: true,
            registry_online: true,
            cli_connected: true,
            project_count: 0,
        }
    }
}
