use super::*;
use uuid::Uuid;

fn temp_store() -> Store {
    let path = std::env::temp_dir().join(format!(
        "elon_project_invites_{}.db",
        Uuid::new_v4().simple()
    ));
    Store::open(&path).expect("store should open")
}

#[test]
fn project_invite_link_can_be_created_and_used() {
    let store = temp_store();
    let owner = store
        .create_user("invite-owner@example.com", "secret1", None, None)
        .expect("owner should be created");
    let guest = store
        .create_user("invite-guest@example.com", "secret1", None, None)
        .expect("guest should be created");
    let extra = store
        .create_user("invite-extra@example.com", "secret1", None, None)
        .expect("extra guest should be created");
    let project = store
        .create_project(&owner.id, "Invite Link Project", None, None)
        .expect("project should be created");

    let invite = store
        .create_project_invite_link(
            &project.project.id,
            &owner.id,
            "member",
            Some(24),
            Some(1),
            false,
        )
        .expect("invite should be created");
    assert_eq!(invite.role, "member");
    assert_eq!(invite.use_count, 0);

    let preview = store
        .get_project_invite_preview(&invite.code)
        .expect("preview should load");
    assert_eq!(preview.project_id, project.project.id);

    let (already_member, joined_preview) = store
        .join_project_by_invite_link(&guest.id, &invite.code)
        .expect("guest should join");
    assert!(!already_member);
    assert_eq!(joined_preview.project_id, project.project.id);

    let used = store
        .get_project_invite_link_by_code(&invite.code)
        .expect("invite should reload");
    assert_eq!(used.use_count, 1);

    let error = store
        .join_project_by_invite_link(&extra.id, &invite.code)
        .expect_err("second guest should hit max uses");
    assert!(error.to_string().contains("使用次数已达上限"));
}
