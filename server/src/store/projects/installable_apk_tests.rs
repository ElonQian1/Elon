use super::super::Store;

#[test]
fn public_project_filters_include_task_and_landing_apks() {
    let path = std::env::temp_dir().join(format!(
        "elon_project_store_apk_filter_{}.db",
        uuid::Uuid::new_v4().simple()
    ));
    let store = Store::open(&path).expect("store should open");
    let owner = store
        .create_user("apk-filter-owner@example.com", "secret1", None, None)
        .expect("owner should be created");
    let task_project = store
        .create_project(&owner.id, "Task APK", None, None)
        .expect("task project should be created")
        .project;
    let landing_project = store
        .create_project(&owner.id, "Landing APK", None, None)
        .expect("landing project should be created")
        .project;
    let empty_project = store
        .create_project(&owner.id, "No APK", None, None)
        .expect("empty project should be created")
        .project;
    for project in [&task_project, &landing_project, &empty_project] {
        store
            .set_project_visibility(&project.id, true, "open")
            .expect("project should become public");
    }

    let task = store
        .create_task(&task_project.id, &owner.id, Some("conv"), "build apk")
        .expect("task should be created");
    store
        .finish_task(
            &task,
            "done",
            Some("done"),
            Some("https://example.test/task.apk"),
            None,
        )
        .expect("task should finish with apk url");
    store
        .update_project_landing_snapshot(
            &owner.id,
            &landing_project.id,
            &serde_json::json!({
                "title": "Landing APK",
                "downloads": {
                    "android": {
                        "status": "available",
                        "url": "https://example.test/landing.apk"
                    }
                }
            }),
        )
        .expect("landing snapshot should update");

    let installable = store
        .list_public_projects(None, None, Some(true), None, 10, 0)
        .expect("installable projects should list");
    assert_eq!(installable.len(), 2);
    assert!(installable
        .iter()
        .any(|project| project.id == task_project.id));
    assert!(installable
        .iter()
        .any(|project| project.id == landing_project.id));
    assert_eq!(
        store
            .count_public_projects(None, None, Some(true))
            .expect("installable count should work"),
        2
    );

    let missing = store
        .list_public_projects(None, None, Some(false), None, 10, 0)
        .expect("projects without apk should list");
    assert_eq!(missing.len(), 1);
    assert_eq!(missing[0].id, empty_project.id);
}
