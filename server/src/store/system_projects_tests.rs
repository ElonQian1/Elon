use super::*;
use uuid::Uuid;

fn temp_store() -> Store {
    let path = std::env::temp_dir().join(format!(
        "elon_system_projects_{}.db",
        Uuid::new_v4().simple()
    ));
    Store::open(&path).expect("store should open")
}

#[test]
fn system_projects_are_idempotent_and_distinct() {
    let store = temp_store();
    let user = store
        .create_user("system-projects@example.com", "secret1", None, None)
        .expect("user should be created");

    let (phone_id, phone_created) = store
        .ensure_balloon_project_for_user(&user.id)
        .expect("phone project should be created");
    let (phone_id_again, phone_created_again) = store
        .ensure_balloon_project_for_user(&user.id)
        .expect("phone project should be reused");
    let (chat_id, chat_created) = store
        .ensure_chat_memory_project_for_user(&user.id)
        .expect("chat memory project should be created");

    assert!(phone_created);
    assert!(!phone_created_again);
    assert!(chat_created);
    assert_eq!(phone_id, phone_id_again);
    assert_ne!(phone_id, chat_id);

    let names = store
        .list_projects_for_user(&user.id)
        .expect("projects should list")
        .into_iter()
        .map(|project| project.name)
        .collect::<Vec<_>>();
    assert!(names.contains(&PHONE_CONTROL_PROJECT_NAME.to_string()));
    assert!(names.contains(&CHAT_MEMORY_PROJECT_NAME.to_string()));
}

#[test]
fn ensure_system_project_normalizes_legacy_metadata() {
    let store = temp_store();
    let user = store
        .create_user("legacy-system-projects@example.com", "secret1", None, None)
        .expect("user should be created");
    let legacy_id = new_id("prj");
    let now_str = now();
    let conn = store.conn().expect("conn should open");
    conn.execute(
        "INSERT INTO projects (
                id, name, description, workspace_key, template, source_type,
                status, created_by, is_public, join_mode, created_at, updated_at
             )
             VALUES (?1, ?2, '旧数据', ?1, 'android', 'template',
                     'active', ?3, 1, 'open', ?4, ?4)",
        params![legacy_id, PHONE_CONTROL_PROJECT_NAME, user.id, now_str],
    )
    .expect("legacy project should insert");
    conn.execute(
        "INSERT INTO project_members (project_id, user_id, role, created_at)
             VALUES (?1, ?2, 'owner', ?3)",
        params![legacy_id, user.id, now_str],
    )
    .expect("legacy membership should insert");
    drop(conn);

    let (project_id, created) = store
        .ensure_balloon_project_for_user(&user.id)
        .expect("system project should be normalized");

    assert_eq!(project_id, legacy_id);
    assert!(!created);

    let access = store
        .get_project_access(&user.id, &project_id)
        .expect("project should still exist");
    assert_eq!(access.source_type, PHONE_CONTROL_SOURCE_TYPE);
}
