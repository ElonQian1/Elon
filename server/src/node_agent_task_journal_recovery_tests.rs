use crate::node_agent_task_journal::{TaskJournal, TaskJournalRecord, TaskJournalStart};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

#[test]
fn corrupt_registry_falls_back_to_last_valid_backup_and_recovers_on_next_write() {
    let dir = unique_test_dir("corrupt-registry-backup");
    let _ = fs::remove_dir_all(&dir);
    let journal = TaskJournal::new(&dir);

    record_started(&journal, "req-1");
    record_started(&journal, "req-2");
    assert!(
        dir.join("registry.json.bak").exists(),
        "successful registry writes should keep a last-good backup"
    );

    fs::write(dir.join("registry.json"), "{ broken json")
        .expect("corrupt primary registry fixture should write");
    let latest = journal
        .latest_records(10)
        .expect("corrupt primary should fall back to backup");
    assert_eq!(
        task_ids(&latest),
        vec!["req-2", "req-1"],
        "fallback backup should preserve the last successful registry state"
    );

    record_started(&journal, "req-3");
    let latest_after_recovery = journal
        .latest_records(10)
        .expect("journal should recover after writing a new task");
    assert_eq!(latest_after_recovery.len(), 3);
    assert!(
        task_ids(&latest_after_recovery).contains(&"req-3"),
        "new writes after fallback should rebuild a valid primary registry"
    );
    read_registry(&dir.join("registry.json")).expect("primary registry should be valid again");
    read_registry(&dir.join("registry.json.bak")).expect("backup registry should be valid again");
    assert!(
        fs::read_dir(&dir)
            .expect("journal dir should list")
            .filter_map(Result::ok)
            .any(|entry| entry
                .file_name()
                .to_string_lossy()
                .starts_with("registry-corrupt-")),
        "corrupt primary should be moved aside for diagnostics instead of replacing backup"
    );

    let _ = fs::remove_dir_all(dir);
}

fn record_started(journal: &TaskJournal, req_id: &str) {
    journal
        .record_started(TaskJournalStart {
            req_id,
            cli_name: "server-runtime",
            route: Some("route_c_server_runtime"),
            run_handle_id: Some(req_id),
            cwd: Some("D:/demo"),
            runtime_permission: Some("project_write"),
        })
        .expect("task start should persist");
}

fn task_ids(records: &[TaskJournalRecord]) -> Vec<&str> {
    records
        .iter()
        .map(|record| record.req_id.as_str())
        .collect()
}

fn read_registry(path: &Path) -> serde_json::Result<BTreeMap<String, TaskJournalRecord>> {
    let text = fs::read_to_string(path).expect("registry should read");
    serde_json::from_str(&text)
}

fn unique_test_dir(suffix: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "elon-task-journal-recovery-{}-{suffix}",
        std::process::id()
    ))
}
