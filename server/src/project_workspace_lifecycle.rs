use homecli_proto::ProjectWorkspaceInspectStatus;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct ProjectWorkspaceRecoveryAction {
    pub key: &'static str,
    pub label: &'static str,
    pub description: &'static str,
    pub available: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProjectWorkspaceLifecycle {
    pub health_label: String,
    pub health_tone: &'static str,
    pub recommended_action: String,
    pub recovery_actions: Vec<ProjectWorkspaceRecoveryAction>,
}

pub fn workspace_lifecycle(
    workspace_kind: &str,
    node_id: Option<&str>,
    workspace_path: Option<&str>,
    node_online: bool,
    can_run_on_pc: bool,
    verified_can_run_on_pc: Option<bool>,
    live_inspect: Option<&ProjectWorkspaceInspectStatus>,
    warning_count: usize,
) -> ProjectWorkspaceLifecycle {
    if workspace_kind == "system_archive" {
        return ProjectWorkspaceLifecycle {
            health_label: "个人归档".to_string(),
            health_tone: "neutral",
            recommended_action: "仅保存会话和记忆，不需要 PC 工作区".to_string(),
            recovery_actions: Vec::new(),
        };
    }
    if workspace_kind != "pc_node_workspace" {
        return ProjectWorkspaceLifecycle {
            health_label: "外部工作区".to_string(),
            health_tone: if warning_count > 0 { "warn" } else { "neutral" },
            recommended_action: "该项目不是自动创建的 PC 工作区，按已登记路径执行".to_string(),
            recovery_actions: Vec::new(),
        };
    }

    let has_node = node_id.is_some_and(|value| !value.trim().is_empty());
    let has_workspace = workspace_path.is_some_and(|value| !value.trim().is_empty());
    let inspect = live_inspect;
    let path_missing = inspect.is_some_and(|status| !status.path_exists);
    let not_directory = inspect.is_some_and(|status| status.path_exists && !status.is_dir);
    let cli_missing = inspect.is_some_and(|status| !cli_available(status));
    let low_disk = inspect
        .and_then(|status| status.disk_free_bytes)
        .is_some_and(|bytes| bytes < 2 * 1024 * 1024 * 1024);

    if !has_node {
        return pc_lifecycle(
            "未绑定 PC 节点",
            "bad",
            "选择在线 PC 节点并重新绑定项目工作区",
            vec![
                bind_node_action(true),
                recreate_action(false),
                migrate_action(false),
            ],
        );
    }
    if !has_workspace || path_missing || not_directory {
        return pc_lifecycle(
            if path_missing {
                "目录丢失"
            } else {
                "缺少工作区路径"
            },
            "bad",
            "在 PC 节点重新创建目录，或迁移到另一个在线 PC 节点",
            vec![
                recreate_action(node_online),
                migrate_action(true),
                bind_node_action(true),
            ],
        );
    }
    if !node_online {
        return pc_lifecycle(
            "PC 离线",
            "bad",
            "启动绑定的 PC 节点后重试，或迁移到当前在线节点",
            vec![migrate_action(true), bind_node_action(true)],
        );
    }
    if verified_can_run_on_pc == Some(false) || cli_missing {
        return pc_lifecycle(
            "CLI 不可用",
            "bad",
            "在 PC 节点安装或修复 Codex/Copilot CLI 后重试",
            vec![repair_cli_action(false), migrate_action(true)],
        );
    }
    if low_disk {
        return pc_lifecycle(
            "磁盘空间不足",
            "warn",
            "清理 PC 工作区磁盘空间，或迁移到剩余空间更充足的节点",
            vec![migrate_action(true)],
        );
    }
    if can_run_on_pc && warning_count == 0 {
        return pc_lifecycle(
            "PC 可运行",
            "ok",
            "可以继续在该 PC 节点执行项目任务",
            Vec::new(),
        );
    }
    pc_lifecycle(
        "需要处理",
        "warn",
        "查看工作区警告，修复后再继续执行代码任务",
        vec![migrate_action(true), recreate_action(node_online)],
    )
}

fn pc_lifecycle(
    health_label: &str,
    health_tone: &'static str,
    recommended_action: &str,
    recovery_actions: Vec<ProjectWorkspaceRecoveryAction>,
) -> ProjectWorkspaceLifecycle {
    ProjectWorkspaceLifecycle {
        health_label: health_label.to_string(),
        health_tone,
        recommended_action: recommended_action.to_string(),
        recovery_actions,
    }
}

fn bind_node_action(available: bool) -> ProjectWorkspaceRecoveryAction {
    ProjectWorkspaceRecoveryAction {
        key: "bind_pc_node",
        label: "绑定 PC 节点",
        description: "选择一个在线 PC 节点作为该项目的执行位置",
        available,
    }
}

fn recreate_action(available: bool) -> ProjectWorkspaceRecoveryAction {
    ProjectWorkspaceRecoveryAction {
        key: "recreate_workspace",
        label: "重新创建目录",
        description: "在绑定 PC 节点上重新创建项目工作区目录",
        available,
    }
}

fn migrate_action(available: bool) -> ProjectWorkspaceRecoveryAction {
    ProjectWorkspaceRecoveryAction {
        key: "migrate_workspace",
        label: "迁移到其他 PC",
        description: "把项目执行位置切换到另一个在线 PC 节点",
        available,
    }
}

fn repair_cli_action(available: bool) -> ProjectWorkspaceRecoveryAction {
    ProjectWorkspaceRecoveryAction {
        key: "repair_cli",
        label: "修复 CLI 环境",
        description: "安装或修复 PC 节点上的 Codex/Copilot CLI",
        available,
    }
}

fn cli_available(status: &ProjectWorkspaceInspectStatus) -> bool {
    status.codex_available || status.copilot_available
}


#[cfg(test)]
#[path = "project_workspace_lifecycle_tests.rs"]
mod tests;
