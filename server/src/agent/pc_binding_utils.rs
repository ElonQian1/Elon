// server/src/agent/pc_binding_utils.rs
//! PC 绑定辅助工具函数

use crate::{
    pc_agent_runtime_choice::PcRuntimeRoutePreference,
    store::{ProjectAccess, ProjectDevProfile},
    types::{AppState, WsMessage},
};
use homecli_proto::{AgentToServer, ProjectWorkspaceInspectStatus};
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedSender;
use tracing::warn;

pub(super) fn clone_url_for_project_access(
    project: &ProjectAccess,
    target_agent_id: &str,
) -> Option<String> {
    if let Some(repo_url) = project
        .repo_url
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        return Some(repo_url.to_string());
    }
    if project.storage_node_id.as_deref() == Some(target_agent_id) {
        if let Some(path) = project
            .storage_repo_path
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            return Some(path.to_string());
        }
    }
    project
        .storage_repo_url
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

pub(super) fn send_optional_progress(tx: Option<&UnboundedSender<String>>, message: &str) {
    if let Some(tx) = tx {
        let _ = tx.send(WsMessage::progress(message.to_string()).to_json());
    }
}

pub(super) fn send_pc_workspace_unavailable_error(
    project: &ProjectAccess,
    tx: &UnboundedSender<String>,
) {
    let detail = project
        .workspace_path
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("未记录目录");
    let msg = format!(
        "当前项目绑定的 PC 工作区不可用，且没有可自动迁移的 Git/硬盘代码源。原目录：{detail}。请先让原 PC 节点上线，或重新创建项目后再发送开发需求。"
    );
    warn!(project_id = %project.id, "{}", msg);
    let _ = tx.send(WsMessage::error(msg).to_json());
}

pub(super) async fn pc_agent_is_connected(state: &Arc<AppState>, agent_id: &str) -> bool {
    state
        .agent_manager
        .list()
        .await
        .into_iter()
        .any(|agent| agent.agent_id == agent_id)
}

pub(super) async fn inspect_pc_agent_workspace(
    state: &Arc<AppState>,
    agent_id: &str,
    workspace: &str,
) -> std::result::Result<ProjectWorkspaceInspectStatus, String> {
    match state
        .agent_manager
        .dispatch_project_workspace_inspect(agent_id, workspace.to_string())
        .await
    {
        Ok(AgentToServer::ProjectWorkspaceInspected { status, .. }) => Ok(status),
        Ok(AgentToServer::ProjectWorkspaceInspectError { message, .. }) => Err(message),
        Ok(other) => Err(format!("unexpected inspect response: {other:?}")),
        Err(error) => Err(error.to_string()),
    }
}

pub(super) fn pc_workspace_inspect_usable(status: &ProjectWorkspaceInspectStatus) -> bool {
    status.path_exists && status.is_dir && (status.codex_available || status.copilot_available)
}

pub(super) fn pc_workspace_inspect_usable_for_route(
    status: &ProjectWorkspaceInspectStatus,
    pc_runtime_route: Option<PcRuntimeRoutePreference>,
) -> bool {
    if !status.path_exists || !status.is_dir {
        return false;
    }
    match pc_runtime_route {
        Some(
            PcRuntimeRoutePreference::RouteB
            | PcRuntimeRoutePreference::RouteC
            | PcRuntimeRoutePreference::RouteC2,
        ) => true,
        _ => status.codex_available || status.copilot_available,
    }
}

pub(super) fn pc_workspace_inspect_problem(status: &ProjectWorkspaceInspectStatus) -> &'static str {
    if !status.path_exists {
        "workspace_path_missing"
    } else if !status.is_dir {
        "workspace_path_not_directory"
    } else if !status.codex_available && !status.copilot_available {
        "cli_unavailable"
    } else {
        "unknown"
    }
}

pub(super) fn pc_workspace_inspect_error_allows_bound_dispatch(error: &str) -> bool {
    let lower = error.to_ascii_lowercase();
    lower.contains("timeout") || lower.contains("timed out") || lower.contains("超时")
}

/// Codex 额度耗尽或认证失效时返回 true，此时可自动切换到 Copilot
pub(super) fn is_codex_fallback_error(error: &str) -> bool {
    use crate::errors::{classify_ai_error, AiErrorCategory};
    let classified = classify_ai_error(error);
    matches!(
        classified.category,
        AiErrorCategory::Quota | AiErrorCategory::AuthConfig
    )
}

/// 检查指定 PC 节点上某个 CLI 是否可用
pub(super) async fn node_cli_available(
    state: &Arc<AppState>,
    agent_id: &str,
    cli_name: &str,
) -> bool {
    state
        .agent_manager
        .list()
        .await
        .into_iter()
        .find(|a| a.agent_id == agent_id)
        .map(|a| {
            a.allowed_clis
                .iter()
                .any(|c| c.eq_ignore_ascii_case(cli_name))
        })
        .unwrap_or(false)
}

pub(super) fn append_project_dev_profile_context(
    state: &Arc<AppState>,
    user_id: &str,
    project: &ProjectAccess,
    user_message: &str,
) -> String {
    let profile = match state
        .store
        .get_project_dev_profile_for_user(user_id, &project.id)
    {
        Ok(Some(profile)) if !profile.is_empty() => profile,
        Ok(_) => return user_message.to_string(),
        Err(error) => {
            warn!(
                project_id = %project.id,
                "读取项目开发命令 profile 失败，继续使用原始用户消息: {error}"
            );
            return user_message.to_string();
        }
    };
    format!(
        "{user_message}\n\n{}",
        project_dev_profile_prompt_block(&profile)
    )
}

fn project_dev_profile_prompt_block(profile: &ProjectDevProfile) -> String {
    let mut lines = vec![
        "系统自动识别的本地项目开发命令；执行 run/test/build 时优先参考，除非仓库文档给出更明确命令。".to_string(),
        "<project_dev_profile>".to_string(),
    ];
    push_profile_line(&mut lines, "project_type", profile.project_type.as_deref());
    push_profile_line(
        &mut lines,
        "package_manager",
        profile.package_manager.as_deref(),
    );
    push_profile_line(&mut lines, "run_command", profile.run_command.as_deref());
    push_profile_line(&mut lines, "test_command", profile.test_command.as_deref());
    push_profile_line(
        &mut lines,
        "build_command",
        profile.build_command.as_deref(),
    );
    if !profile.detected_files.is_empty() {
        lines.push(format!(
            "detected_files: {}",
            profile.detected_files.join(", ")
        ));
    }
    lines.push("</project_dev_profile>".to_string());
    lines.join("\n")
}

fn push_profile_line(lines: &mut Vec<String>, key: &str, value: Option<&str>) {
    if let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) {
        lines.push(format!("{key}: {value}"));
    }
}
