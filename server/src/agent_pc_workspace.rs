use crate::{
    ai_cli, intent_router, pc_agent_runtime_choice::PcRuntimeRoutePreference, store::ProjectAccess,
};

pub(crate) fn should_attempt_pc_apk_sync(project: &ProjectAccess, user_message: &str) -> bool {
    let development_request = intent_router::looks_like_development_request(user_message);
    (project_template_is_android(&project.template) && development_request)
        || ai_cli::looks_like_android_task(user_message)
        || (project_requires_pc_workspace(project) && development_request)
}

fn project_template_is_android(template: &str) -> bool {
    matches!(
        template.trim().to_ascii_lowercase().as_str(),
        "android" | "apk" | "android_kotlin" | "android_compose"
    )
}

pub(crate) fn project_cli_runtime_permission(project: &ProjectAccess) -> String {
    if project_requires_pc_workspace(project) {
        "full_access".to_string()
    } else {
        project.runtime_permission.clone()
    }
}

pub(crate) fn project_cli_runtime_permission_fallback(
    runtime_permission: &str,
    error_message: &str,
) -> Option<&'static str> {
    if runtime_permission == "full_access" && route_a_full_access_grant_error(error_message) {
        Some("project_write")
    } else {
        None
    }
}

pub(crate) fn route_a_full_access_grant_error(message: &str) -> bool {
    let lower = message.to_ascii_lowercase();
    (message.contains("Route A") || lower.contains("route a"))
        && (message.contains("完全访问尚未在本机授权")
            || message.contains("完全访问尚未")
            || (lower.contains("full access")
                && (lower.contains("grant")
                    || lower.contains("authoriz")
                    || lower.contains("permission"))))
}

pub(crate) fn pc_cli_chat_requested(pc_runtime_route: Option<PcRuntimeRoutePreference>) -> bool {
    matches!(
        pc_runtime_route,
        Some(PcRuntimeRoutePreference::RouteA | PcRuntimeRoutePreference::RouteC3)
    )
}

pub(crate) fn project_chat_should_use_pc_cli(
    pc_runtime_route: Option<PcRuntimeRoutePreference>,
    agent_name: Option<&str>,
    agent_is_local_cli: bool,
) -> bool {
    if pc_cli_chat_requested(pc_runtime_route) {
        return true;
    }
    if matches!(
        pc_runtime_route,
        Some(
            PcRuntimeRoutePreference::RouteB
                | PcRuntimeRoutePreference::RouteC
                | PcRuntimeRoutePreference::RouteC2
        )
    ) {
        return false;
    }
    if agent_name
        .map(str::trim)
        .is_some_and(|name| !name.is_empty())
    {
        return agent_is_local_cli;
    }
    false
}

#[cfg(test)]
pub(crate) fn pc_cli_chat_route_label(
    pc_runtime_route: Option<PcRuntimeRoutePreference>,
) -> &'static str {
    match pc_runtime_route {
        Some(PcRuntimeRoutePreference::RouteC3) => "远程 Codex",
        _ => "本机 AI",
    }
}

pub(crate) fn project_requires_pc_workspace(project: &ProjectAccess) -> bool {
    project_fields_require_pc_workspace(
        &project.source_type,
        project.node_id.as_deref(),
        project.workspace_path.as_deref(),
    )
}

pub(crate) fn project_fields_require_pc_workspace(
    source_type: &str,
    node_id: Option<&str>,
    workspace_path: Option<&str>,
) -> bool {
    if source_type == "pc_managed" {
        return true;
    }
    if node_id
        .map(str::trim)
        .is_some_and(|value| !value.is_empty())
    {
        return true;
    }
    workspace_path
        .map(str::trim)
        .is_some_and(path_looks_windows_workspace)
}

fn path_looks_windows_workspace(path: &str) -> bool {
    let value = path.trim();
    if value.starts_with("\\\\") || value.starts_with("//") {
        return true;
    }
    let bytes = value.as_bytes();
    bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && (bytes[2] == b'\\' || bytes[2] == b'/')
}

#[cfg(test)]
#[path = "agent_pc_workspace_tests.rs"]
mod tests;
