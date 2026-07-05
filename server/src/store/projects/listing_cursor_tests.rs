use super::super::Store;
use rusqlite::params;
use uuid::Uuid;

fn temp_store() -> Store {
    let path = std::env::temp_dir().join(format!(
        "elon_project_store_cursor_{}.db",
        Uuid::new_v4().simple()
    ));
    Store::open(&path).expect("store should open")
}

#[test]
fn public_projects_cursor_paginates_by_updated_keyset() {
    let store = temp_store();
    let owner = store
        .create_user("cursor-owner@example.com", "secret1", None, None)
        .expect("owner should be created");
    let first = store
        .create_project(&owner.id, "Cursor First", None, None)
        .expect("first project should be created")
        .project;
    let second = store
        .create_project(&owner.id, "Cursor Second", None, None)
        .expect("second project should be created")
        .project;
    let third = store
        .create_project(&owner.id, "Cursor Third", None, None)
        .expect("third project should be created")
        .project;

    for (project, stamp) in [
        (&first, "2026-07-01T00:00:00Z"),
        (&second, "2026-07-02T00:00:00Z"),
        (&third, "2026-07-03T00:00:00Z"),
    ] {
        store
            .set_project_visibility(&project.id, true, "open")
            .expect("project should become public");
        store
            .conn()
            .expect("db connection")
            .execute(
                "UPDATE projects SET created_at = ?2, updated_at = ?2 WHERE id = ?1",
                params![project.id, stamp],
            )
            .expect("project timestamps should update");
    }

    let page_one = store
        .list_public_projects_cursor_page_for_viewer(None, None, None, None, 2, None, None)
        .expect("first cursor page should list");
    assert_eq!(
        page_one
            .projects
            .iter()
            .map(|project| project.id.as_str())
            .collect::<Vec<_>>(),
        vec![third.id.as_str(), second.id.as_str()]
    );
    assert!(page_one.has_more);
    let cursor = page_one
        .next_cursor
        .as_deref()
        .expect("first page should expose a cursor");

    let page_two = store
        .list_public_projects_cursor_page_for_viewer(None, None, None, None, 2, Some(cursor), None)
        .expect("second cursor page should list");
    assert_eq!(page_two.projects.len(), 1);
    assert_eq!(page_two.projects[0].id, first.id);
    assert!(!page_two.has_more);
    assert!(page_two.next_cursor.is_none());
}

#[test]
fn public_projects_cursor_paginates_member_sort() {
    let store = temp_store();
    let owner = store
        .create_user("cursor-members-owner@example.com", "secret1", None, None)
        .expect("owner should be created");
    let member_one = store
        .create_user("cursor-members-one@example.com", "secret1", None, None)
        .expect("member one should be created");
    let member_two = store
        .create_user("cursor-members-two@example.com", "secret1", None, None)
        .expect("member two should be created");
    let hot = store
        .create_project(&owner.id, "Cursor Hot", None, None)
        .expect("hot project should be created")
        .project;
    let warm = store
        .create_project(&owner.id, "Cursor Warm", None, None)
        .expect("warm project should be created")
        .project;
    let cold = store
        .create_project(&owner.id, "Cursor Cold", None, None)
        .expect("cold project should be created")
        .project;

    for project in [&hot, &warm, &cold] {
        store
            .set_project_visibility(&project.id, true, "open")
            .expect("project should become public");
    }
    store
        .join_project(&member_one.id, &hot.id)
        .expect("member one should join hot project");
    store
        .join_project(&member_two.id, &hot.id)
        .expect("member two should join hot project");
    store
        .join_project(&member_one.id, &warm.id)
        .expect("member one should join warm project");

    let page_one = store
        .list_public_projects_cursor_page_for_viewer(
            None,
            None,
            None,
            Some("members"),
            1,
            None,
            None,
        )
        .expect("first member page should list");
    assert_eq!(page_one.projects[0].id, hot.id);
    assert!(page_one.has_more);

    let page_two = store
        .list_public_projects_cursor_page_for_viewer(
            None,
            None,
            None,
            Some("members"),
            2,
            page_one.next_cursor.as_deref(),
            None,
        )
        .expect("second member page should list");
    assert_eq!(
        page_two
            .projects
            .iter()
            .map(|project| project.id.as_str())
            .collect::<Vec<_>>(),
        vec![warm.id.as_str(), cold.id.as_str()]
    );
    assert!(!page_two.has_more);
}
