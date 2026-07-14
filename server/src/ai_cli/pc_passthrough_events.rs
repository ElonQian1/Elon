use serde_json::Value;

use super::strip_terminal_control_sequences;

pub(crate) fn pc_cli_passthrough_event(text: &str) -> Option<String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return None;
    }
    let value = serde_json::from_str::<Value>(trimmed).ok()?;
    match value.get("type").and_then(Value::as_str)? {
        "tool_approval_required"
        | "tool_approval_decision"
        | "tool_call"
        | "tool_result"
        | "usage"
        | "progress" => serde_json::to_string(&value).ok(),
        _ => None,
    }
}

fn pc_cli_passthrough_events(text: &str, model_used: Option<&str>) -> Vec<String> {
    text.lines()
        .flat_map(|line| {
            let mut events = crate::codex_stream::stream_event_to_ws_messages(line, model_used);
            if let Some(event) = pc_cli_passthrough_event(line) {
                events.push(event);
            }
            events
        })
        .collect()
}

pub(crate) fn pc_cli_passthrough_events_from_chunk(
    buffer: &mut String,
    text: &str,
    model_used: Option<&str>,
) -> Vec<String> {
    let clean = strip_terminal_control_sequences(text);
    let mut out = Vec::new();

    for ch in clean.chars() {
        if buffer.is_empty() {
            if ch == '{' {
                buffer.push(ch);
            }
            continue;
        }

        if matches!(ch, '\r' | '\n') {
            if let Some(events) = buffered_json_event(buffer, model_used) {
                out.extend(events);
                buffer.clear();
            } else if !looks_like_json_event_fragment(buffer) {
                buffer.clear();
            }
            continue;
        }

        buffer.push(ch);
        if let Some(events) = buffered_json_event(buffer, model_used) {
            out.extend(events);
            buffer.clear();
        } else if buffer.len() > MAX_BUFFERED_JSON_EVENT_CHARS {
            buffer.clear();
        }
    }

    out
}

pub(crate) fn pc_cli_passthrough_events_flush(
    buffer: &mut String,
    model_used: Option<&str>,
) -> Vec<String> {
    if buffer.trim().is_empty() {
        buffer.clear();
        return Vec::new();
    }
    let pending = std::mem::take(buffer);
    pc_cli_passthrough_events(&pending, model_used)
}

fn looks_like_json_event_fragment(text: &str) -> bool {
    text.trim_start_matches(|ch: char| ch.is_whitespace() || ch == '\r')
        .starts_with('{')
}

const MAX_BUFFERED_JSON_EVENT_CHARS: usize = 1024 * 1024;

fn buffered_json_event(buffer: &str, model_used: Option<&str>) -> Option<Vec<String>> {
    let trimmed = buffer.trim();
    if trimmed.is_empty() || serde_json::from_str::<Value>(trimmed).is_err() {
        return None;
    }
    Some(pc_cli_passthrough_events(trimmed, model_used))
}

#[cfg(test)]
#[path = "pc_passthrough_events_tests.rs"]
mod tests;
