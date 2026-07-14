use serde::Serialize;
use serde_json::{json, Value};
use std::{
    collections::VecDeque,
    sync::Mutex,
    time::{Instant, SystemTime, UNIX_EPOCH},
};

const MAX_SERVER_TRACE_EVENTS: usize = 800;

#[derive(Clone, Debug, Serialize)]
pub struct ServerTraceEvent {
    pub trace_id: String,
    pub phase: String,
    pub wall_time_ms: i64,
    pub elapsed_ms: u128,
    pub details: Value,
}

pub struct ServerTraceStore {
    started_at: Instant,
    events: Mutex<VecDeque<ServerTraceEvent>>,
}

impl ServerTraceStore {
    pub fn new() -> Self {
        Self {
            started_at: Instant::now(),
            events: Mutex::new(VecDeque::new()),
        }
    }

    pub fn record(&self, trace_id: impl AsRef<str>, phase: impl Into<String>, details: Value) {
        let trace_id = trace_id.as_ref().trim();
        if trace_id.is_empty() {
            return;
        }
        let event = ServerTraceEvent {
            trace_id: trace_id.to_string(),
            phase: phase.into(),
            wall_time_ms: now_ms(),
            elapsed_ms: self.started_at.elapsed().as_millis(),
            details,
        };
        let mut events = self
            .events
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        events.push_back(event);
        while events.len() > MAX_SERVER_TRACE_EVENTS {
            events.pop_front();
        }
    }

    pub fn trace_json(&self, trace_id: &str, limit: usize) -> Value {
        let trace_id = trace_id.trim();
        let limit = limit.clamp(1, 300);
        let events = self
            .events
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let matched: Vec<ServerTraceEvent> = events
            .iter()
            .filter(|event| event.trace_id == trace_id)
            .cloned()
            .collect();
        let returned: Vec<ServerTraceEvent> = matched.iter().rev().take(limit).cloned().collect();
        let mut returned = returned;
        returned.reverse();
        json!({
            "trace_id": trace_id,
            "matched_count": matched.len(),
            "returned_count": returned.len(),
            "limit": limit,
            "events": returned,
            "summary": summarize_trace(&matched),
        })
    }
}

fn summarize_trace(events: &[ServerTraceEvent]) -> Value {
    let first = events.first();
    let last = events.last();
    let first_outgoing = events
        .iter()
        .find(|event| event.phase == "server_message_to_phone");
    let first_codex_start = events.iter().find(|event| event.phase == "codex_cli_start");
    let last_codex_terminal = events
        .iter()
        .rev()
        .find(|event| event.phase == "codex_cli_done" || event.phase == "codex_cli_error");
    let codex_attempts = events
        .iter()
        .filter(|event| event.phase == "codex_cli_start")
        .count();
    let server_terminal = events
        .iter()
        .find(|event| event.phase == "server_done" || event.phase == "server_error");
    let client_disconnect = events
        .iter()
        .find(|event| event.phase == "server_client_disconnected");
    let finish = server_terminal.or(client_disconnect);
    json!({
        "first_phase": first.map(|event| event.phase.as_str()),
        "last_phase": last.map(|event| event.phase.as_str()),
        "first_outgoing_elapsed_from_receive_ms": duration_between(first, first_outgoing),
        "finish_elapsed_from_receive_ms": duration_between(first, finish),
        "client_disconnect_elapsed_from_receive_ms": duration_between(first, client_disconnect),
        "codex_cli_elapsed_ms": duration_between(first_codex_start, last_codex_terminal),
        "codex_cli_attempts": codex_attempts,
        "terminal": finish.map(|event| event.phase.as_str()),
    })
}

fn duration_between(
    start: Option<&ServerTraceEvent>,
    end: Option<&ServerTraceEvent>,
) -> Option<i64> {
    let start = start?;
    let end = end?;
    Some((end.wall_time_ms - start.wall_time_ms).max(0))
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or_default()
}

#[cfg(test)]
#[path = "server_trace_tests.rs"]
mod tests;
