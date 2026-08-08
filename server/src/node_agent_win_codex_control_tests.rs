use super::*;

#[test]
fn action_contract_rejects_arbitrary_urls_scripts_and_routes() {
    let hub = WinCodexControlHub::default();
    assert!(hub
        .enqueue_action("trace", "navigate", Some("/codex-control"), "test")
        .is_ok());
    assert!(hub
        .enqueue_action("trace", "navigate", Some("https://example.com"), "test")
        .is_err());
    assert!(hub
        .enqueue_action("trace", "navigate", Some("/unknown"), "test")
        .is_err());
    assert!(hub
        .enqueue_action("trace", "eval_javascript", None, "test")
        .is_err());
}

#[test]
fn event_fields_and_sensitive_summaries_are_redacted() {
    let hub = WinCodexControlHub::default();
    let event = hub.record(
        "trace-1",
        "network",
        "info",
        "request.done",
        "Authorization: Bearer private",
        json!({"path":"/api/me", "cookie":"private", "nested":{"access_token":"private"}}),
    );
    assert!(event.summary.contains("已脱敏"));
    assert_eq!(event.fields["cookie"], "[REDACTED]");
    assert_eq!(event.fields["nested"]["access_token"], "[REDACTED]");
}

#[test]
fn receipts_are_idempotent_but_conflicting_terminal_states_fail() {
    let hub = WinCodexControlHub::default();
    let action = hub
        .enqueue_action("trace", "focus_window", None, "test")
        .unwrap();
    let receipt = WinControlReceipt {
        status: "succeeded".to_string(),
        message: Some("focused".to_string()),
        route: None,
        at_ms: None,
    };
    assert_eq!(
        hub.claim_action(&action.action_id).unwrap().status,
        "executing"
    );
    assert!(hub
        .record_receipt(&action.action_id, receipt.clone())
        .is_ok());
    assert!(hub.record_receipt(&action.action_id, receipt).is_ok());
    assert!(hub
        .record_receipt(
            &action.action_id,
            WinControlReceipt {
                status: "failed".to_string(),
                message: None,
                route: None,
                at_ms: None,
            },
        )
        .is_err());
}

#[test]
fn claiming_an_action_removes_it_from_the_pending_queue() {
    let hub = WinCodexControlHub::default();
    let action = hub
        .enqueue_action("trace", "reload_page", None, "test")
        .unwrap();
    assert_eq!(hub.pending_actions(10).len(), 1);
    hub.claim_action(&action.action_id).unwrap();
    assert!(hub.pending_actions(10).is_empty());
    assert_eq!(
        hub.claim_action(&action.action_id).unwrap().status,
        "executing"
    );
}

#[test]
fn source_filter_and_cursor_are_bounded() {
    let hub = WinCodexControlHub::default();
    let first = hub.record("t", "frontend", "info", "ready", "ready", json!({}));
    let second = hub.record("t", "network", "info", "request", "request", json!({}));
    let sources = HashSet::from(["network".to_string()]);
    let events = hub.events(first.seq, 50, &sources);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].event_id, second.event_id);
}

#[test]
fn recent_cli_projection_keeps_the_newest_events() {
    let dir = std::env::temp_dir().join(format!(
        "elon_win_control_recent_cli_{}",
        uuid::Uuid::new_v4().simple()
    ));
    let journal = crate::node_agent_task_journal::TaskJournal::new(&dir);
    journal
        .record_started(crate::node_agent_task_journal::TaskJournalStart {
            req_id: "task-recent",
            cli_name: "codex",
            route: Some("route_a"),
            run_handle_id: None,
            cwd: None,
            runtime_permission: None,
        })
        .unwrap();
    for index in 0..12 {
        journal
            .record_cli_chunk("task-recent", "stdout", &format!("chunk-{index}"))
            .unwrap();
    }
    let events = journal
        .recent_events_for_tasks(&HashSet::from(["task-recent".to_string()]), 3)
        .unwrap();
    let task_events = events.get("task-recent").unwrap();
    assert_eq!(task_events.len(), 3);
    assert!(task_events
        .last()
        .and_then(|event| event.event.get("text"))
        .and_then(Value::as_str)
        .unwrap_or_default()
        .contains("chunk-11"));
    let _ = std::fs::remove_dir_all(dir);
}
