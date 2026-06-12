//! PC node capacity policy for code-project workspaces.
//!
//! Servers are not the default place for new code projects anymore, so PC nodes
//! need an explicit admission check before provisioning another workspace.

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
        warnings.push("PC CLI 通道未连接，不能创建新项目目录".to_string());
    } else if !node.cli_project_ready() {
        warnings.push("PC 节点未上报 Codex/Copilot CLI 能力".to_string());
    }
    if node.project_count >= project_limit {
        warnings.push(format!("PC 节点项目数已达上限 {project_limit} 个"));
    }
    match disk_free_bytes {
        Some(bytes) if bytes < min_free_bytes => warnings.push(format!(
            "PC 节点最近巡检磁盘剩余低于 {}",
            human_bytes(min_free_bytes)
        )),
        None => warnings.push("暂无 PC 磁盘健康快照，创建后会在健康页补充巡检".to_string()),
        _ => {}
    }

    let hard_blocked = !node.online
        || !node.cli_connected
        || !node.cli_project_ready()
        || node.project_count >= project_limit
        || disk_free_bytes.is_some_and(|bytes| bytes < min_free_bytes);
    let (label, tone) = if !node.online {
        ("离线", "bad")
    } else if !node.cli_connected || !node.cli_project_ready() {
        ("CLI 不可用", "bad")
    } else if node.project_count >= project_limit {
        ("项目数已满", "bad")
    } else if disk_free_bytes.is_some_and(|bytes| bytes < min_free_bytes) {
        ("磁盘不足", "bad")
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
        .unwrap_or_else(|| "容量策略不允许继续创建项目".to_string());
    format!("PC 节点 {} 暂不能创建新项目：{reason}", node.display_name)
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
    fn capacity_blocks_full_node() {
        let mut node = test_node();
        node.project_count = max_projects_per_node();

        let capacity = assess_pc_node_capacity(&node, None);

        assert!(!capacity.can_accept_project);
        assert_eq!(capacity.label, "项目数已满");
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
            hardware: None,
            storage: None,
            display_name: "PC-A".to_string(),
            short_id: "node-a".to_string(),
            models: Vec::new(),
            allowed_clis: vec!["codex".to_string()],
            allowed_cwds: Vec::new(),
            connected_at: 1,
            created_at: String::new(),
            online: true,
            registry_online: true,
            cli_connected: true,
            project_count: 0,
        }
    }
}
