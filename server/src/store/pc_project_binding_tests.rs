use super::*;
use uuid::Uuid;

fn temp_store() -> Store {
    let path = std::env::temp_dir().join(format!(
        "elon-member-pc-binding-{}.db",
        Uuid::new_v4().simple()
    ));
    Store::open(&path).expect("store should open")
}

#[test]
fn editor_binding_overlays_only_the_editors_workspace() {
    let store = temp_store();
    let owner = store
        .create_user("pc-owner@example.com", "secret1", None, None)
        .expect("owner should be created");
    let editor = store
        .create_user("pc-editor@example.com", "secret1", None, None)
        .expect("editor should be created");
    let project = store
        .create_project(&owner.id, "Collaborative PC Project", None, None)
        .expect("project should be created")
        .project;
    store
        .add_project_member_by_account(&project.id, &editor.id, "editor")
        .expect("editor should be added");
    store
        .bind_project_to_pc_workspace(
            &owner.id,
            &project.id,
            "D:/owner/project",
            "node-owner",
            Some("owner-head"),
            Some("git@example.com:owner/project.git"),
            Some("main"),
        )
        .expect("owner workspace should bind");

    let rebound = store
        .bind_project_member_to_pc_workspace(
            &editor.id,
            &project.id,
            "E:/editor/project",
            "node-editor",
            Some("editor-head"),
            Some("git@example.com:owner/project.git"),
            Some("main"),
        )
        .expect("editor workspace should bind");

    assert_eq!(rebound.role, "editor");
    assert_eq!(rebound.node_id.as_deref(), Some("node-editor"));
    assert_eq!(rebound.workspace_path.as_deref(), Some("E:/editor/project"));

    let editor_access = store
        .get_project_access(&editor.id, &project.id)
        .expect("editor access should resolve");
    assert_eq!(editor_access.node_id.as_deref(), Some("node-editor"));
    assert_eq!(
        editor_access.workspace_path.as_deref(),
        Some("E:/editor/project")
    );

    let owner_access = store
        .get_project_access(&owner.id, &project.id)
        .expect("owner access should resolve");
    assert_eq!(owner_access.node_id.as_deref(), Some("node-owner"));
    assert_eq!(
        owner_access.workspace_path.as_deref(),
        Some("D:/owner/project")
    );
}
