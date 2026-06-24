use crate::{
    node_agent_active_task::ActiveCliPromptHandle,
    node_agent_active_task_registry::ActiveCliPromptRegistry,
    node_agent_codex_session::clear_stale_session,
    node_agent_task_journal::{TaskJournal, TaskJournalRecord, TaskJournalStart},
    node_agent_task_resume::{
        task_attach_state, task_resume_contract, task_resume_contract_with_journal_approvals,
    },
    node_agent_tool_approval::{ToolApprovalDecision, ToolApprovalState},
};
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    fs::{self, OpenOptions},
    io::Write,
    path::PathBuf,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    thread,
};
use tokio::sync::watch;

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
fn stress_workspace_latest_records_filter_before_global_clamp() {
    let dir = unique_test_dir("workspace-filter-before-clamp");
    let _ = fs::remove_dir_all(&dir);
    let journal_dir = dir.join("journal");
    let target_workspace = dir.join("target-workspace");
    let target_project = target_workspace.join("project");
    let other_project = dir.join("other-workspace").join("project");
    fs::create_dir_all(&target_project).expect("target project dir should create");
    fs::create_dir_all(&other_project).expect("other project dir should create");

    let mut registry = BTreeMap::new();
    for index in 0..12 {
        let req_id = format!("req-target-old-{index:03}");
        registry.insert(
            req_id.clone(),
            journal_record_for_workspace(&req_id, &target_project, 1_000 + index as u128),
        );
    }
    for index in 0..180 {
        let req_id = format!("req-other-new-{index:03}");
        registry.insert(
            req_id.clone(),
            journal_record_for_workspace(&req_id, &other_project, 10_000 + index as u128),
        );
    }
    fs::create_dir_all(&journal_dir).expect("journal dir should create");
    fs::write(
        journal_dir.join("registry.json"),
        serde_json::to_string_pretty(&registry).expect("registry should serialize"),
    )
    .expect("registry should write");

    let journal = TaskJournal::new(&journal_dir);
    let global_latest = journal
        .latest_records(100)
        .expect("global latest records should read");
    assert_eq!(global_latest.len(), 100);
    assert!(
        global_latest
            .iter()
            .all(|record| record.req_id.starts_with("req-other-new-")),
        "fixture should prove global latest output is saturated by other workspaces"
    );

    let workspace_latest = journal
        .latest_records_for_workspace(&target_workspace, 6)
        .expect("workspace latest records should read before global clamp");
    assert_eq!(
        workspace_latest
            .iter()
            .map(|record| record.req_id.as_str())
            .collect::<Vec<_>>(),
        vec![
            "req-target-old-011",
            "req-target-old-010",
            "req-target-old-009",
            "req-target-old-008",
            "req-target-old-007",
            "req-target-old-006",
        ]
    );

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn stress_late_cancel_requests_never_reopen_terminal_tasks() {
    let dir = unique_test_dir("late-cancel-after-terminal-pressure");
    let _ = fs::remove_dir_all(&dir);
    let journal = TaskJournal::new(&dir);
    let terminal_outcomes = [
        ("done", None, "done"),
        ("failed", Some("provider unavailable"), "failed"),
        ("canceled", Some("用户已停止 PC CLI 任务"), "canceled"),
    ];

    for task_index in 0..300 {
        let (finish_status, error, expected_status) =
            terminal_outcomes[task_index % terminal_outcomes.len()];
        let req_id = format!("req-late-cancel-{task_index:03}");
        journal
            .record_started(TaskJournalStart {
                req_id: &req_id,
                cli_name: "server-runtime",
                route: Some("route_c_server_runtime"),
                run_handle_id: Some(&req_id),
                cwd: Some("D:/demo"),
                runtime_permission: Some("project_write"),
            })
            .expect("task start should persist before late cancel pressure");
        journal
            .record_finished_with_outcome(&req_id, finish_status, error)
            .expect("terminal outcome should persist before late cancel pressure");
        journal
            .record_cancel_requested(&req_id)
            .expect("late cancel request should remain auditable");

        let registry = read_registry(&dir);
        let record = registry
            .get(&req_id)
            .expect("terminal record should stay in registry");
        assert_eq!(
            record.status, expected_status,
            "late cancel must not reopen terminal task {req_id}"
        );
        assert!(
            record.cancel_requested_at_ms.is_none(),
            "late cancel after terminal should not look like an active cancel request"
        );
    }

    let registry = read_registry(&dir);
    assert_eq!(registry.len(), 300);
    assert_eq!(count_status(&registry, "done"), 100);
    assert_eq!(count_status(&registry, "failed"), 100);
    assert_eq!(count_status(&registry, "canceled"), 100);
    assert_eq!(count_status(&registry, "cancel_requested"), 0);

    let sample = journal
        .snapshot("req-late-cancel-002", 0, 20)
        .expect("late cancel sample snapshot should read");
    assert_eq!(
        sample.record.as_ref().map(|record| record.status.as_str()),
        Some("canceled")
    );
    assert!(sample.events.iter().any(|event| {
        event.event.get("type").and_then(Value::as_str) == Some("cancel_requested")
            && event.event.get("ignored").and_then(Value::as_bool) == Some(true)
            && event.event.get("reason").and_then(Value::as_str) == Some("task_already_terminal")
    }));

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

#[test]
fn stress_concurrent_task_journal_writes_keep_registry_and_events_consistent() {
    let dir = unique_test_dir("concurrent-writes-pressure");
    let _ = fs::remove_dir_all(&dir);
    let journal = Arc::new(TaskJournal::new(&dir));
    let workers = 16;
    let tasks_per_worker = 24;
    let chunks_per_task = 3;

    let handles = (0..workers)
        .map(|worker| {
            let journal = Arc::clone(&journal);
            thread::spawn(move || {
                for task in 0..tasks_per_worker {
                    let req_id = format!("req-w{worker:02}-t{task:02}");
                    let route = match worker % 3 {
                        0 => "route_a_external_cli",
                        1 => "route_b_api_runtime",
                        _ => "route_c_server_runtime",
                    };
                    let cli_name = match worker % 3 {
                        0 => "codex",
                        1 => "api-runtime",
                        _ => "server-runtime",
                    };
                    journal
                        .record_started(TaskJournalStart {
                            req_id: &req_id,
                            cli_name,
                            route: Some(route),
                            run_handle_id: Some(&req_id),
                            cwd: Some("D:/demo"),
                            runtime_permission: Some("project_write"),
                        })
                        .expect("concurrent task start should persist");
                    for chunk in 0..chunks_per_task {
                        journal
                            .record_cli_chunk(
                                &req_id,
                                "runtime",
                                &format!("worker {worker} task {task} chunk {chunk}\n"),
                            )
                            .expect("concurrent task chunk should persist");
                    }
                    if task % 4 == 0 {
                        journal
                            .record_cancel_requested(&req_id)
                            .expect("concurrent cancel should persist");
                        journal
                            .record_finished_with_outcome(
                                &req_id,
                                "canceled",
                                Some("用户已停止 PC CLI 任务"),
                            )
                            .expect("concurrent canceled finish should persist");
                    } else if task % 4 == 1 {
                        journal
                            .record_finished_with_outcome(
                                &req_id,
                                "failed",
                                Some("provider unavailable"),
                            )
                            .expect("concurrent failed finish should persist");
                    } else {
                        journal
                            .record_finished_with_outcome(&req_id, "done", None)
                            .expect("concurrent done finish should persist");
                    }
                    journal
                        .record_finished(&req_id)
                        .expect("generic cleanup should stay idempotent under concurrency");
                }
            })
        })
        .collect::<Vec<_>>();

    for handle in handles {
        handle.join().expect("pressure worker should not panic");
    }

    let expected_tasks = workers * tasks_per_worker;
    let registry = read_registry(&dir);
    assert_eq!(
        registry.len(),
        expected_tasks,
        "concurrent read/modify/write must not lose registry records"
    );
    assert_eq!(count_status(&registry, "canceled"), workers * 6);
    assert_eq!(count_status(&registry, "failed"), workers * 6);
    assert_eq!(count_status(&registry, "done"), workers * 12);

    let events = fs::read_to_string(dir.join("events.jsonl")).expect("events should read");
    assert_eq!(
        events.matches(r#""type":"started""#).count(),
        expected_tasks
    );
    assert_eq!(
        events.matches(r#""type":"cli_chunk""#).count(),
        expected_tasks * chunks_per_task
    );
    assert_eq!(
        events.matches(r#""type":"cancel_requested""#).count(),
        workers * 6
    );
    assert_eq!(
        events.matches(r#""type":"finished""#).count(),
        expected_tasks
    );

    let sample = journal
        .snapshot("req-w15-t23", 0, 20)
        .expect("concurrent sample snapshot should read");
    assert_eq!(
        sample.record.as_ref().map(|record| record.status.as_str()),
        Some("done")
    );
    assert_eq!(
        sample.events.len(),
        5,
        "started + 3 chunks + finished should be replayable for the sampled task"
    );
    assert!(sample
        .events
        .iter()
        .all(|event| { event.event.get("req_id").and_then(Value::as_str) == Some("req-w15-t23") }));

    let latest = journal
        .latest_records(500)
        .expect("latest records should read after concurrent pressure");
    assert_eq!(latest.len(), 100, "latest_records must keep public clamp");

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn stress_restart_resume_contract_never_claims_lost_control_handles() {
    let dir = unique_test_dir("restart-resume-contract-pressure");
    let _ = fs::remove_dir_all(&dir);
    let journal = TaskJournal::new(&dir);
    let routes = [
        ("codex", "route_a_external_cli"),
        ("api-runtime", "route_b_api_runtime"),
        ("server-runtime", "route_c_server_runtime"),
    ];

    for task_index in 0..270 {
        let (cli_name, route) = routes[task_index % routes.len()];
        let req_id = format!("req-restart-{task_index:03}");
        journal
            .record_started(TaskJournalStart {
                req_id: &req_id,
                cli_name,
                route: Some(route),
                run_handle_id: Some(&req_id),
                cwd: Some("D:/demo"),
                runtime_permission: Some("project_write"),
            })
            .expect("restart pressure task should start");
        journal
            .record_process_started(&req_id, 10_000 + task_index as u32)
            .expect("process pid should persist before restart");
        if cli_name == "codex" {
            let scope_key = format!("scope-{task_index:03}");
            let session_id = format!("session-{task_index:03}");
            journal
                .record_codex_session(&req_id, &scope_key, &session_id)
                .expect("codex session should persist before restart");
        }
        if task_index % 5 == 0 {
            journal
                .record_cancel_requested(&req_id)
                .expect("cancel request should persist before restart");
        }
    }

    let registry = read_registry(&dir);
    assert_eq!(registry.len(), 270);
    for record in registry.values() {
        let attach = task_attach_state(Some(record), None);
        let resume = task_resume_contract(&attach);
        let resume_json =
            serde_json::to_value(resume).expect("restart resume contract should serialize");

        assert_eq!(resume_json["status"], "detached");
        assert_eq!(resume_json["strategy"]["kind"], "snapshot_continue");
        assert_eq!(resume_json["next_action"], "continue_from_snapshot");
        assert_eq!(resume_json["can_reconnect"], false);
        assert_eq!(resume_json["can_cancel"], false);
        assert_eq!(resume_json["can_stream_live_output"], false);
        assert_eq!(resume_json["can_approve_tools"], false);
        assert_eq!(resume_json["can_replay_journal_events"], true);
        assert_eq!(resume_json["run_handle"], Value::Null);
        assert_eq!(resume_json["strategy"]["requires_new_task"], true);

        if record.cli_name == "codex" {
            assert_eq!(resume_json["can_resume_codex_session"], true);
            assert!(resume_json["codex_session"]["id"]
                .as_str()
                .is_some_and(|value| value.starts_with("session-")));
        } else {
            assert_eq!(resume_json["can_resume_codex_session"], false);
            assert_eq!(resume_json["codex_session"], Value::Null);
        }
    }

    let _ = fs::remove_dir_all(dir);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn stress_restart_waiting_approvals_fall_back_to_snapshot_continue() {
    let dir = unique_test_dir("restart-waiting-approval-pressure");
    let _ = fs::remove_dir_all(&dir);
    let journal = TaskJournal::new(&dir);
    let registry = ActiveCliPromptRegistry::new();
    let approval_state = ToolApprovalState::default();
    let task_count = 72;

    for task_index in 0..task_count {
        let route = if task_index % 2 == 0 {
            "route_b_api_runtime"
        } else {
            "route_c_server_runtime"
        };
        let cli_name = if task_index % 2 == 0 {
            "api-runtime"
        } else {
            "server-runtime"
        };
        let req_id = format!("req-restart-approval-{task_index:03}");
        let approval_id = format!("tap_{task_index}_1");
        journal
            .record_started(TaskJournalStart {
                req_id: &req_id,
                cli_name,
                route: Some(route),
                run_handle_id: Some(&req_id),
                cwd: Some("D:/demo"),
                runtime_permission: Some("project_write"),
            })
            .expect("waiting-approval task should start before restart pressure");
        journal
            .record_cli_chunk(
                &req_id,
                "runtime",
                &serde_json::to_string(&json!({
                    "type": "tool_approval_required",
                    "approval_id": approval_id,
                    "tool": "write_file",
                    "status": "waiting_approval"
                }))
                .expect("approval event should serialize"),
            )
            .expect("waiting approval event should persist before restart");

        let (_waiter, pending) = {
            let waiter = approval_state.register(&req_id, &approval_id).await;
            let pending = approval_state.pending_for_req(&req_id).await;
            (waiter, pending)
        };
        assert_eq!(
            pending
                .iter()
                .map(|approval| approval.approval_id.as_str())
                .collect::<Vec<_>>(),
            vec![approval_id.as_str()],
            "live task should expose its pending approval before restart"
        );

        let (handle, _cancel_rx) = active_handle_with_rx(&req_id, route);
        assert!(
            registry.try_insert(handle).await,
            "live handle should be inserted before restart"
        );
        let active = registry
            .view(&req_id, pending)
            .await
            .expect("live handle should be visible before restart");
        let record = read_registry(&dir)
            .get(&req_id)
            .cloned()
            .expect("journal record should exist before restart");
        let live_resume = task_resume_contract(&task_attach_state(Some(&record), Some(active)));
        let live_resume_json =
            serde_json::to_value(live_resume).expect("live resume should serialize");
        assert_eq!(live_resume_json["status"], "live");
        assert_eq!(live_resume_json["can_approve_tools"], true);
        assert_eq!(
            live_resume_json["active_approval_ids"],
            json!([approval_id])
        );
        assert_eq!(
            live_resume_json["strategy"]["kind"],
            "control_handle_reconnect"
        );
    }

    let restarted_approval_state = ToolApprovalState::default();
    let registry_after_restart = read_registry(&dir);
    assert_eq!(registry_after_restart.len(), task_count);
    for task_index in 0..task_count {
        let req_id = format!("req-restart-approval-{task_index:03}");
        let approval_id = format!("tap_{task_index}_1");
        let record = registry_after_restart
            .get(&req_id)
            .expect("waiting-approval record should survive restart");
        let attach = task_attach_state(Some(record), None);
        let resume = task_resume_contract(&attach);
        let resume_json = serde_json::to_value(resume).expect("detached resume should serialize");

        assert_eq!(resume_json["status"], "detached");
        assert_eq!(resume_json["strategy"]["kind"], "snapshot_continue");
        assert_eq!(resume_json["next_action"], "continue_from_snapshot");
        assert_eq!(resume_json["can_reconnect"], false);
        assert_eq!(resume_json["can_cancel"], false);
        assert_eq!(resume_json["can_approve_tools"], false);
        assert_eq!(resume_json["active_approval_ids"], json!([]));
        assert_eq!(resume_json["run_handle"], Value::Null);
        assert!(
            restarted_approval_state
                .pending_for_req(&req_id)
                .await
                .is_empty(),
            "approval waiters are in-memory only and must not survive restart"
        );

        let snapshot = journal
            .snapshot(&req_id, 0, 20)
            .expect("waiting approval snapshot should replay after restart");
        assert_eq!(snapshot.approvals.pending_count, 1);
        assert_eq!(snapshot.approvals.approvals[0].approval_id, approval_id);
        let resume_with_journal =
            task_resume_contract_with_journal_approvals(&attach, &snapshot.approvals);
        let resume_with_journal_json =
            serde_json::to_value(resume_with_journal).expect("journal resume should serialize");
        assert_eq!(
            resume_with_journal_json["tool_approval_recovery"]["journal_pending_approval_ids"],
            json!([approval_id])
        );
        assert_eq!(
            resume_with_journal_json["tool_approval_recovery"]["journal_pending_count"],
            json!(1)
        );
        assert_eq!(resume_with_journal_json["can_approve_tools"], false);
        let approval_state =
            snapshot
                .approvals
                .resolve_runtime_state_for_task_status(&[], false, None);
        assert_eq!(approval_state.actionable_count, 0);
        assert_eq!(approval_state.unavailable_count, 1);
        assert_eq!(approval_state.approvals[0].status, "unavailable");
        assert!(!approval_state.approvals[0].actionable);
        assert!(snapshot.events.iter().any(|event| {
            event.event.get("type").and_then(Value::as_str) == Some("tool_event")
                && event
                    .event
                    .get("event")
                    .and_then(|value| value.get("type"))
                    .and_then(Value::as_str)
                    == Some("tool_approval_required")
                && event
                    .event
                    .get("event")
                    .and_then(|value| value.get("approval_id"))
                    .and_then(Value::as_str)
                    == Some(approval_id.as_str())
        }));
    }

    let _ = fs::remove_dir_all(dir);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn stress_stale_codex_session_clear_is_scoped_under_many_tasks() {
    let dir = unique_test_dir("stale-codex-session-clear-pressure");
    let _ = fs::remove_dir_all(&dir);
    let journal = TaskJournal::new(&dir);
    let legacy = dir.join("legacy-codex-sessions.json");
    let task_count = 180;
    let mut legacy_sessions = BTreeMap::new();

    for task_index in 0..task_count {
        let req_id = format!("req-codex-session-{task_index:03}");
        let scope_key = format!("scope-{task_index:03}");
        let session_id = format!("session-{task_index:03}");
        journal
            .record_started(TaskJournalStart {
                req_id: &req_id,
                cli_name: "codex",
                route: Some("route_a_external_cli"),
                run_handle_id: Some(&req_id),
                cwd: Some("D:/demo"),
                runtime_permission: Some("project_write"),
            })
            .expect("codex session task should start");
        journal
            .record_codex_session(&req_id, &scope_key, &session_id)
            .expect("codex session should persist");
        legacy_sessions.insert(scope_key, format!("legacy-session-{task_index:03}"));
    }
    fs::write(
        &legacy,
        serde_json::to_string_pretty(&legacy_sessions).expect("legacy sessions should serialize"),
    )
    .expect("legacy sessions should write");

    for task_index in (0..task_count).step_by(5) {
        let req_id = format!("req-codex-session-{task_index:03}");
        let scope_key = format!("scope-{task_index:03}");
        clear_stale_session(&journal, &legacy, &req_id, &scope_key).await;
    }

    let registry = read_registry(&dir);
    assert_eq!(registry.len(), task_count);
    let legacy_after: BTreeMap<String, String> = serde_json::from_str(
        &fs::read_to_string(&legacy).expect("legacy sessions should read after clears"),
    )
    .expect("legacy sessions should parse after clears");

    for task_index in 0..task_count {
        let req_id = format!("req-codex-session-{task_index:03}");
        let scope_key = format!("scope-{task_index:03}");
        let record = registry
            .get(&req_id)
            .expect("codex session task should remain in registry");
        if task_index % 5 == 0 {
            assert_eq!(
                journal
                    .load_codex_session(&scope_key)
                    .expect("codex session cache should read"),
                None,
                "stale scope {scope_key} should be cleared from journal cache"
            );
            assert_eq!(
                record.codex_session_id, None,
                "stale scope {scope_key} should be cleared from its task record"
            );
            assert!(
                !legacy_after.contains_key(&scope_key),
                "stale scope {scope_key} should be cleared from legacy cache"
            );
        } else {
            let expected_session = format!("session-{task_index:03}");
            assert_eq!(
                journal
                    .load_codex_session(&scope_key)
                    .expect("codex session cache should read"),
                Some(expected_session.clone()),
                "unrelated scope {scope_key} must stay resumable"
            );
            assert_eq!(
                record.codex_session_id.as_deref(),
                Some(expected_session.as_str()),
                "unrelated task record {req_id} must keep its Codex session"
            );
            assert_eq!(
                legacy_after.get(&scope_key).map(String::as_str),
                Some(format!("legacy-session-{task_index:03}").as_str()),
                "unrelated legacy scope {scope_key} must stay intact"
            );
        }
    }

    let events = fs::read_to_string(dir.join("events.jsonl")).expect("events should read");
    assert_eq!(
        events.matches(r#""type":"codex_session_cleared""#).count(),
        task_count / 5
    );

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn stress_oversized_tool_events_remain_structured_and_bounded() {
    let dir = unique_test_dir("oversized-tool-event-pressure");
    let _ = fs::remove_dir_all(&dir);
    let journal = TaskJournal::new(&dir);
    let routes = [
        ("codex", "route_a_external_cli"),
        ("api-runtime", "route_b_api_runtime"),
        ("server-runtime", "route_c_server_runtime"),
    ];
    let long_result = "r".repeat(18_000);
    let long_error = "e".repeat(5_000);

    for task_index in 0..90 {
        let (cli_name, route) = routes[task_index % routes.len()];
        let req_id = format!("req-oversized-tool-{task_index:03}");
        journal
            .record_started(TaskJournalStart {
                req_id: &req_id,
                cli_name,
                route: Some(route),
                run_handle_id: Some(&req_id),
                cwd: Some("D:/demo"),
                runtime_permission: Some("project_write"),
            })
            .expect("oversized tool task should start");

        let tool_event = serde_json::to_string(&json!({
            "type": "tool_result",
            "tool": "run_command",
            "status": "ok",
            "result": long_result,
        }))
        .expect("tool event should serialize");
        assert!(
            tool_event.chars().count() > 12_000,
            "pressure fixture must exceed the journal text bound"
        );
        journal
            .record_cli_chunk(&req_id, "runtime", &tool_event)
            .expect("oversized tool event should persist");
        journal
            .record_finished_with_outcome(&req_id, "failed", Some(&long_error))
            .expect("oversized terminal error should persist");
    }

    let registry = read_registry(&dir);
    assert_eq!(registry.len(), 90);
    assert_eq!(count_status(&registry, "failed"), 90);

    let sample = journal
        .snapshot("req-oversized-tool-089", 0, 20)
        .expect("oversized tool snapshot should read");
    let tool_event = sample
        .events
        .iter()
        .find(|event| event.event.get("type").and_then(Value::as_str) == Some("tool_event"))
        .expect("oversized structured tool event should not degrade to cli_chunk");
    assert_eq!(tool_event.event["event"]["type"], "tool_result");
    assert_eq!(tool_event.event["event"]["tool"], "run_command");
    assert!(tool_event.event["event"]["result"]
        .as_str()
        .unwrap_or_default()
        .contains("本机 journal 输出已截断"));
    assert!(tool_event.event["text"]
        .as_str()
        .unwrap_or_default()
        .contains("本机 journal 输出已截断"));
    assert!(
        tool_event.event["text"]
            .as_str()
            .unwrap_or_default()
            .chars()
            .count()
            < 12_100,
        "saved raw tool text should stay bounded for replay"
    );

    let finished = sample
        .events
        .iter()
        .find(|event| event.event.get("type").and_then(Value::as_str) == Some("finished"))
        .expect("terminal event should be replayable");
    assert!(finished.event["error"]
        .as_str()
        .unwrap_or_default()
        .contains("本机 journal 输出已截断"));
    assert!(
        finished.event["error"]
            .as_str()
            .unwrap_or_default()
            .chars()
            .count()
            < 2_100,
        "terminal errors should stay bounded for the task journal UI"
    );

    let events = fs::read_to_string(dir.join("events.jsonl")).expect("events should read");
    assert_eq!(events.matches(r#""type":"tool_event""#).count(), 90);
    assert_eq!(events.matches(r#""type":"cli_chunk""#).count(), 0);

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn stress_corrupt_journal_lines_do_not_block_resume_replay() {
    let dir = unique_test_dir("corrupt-journal-replay-pressure");
    let _ = fs::remove_dir_all(&dir);
    let journal = TaskJournal::new(&dir);
    let target_req = "req-route-c-corrupt-replay";
    journal
        .record_started(TaskJournalStart {
            req_id: target_req,
            cli_name: "server-runtime",
            route: Some("route_c_server_runtime"),
            run_handle_id: Some(target_req),
            cwd: Some("D:/demo"),
            runtime_permission: Some("project_write"),
        })
        .expect("target task should start before corrupt journal pressure");

    for index in 0..48 {
        append_corrupt_event_line(&dir, index);
        journal
            .record_cli_chunk(target_req, "runtime", &format!("target chunk {index}\n"))
            .expect("valid target event should persist after corrupt line");
    }
    append_corrupt_event_line(&dir, 999);
    journal
        .record_finished_with_outcome(target_req, "done", None)
        .expect("target task should finish after corrupt journal pressure");

    let snapshot = journal
        .snapshot(target_req, 0, 200)
        .expect("snapshot should skip corrupt journal lines");
    assert_eq!(
        snapshot.events.len(),
        50,
        "started + 48 chunks + finished should all replay despite corrupt lines"
    );
    assert!(!snapshot.has_more);
    assert!(snapshot.last_event_seq > snapshot.events.len());
    assert!(snapshot
        .events
        .iter()
        .all(|event| { event.event.get("req_id").and_then(Value::as_str) == Some(target_req) }));
    assert_eq!(
        snapshot
            .record
            .as_ref()
            .map(|record| record.status.as_str()),
        Some("done")
    );

    let _ = fs::remove_dir_all(dir);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn stress_active_registry_rejects_duplicate_handles_and_cleans_up() {
    let registry = Arc::new(ActiveCliPromptRegistry::new());
    let task_count = 96;
    let duplicate_attempts_per_task = 12;
    let mut cancel_receivers = Vec::new();

    for task_index in 0..task_count {
        let req_id = format!("req-active-{task_index:03}");
        let (handle, cancel_rx) = active_handle_with_rx(&req_id, "route_a_external_cli");
        assert!(
            registry.try_insert(handle).await,
            "initial active handle should be inserted"
        );
        cancel_receivers.push((req_id, cancel_rx));
    }

    let mut duplicate_workers = Vec::new();
    for attempt in 0..duplicate_attempts_per_task {
        for task_index in 0..task_count {
            let registry = Arc::clone(&registry);
            duplicate_workers.push(tokio::spawn(async move {
                let req_id = format!("req-active-{task_index:03}");
                let route = match attempt % 3 {
                    0 => "route_a_external_cli",
                    1 => "route_b_api_runtime",
                    _ => "route_c_server_runtime",
                };
                let (handle, _duplicate_cancel_rx) = active_handle_with_rx(&req_id, route);
                assert!(
                    !registry.try_insert(handle).await,
                    "duplicate active handle must not replace {req_id}"
                );
                let view = registry
                    .view(&req_id, Vec::new())
                    .await
                    .expect("original active handle should remain visible");
                assert_eq!(
                    view.route, "route_a_external_cli",
                    "duplicate route must not replace original handle"
                );
            }));
        }
    }

    for worker in duplicate_workers {
        worker
            .await
            .expect("duplicate active registry worker should not panic");
    }
    assert_eq!(registry.len().await, task_count);

    for task_index in 0..task_count {
        let req_id = format!("req-active-{task_index:03}");
        registry
            .set_os_pid(&req_id, Some(20_000 + task_index as u32))
            .await;
        let cancel_tx = registry
            .cancel_tx(&req_id)
            .await
            .expect("live handle should expose cancel sender");
        assert!(
            cancel_tx.send(true).is_ok(),
            "cancel should reach the original active handle"
        );
        let view = registry
            .view(&req_id, Vec::new())
            .await
            .expect("active handle should remain visible until removed");
        assert_eq!(view.os_pid, Some(20_000 + task_index as u32));
    }

    for (_req_id, mut cancel_rx) in cancel_receivers {
        assert!(
            cancel_rx.changed().await.is_ok(),
            "original cancel receiver should observe cancellation"
        );
        assert!(*cancel_rx.borrow());
    }

    let removed_count = Arc::new(AtomicUsize::new(0));
    let mut remove_workers = Vec::new();
    for _attempt in 0..2 {
        for task_index in 0..task_count {
            let registry = Arc::clone(&registry);
            let removed_count = Arc::clone(&removed_count);
            remove_workers.push(tokio::spawn(async move {
                let req_id = format!("req-active-{task_index:03}");
                if registry.remove(&req_id).await {
                    removed_count.fetch_add(1, Ordering::SeqCst);
                }
            }));
        }
    }

    for worker in remove_workers {
        worker
            .await
            .expect("active registry remove worker should not panic");
    }
    assert_eq!(removed_count.load(Ordering::SeqCst), task_count);
    assert_eq!(registry.len().await, 0);
    assert!(registry.cancel_tx("req-active-000").await.is_none());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn stress_tool_approval_state_keeps_waiters_scoped_and_single_consume() {
    let state = ToolApprovalState::default();
    let task_count = 64;
    let approvals_per_task = 4;
    let mut waiters = Vec::new();

    for task_index in 0..task_count {
        let req_id = format!("req-approval-{task_index:03}");
        for approval_index in 0..approvals_per_task {
            let approval_id = format!("tap_{task_index}_{approval_index}");
            let expected = if (task_index + approval_index) % 2 == 0 {
                ToolApprovalDecision::Approve
            } else {
                ToolApprovalDecision::Deny
            };
            let waiter = state.register(&req_id, &approval_id).await;
            waiters.push((req_id.clone(), approval_id, expected, waiter));
        }
    }

    for task_index in 0..task_count {
        let req_id = format!("req-approval-{task_index:03}");
        let pending = state.pending_for_req(&req_id).await;
        assert_eq!(
            pending.len(),
            approvals_per_task,
            "pending approval list must stay scoped to {req_id}"
        );
        assert!(pending.iter().all(|approval| approval
            .approval_id
            .starts_with(&format!("tap_{task_index}_"))));
    }

    let consumed_count = Arc::new(AtomicUsize::new(0));
    let mut deciders = Vec::new();
    for task_index in 0..task_count {
        for approval_index in 0..approvals_per_task {
            let req_id = format!("req-approval-{task_index:03}");
            let approval_id = format!("tap_{task_index}_{approval_index}");
            let decision = if (task_index + approval_index) % 2 == 0 {
                "approve"
            } else {
                "deny"
            };
            for duplicate_attempt in 0..2 {
                let state = state.clone();
                let req_id = req_id.clone();
                let approval_id = approval_id.clone();
                let consumed_count = Arc::clone(&consumed_count);
                deciders.push(tokio::spawn(async move {
                    assert!(
                        !state.decide(&req_id, "missing-approval", "approve").await,
                        "wrong approval id must not consume {req_id}"
                    );
                    assert!(
                        !state.decide(&req_id, &approval_id, "maybe").await,
                        "invalid decision must not consume {req_id}/{approval_id}"
                    );
                    if state.decide(&req_id, &approval_id, decision).await {
                        consumed_count.fetch_add(1, Ordering::SeqCst);
                    } else if duplicate_attempt == 0 {
                        assert!(
                            state.pending_for_req(&req_id).await.is_empty()
                                || !state
                                    .pending_for_req(&req_id)
                                    .await
                                    .iter()
                                    .any(|pending| pending.approval_id == approval_id),
                            "failed first attempt should only happen after another duplicate consumed it"
                        );
                    }
                }));
            }
        }
    }

    for decider in deciders {
        decider
            .await
            .expect("approval pressure decider should not panic");
    }
    assert_eq!(
        consumed_count.load(Ordering::SeqCst),
        task_count * approvals_per_task,
        "each pending approval must be consumed exactly once"
    );

    for (req_id, _approval_id, expected, mut waiter) in waiters {
        assert!(
            waiter.changed().await,
            "waiter for {req_id} should observe a terminal decision"
        );
        assert_eq!(waiter.decision(), Some(expected));
        waiter.cleanup().await;
    }

    for task_index in 0..task_count {
        let req_id = format!("req-approval-{task_index:03}");
        assert!(
            state.pending_for_req(&req_id).await.is_empty(),
            "all approvals for {req_id} should be consumed or cleaned up"
        );
    }
}

fn read_registry(dir: &std::path::Path) -> BTreeMap<String, TaskJournalRecord> {
    let text = fs::read_to_string(dir.join("registry.json")).expect("registry should read");
    serde_json::from_str(&text).expect("registry should parse")
}

fn append_corrupt_event_line(dir: &std::path::Path, index: usize) {
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(dir.join("events.jsonl"))
        .expect("events file should open for corrupt fixture");
    writeln!(
        file,
        "{{\"type\":\"cli_chunk\",\"req_id\":\"broken-{index}\""
    )
    .expect("corrupt fixture line should write");
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

fn count_status(registry: &BTreeMap<String, TaskJournalRecord>, status: &str) -> usize {
    registry
        .values()
        .filter(|record| record.status == status)
        .count()
}

fn unique_test_dir(suffix: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "elon-task-lifecycle-pressure-{}-{suffix}",
        std::process::id()
    ))
}

fn active_handle_with_rx(
    req_id: &str,
    route: &str,
) -> (ActiveCliPromptHandle, watch::Receiver<bool>) {
    let (cancel_tx, cancel_rx) = watch::channel(false);
    (
        ActiveCliPromptHandle::new(
            req_id.to_string(),
            "codex".to_string(),
            route.to_string(),
            Some("D:/demo".to_string()),
            Some("project_write".to_string()),
            cancel_tx,
        ),
        cancel_rx,
    )
}

fn journal_record_for_workspace(
    req_id: &str,
    cwd: &std::path::Path,
    updated_at_ms: u128,
) -> TaskJournalRecord {
    TaskJournalRecord {
        req_id: req_id.to_string(),
        cli_name: "server-runtime".to_string(),
        route: Some("route_c_server_runtime".to_string()),
        run_handle_id: Some(req_id.to_string()),
        cwd: Some(cwd.to_string_lossy().to_string()),
        runtime_permission: Some("project_write".to_string()),
        os_pid: None,
        process_started_at_ms: None,
        codex_session_id: None,
        codex_session_scope_key: None,
        codex_session_updated_at_ms: None,
        status: "canceled".to_string(),
        started_at_ms: updated_at_ms.saturating_sub(5),
        updated_at_ms,
        cancel_requested_at_ms: Some(updated_at_ms.saturating_sub(1)),
    }
}
