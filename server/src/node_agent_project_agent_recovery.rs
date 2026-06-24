// server/src/node_agent_project_agent_recovery.rs

use serde::Serialize;

use crate::node_agent_project_agent_runs::{ProjectAgentRunControl, ProjectAgentRunTaskResume};

#[derive(Debug, Serialize)]
pub(crate) struct ProjectAgentRunRecoveryEntry {
    pub(crate) kind: String,
    pub(crate) task_id: String,
    pub(crate) cli_name: String,
    pub(crate) route: Option<String>,
    pub(crate) cwd: Option<String>,
    pub(crate) runtime_permission: Option<String>,
    pub(crate) status: String,
    pub(crate) recommended_action: String,
    pub(crate) reason: String,
    pub(crate) tty_reconnect: ProjectAgentRunTtyReconnect,
    pub(crate) can_cancel: bool,
    pub(crate) can_continue: bool,
    pub(crate) updated_at_ms: Option<u128>,
}

#[derive(Debug, Serialize)]
pub(crate) struct ProjectAgentRunTtyReconnect {
    pub(crate) supported: bool,
    pub(crate) user_label: String,
    pub(crate) reason: String,
    pub(crate) fallback_action: String,
}

pub(crate) fn recovery_entry_from(
    active_controls: &[ProjectAgentRunControl],
    recent_tasks: &[ProjectAgentRunTaskResume],
) -> Option<ProjectAgentRunRecoveryEntry> {
    active_controls
        .iter()
        .find(|control| !control.task_id.trim().is_empty())
        .map(ProjectAgentRunRecoveryEntry::from_active_control)
        .or_else(|| {
            recent_tasks
                .iter()
                .find(|task| !task.task_id.trim().is_empty())
                .map(ProjectAgentRunRecoveryEntry::from_recent_task)
        })
}

impl ProjectAgentRunRecoveryEntry {
    fn from_active_control(control: &ProjectAgentRunControl) -> Self {
        let fallback_action = "wait_or_cancel".to_string();
        Self {
            kind: "active_control".to_string(),
            task_id: control.task_id.clone(),
            cli_name: control.cli_name.clone(),
            route: Some(control.route.clone()),
            cwd: control.cwd.clone(),
            runtime_permission: control.runtime_permission.clone(),
            status: "running".to_string(),
            recommended_action: fallback_action.clone(),
            reason: "当前本机节点仍持有运行控制句柄，PC 端应优先展示继续观察或停止入口。"
                .to_string(),
            tty_reconnect: active_control_tty_reconnect(fallback_action),
            can_cancel: control.can_cancel,
            can_continue: false,
            updated_at_ms: Some(control.last_heartbeat_ms),
        }
    }

    fn from_recent_task(task: &ProjectAgentRunTaskResume) -> Self {
        let recommended_action = task.resume.next_action().to_string();
        Self {
            kind: "snapshot_resume".to_string(),
            task_id: task.task_id.clone(),
            cli_name: task.cli_name.clone(),
            route: task.route.clone(),
            cwd: task.cwd.clone(),
            runtime_permission: task.runtime_permission.clone(),
            status: task.resume.status().to_string(),
            can_cancel: false,
            can_continue: recommended_action == "continue_from_snapshot",
            tty_reconnect: snapshot_tty_reconnect(recommended_action.clone()),
            recommended_action,
            reason: task.resume.reason().to_string(),
            updated_at_ms: Some(task.updated_at_ms),
        }
    }
}

fn active_control_tty_reconnect(fallback_action: String) -> ProjectAgentRunTtyReconnect {
    ProjectAgentRunTtyReconnect {
        supported: false,
        user_label: "原 CLI 终端不可重接".to_string(),
        reason: "浏览器/PC 页不能重新接管已经打开的原始 CLI TTY；当前只能继续观察本机控制句柄，或由用户授权停止任务。"
            .to_string(),
        fallback_action,
    }
}

fn snapshot_tty_reconnect(fallback_action: String) -> ProjectAgentRunTtyReconnect {
    ProjectAgentRunTtyReconnect {
        supported: false,
        user_label: "原 CLI 终端不可重接".to_string(),
        reason: "原始 CLI TTY 已经脱离当前页面，不能恢复成同一个窗口；只能基于本机 journal、任务快照和项目工作区状态开启新的继续处理。"
            .to_string(),
        fallback_action,
    }
}
