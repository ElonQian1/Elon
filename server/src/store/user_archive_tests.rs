use super::*;
use crate::store::{
    is_system_project_source_type, CHAT_MEMORY_PROJECT_NAME, PHONE_CONTROL_PROJECT_NAME,
};
use uuid::Uuid;

fn temp_store() -> Store {
    let path =
        std::env::temp_dir().join(format!("elon_user_archive_{}.db", Uuid::new_v4().simple()));
    Store::open(&path).expect("store should open")
}

#[test]
fn archive_lists_system_and_regular_projects() {
    let store = temp_store();
    let user = store
        .create_user("archive@example.com", "secret1", Some("归档用户"), None)
        .expect("user should be created");

    store
        .ensure_balloon_project_for_user(&user.id)
        .expect("phone project should exist");
    store
        .ensure_chat_memory_project_for_user(&user.id)
        .expect("chat project should exist");
    store
        .create_project(&user.id, "工作台", Some("PC 项目"), Some("android"))
        .expect("regular project should create");

    let archive = store
        .list_archive_projects_for_user(&user.id)
        .expect("archive should load");

    assert_eq!(archive.len(), 3);
    let names = archive
        .iter()
        .map(|item| item.project.name.as_str())
        .collect::<Vec<_>>();
    assert!(names.contains(&PHONE_CONTROL_PROJECT_NAME));
    assert!(names.contains(&CHAT_MEMORY_PROJECT_NAME));
    assert!(names.contains(&"工作台"));

    let system_projects = archive
        .iter()
        .filter(|item| is_system_project_source_type(&item.project.source_type))
        .collect::<Vec<_>>();
    assert_eq!(system_projects.len(), 2);
    assert!(system_projects
        .iter()
        .all(|item| item.workspace_kind == "system_archive"));
    assert!(system_projects
        .iter()
        .all(|item| item.owner_account == "系统"));
    assert!(system_projects
        .iter()
        .all(|item| item.project_origin_type == "system"));
    assert!(system_projects
        .iter()
        .all(|item| item.project_origin_label == "系统创建"));
    assert!(system_projects
        .iter()
        .all(|item| item.conversation_count == 0));

    let regular = archive
        .iter()
        .find(|item| item.project.name == "工作台")
        .expect("regular project should be present");
    assert_eq!(regular.owner_account, "归档用户");
    assert_eq!(regular.project_origin_type, "self");
    assert_eq!(regular.project_origin_label, "我创建");
}

#[test]
fn archive_marks_admin_created_member_projects() {
    let store = temp_store();
    let admin = store
        .create_user(
            "admin-created@example.com",
            "secret1",
            Some("Admin"),
            Some("admin"),
        )
        .expect("admin should be created");
    let user = store
        .create_user(
            "member-created@example.com",
            "secret1",
            Some("Member"),
            None,
        )
        .expect("member should be created");
    let project = store
        .create_project(
            &admin.id,
            "管理员项目",
            Some("由管理员创建"),
            Some("android"),
        )
        .expect("admin project should create")
        .project;
    store
        .add_project_member_by_account(&project.id, "member-created@example.com", "member")
        .expect("member should be added");

    let archive = store
        .list_archive_projects_for_user(&user.id)
        .expect("archive should load");
    let item = archive
        .iter()
        .find(|item| item.project.id == project.id)
        .expect("admin-created project should be visible");

    assert_eq!(item.project_origin_type, "admin");
    assert_eq!(item.project_origin_label, "管理员创建");
}
