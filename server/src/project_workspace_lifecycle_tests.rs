use super::*;

#[test]
fn system_archive_is_personal_archive_without_recovery_actions() {
    let lifecycle = workspace_lifecycle("system_archive", None, None, false, false, None, None, 0);

    assert_eq!(lifecycle.health_label, "个人归档");
    assert_eq!(lifecycle.health_tone, "neutral");
    assert!(lifecycle.recovery_actions.is_empty());
}

#[test]
fn pc_workspace_missing_directory_recommends_recreate_or_migrate() {
    let inspect = inspect_status(false, false, false, true, Some(8 * 1024 * 1024 * 1024));
    let lifecycle = workspace_lifecycle(
        "pc_node_workspace",
        Some("node-1"),
        Some("D:/projects/demo"),
        true,
        true,
        Some(false),
        Some(&inspect),
        1,
    );

    assert_eq!(lifecycle.health_label, "目录丢失");
    assert_eq!(lifecycle.health_tone, "bad");
    assert!(lifecycle
        .recovery_actions
        .iter()
        .any(|action| action.key == "recreate_workspace" && action.available));
}

#[test]
fn online_clean_pc_workspace_is_ready() {
    let inspect = inspect_status(true, true, true, true, Some(8 * 1024 * 1024 * 1024));
    let lifecycle = workspace_lifecycle(
        "pc_node_workspace",
        Some("node-1"),
        Some("D:/projects/demo"),
        true,
        true,
        Some(true),
        Some(&inspect),
        0,
    );

    assert_eq!(lifecycle.health_label, "PC 可运行");
    assert_eq!(lifecycle.health_tone, "ok");
    assert!(lifecycle.recovery_actions.is_empty());
}

fn inspect_status(
    path_exists: bool,
    is_dir: bool,
    is_git_worktree: bool,
    codex_available: bool,
    disk_free_bytes: Option<u64>,
) -> ProjectWorkspaceInspectStatus {
    ProjectWorkspaceInspectStatus {
        workspace_path: "D:/projects/demo".to_string(),
        path_exists,
        is_dir,
        is_git_worktree,
        git_branch: Some("main".to_string()),
        git_head: Some("abc1234".to_string()),
        git_remote_origin: None,
        has_uncommitted_changes: false,
        uncommitted_count: Some(0),
        disk_free_bytes,
        codex_available,
        copilot_available: false,
    }
}
