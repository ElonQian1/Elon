use crate::{ai_cli, pc_agent_runtime_choice::PcRuntimeRoutePreference, store::ProjectAccess};

pub(crate) fn should_attempt_pc_apk_sync(project: &ProjectAccess, user_message: &str) -> bool {
    project_template_is_android(&project.template) || ai_cli::looks_like_android_task(user_message)
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
mod tests {
    use super::{
        pc_cli_chat_requested, pc_cli_chat_route_label, project_chat_should_use_pc_cli,
        project_cli_runtime_permission, project_fields_require_pc_workspace,
        should_attempt_pc_apk_sync,
    };
    use crate::pc_agent_runtime_choice::PcRuntimeRoutePreference;
    use crate::store::ProjectAccess;

    #[test]
    fn explicit_pc_cli_routes_are_cli_first_for_chat() {
        assert!(pc_cli_chat_requested(Some(
            PcRuntimeRoutePreference::RouteA
        )));
        assert!(pc_cli_chat_requested(Some(
            PcRuntimeRoutePreference::RouteC3
        )));
        assert!(!pc_cli_chat_requested(Some(
            PcRuntimeRoutePreference::RouteC
        )));
        assert!(!pc_cli_chat_requested(None));

        assert!(!project_chat_should_use_pc_cli(None, None, false));
        assert!(project_chat_should_use_pc_cli(
            Some(PcRuntimeRoutePreference::RouteA),
            None,
            false
        ));
        assert!(project_chat_should_use_pc_cli(None, Some("codex"), true));
        assert!(!project_chat_should_use_pc_cli(
            Some(PcRuntimeRoutePreference::RouteC),
            None,
            false
        ));
        assert!(!project_chat_should_use_pc_cli(None, Some("api"), false));
    }

    #[test]
    fn pc_cli_chat_labels_match_user_selected_route() {
        assert!(pc_cli_chat_route_label(Some(PcRuntimeRoutePreference::RouteA)).contains("AI"));
        assert!(pc_cli_chat_route_label(Some(PcRuntimeRoutePreference::RouteC3)).contains("Codex"));
    }

    #[test]
    fn android_template_pc_project_attempts_apk_sync_for_ui_changes() {
        let project = pc_project("prj_android", "android_kotlin", Some("node-local"));

        assert!(should_attempt_pc_apk_sync(&project, "change button color"));
    }

    #[test]
    fn pc_managed_projects_use_full_access_for_cli() {
        let project = pc_project("prj_pc", "android_kotlin", Some("node-local"));

        assert_eq!(project_cli_runtime_permission(&project), "full_access");
    }

    #[test]
    fn pc_managed_projects_require_pc_workspace_route() {
        assert!(project_fields_require_pc_workspace(
            "pc_managed",
            None,
            Some("/srv/elon/project")
        ));
    }

    #[test]
    fn bound_node_projects_require_pc_workspace_route() {
        assert!(project_fields_require_pc_workspace(
            "local_path",
            Some("node-local"),
            Some("/srv/elon/project")
        ));
    }

    #[test]
    fn windows_local_paths_require_pc_workspace_route() {
        assert!(project_fields_require_pc_workspace(
            "local_path",
            None,
            Some(r"D:\rust\active-projects\elon cli")
        ));
        assert!(project_fields_require_pc_workspace(
            "local_path",
            None,
            Some("D:/rust/active-projects/elon cli")
        ));
    }

    #[test]
    fn unc_paths_require_pc_workspace_route() {
        assert!(project_fields_require_pc_workspace(
            "local_path",
            None,
            Some(r"\\workstation\repos\elon")
        ));
        assert!(project_fields_require_pc_workspace(
            "local_path",
            None,
            Some("//workstation/repos/elon")
        ));
    }

    #[test]
    fn server_local_paths_can_still_use_server_git_route() {
        assert!(!project_fields_require_pc_workspace(
            "local_path",
            None,
            Some("/srv/elon/project")
        ));
    }

    fn pc_project(id: &str, template: &str, node_id: Option<&str>) -> ProjectAccess {
        ProjectAccess {
            id: id.into(),
            name: "PC App".into(),
            workspace_key: id.into(),
            template: template.into(),
            source_type: "pc_managed".into(),
            repo_url: None,
            branch: None,
            workspace_path: Some(format!(
                r"C:\Users\Administrator\Elon\workspaces\usr\{id}\repo"
            )),
            node_id: node_id.map(str::to_string),
            storage_node_id: None,
            storage_repo_path: None,
            storage_repo_url: None,
            storage_worktree_path: None,
            storage_status: "none".into(),
            role: "owner".into(),
            status: "active".into(),
            runtime_permission: "project_write".into(),
        }
    }
}
