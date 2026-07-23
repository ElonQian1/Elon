use super::*;

#[test]
fn task_event_cache_incrementally_reads_append_only_tail_without_cross_task_leakage() {
    let dir = std::env::temp_dir().join(format!(
        "elon-task-journal-cache-{}",
        uuid::Uuid::new_v4().simple()
    ));
    let journal = TaskJournal::new(&dir);
    for req_id in ["task-a", "task-b"] {
        journal
            .record_started(TaskJournalStart {
                req_id,
                cli_name: "codex",
                route: Some("route_a_external_cli"),
                run_handle_id: Some(req_id),
                cwd: Some("D:/demo"),
                runtime_permission: Some("full_access"),
            })
            .unwrap();
        if req_id == "task-a" {
            let first = journal.task_events(req_id).unwrap();
            assert_eq!(first.len(), 1);
            assert_eq!(first[0].event["type"], "started");
        }
    }
    journal
        .record_finished_with_outcome("task-a", "done", None)
        .unwrap();
    let second = journal.task_events("task-a").unwrap();
    assert_eq!(second.len(), 2);
    assert_eq!(second[0].event["type"], "started");
    assert_eq!(second[1].event["type"], "finished");
    assert!(second.iter().all(|view| view.event["req_id"] == "task-a"));
    let _ = std::fs::remove_dir_all(dir);
}
