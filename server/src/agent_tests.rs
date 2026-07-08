use super::{
    cli_lists_intersect, pc_workspace_inspect_error_allows_bound_dispatch,
    pc_workspace_inspect_problem, pc_workspace_inspect_usable,
    pc_workspace_inspect_usable_for_route, public_dev_runtime_ready_for_route,
    requires_project_workflow_for_message, route_allows_public_dev_node,
    AUTO_BOUND_PC_NODE_RECONNECT_WAIT_SECS, BOUND_PC_NODE_RECONNECT_WAIT_SECS,
};
use homecli_proto::{NodeDevRuntimeProfile, ProjectWorkspaceInspectStatus};
use std::path::Path;

use crate::{node_runtime::NodeRuntime, pc_agent_runtime_choice::PcRuntimeRoutePreference};

#[test]
fn casual_greeting_does_not_require_project_workflow() {
    assert!(!requires_project_workflow_for_message(
        "你好",
        Path::new("C:/tmp/project")
    ));
    assert!(!requires_project_workflow_for_message(
        "你好吗？",
        Path::new("C:/tmp/project")
    ));
}

#[test]
fn app_change_requires_project_workflow() {
    assert!(requires_project_workflow_for_message(
        "帮我在首页加一个按钮",
        Path::new("C:/tmp/project")
    ));
}

#[test]
fn open_idea_stays_in_chat_route() {
    assert!(!requires_project_workflow_for_message(
        "我有一个想法",
        Path::new("C:/tmp/project")
    ));
}

#[test]
fn completed_project_install_question_stays_in_chat_route() {
    assert!(!requires_project_workflow_for_message(
        "完成的项目我在哪里下载安装呢",
        Path::new("C:/tmp/project__demo")
    ));
}

#[test]
fn pc_workspace_inspect_requires_existing_dir_and_cli() {
    let mut status = inspect_status();
    assert!(pc_workspace_inspect_usable(&status));

    status.path_exists = false;
    assert!(!pc_workspace_inspect_usable(&status));
    assert_eq!(
        pc_workspace_inspect_problem(&status),
        "workspace_path_missing"
    );

    status = inspect_status();
    status.codex_available = false;
    status.copilot_available = false;
    assert!(!pc_workspace_inspect_usable(&status));
    assert_eq!(pc_workspace_inspect_problem(&status), "cli_unavailable");
}

#[test]
fn pc_workspace_route_b_and_route_c_only_require_existing_dir() {
    let mut status = inspect_status();
    status.codex_available = false;
    status.copilot_available = false;

    assert!(!pc_workspace_inspect_usable_for_route(
        &status,
        Some(PcRuntimeRoutePreference::RouteA)
    ));
    assert!(pc_workspace_inspect_usable_for_route(
        &status,
        Some(PcRuntimeRoutePreference::RouteB)
    ));
    assert!(pc_workspace_inspect_usable_for_route(
        &status,
        Some(PcRuntimeRoutePreference::RouteC)
    ));
    assert!(pc_workspace_inspect_usable_for_route(
        &status,
        Some(PcRuntimeRoutePreference::RouteC2)
    ));
    assert!(!pc_workspace_inspect_usable_for_route(
        &status,
        Some(PcRuntimeRoutePreference::RouteC3)
    ));
}

#[test]
fn pc_workspace_inspect_timeout_keeps_bound_node() {
    assert!(pc_workspace_inspect_error_allows_bound_dispatch(
        "project workspace inspect timeout (3s)"
    ));
    assert!(pc_workspace_inspect_error_allows_bound_dispatch(
        "PC 节点创建项目工作区超时（30 秒）"
    ));
    assert!(!pc_workspace_inspect_error_allows_bound_dispatch(
        "workspace path does not exist"
    ));
}

#[test]
fn bound_pc_node_reconnect_window_covers_server_restart() {
    assert!(BOUND_PC_NODE_RECONNECT_WAIT_SECS >= 90);
}

#[test]
fn auto_bound_pc_node_reconnect_window_fails_fast() {
    assert!(AUTO_BOUND_PC_NODE_RECONNECT_WAIT_SECS <= 15);
    assert!(AUTO_BOUND_PC_NODE_RECONNECT_WAIT_SECS < BOUND_PC_NODE_RECONNECT_WAIT_SECS);
}

#[test]
fn public_dev_nodes_are_auto_and_route_c_shared() {
    assert!(route_allows_public_dev_node(None));
    assert!(!route_allows_public_dev_node(Some(
        PcRuntimeRoutePreference::RouteA
    )));
    assert!(!route_allows_public_dev_node(Some(
        PcRuntimeRoutePreference::RouteB
    )));
    assert!(route_allows_public_dev_node(Some(
        PcRuntimeRoutePreference::RouteC2
    )));
    assert!(route_allows_public_dev_node(Some(
        PcRuntimeRoutePreference::RouteC3
    )));
}

#[test]
fn shared_cli_allowlist_matches_case_insensitively() {
    assert!(cli_lists_intersect(
        &["codex".to_string()],
        &["Codex".to_string(), "copilot".to_string()]
    ));
    assert!(!cli_lists_intersect(
        &["codex".to_string()],
        &["gemini".to_string()]
    ));
}

#[test]
fn public_dev_auto_route_uses_remote_cli_readiness() {
    let mut runtime = test_public_dev_runtime(vec!["Codex".to_string()]);
    runtime.dev_runtime = Some(NodeDevRuntimeProfile {
        route_a_ready: true,
        api_runtime_ready: false,
        ..Default::default()
    });

    assert!(public_dev_runtime_ready_for_route(
        None,
        &["codex".to_string()],
        &runtime
    ));
    assert!(!public_dev_runtime_ready_for_route(
        Some(PcRuntimeRoutePreference::RouteC2),
        &["codex".to_string()],
        &runtime
    ));
    assert!(!public_dev_runtime_ready_for_route(
        Some(PcRuntimeRoutePreference::RouteA),
        &["codex".to_string()],
        &runtime
    ));
}

fn inspect_status() -> ProjectWorkspaceInspectStatus {
    ProjectWorkspaceInspectStatus {
        workspace_path: r"D:\rust\active-projects\elon cli".to_string(),
        path_exists: true,
        is_dir: true,
        is_git_worktree: true,
        git_branch: Some("main".to_string()),
        git_head: Some("2580208".to_string()),
        git_remote_origin: Some("git@github.com:ElonQian1/Elon.git".to_string()),
        has_uncommitted_changes: false,
        uncommitted_count: Some(0),
        disk_free_bytes: Some(10 * 1024 * 1024 * 1024),
        codex_available: true,
        copilot_available: false,
    }
}

fn test_public_dev_runtime(allowed_clis: Vec<String>) -> NodeRuntime {
    NodeRuntime {
        node_id: "node-public".to_string(),
        owner_user_id: "user-provider".to_string(),
        label: "Public PC".to_string(),
        device_name: Some("Public PC".to_string()),
        install_id: None,
        public_dev_enabled: true,
        public_dev_allowed_clis: vec!["codex".to_string()],
        public_dev_permission_level: "project_write".to_string(),
        last_handshake_at: Some("2026-07-06T00:00:00Z".to_string()),
        last_handshake_agent_version: Some("1.0.0".to_string()),
        last_handshake_allowed_clis: allowed_clis.clone(),
        last_handshake_route_a_ready: true,
        last_handshake_api_runtime_ready: false,
        last_handshake_server_runtime_ready: false,
        last_handshake_ai_cli_ready: true,
        hardware: None,
        storage: None,
        dev_runtime: None,
        lifecycle: None,
        display_name: "Public PC".to_string(),
        short_id: "public".to_string(),
        models: Vec::new(),
        allowed_clis,
        allowed_cwds: Vec::new(),
        agent_version: Some("1.0.0".to_string()),
        connected_at: 1,
        created_at: "2026-07-06T00:00:00Z".to_string(),
        online: true,
        registry_online: true,
        cli_connected: true,
        project_count: 0,
    }
}
