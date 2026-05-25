use crate::{project_ws_protocol::server_message_details, types::AppState};

pub(crate) fn record_server_message(
    state: &AppState,
    trace_id: &str,
    value: &serde_json::Value,
    bytes: usize,
) {
    if trace_id.trim().is_empty() {
        return;
    }
    let details = server_message_details(value, bytes);
    state
        .server_traces
        .record(trace_id, "server_message_to_phone", details.clone());
    match value.get("type").and_then(|kind| kind.as_str()) {
        Some("done") => state.server_traces.record(trace_id, "server_done", details),
        Some("error") => state
            .server_traces
            .record(trace_id, "server_error", details),
        _ => {}
    }
}

pub(crate) fn record_server_transport(
    state: &AppState,
    trace_id: &str,
    phase: &str,
    raw: &str,
    task_id: &str,
) {
    if trace_id.trim().is_empty() {
        return;
    }
    let mut details = serde_json::from_str::<serde_json::Value>(raw)
        .map(|value| server_message_details(&value, raw.len()))
        .unwrap_or_else(|_| {
            serde_json::json!({
                "type": "invalid_json",
                "bytes": raw.len(),
            })
        });
    if let Some(object) = details.as_object_mut() {
        object.insert("task_id".into(), serde_json::json!(task_id));
    }
    state.server_traces.record(trace_id, phase, details);
}
