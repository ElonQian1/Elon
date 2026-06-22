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

#[test]
fn stress_local_journal_cursor_replays_long_interleaved_task_without_skipping() {
    let dir = unique_test_dir("cursor-replay-pressure");
    let _ = fs::remove_dir_all(&dir);
    let journal = TaskJournal::new(&dir);
    let target_req = "req-route-c-long";
    journal
        .record_started(TaskJournalStart {
            req_id: target_req,
            cli_name: "server-runtime",
            route: Some("route_c_server_runtime"),
            run_handle_id: Some(target_req),
            cwd: Some("D:/demo"),
            runtime_permission: Some("project_write"),
        })
        .expect("target task should start");

    for index in 0..240 {
        let other_req = format!("req-other-{index:03}");
        journal
            .record_started(TaskJournalStart {
                req_id: &other_req,
                cli_name: "codex",
                route: Some("route_a_external_cli"),
                run_handle_id: Some(&other_req),
                cwd: Some("D:/other"),
                runtime_permission: Some("project_write"),
            })
            .expect("other task should start under pressure");
        journal
            .record_cli_chunk(target_req, "runtime", &format!("target chunk {index}\n"))
            .expect("target chunk should persist under pressure");
        journal
            .record_cli_chunk(&other_req, "stdout", &format!("other chunk {index}\n"))
            .expect("other chunk should persist under pressure");
        if index % 3 == 0 {
            journal
                .record_finished(&other_req)
                .expect("other task finish should persist under pressure");
        }
    }
    journal
        .record_finished_with_outcome(target_req, "done", None)
        .expect("target task finish should persist");

    let mut since = 0;
    let mut total_target_events = 0;
    let mut pages = 0;
    loop {
        let snapshot = journal
            .snapshot(target_req, since, 50)
            .expect("target snapshot page should read");
        pages += 1;
        assert!(
            snapshot.last_event_seq >= since,
            "cursor should not move backwards"
        );
        assert!(snapshot.events.iter().all(|event| {
            event.event.get("req_id").and_then(Value::as_str) == Some(target_req)
        }));
        total_target_events += snapshot.events.len();
        if !snapshot.has_more {
            break;
        }
        assert!(
            snapshot.last_event_seq > since,
            "a page with more target events must advance only to the returned page tail"
        );
        since = snapshot.last_event_seq;
        assert!(pages < 20, "cursor replay should finish in bounded pages");
    }

    assert_eq!(
        total_target_events, 242,
        "started + 240 chunks + finished should all be replayed"
    );
    assert!(pages >= 5, "pressure case should require real pagination");

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
