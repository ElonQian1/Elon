use crate::{
    node_agent_task_journal::{TaskJournal, TaskJournalRecord, TaskJournalStart},
    node_agent_task_resume::{task_attach_state, task_resume_contract},
};
use serde_json::Value;
use std::{collections::BTreeMap, fs, path::PathBuf};

#[test]
fn stress_terminal_outcomes_across_all_pc_task_routes() {
    let dir = unique_test_dir("route-terminal-pressure");
    let _ = fs::remove_dir_all(&dir);
    let journal = TaskJournal::new(&dir);
    let routes = [
        ("codex", "route_a_external_cli"),
        ("api-runtime", "route_b_api_runtime"),
        ("server-runtime", "route_c_server_runtime"),
    ];
    let outcomes = [
        ("done", None, "done"),
        ("failed", Some("provider unavailable"), "failed"),
        ("canceled", Some("用户已停止 PC CLI 任务"), "canceled"),
    ];

    for task_index in 0..360 {
        let (cli_name, route) = routes[task_index % routes.len()];
        let (finish_status, error, expected_status) =
            outcomes[(task_index / routes.len()) % outcomes.len()];
        let req_id = format!("req-{task_index:03}");
        journal
            .record_started(TaskJournalStart {
                req_id: &req_id,
                cli_name,
                route: Some(route),
                run_handle_id: Some(&req_id),
                cwd: Some("D:/demo"),
                runtime_permission: Some("project_write"),
            })
            .expect("task start should persist under pressure");
        journal
            .record_cli_chunk(
                &req_id,
                "runtime",
                &format!(
                    r#"{{"type":"runtime_status","runtime":"{cli_name}","phase":"thinking","status":"running","message":"task {task_index}"}}"#
                ),
            )
            .expect("runtime event should persist under pressure");
        if expected_status == "canceled" {
            journal
                .record_cancel_requested(&req_id)
                .expect("cancel request should persist under pressure");
        }
        journal
            .record_finished_with_outcome(&req_id, finish_status, error)
            .expect("terminal outcome should persist under pressure");
        journal
            .record_finished(&req_id)
            .expect("generic cleanup should stay idempotent after explicit outcome");
    }

    let registry = read_registry(&dir);
    assert_eq!(registry.len(), 360);
    for (route, status) in [
        ("route_a_external_cli", "done"),
        ("route_a_external_cli", "failed"),
        ("route_a_external_cli", "canceled"),
        ("route_b_api_runtime", "done"),
        ("route_b_api_runtime", "failed"),
        ("route_b_api_runtime", "canceled"),
        ("route_c_server_runtime", "done"),
        ("route_c_server_runtime", "failed"),
        ("route_c_server_runtime", "canceled"),
    ] {
        assert_eq!(
            count_records(&registry, route, status),
            40,
            "{route} should preserve {status} outcomes under pressure"
        );
    }

    let events = fs::read_to_string(dir.join("events.jsonl")).expect("events should read");
    assert_eq!(events.matches(r#""type":"finished""#).count(), 360);
    assert_eq!(events.matches(r#""type":"cancel_requested""#).count(), 120);

    let latest = journal
        .latest_records(500)
        .expect("latest records should read after pressure run");
    assert_eq!(latest.len(), 100, "latest_records must keep public clamp");
    assert!(latest
        .iter()
        .all(|record| { matches!(record.status.as_str(), "done" | "failed" | "canceled") }));

    let sample = journal
        .snapshot("req-008", 0, 20)
        .expect("canceled Route C snapshot should read");
    let record = sample.record.expect("sample record should exist");
    assert_eq!(record.route.as_deref(), Some("route_c_server_runtime"));
    assert_eq!(record.status, "canceled");
    assert!(sample.events.iter().any(|event| {
        event.event.get("type").and_then(Value::as_str) == Some("finished")
            && event.event.get("status").and_then(Value::as_str) == Some("canceled")
    }));

    let attach = task_attach_state(Some(&record), None);
    let resume = task_resume_contract(&attach);
    let resume_json = serde_json::to_value(resume).expect("resume contract should serialize");
    assert_eq!(resume_json["status"], "terminal");
    assert_eq!(resume_json["next_action"], "continue_from_snapshot");
    assert_eq!(resume_json["can_reconnect"], false);
    assert_eq!(resume_json["can_cancel"], false);
    assert_eq!(resume_json["can_replay_journal_events"], true);

    let _ = fs::remove_dir_all(dir);
}

fn read_registry(dir: &std::path::Path) -> BTreeMap<String, TaskJournalRecord> {
    let text = fs::read_to_string(dir.join("registry.json")).expect("registry should read");
    serde_json::from_str(&text).expect("registry should parse")
}

fn count_records(
    registry: &BTreeMap<String, TaskJournalRecord>,
    route: &str,
    status: &str,
) -> usize {
    registry
        .values()
        .filter(|record| record.route.as_deref() == Some(route) && record.status == status)
        .count()
}

fn unique_test_dir(suffix: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "elon-task-lifecycle-pressure-{}-{suffix}",
        std::process::id()
    ))
}
