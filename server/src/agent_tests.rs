use super::{
    pc_workspace_inspect_error_allows_bound_dispatch, pc_workspace_inspect_problem,
    pc_workspace_inspect_usable, requires_project_workflow_for_message,
    BOUND_PC_NODE_RECONNECT_WAIT_SECS,
};
use homecli_proto::ProjectWorkspaceInspectStatus;
use std::path::Path;

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
