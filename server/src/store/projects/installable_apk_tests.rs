use super::super::Store;
use crate::{
    project_releases::admission::validated_apk_for_test,
    store::project_releases::ProjectReleaseWrite,
};

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

#[test]
fn official_quant_filter_ignores_legacy_downloads_until_an_admitted_release_exists() {
    let path = std::env::temp_dir().join(format!(
        "elon_official_quant_apk_filter_{}.db",
        uuid::Uuid::new_v4().simple()
    ));
    let store = Store::open(&path).expect("store should open");
    let owner = store
        .create_user("quant-apk-filter-owner@example.com", "secret1", None, None)
        .expect("owner should be created");
    let now = "2026-09-05T00:00:00Z";
    let legacy_landing = serde_json::json!({
        "downloads": [{
            "platform": "android",
            "status": "available",
            "url": "http://old.example/yilong-quant-0.2.0.apk"
        }]
    })
    .to_string();
    let conn = store.conn().expect("connection should open");
    conn.execute(
        "INSERT INTO projects (
           id, name, workspace_key, template, source_type, status, created_by,
           created_at, updated_at, is_public, join_mode, landing_json
         ) VALUES (
           'yilong-quant', '一龙量化交易', 'yilong-quant', 'blank', 'template',
           'active', ?1, ?2, ?2, 1, 'open', ?3
         )",
        rusqlite::params![owner.id, now, legacy_landing],
    )
    .expect("official project should be inserted");
    drop(conn);

    assert!(store
        .list_public_projects(None, None, Some(true), None, 10, 0)
        .expect("installable filter should run")
        .is_empty());
    assert_eq!(
        store
            .count_public_projects(None, None, Some(false))
            .expect("non-installable count should run"),
        1
    );

    let sha256 = "5".repeat(64);
    let source_git_sha = "a".repeat(40);
    let proof = validated_apk_for_test(&sha256, 1024);
    store
        .create_project_release_with_admission(
            ProjectReleaseWrite {
                id: Some("rel_quant_v5"),
                project_id: "yilong-quant",
                task_id: None,
                uploaded_by: Some(&owner.id),
                version_name: Some("0.5.0"),
                package_name: Some("com.elon.quant"),
                version_code: Some(5),
                channel: Some("paper"),
                status: Some("published"),
                apk_url: "http://example.test/api/projects/yilong-quant/download/latest.apk",
                file_name: "yilong-quant-0.5.0.apk",
                file_path: Some("C:/managed/yilong-quant-0.5.0.apk"),
                sha256: Some(&sha256),
                size_bytes: Some(1024),
                changelog: None,
                build_started_at: None,
                source_git_sha: Some(&source_git_sha),
                source_worktree: None,
                metadata_json: None,
            },
            Some(&proof),
        )
        .expect("admitted release should be created");

    let installable = store
        .list_public_projects(None, None, Some(true), None, 10, 0)
        .expect("installable filter should run");
    assert_eq!(installable.len(), 1);
    assert_eq!(installable[0].id, "yilong-quant");
    assert_eq!(
        store
            .count_public_projects(None, None, Some(true))
            .expect("installable count should run"),
        1
    );
}
