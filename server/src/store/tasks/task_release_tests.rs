use crate::store::Store;
use uuid::Uuid;

fn temp_store() -> Store {
    let path = std::env::temp_dir().join(format!(
        "elon_store_release_test_{}.db",
        Uuid::new_v4().simple()
    ));
    Store::open(&path).expect("store should open")
}

#[test]
fn finish_task_binds_pre_synced_project_release() {
    let store = temp_store();
    let user = store
        .create_user("release-bind@example.com", "secret1", None, None)
        .expect("user should be created");
    let project = store
        .create_project(&user.id, "Release Bind", None, None)
        .expect("project should be created")
        .project;
    let task_id = store
        .create_task(&project.id, &user.id, Some("conv-release"), "build apk")
        .expect("task should be created");
    let apk_url = "https://example.test/api/projects/prj/download/latest.apk";
    store
        .create_project_release(crate::store::project_releases::ProjectReleaseWrite {
            id: Some("rel_pre_synced"),
            project_id: &project.id,
            task_id: None,
            uploaded_by: None,
            version_name: Some("PC node debug build"),
            package_name: Some("com.example.releasebind"),
            version_code: Some(7),
            channel: Some("pc_node"),
            status: Some("published"),
            apk_url,
            file_name: "app-debug.apk",
            file_path: Some("D:/elon/project/artifacts/app-debug.apk"),
            sha256: Some("abc123"),
            size_bytes: Some(3),
            changelog: Some("synced"),
            build_started_at: Some("2026-07-05T10:00:00Z"),
            source_git_sha: Some("abc1234"),
            source_worktree: None,
            metadata_json: None,
        })
        .expect("pre-synced release should insert");

    store
        .finish_task(&task_id, "done", Some("done"), Some(apk_url), None)
        .expect("task should finish");
    let releases = store
        .list_project_releases(&project.id, 10)
        .expect("releases should list");

    assert_eq!(releases.len(), 1);
    assert_eq!(releases[0].task_id.as_deref(), Some(task_id.as_str()));
    assert_eq!(
        releases[0].file_path.as_deref(),
        Some("D:/elon/project/artifacts/app-debug.apk")
    );
    assert_eq!(releases[0].channel, "pc_node");
    assert_eq!(releases[0].release_number, Some(1));
    assert_eq!(
        releases[0].package_name.as_deref(),
        Some("com.example.releasebind")
    );
    assert_eq!(releases[0].version_code, Some(7));
}

#[test]
fn finish_task_clones_latest_release_file_path_when_reusing_apk_url() {
    let store = temp_store();
    let user = store
        .create_user("release-clone@example.com", "secret1", None, None)
        .expect("user should be created");
    let project = store
        .create_project(&user.id, "Release Clone", None, None)
        .expect("project should be created")
        .project;
    let old_task_id = store
        .create_task(&project.id, &user.id, Some("conv-release-old"), "old build")
        .expect("old task should be created");
    let task_id = store
        .create_task(&project.id, &user.id, Some("conv-release-new"), "reuse apk")
        .expect("task should be created");
    let apk_url = "https://example.test/api/projects/prj/download/latest.apk";
    store
        .create_project_release(crate::store::project_releases::ProjectReleaseWrite {
            id: Some("rel_old_bound"),
            project_id: &project.id,
            task_id: Some(&old_task_id),
            uploaded_by: Some(&user.id),
            version_name: Some("old build"),
            package_name: Some("com.example.releaseclone"),
            version_code: Some(8),
            channel: Some("pc_node"),
            status: Some("published"),
            apk_url,
            file_name: "app-debug.apk",
            file_path: Some("D:/elon/project/artifacts/app-debug.apk"),
            sha256: Some("abc123"),
            size_bytes: Some(3),
            changelog: Some("synced"),
            build_started_at: Some("2026-07-05T10:00:00Z"),
            source_git_sha: Some("abc1234"),
            source_worktree: None,
            metadata_json: None,
        })
        .expect("old release should insert");

    store
        .finish_task(&task_id, "done", Some("done"), Some(apk_url), None)
        .expect("task should finish");
    let releases = store
        .list_project_releases(&project.id, 10)
        .expect("releases should list");
    let current_release = releases
        .iter()
        .find(|release| release.task_id.as_deref() == Some(task_id.as_str()))
        .expect("current task release should exist");

    assert_eq!(releases.len(), 2);
    assert_eq!(
        current_release.file_path.as_deref(),
        Some("D:/elon/project/artifacts/app-debug.apk")
    );
    assert_eq!(current_release.sha256.as_deref(), Some("abc123"));
    assert_eq!(current_release.channel, "pc_node");
    assert_eq!(
        current_release.package_name.as_deref(),
        Some("com.example.releaseclone")
    );
    assert_eq!(current_release.version_code, Some(8));
    assert_eq!(current_release.release_number, Some(2));
}

#[test]
fn finish_task_with_only_stable_url_does_not_publish_release() {
    let store = temp_store();
    let user = store
        .create_user("release-url-only@example.com", "secret1", None, None)
        .expect("user should be created");
    let project = store
        .create_project(&user.id, "Release URL Only", None, None)
        .expect("project should be created")
        .project;
    let task_id = store
        .create_task(
            &project.id,
            &user.id,
            Some("conv-release-url"),
            "reuse old apk",
        )
        .expect("task should be created");
    let apk_url = "https://example.test/api/projects/prj/download/latest.apk";

    store
        .finish_task(&task_id, "done", Some("done"), Some(apk_url), None)
        .expect("task should finish");

    assert!(store
        .list_project_releases(&project.id, 10)
        .expect("releases should list")
        .is_empty());
    assert!(store
        .latest_project_apk_delivery(&project.id)
        .expect("delivery should query")
        .is_none());
}

#[test]
fn metadata_only_release_is_not_latest_or_downloadable() {
    let store = temp_store();
    let user = store
        .create_user("release-metadata@example.com", "secret1", None, None)
        .expect("user should be created");
    let project = store
        .create_project(&user.id, "Release Metadata", None, None)
        .expect("project should be created")
        .project;
    let apk_url = "https://example.test/api/projects/prj/download/latest.apk";

    store
        .create_project_release(crate::store::project_releases::ProjectReleaseWrite {
            id: Some("rel_metadata_only"),
            project_id: &project.id,
            task_id: None,
            uploaded_by: Some(&user.id),
            version_name: Some("metadata only"),
            package_name: None,
            version_code: None,
            channel: Some("internal"),
            status: Some("published"),
            apk_url,
            file_name: "latest.apk",
            file_path: None,
            sha256: None,
            size_bytes: None,
            changelog: Some("no file"),
            build_started_at: None,
            source_git_sha: None,
            source_worktree: None,
            metadata_json: None,
        })
        .expect("metadata release should insert");

    assert_eq!(
        store
            .list_project_releases(&project.id, 10)
            .expect("releases should list")
            .len(),
        1
    );
    assert!(store
        .latest_project_release(&project.id)
        .expect("latest release should query")
        .is_none());
    assert!(store
        .project_release_for_download(&project.id, "latest.apk")
        .expect("download release should query")
        .is_none());
    assert!(store
        .latest_project_apk_delivery(&project.id)
        .expect("delivery should query")
        .is_none());
}

#[test]
fn file_backed_release_stays_latest_when_metadata_only_release_is_newer() {
    let store = temp_store();
    let user = store
        .create_user("release-file-backed@example.com", "secret1", None, None)
        .expect("user should be created");
    let project = store
        .create_project(&user.id, "Release File Backed", None, None)
        .expect("project should be created")
        .project;
    let apk_url = "https://example.test/api/projects/prj/download/latest.apk";

    store
        .create_project_release(crate::store::project_releases::ProjectReleaseWrite {
            id: Some("rel_file_backed"),
            project_id: &project.id,
            task_id: None,
            uploaded_by: Some(&user.id),
            version_name: Some("file backed"),
            package_name: Some("com.example.filebacked"),
            version_code: Some(9),
            channel: Some("pc_node"),
            status: Some("published"),
            apk_url,
            file_name: "app-debug.apk",
            file_path: Some("D:/elon/project/artifacts/app-debug.apk"),
            sha256: Some("abc123"),
            size_bytes: Some(3),
            changelog: Some("synced"),
            build_started_at: Some("2026-07-05T10:00:00Z"),
            source_git_sha: Some("abc1234"),
            source_worktree: None,
            metadata_json: None,
        })
        .expect("file backed release should insert");
    store
        .create_project_release(crate::store::project_releases::ProjectReleaseWrite {
            id: Some("rel_later_metadata_only"),
            project_id: &project.id,
            task_id: None,
            uploaded_by: Some(&user.id),
            version_name: Some("metadata only"),
            package_name: None,
            version_code: None,
            channel: Some("internal"),
            status: Some("published"),
            apk_url,
            file_name: "latest.apk",
            file_path: None,
            sha256: None,
            size_bytes: None,
            changelog: Some("no file"),
            build_started_at: None,
            source_git_sha: None,
            source_worktree: None,
            metadata_json: None,
        })
        .expect("metadata release should insert");

    let latest = store
        .latest_project_release(&project.id)
        .expect("latest release should query")
        .expect("file backed release should be latest");
    assert_eq!(latest.id, "rel_file_backed");

    let download = store
        .project_release_for_download(&project.id, "latest.apk")
        .expect("download release should query")
        .expect("file backed release should be downloadable");
    assert_eq!(download.id, "rel_file_backed");

    let delivery = store
        .latest_project_apk_delivery(&project.id)
        .expect("delivery should query")
        .expect("file backed delivery should exist");
    assert_eq!(delivery.0, "rel_file_backed");
}

#[test]
fn file_backed_release_updates_landing_android_download() {
    let store = temp_store();
    let user = store
        .create_user("release-landing@example.com", "secret1", None, None)
        .expect("user should be created");
    let project = store
        .create_project(&user.id, "Release Landing", Some("Keep downloads"), None)
        .expect("project should be created")
        .project;
    store
        .update_project_landing_snapshot(
            &user.id,
            &project.id,
            &serde_json::json!({
                "title": "Existing Landing",
                "downloads": [
                    {
                        "platform": "windows",
                        "label": "Windows Client",
                        "url": "https://example.test/client.exe"
                    },
                    {
                        "platform": "android",
                        "label": "Old APK",
                        "url": "https://example.test/old.apk"
                    }
                ],
                "resources": [
                    { "label": "Docs", "url": "https://example.test/docs" }
                ]
            }),
        )
        .expect("landing should update");

    let apk_url = "https://example.test/api/projects/prj/download/latest.apk";
    let release = store
        .create_project_release(crate::store::project_releases::ProjectReleaseWrite {
            id: Some("rel_landing"),
            project_id: &project.id,
            task_id: None,
            uploaded_by: Some(&user.id),
            version_name: Some("1.2.3"),
            package_name: Some("com.example.landing"),
            version_code: Some(42),
            channel: Some("pc_node"),
            status: Some("published"),
            apk_url,
            file_name: "app-debug.apk",
            file_path: Some("D:/elon/project/artifacts/app-debug.apk"),
            sha256: Some("abc123"),
            size_bytes: Some(1_572_864),
            changelog: Some("Synced from PC node"),
            build_started_at: Some("2026-07-05T10:00:00Z"),
            source_git_sha: Some("abc1234"),
            source_worktree: None,
            metadata_json: None,
        })
        .expect("release should insert");

    assert_eq!(release.release_number, Some(1));
    let landing = store
        .project_landing_snapshot(&user.id, &project.id)
        .expect("landing should query")
        .expect("landing should exist");
    let downloads = landing["downloads"].as_array().expect("downloads array");
    let android = downloads
        .iter()
        .filter(|download| download["platform"] == "android")
        .collect::<Vec<_>>();
    let windows = downloads
        .iter()
        .find(|download| download["platform"] == "windows")
        .expect("windows download should remain");

    assert_eq!(android.len(), 1);
    assert_eq!(android[0]["url"], apk_url);
    assert_eq!(android[0]["version"], "1.2.3 (build 1)");
    assert_eq!(android[0]["size_label"], "1.5 MB");
    assert_eq!(windows["url"], "https://example.test/client.exe");
    assert_eq!(landing["resources"][0]["url"], "https://example.test/docs");
}
