use super::*;
use serde_json::json;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn lists_project_agent_runs_without_file_contents() {
    let root = temp_dir("agent-runs-list");
    let log_dir = root.join(".elon").join("agent-runs");
    fs::create_dir_all(&log_dir).unwrap();
    fs::write(
            log_dir.join("run-1.jsonl"),
            [
                r#"{"ts":"2026-06-23T01:00:00Z","run_id":"run-1","type":"run_started","data":{"mode":"api-runtime","prompt_chars":12}}"#,
                r#"{"ts":"2026-06-23T01:00:01Z","run_id":"run-1","type":"turn_started","data":{"turn":1}}"#,
                r#"{"ts":"2026-06-23T01:00:02Z","run_id":"run-1","type":"tool_started","data":{"turn":1,"tool":"read_file","target":"README.md"}}"#,
                r#"{"ts":"2026-06-23T01:00:03Z","run_id":"run-1","type":"tool_finished","data":{"turn":1,"tool":"read_file","target":"README.md","result_chars":120}}"#,
                r#"{"ts":"2026-06-23T01:00:04Z","run_id":"run-1","type":"run_finished","data":{"status":"completed","run_commands_used":0}}"#,
            ]
            .join("\n"),
        )
        .unwrap();

    let response = list_project_agent_runs(&ProjectAgentRunsReq {
        workspace_path: root.to_string_lossy().to_string(),
        limit: Some(10),
        event_limit: Some(3),
    })
    .unwrap();
    let _ = fs::remove_dir_all(root);

    assert!(response.ok);
    assert!(response.active_controls.is_empty());
    assert!(response.recent_tasks.is_empty());
    assert!(response.recovery_entry.is_none());
    assert_eq!(response.runs.len(), 1);
    let run = &response.runs[0];
    assert_eq!(run.run_id, "run-1");
    assert_eq!(run.status, "completed");
    assert_eq!(run.mode.as_deref(), Some("api-runtime"));
    assert_eq!(run.event_count, 5);
    assert_eq!(run.turn_count, 1);
    assert_eq!(run.tool_count, 1);
    assert_eq!(run.tool_names, vec!["read_file"]);
    assert_eq!(run.events.len(), 3);
    let serialized = serde_json::to_string(run).unwrap();
    assert!(!serialized.contains("file content"));
    assert!(!serialized.contains("prompt"));
}

#[test]
fn missing_agent_runs_directory_returns_empty_list() {
    let root = temp_dir("agent-runs-empty");
    fs::create_dir_all(&root).unwrap();
    let response = list_project_agent_runs(&ProjectAgentRunsReq {
        workspace_path: root.to_string_lossy().to_string(),
        limit: None,
        event_limit: None,
    })
    .unwrap();
    let _ = fs::remove_dir_all(root);

    assert!(response.ok);
    assert!(response.runs.is_empty());
}

#[test]
fn stress_agent_run_summary_reads_long_run_to_terminal_status() {
    let root = temp_dir("agent-runs-long-terminal");
    let log_dir = root.join(".elon").join("agent-runs");
    fs::create_dir_all(&log_dir).unwrap();
    let mut lines = Vec::new();
    lines.push(
            r#"{"ts":"2026-06-23T01:00:00Z","run_id":"run-long","type":"run_started","data":{"mode":"server-runtime","prompt_chars":22,"max_context_chars":60000}}"#
                .to_string(),
        );
    for turn in 1..=2_100 {
        lines.push(format!(
                r#"{{"ts":"2026-06-23T01:00:01Z","run_id":"run-long","type":"turn_started","data":{{"turn":{turn}}}}}"#
            ));
        lines.push(format!(
                r#"{{"ts":"2026-06-23T01:00:02Z","run_id":"run-long","type":"tool_started","data":{{"turn":{turn},"tool":"read_file","target":"src/lib.rs"}}}}"#
            ));
        lines.push(format!(
                r#"{{"ts":"2026-06-23T01:00:03Z","run_id":"run-long","type":"tool_finished","data":{{"turn":{turn},"tool":"read_file","target":"src/lib.rs","result_chars":64}}}}"#
            ));
    }
    lines.push(
            r#"{"ts":"2026-06-23T01:59:58Z","run_id":"run-long","type":"context_compacted","data":{"turn":2100,"before_chars":90000,"after_chars":42000,"omitted_messages":300,"omitted_chars":48000,"max_context_chars":60000,"compaction_count":7}}"#
                .to_string(),
        );
    lines.push(
            r#"{"ts":"2026-06-23T01:59:59Z","run_id":"run-long","type":"run_finished","data":{"status":"completed","run_commands_used":8,"context_compactions":7}}"#
                .to_string(),
        );
    fs::write(log_dir.join("run-long.jsonl"), lines.join("\n")).unwrap();

    let response = list_project_agent_runs(&ProjectAgentRunsReq {
        workspace_path: root.to_string_lossy().to_string(),
        limit: Some(1),
        event_limit: Some(5),
    })
    .unwrap();
    let _ = fs::remove_dir_all(root);

    let run = response.runs.first().expect("long run should be listed");
    assert_eq!(run.run_id, "run-long");
    assert_eq!(run.status, "completed");
    assert_eq!(run.mode.as_deref(), Some("server-runtime"));
    assert_eq!(run.event_count, 6_303);
    assert_eq!(run.scanned_event_count, run.event_count);
    assert!(run.truncated);
    assert_eq!(run.turn_count, 2_100);
    assert_eq!(run.tool_count, 2_100);
    assert_eq!(run.tool_names, vec!["read_file"]);
    assert_eq!(run.events.len(), 5);
    assert_eq!(
        run.events.last().map(|event| event.event_type.as_str()),
        Some("run_finished")
    );
    assert!(serde_json::to_string(run)
        .unwrap()
        .contains("context_compacted"));
}

#[test]
fn rejects_missing_workspace() {
    let missing = temp_dir("agent-runs-missing");
    let error = list_project_agent_runs(&ProjectAgentRunsReq {
        workspace_path: missing.to_string_lossy().to_string(),
        limit: None,
        event_limit: None,
    })
    .unwrap_err();
    assert!(error.to_string().contains("PC 本地路径不存在"));
}

#[test]
fn task_resume_view_uses_snapshot_continue_contract() {
    let view = project_task_resume_view(TaskJournalRecord {
        req_id: "req-detached".to_string(),
        cli_name: "server-runtime".to_string(),
        route: Some("route_c_server_runtime".to_string()),
        run_handle_id: Some("req-detached".to_string()),
        cwd: Some("D:/demo".to_string()),
        runtime_permission: Some("project_write".to_string()),
        os_pid: Some(42),
        process_started_at_ms: Some(100),
        codex_session_id: None,
        codex_session_scope_key: None,
        codex_session_updated_at_ms: None,
        status: "cancel_requested".to_string(),
        started_at_ms: 100,
        updated_at_ms: 200,
        cancel_requested_at_ms: Some(180),
    });

    assert_eq!(view.task_id, "req-detached");
    assert_eq!(view.status, "cancel_requested");
    let serialized = serde_json::to_string(&view).unwrap();
    assert!(serialized.contains("continue_from_snapshot"));
    assert!(serialized.contains("tool_approval_recovery"));
    assert!(serialized.contains("lost_after_restart"));
    assert!(serialized.contains("本机 journal"));
    assert!(!serialized.contains("secret prompt"));
    assert!(!serialized.contains("sk-live-secret"));
}

#[test]
fn task_resume_view_includes_journal_pending_approvals_without_enabling_clicks() {
    let mut approval_tracker =
        crate::node_agent_task_approval_snapshot::TaskApprovalJournalTracker::default();
    approval_tracker.observe_event(
        1,
        &json!({
            "type": "tool_approval_required",
            "approval_id": "tap_restart_pending",
            "tool": "write_file"
        }),
    );
    let approvals = approval_tracker.finish();
    let view = project_task_resume_view_with_approvals(
        TaskJournalRecord {
            req_id: "req-detached-approval".to_string(),
            cli_name: "server-runtime".to_string(),
            route: Some("route_c_server_runtime".to_string()),
            run_handle_id: Some("req-detached-approval".to_string()),
            cwd: Some("D:/demo".to_string()),
            runtime_permission: Some("project_write".to_string()),
            os_pid: Some(42),
            process_started_at_ms: Some(100),
            codex_session_id: None,
            codex_session_scope_key: None,
            codex_session_updated_at_ms: None,
            status: "running".to_string(),
            started_at_ms: 100,
            updated_at_ms: 200,
            cancel_requested_at_ms: None,
        },
        Some(&approvals),
    );
    let resume = serde_json::to_value(&view.resume).unwrap();

    assert_eq!(resume["status"], "detached");
    assert_eq!(resume["can_approve_tools"], false);
    assert_eq!(resume["active_approval_ids"], json!([]));
    assert_eq!(
        resume["tool_approval_recovery"]["journal_pending_approval_ids"],
        json!(["tap_restart_pending"])
    );
    assert_eq!(
        resume["tool_approval_recovery"]["journal_pending_count"],
        json!(1)
    );
    assert_eq!(
        resume["tool_approval_recovery"]["pending_after_restart_action"],
        "continue_from_snapshot"
    );
}

#[test]
fn recent_task_resume_views_filter_active_ids_dedupe_and_cap() {
    let records = vec![
        task_record("req-live", 900),
        task_record("req-9", 899),
        task_record("req-8", 898),
        task_record("req-8", 897),
        task_record("req-7", 896),
        task_record("req-6", 895),
        task_record("req-5", 894),
        task_record("req-4", 893),
        task_record("req-3", 892),
    ];
    let active = BTreeSet::from(["req-live".to_string()]);

    let views = recent_task_resume_views(records, &active, 6);

    assert_eq!(views.len(), 6);
    assert_eq!(
        views
            .iter()
            .map(|view| view.task_id.as_str())
            .collect::<Vec<_>>(),
        vec!["req-9", "req-8", "req-7", "req-6", "req-5", "req-4"]
    );
    assert!(views.iter().all(|view| {
        let resume = serde_json::to_value(&view.resume).expect("resume should serialize");
        resume["next_action"] == "continue_from_snapshot" && resume["can_cancel"] == false
    }));
}

#[test]
fn recovery_entry_prefers_live_control_handle() {
    let control = ProjectAgentRunControl {
        task_id: "req-live".to_string(),
        run_handle_id: "req-live".to_string(),
        cli_name: "server-runtime".to_string(),
        route: "route_c_server_runtime".to_string(),
        cwd: Some("D:/demo".to_string()),
        runtime_permission: Some("project_write".to_string()),
        started_at_ms: 100,
        last_heartbeat_ms: 200,
        control_lease_expires_at_ms: 47_000,
        os_pid: Some(1234),
        can_cancel: true,
    };
    let recent = project_task_resume_view(task_record("req-detached", 190));

    let entry = recovery_entry_from(&[control], &[recent]).expect("entry should exist");

    assert_eq!(entry.kind, "active_control");
    assert_eq!(entry.task_id, "req-live");
    assert_eq!(entry.status, "running");
    assert_eq!(entry.recommended_action, "wait_or_cancel");
    assert!(!entry.tty_reconnect.supported);
    assert_eq!(entry.tty_reconnect.user_label, "原 CLI 终端不可重接");
    assert_eq!(entry.tty_reconnect.fallback_action, "wait_or_cancel");
    assert!(entry.can_cancel);
    assert!(!entry.can_continue);
}

#[test]
fn recovery_entry_points_to_snapshot_continue_without_secrets() {
    let mut record = task_record("req-detached", 900);
    record.codex_session_id = Some("session-secret-uuid".to_string());
    record.codex_session_scope_key = Some("scope-secret".to_string());
    let recent = project_task_resume_view(record);

    let entry = recovery_entry_from(&[], &[recent]).expect("entry should exist");
    let serialized = serde_json::to_string(&entry).unwrap();

    assert_eq!(entry.kind, "snapshot_resume");
    assert_eq!(entry.task_id, "req-detached");
    assert_eq!(entry.status, "terminal");
    assert_eq!(entry.recommended_action, "continue_from_snapshot");
    assert!(!entry.tty_reconnect.supported);
    assert_eq!(entry.tty_reconnect.user_label, "原 CLI 终端不可重接");
    assert_eq!(
        entry.tty_reconnect.fallback_action,
        "continue_from_snapshot"
    );
    assert!(!entry.can_cancel);
    assert!(entry.can_continue);
    assert!(serialized.contains("本机进程已经结束"));
    assert!(!serialized.contains("session-secret-uuid"));
    assert!(!serialized.contains("scope-secret"));
}

fn temp_dir(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("elon-{label}-{}-{nanos}", std::process::id()))
}

fn task_record(req_id: &str, updated_at_ms: u128) -> TaskJournalRecord {
    TaskJournalRecord {
        req_id: req_id.to_string(),
        cli_name: "server-runtime".to_string(),
        route: Some("route_c_server_runtime".to_string()),
        run_handle_id: Some(req_id.to_string()),
        cwd: Some("D:/demo".to_string()),
        runtime_permission: Some("project_write".to_string()),
        os_pid: None,
        process_started_at_ms: None,
        codex_session_id: None,
        codex_session_scope_key: None,
        codex_session_updated_at_ms: None,
        status: "canceled".to_string(),
        started_at_ms: updated_at_ms.saturating_sub(10),
        updated_at_ms,
        cancel_requested_at_ms: Some(updated_at_ms.saturating_sub(1)),
    }
}
