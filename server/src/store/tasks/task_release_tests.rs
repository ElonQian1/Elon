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
            channel: Some("pc_node"),
            status: Some("published"),
            apk_url,
            file_name: "app-debug.apk",
            file_path: Some("D:/elon/project/artifacts/app-debug.apk"),
            sha256: Some("abc123"),
            size_bytes: Some(3),
            changelog: Some("synced"),
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
            channel: Some("pc_node"),
            status: Some("published"),
            apk_url,
            file_name: "app-debug.apk",
            file_path: Some("D:/elon/project/artifacts/app-debug.apk"),
            sha256: Some("abc123"),
            size_bytes: Some(3),
            changelog: Some("synced"),
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
}
