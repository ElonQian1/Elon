use super::*;

#[test]
fn filters_events_and_summarizes_terminal_trace() {
    let store = ServerTraceStore::new();
    store.record("trace_a", "ws_project_message_received", json!({}));
    store.record("trace_b", "ws_project_message_received", json!({}));
    store.record(
        "trace_a",
        "server_message_to_phone",
        json!({"type": "progress"}),
    );
    store.record("trace_a", "server_done", json!({"type": "done"}));

    let trace = store.trace_json("trace_a", 10);
    assert_eq!(trace["matched_count"], 3);
    assert_eq!(trace["returned_count"], 3);
    assert_eq!(
        trace["summary"]["first_phase"],
        "ws_project_message_received"
    );
    assert_eq!(trace["summary"]["terminal"], "server_done");
}

#[test]
fn summarizes_codex_cli_timing() {
    let store = ServerTraceStore::new();
    store.record("trace_a", "ws_project_message_received", json!({}));
    store.record(
        "trace_a",
        "codex_cli_start",
        json!({"operation": "project_request"}),
    );
    store.record(
        "trace_a",
        "codex_cli_done",
        json!({"operation": "project_request", "success": true}),
    );
    store.record("trace_a", "server_done", json!({"type": "done"}));

    let trace = store.trace_json("trace_a", 10);
    assert_eq!(trace["summary"]["codex_cli_attempts"], 1);
    assert!(trace["summary"]["codex_cli_elapsed_ms"].as_i64().is_some());
}

#[test]
fn prefers_server_terminal_over_client_disconnect() {
    let store = ServerTraceStore::new();
    store.record("trace_a", "ws_project_message_received", json!({}));
    store.record("trace_a", "server_client_disconnected", json!({}));
    store.record("trace_a", "server_done", json!({"type": "done"}));

    let trace = store.trace_json("trace_a", 10);
    assert_eq!(trace["summary"]["terminal"], "server_done");
    assert!(
        trace["summary"]["client_disconnect_elapsed_from_receive_ms"]
            .as_i64()
            .is_some()
    );
}
