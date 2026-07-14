use super::{get_user_workspace, CodexPrewarmRegistry, ProjectTaskScheduler};
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

#[test]
fn legacy_ws_workspace_keeps_project_suffix() {
    let workspace = get_user_workspace(
        "/tmp/elon",
        "82ee3288e852435c90ed2a609e474aaf__677b1bb2-09c9-419a-b998-960dd0539796",
    );

    assert_eq!(
        workspace.file_name().and_then(|name| name.to_str()),
        Some("82ee3288e852435c90ed2a609e474aaf__677b1bb2-09c9-419a-b998-960dd0539796")
    );
}

#[tokio::test]
async fn project_task_scheduler_queues_same_project() {
    let scheduler = Arc::new(ProjectTaskScheduler::new());
    let first = scheduler.acquire("project-a", || {}).await;
    let queued_notice_sent = Arc::new(AtomicBool::new(false));

    let task_scheduler = scheduler.clone();
    let task_notice = queued_notice_sent.clone();
    let waiting_task = tokio::spawn(async move {
        let permit = task_scheduler
            .acquire("project-a", || {
                task_notice.store(true, Ordering::SeqCst);
            })
            .await;
        permit.was_queued()
    });

    tokio::time::sleep(Duration::from_millis(20)).await;
    assert!(queued_notice_sent.load(Ordering::SeqCst));
    assert!(!waiting_task.is_finished());

    drop(first);
    assert!(waiting_task.await.unwrap());
}

#[tokio::test]
async fn project_task_scheduler_allows_different_projects() {
    let scheduler = ProjectTaskScheduler::new();
    let _first = scheduler.acquire("project-a", || {}).await;
    let second = scheduler
        .acquire("project-b", || panic!("different projects must not queue"))
        .await;

    assert!(!second.was_queued());
}

#[tokio::test]
async fn codex_prewarm_registry_tracks_active_and_cancelled_runs() {
    let registry = CodexPrewarmRegistry::new();
    assert!(
        registry
            .start_if_allowed("project:user:conversation", Duration::from_secs(120))
            .await
    );
    assert!(
        !registry
            .start_if_allowed("project:user:conversation", Duration::from_secs(120))
            .await
    );

    registry.cancel("project:user:conversation").await;
    assert!(!registry.finish("project:user:conversation").await);
}
