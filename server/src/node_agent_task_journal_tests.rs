use super::*;

use homecli_proto::CancelRequestAudit;
use std::{fs, path::PathBuf};

#[test]
fn stale_cursor_resets_with_stable_journal_epoch() {
    let dir = unique_test_dir("cursor-reset");
    let journal = TaskJournal::new(&dir);
    journal
        .record_started(TaskJournalStart {
            req_id: "req-reset",
            cli_name: "codex",
            route: Some("managed_pipe_json_sidecar"),
            run_handle_id: Some("run-stable"),
            cwd: Some("D:/isolated"),
            runtime_permission: Some("full_access"),
        })
        .unwrap();
    let first = journal.snapshot("req-reset", 0, 20).unwrap();
    let reset = journal.snapshot("req-reset", 9999, 20).unwrap();
    assert!(reset.cursor_reset);
    assert_eq!(reset.requested_cursor, 9999);
    assert_eq!(reset.old_cursor, 9999);
    assert_eq!(reset.new_cursor, reset.resume_cursor);
    assert_eq!(reset.cursor_epoch, first.cursor_epoch);
    assert_eq!(reset.sidecar_update_epoch, first.sidecar_update_epoch);
    assert!(reset.cursor_epoch.starts_with("journal-"));
    assert!(!reset.cursor_epoch.contains("run-stable"));
}

#[test]
fn records_started_cancel_and_finished_events() {
    let dir = unique_test_dir("lifecycle");
    let journal = TaskJournal::new(&dir);
    journal
        .record_started(TaskJournalStart {
            req_id: "req-1",
            cli_name: "codex",
            route: Some("route_a_external_cli"),
            run_handle_id: Some("req-1"),
            cwd: Some("D:/demo"),
            runtime_permission: Some("project_write"),
        })
        .expect("started event should persist");
    journal
        .record_cancel_requested_with_audit(
            "req-1",
            &homecli_proto::CancelRequestAudit {
                requested_by: Some("owner-1".to_string()),
                source: Some("pc_ui".to_string()),
                reason: Some("user_stop_button".to_string()),
                requested_at_ms: Some(1234),
                interruption_source: Some(
                    homecli_proto::InterruptionSource::SupervisorIntervention,
                ),
            },
        )
        .expect("cancel event should persist");
    journal
        .record_finished("req-1")
        .expect("finished event should persist");

    let registry = journal
        .read_registry_for_test()
        .expect("registry should read");
    let record = registry.get("req-1").expect("record should exist");
    assert_eq!(record.status, "finished");
    assert_eq!(record.cli_name, "codex");
    assert_eq!(record.route.as_deref(), Some("route_a_external_cli"));
    assert_eq!(record.run_handle_id.as_deref(), Some("req-1"));
    assert_eq!(record.cwd.as_deref(), Some("D:/demo"));
    assert!(record.cancel_requested_at_ms.is_some());

    let events = fs::read_to_string(dir.join("events.jsonl")).expect("events should read");
    assert!(events.contains(r#""type":"started""#));
    assert!(events.contains(r#""type":"cancel_requested""#));
    assert!(events.contains(r#""requested_by":"owner-1""#));
    assert!(events.contains(r#""source":"pc_ui""#));
    assert!(events.contains(r#""reason":"user_stop_button""#));
    assert!(events.contains(r#""requested_at_ms":1234"#));
    assert!(events.contains(r#""type":"finished""#));
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn preserves_explicit_terminal_outcome_from_generic_cleanup() {
    let dir = unique_test_dir("terminal-outcome");
    let journal = TaskJournal::new(&dir);
    journal
        .record_started(TaskJournalStart {
            req_id: "req-1",
            cli_name: "server-runtime",
            route: Some("route_b_api_runtime"),
            run_handle_id: Some("req-1"),
            cwd: Some("D:/demo"),
            runtime_permission: Some("project_write"),
        })
        .expect("started event should persist");
    journal
        .record_finished_with_outcome("req-1", "canceled", Some("用户已停止 PC CLI 任务"))
        .expect("terminal outcome should persist");
    journal
        .record_finished("req-1")
        .expect("generic cleanup should not overwrite terminal status");

    let registry = journal
        .read_registry_for_test()
        .expect("registry should read");
    let record = registry.get("req-1").expect("record should exist");
    assert_eq!(record.status, "canceled");

    let events = fs::read_to_string(dir.join("events.jsonl")).expect("events should read");
    assert_eq!(events.matches(r#""type":"finished""#).count(), 1);
    assert!(events.contains(r#""status":"canceled""#));
    assert!(events.contains(r#""error":"用户已停止 PC CLI 任务""#));
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn recovery_running_is_visible_to_inspect_and_wait_without_terminal_downgrade() {
    let dir = unique_test_dir("recovery-running-visible");
    let journal = TaskJournal::new(&dir);
    journal
        .record_started(TaskJournalStart {
            req_id: "req-recovery",
            cli_name: "codex",
            route: Some("route_a_external_cli"),
            run_handle_id: Some("req-recovery"),
            cwd: Some("D:/demo"),
            runtime_permission: Some("full_access"),
        })
        .unwrap();

    assert!(journal
        .record_recovery_running(
            "req-recovery",
            "verification",
            Some("cargo test --bin elon-pc-node"),
            "sidecar_output_replayed",
        )
        .unwrap());
    let active = journal.snapshot("req-recovery", 0, 20).unwrap();
    let runtime = runtime_status_payload(active.record.as_ref());
    assert_eq!(active.record.as_ref().unwrap().status, "running");
    assert_eq!(runtime["phase"], "verification");
    assert_eq!(runtime["current_command"], "cargo test --bin elon-pc-node");
    assert!(runtime["last_progress"].as_u64().unwrap_or_default() > 0);
    assert!(runtime["heartbeat"].as_u64().unwrap_or_default() > 0);
    assert!(active.events.iter().any(|event| {
        event.event["type"] == "recovery_running" && event.event["status"] == "running"
    }));
    let wait_cursor = active.last_event_seq;

    journal
        .record_finished_with_outcome("req-recovery", "done", None)
        .unwrap();
    assert!(!journal
        .record_recovery_running("req-recovery", "reasoning", None, "late_replay",)
        .unwrap());
    journal
        .record_finished_with_outcome("req-recovery", "failed", Some("late timeout"))
        .unwrap();

    let terminal = journal.snapshot("req-recovery", wait_cursor, 20).unwrap();
    assert_eq!(terminal.record.as_ref().unwrap().status, "done");
    assert_eq!(terminal.record.as_ref().unwrap().phase, "done");
    assert_eq!(
        terminal
            .events
            .iter()
            .filter(|event| event.event["type"] == "finished")
            .count(),
        1
    );
    assert!(terminal.last_event_seq > wait_cursor);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn late_cancel_after_terminal_is_audit_only() {
    let dir = unique_test_dir("late-cancel-terminal");
    let journal = TaskJournal::new(&dir);
    journal
        .record_started(TaskJournalStart {
            req_id: "req-1",
            cli_name: "server-runtime",
            route: Some("route_c_server_runtime"),
            run_handle_id: Some("req-1"),
            cwd: Some("D:/demo"),
            runtime_permission: Some("project_write"),
        })
        .expect("started event should persist");
    journal
        .record_finished_with_outcome("req-1", "done", None)
        .expect("terminal outcome should persist");
    journal
        .record_cancel_requested_with_audit(
            "req-1",
            &CancelRequestAudit {
                requested_by: Some("owner-1".to_string()),
                source: Some("pc_ui".to_string()),
                reason: Some("user_requested".to_string()),
                requested_at_ms: Some(4321),
                interruption_source: None,
            },
        )
        .expect("late cancel should remain auditable");

    let registry = journal
        .read_registry_for_test()
        .expect("registry should read");
    let record = registry.get("req-1").expect("record should exist");
    assert_eq!(record.status, "done");
    assert!(record.cancel_requested_at_ms.is_none());

    let events = fs::read_to_string(dir.join("events.jsonl")).expect("events should read");
    assert_eq!(events.matches(r#""type":"cancel_requested""#).count(), 1);
    assert!(events.contains(r#""ignored":true"#));
    assert!(events.contains(r#""requested_by":"owner-1""#));
    assert!(events.contains(r#""source":"pc_ui""#));
    assert!(events.contains(r#""reason":"user_requested""#));
    assert!(events.contains(r#""requested_at_ms":4321"#));
    assert!(events.contains(r#""ignored_reason":"task_already_terminal""#));
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn records_cli_chunks_without_prompt_or_secret_fields() {
    let dir = unique_test_dir("chunks");
    let journal = TaskJournal::new(&dir);
    journal
        .record_started(TaskJournalStart {
            req_id: "req-1",
            cli_name: "codex",
            route: Some("route_a_external_cli"),
            run_handle_id: Some("req-1"),
            cwd: Some("D:/demo"),
            runtime_permission: Some("project_write"),
        })
        .expect("start event should persist");
    journal
        .record_cli_chunk("req-1", "stdout", "hello from cli\n")
        .expect("chunk should persist");

    let snapshot = journal
        .snapshot("req-1", 0, 20)
        .expect("snapshot should read");
    let chunk = snapshot
        .events
        .iter()
        .find(|event| event.event.get("type").and_then(|value| value.as_str()) == Some("cli_chunk"))
        .expect("chunk event should be present");
    assert_eq!(chunk.event["text"], "hello from cli\n");
    assert!(chunk.event.get("prompt").is_none());
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn records_structured_tool_events_for_replay() {
    let dir = unique_test_dir("tool-event");
    let journal = TaskJournal::new(&dir);
    journal
        .record_started(TaskJournalStart {
            req_id: "req-1",
            cli_name: "server-runtime",
            route: Some("route_c_server_runtime"),
            run_handle_id: Some("req-1"),
            cwd: Some("D:/demo"),
            runtime_permission: Some("project_write"),
        })
        .expect("start event should persist");
    journal
        .record_cli_chunk(
            "req-1",
            "runtime",
            r#"{"type":"tool_call","tool":"run_command","args":{"program":"git"}}"#,
        )
        .expect("tool event should persist");

    let snapshot = journal
        .snapshot("req-1", 0, 20)
        .expect("snapshot should read");
    let tool_event = snapshot
        .events
        .iter()
        .find(|event| {
            event.event.get("type").and_then(|value| value.as_str()) == Some("tool_event")
        })
        .expect("tool event should be present");
    assert_eq!(tool_event.event["event"]["type"], "tool_call");
    assert_eq!(tool_event.event["event"]["tool"], "run_command");
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn snapshot_filters_events_by_task_and_cursor() {
    let dir = unique_test_dir("snapshot");
    let journal = TaskJournal::new(&dir);
    journal
        .record_started(TaskJournalStart {
            req_id: "req-1",
            cli_name: "codex",
            route: Some("route_a_external_cli"),
            run_handle_id: Some("req-1"),
            cwd: Some("D:/demo"),
            runtime_permission: Some("project_write"),
        })
        .expect("first task should persist");
    journal
        .record_started(TaskJournalStart {
            req_id: "req-2",
            cli_name: "claude",
            route: Some("route_a_external_cli"),
            run_handle_id: Some("req-2"),
            cwd: Some("D:/other"),
            runtime_permission: Some("read_only"),
        })
        .expect("second task should persist");
    journal
        .record_finished("req-1")
        .expect("finish event should persist");

    let snapshot = journal
        .snapshot("req-1", 1, 20)
        .expect("snapshot should read");
    assert_eq!(snapshot.task_id, "req-1");
    assert_eq!(snapshot.record.as_ref().unwrap().status, "finished");
    assert_eq!(snapshot.events.len(), 1);
    assert_eq!(
        snapshot.events[0]
            .event
            .get("type")
            .and_then(|value| value.as_str()),
        Some("finished")
    );
    assert!(snapshot.last_event_seq >= 3);
    assert!(!snapshot.has_more);

    let latest = journal
        .latest_records(10)
        .expect("latest records should read");
    assert_eq!(latest.len(), 2);
    assert_eq!(latest[0].req_id, "req-1");
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn expected_cursor_epoch_resets_after_log_replacement_even_with_more_lines() {
    let dir = unique_test_dir("cursor-log-replacement");
    let journal = TaskJournal::new(&dir);
    journal
        .record_started(TaskJournalStart {
            req_id: "req-epoch",
            cli_name: "codex",
            route: Some("route_a_external_cli"),
            run_handle_id: Some("req-epoch"),
            cwd: Some("D:/demo"),
            runtime_permission: Some("project_write"),
        })
        .unwrap();
    let before = journal.snapshot("req-epoch", 0, 20).unwrap();
    let replacement = dir.join("replacement.jsonl");
    std::thread::sleep(std::time::Duration::from_millis(2));
    let lines = (0..before.last_event_seq + 10)
        .map(|index| {
            serde_json::json!({"type":"heartbeat", "req_id":"req-epoch", "index":index}).to_string()
        })
        .collect::<Vec<_>>()
        .join("\n");
    fs::write(&replacement, format!("{lines}\n")).unwrap();
    fs::remove_file(journal.events_path()).unwrap();
    fs::rename(replacement, journal.events_path()).unwrap();

    let after = journal
        .snapshot_with_epoch(
            "req-epoch",
            before.last_event_seq,
            200,
            Some(&before.cursor_epoch),
        )
        .unwrap();
    assert_ne!(after.cursor_epoch, before.cursor_epoch);
    assert!(after.cursor_reset);
    assert_eq!(after.old_cursor, before.last_event_seq);
    assert_eq!(after.events.first().unwrap().seq, 1);
    assert_eq!(
        after.requested_cursor_epoch.as_deref(),
        Some(before.cursor_epoch.as_str())
    );
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn expected_cursor_epoch_resets_after_node_restart_without_missing_events() {
    let dir = unique_test_dir("cursor-node-restart");
    let first = TaskJournal::new(&dir);
    first
        .record_started(TaskJournalStart {
            req_id: "req-restart",
            cli_name: "codex",
            route: Some("route_a_external_cli"),
            run_handle_id: Some("req-restart"),
            cwd: Some("D:/demo"),
            runtime_permission: Some("project_write"),
        })
        .unwrap();
    let before = first.snapshot("req-restart", 0, 20).unwrap();
    let restarted = TaskJournal::new(&dir);
    restarted
        .append_event(serde_json::json!({"type":"heartbeat", "req_id":"req-restart"}))
        .unwrap();
    let after = restarted
        .snapshot_with_epoch(
            "req-restart",
            before.last_event_seq,
            20,
            Some(&before.cursor_epoch),
        )
        .unwrap();
    assert_ne!(after.cursor_epoch, before.cursor_epoch);
    assert!(after.cursor_reset);
    assert_eq!(after.events.first().unwrap().seq, 1);
    assert!(after.events.len() >= 2);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn stress_snapshot_handles_many_interleaved_task_events() {
    let dir = unique_test_dir("stress-snapshot");
    let journal = TaskJournal::new(&dir);
    for task_index in 0..120 {
        let req_id = format!("req-{task_index:03}");
        journal
            .record_started(TaskJournalStart {
                req_id: &req_id,
                cli_name: if task_index % 2 == 0 {
                    "server-runtime"
                } else {
                    "codex"
                },
                route: Some(if task_index % 2 == 0 {
                    "route_c_server_runtime"
                } else {
                    "route_a_external_cli"
                }),
                run_handle_id: Some(&req_id),
                cwd: Some("D:/demo"),
                runtime_permission: Some("project_write"),
            })
            .expect("start event should persist");
        for chunk_index in 0..4 {
            journal
                .record_cli_chunk(
                    &req_id,
                    "stdout",
                    &format!("task {task_index} chunk {chunk_index}\n"),
                )
                .expect("chunk should persist");
        }
        if task_index % 3 == 0 {
            journal
                .record_finished(&req_id)
                .expect("finish event should persist");
        }
    }

    let snapshot = journal
        .snapshot("req-000", 0, 3)
        .expect("snapshot should read under pressure");
    assert_eq!(snapshot.events.len(), 3);
    assert!(snapshot.has_more);
    assert_eq!(
        snapshot.last_event_seq,
        snapshot.events.last().expect("page should have events").seq,
        "paginated snapshot cursor should stay at the returned page tail"
    );
    let next = journal
        .snapshot("req-000", snapshot.last_event_seq, 20)
        .expect("next snapshot should continue target task");
    assert!(next.events.len() >= 3);
    assert!(next.events.iter().all(|event| {
        event.event.get("req_id").and_then(|value| value.as_str()) == Some("req-000")
    }));
    assert!(snapshot.events.iter().all(|event| {
        event.event.get("req_id").and_then(|value| value.as_str()) == Some("req-000")
    }));

    let latest = journal
        .latest_records(500)
        .expect("latest records should read under pressure");
    assert_eq!(latest.len(), 100, "latest_records clamps public output");
    assert!(latest
        .iter()
        .all(|record| record.req_id.starts_with("req-")));
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn stress_snapshot_streams_large_interleaved_journal() {
    let dir = unique_test_dir("stress-streaming-snapshot");
    let journal = TaskJournal::new(&dir);
    for task_index in 0..400 {
        let req_id = format!("req-{task_index:03}");
        journal
            .record_started(TaskJournalStart {
                req_id: &req_id,
                cli_name: "server-runtime",
                route: Some("route_c_server_runtime"),
                run_handle_id: Some(&req_id),
                cwd: Some("D:/demo"),
                runtime_permission: Some("project_write"),
            })
            .expect("start event should persist");
        for chunk_index in 0..8 {
            journal
                .record_cli_chunk(
                    &req_id,
                    "runtime",
                    &format!("task {task_index} chunk {chunk_index}\n"),
                )
                .expect("chunk should persist");
        }
        if task_index % 5 == 0 {
            journal
                .record_finished(&req_id)
                .expect("finish event should persist");
        }
    }

    let snapshot = journal
        .snapshot("req-399", 0, 4)
        .expect("snapshot should stream large journal");
    assert_eq!(snapshot.events.len(), 4);
    assert!(snapshot.has_more);
    assert_eq!(
        snapshot.last_event_seq,
        snapshot.events.last().expect("page should have events").seq,
        "limited page cursor should not jump to the global journal tail"
    );
    assert!(snapshot.events.iter().all(|event| {
        event.event.get("req_id").and_then(|value| value.as_str()) == Some("req-399")
    }));

    let next = journal
        .snapshot("req-399", snapshot.last_event_seq, 20)
        .expect("cursor snapshot should continue target task only");
    assert!(next.events.iter().all(|event| {
        event.event.get("req_id").and_then(|value| value.as_str()) == Some("req-399")
    }));
    assert_eq!(next.events.len(), 5);
    assert!(!next.has_more);

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn records_process_pid_for_active_route_a_handle() {
    let dir = unique_test_dir("pid");
    let journal = TaskJournal::new(&dir);
    journal
        .record_started(TaskJournalStart {
            req_id: "req-1",
            cli_name: "codex",
            route: Some("route_a_external_cli"),
            run_handle_id: Some("req-1"),
            cwd: Some("D:/demo"),
            runtime_permission: Some("project_write"),
        })
        .expect("start event should persist");
    journal
        .record_process_started("req-1", 4242)
        .expect("pid event should persist");

    let snapshot = journal
        .snapshot("req-1", 0, 20)
        .expect("snapshot should read");
    let record = snapshot.record.expect("record should exist");
    assert_eq!(record.os_pid, Some(4242));
    assert!(record.process_started_at_ms.is_some());
    assert!(snapshot.events.iter().any(|event| {
        event.event.get("type").and_then(|value| value.as_str()) == Some("process_started")
            && event.event.get("pid").and_then(|value| value.as_u64()) == Some(4242)
    }));
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn records_codex_session_for_task_resume() {
    let dir = unique_test_dir("codex-session");
    let journal = TaskJournal::new(&dir);
    journal
        .record_started(TaskJournalStart {
            req_id: "req-1",
            cli_name: "codex",
            route: Some("route_a_external_cli"),
            run_handle_id: Some("req-1"),
            cwd: Some("D:/demo"),
            runtime_permission: Some("project_write"),
        })
        .expect("start event should persist");
    journal
        .record_codex_session("req-1", "scope-a", "session-uuid")
        .expect("codex session should persist");

    let snapshot = journal
        .snapshot("req-1", 0, 20)
        .expect("snapshot should read");
    let record = snapshot.record.expect("record should exist");
    assert_eq!(record.codex_session_id.as_deref(), Some("session-uuid"));
    assert_eq!(record.codex_session_scope_key.as_deref(), Some("scope-a"));
    assert!(record.codex_session_updated_at_ms.is_some());
    assert_eq!(
        journal
            .load_codex_session("scope-a")
            .expect("codex session should load")
            .as_deref(),
        Some("session-uuid")
    );
    assert!(snapshot.events.iter().any(|event| {
        event.event.get("type").and_then(|value| value.as_str()) == Some("codex_session")
            && event
                .event
                .get("session_id")
                .and_then(|value| value.as_str())
                == Some("session-uuid")
    }));
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn clears_stale_codex_session_for_fresh_retry() {
    let dir = unique_test_dir("codex-session-clear");
    let journal = TaskJournal::new(&dir);
    journal
        .record_started(TaskJournalStart {
            req_id: "req-1",
            cli_name: "codex",
            route: Some("route_a_external_cli"),
            run_handle_id: Some("req-1"),
            cwd: Some("D:/demo"),
            runtime_permission: Some("project_write"),
        })
        .expect("start event should persist");
    journal
        .record_codex_session("req-1", "scope-a", "session-uuid")
        .expect("codex session should persist");
    journal
        .clear_codex_session("req-1", "scope-a")
        .expect("stale session should clear");

    let snapshot = journal
        .snapshot("req-1", 0, 20)
        .expect("snapshot should read");
    let record = snapshot.record.expect("record should exist");
    assert!(record.codex_session_id.is_none());
    assert!(record.codex_session_scope_key.is_none());
    assert_eq!(
        journal
            .load_codex_session("scope-a")
            .expect("codex session file should load"),
        None
    );
    assert!(snapshot.events.iter().any(|event| {
        event.event.get("type").and_then(|value| value.as_str()) == Some("codex_session_cleared")
    }));
    let _ = fs::remove_dir_all(dir);
}

fn unique_test_dir(suffix: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "elon-task-journal-test-{}-{}",
        std::process::id(),
        suffix
    ))
}
