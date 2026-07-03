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
mod tests {
    use super::{
        pc_cli_passthrough_event, pc_cli_passthrough_events, pc_cli_passthrough_events_flush,
        pc_cli_passthrough_events_from_chunk,
    };
    use serde_json::Value;

    #[test]
    fn keeps_tool_approval_events() {
        let line =
            r#"{"type":"tool_approval_required","tool":"write_file","approval_id":"tap_1_1"}"#;
        let out = pc_cli_passthrough_event(line).expect("approval event should pass through");
        let value: Value = serde_json::from_str(&out).unwrap();

        assert_eq!(value["type"], "tool_approval_required");
        assert_eq!(value["approval_id"], "tap_1_1");
    }

    #[test]
    fn rejects_unknown_json_events() {
        assert!(pc_cli_passthrough_event(r#"{"type":"unknown","message":"x"}"#).is_none());
        assert!(pc_cli_passthrough_event("not json").is_none());
    }

    #[test]
    fn translates_codex_json_stream() {
        let raw = r#"{"type":"item.started","item":{"id":"call_1","type":"command_execution","command":"rg -n \"TODO\" server/src"}}"#;
        let events = pc_cli_passthrough_events(raw, Some("Codex"));

        assert_eq!(events.len(), 2);
        let tool: Value = serde_json::from_str(&events[0]).unwrap();
        assert_eq!(tool["type"], "tool_call");
        assert_eq!(tool["tool"], "shell");
        assert_eq!(tool["args"]["command"], "rg -n \"TODO\" server/src");
        let progress: Value = serde_json::from_str(&events[1]).unwrap();
        assert_eq!(progress["type"], "progress");
        assert!(progress["message"].as_str().unwrap().contains("rg -n"));
    }

    #[test]
    fn buffers_split_codex_json_stream() {
        let mut buffer = String::new();
        let first = r#"{"type":"item.started","item":{"id":"call_1","type":"command"#;
        let second = "_execution\",\"command\":\"cargo test\"}}\r\n";

        assert!(pc_cli_passthrough_events_from_chunk(&mut buffer, first, Some("Codex")).is_empty());
        assert!(!buffer.is_empty());

        let events = pc_cli_passthrough_events_from_chunk(&mut buffer, second, Some("Codex"));
        assert!(buffer.is_empty());
        assert_eq!(events.len(), 2);
        let tool: Value = serde_json::from_str(&events[0]).unwrap();
        assert_eq!(tool["type"], "tool_call");
        assert_eq!(tool["tool"], "shell");
        assert_eq!(tool["args"]["command"], "cargo test");
    }

    #[test]
    fn repairs_terminal_wrapped_codex_json_stream() {
        let mut buffer = String::new();
        let raw = concat!(
            "{\"type\":\"item.started\",\"item\":{\"id\":\"call_1\",\"type\":\"command",
            "\r\n\u{1b}[29;120H",
            "_execution\",\"command\":\"git status --short\"}}\r\n"
        );

        let events = pc_cli_passthrough_events_from_chunk(&mut buffer, raw, Some("Codex"));

        assert!(buffer.is_empty());
        assert_eq!(events.len(), 2);
        let tool: Value = serde_json::from_str(&events[0]).unwrap();
        assert_eq!(tool["type"], "tool_call");
        assert_eq!(tool["tool"], "shell");
        assert_eq!(tool["args"]["command"], "git status --short");
    }

    #[test]
    fn parses_multiple_codex_events_in_one_chunk() {
        let mut buffer = String::new();
        let raw = concat!(
            "{\"type\":\"item.started\",\"item\":{\"id\":\"call_1\",\"type\":\"command_execution\",\"command\":\"pwd\"}}\n",
            "{\"type\":\"item.completed\",\"item\":{\"id\":\"call_1\",\"type\":\"command_execution\",\"exit_code\":0,\"aggregated_output\":\"ok\"}}\n"
        );

        let events = pc_cli_passthrough_events_from_chunk(&mut buffer, raw, Some("Codex"));

        assert!(buffer.is_empty());
        assert_eq!(events.len(), 4);
        let started: Value = serde_json::from_str(&events[0]).unwrap();
        let completed: Value = serde_json::from_str(&events[2]).unwrap();
        assert_eq!(started["type"], "tool_call");
        assert_eq!(completed["type"], "tool_result");
    }

    #[test]
    fn parses_complete_json_without_newline() {
        let mut buffer = String::new();
        let raw = r#"{"type":"item.started","item":{"id":"call_1","type":"command_execution","command":"cargo check"}}"#;
        let events = pc_cli_passthrough_events_from_chunk(&mut buffer, raw, Some("Codex"));
        assert!(buffer.is_empty());
        assert_eq!(events.len(), 2);
        let tool: Value = serde_json::from_str(&events[0]).unwrap();
        assert_eq!(tool["args"]["command"], "cargo check");
    }

    #[test]
    fn flush_keeps_incomplete_json_silent() {
        let mut buffer = r#"{"type":"item.started","item":{"id":"call_1""#.to_string();

        let events = pc_cli_passthrough_events_flush(&mut buffer, Some("Codex"));
        assert!(buffer.is_empty());
        assert!(events.is_empty());
    }

    #[test]
    fn keeps_legacy_tool_event_lines() {
        let raw = r#"{"type":"tool_result","tool":"shell","result":"ok"}"#;
        let events = pc_cli_passthrough_events(raw, None);

        assert_eq!(events.len(), 1);
        let value: Value = serde_json::from_str(&events[0]).unwrap();
        assert_eq!(value["type"], "tool_result");
        assert_eq!(value["tool"], "shell");
        assert_eq!(value["result"], "ok");
    }
}
