use super::*;
use uuid::Uuid;

fn temp_store() -> Store {
    let path = std::env::temp_dir().join(format!(
        "elon_workspace_health_{}.db",
        Uuid::new_v4().simple()
    ));
    Store::open(&path).expect("store should open")
}

#[test]
fn health_snapshot_upsert_keeps_latest_project_state() {
    let store = temp_store();
    let user = store
        .create_user("workspace-health@example.com", "secret1", None, None)
        .expect("user should be created");
    let project = store
        .create_project(&user.id, "健康快照项目", None, Some("android"))
        .expect("project should be created")
        .project;
    let warnings = vec!["PC 工作区目录不存在".to_string()];

    let snapshot = store
        .upsert_project_workspace_health_snapshot(ProjectWorkspaceHealthSnapshotWrite {
            project_id: &project.id,
            node_id: Some("node-a"),
            workspace_path: Some("D:/projects/demo"),
            can_run_on_pc: false,
            verified_can_run_on_pc: Some(false),
            health_label: "目录丢失",
            health_tone: "bad",
            recommended_action: "重新创建目录",
            warnings: &warnings,
            live_inspect: Some(&inspect_status(false, false, false, true)),
            inspect_error: None,
        })
        .expect("snapshot should upsert");

    assert_eq!(snapshot.project_id, project.id);
    assert_eq!(snapshot.node_id.as_deref(), Some("node-a"));
    assert_eq!(snapshot.disk_free_bytes, Some(8 * 1024 * 1024 * 1024));
    assert_eq!(snapshot.path_exists, Some(false));
    assert_eq!(snapshot.warnings, warnings);

    let latest_for_node = store
        .latest_workspace_health_snapshot_for_node("node-a")
        .expect("node snapshot should query")
        .expect("node snapshot should exist");
    assert_eq!(latest_for_node.project_id, project.id);
}

fn inspect_status(
    path_exists: bool,
    is_dir: bool,
    is_git_worktree: bool,
    codex_available: bool,
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
        disk_free_bytes: Some(8 * 1024 * 1024 * 1024),
        codex_available,
        copilot_available: false,
    }
}
