// server/src/node_agent_runtime_events.rs

use crate::node_agent_tool_guard::truncate_chars;
use serde_json::{json, Map, Value};

const MAX_EVENT_RESULT_CHARS: usize = 6_000;

pub(crate) fn tool_name(action: &Value) -> String {
    action
        .get("tool")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("unknown")
        .to_string()
}

pub(crate) fn tool_call_chunk(req_id: &str, turn: usize, index: usize, action: &Value) -> String {
    let tool = tool_name(action);
    let event = json!({
        "type": "tool_call",
        "tool": tool,
        "args": action_preview(action),
        "call_id": call_id(req_id, turn, index),
        "turn": turn,
        "index": index,
        "status": "running"
    });
    event_line(event)
}

pub(crate) fn tool_result_chunk(
    req_id: &str,
    turn: usize,
    index: usize,
    tool: &str,
    result: &str,
) -> String {
    let result = truncate_chars(result, MAX_EVENT_RESULT_CHARS);
    let status = if result
        .trim_start()
        .to_ascii_lowercase()
        .starts_with("error:")
    {
        "error"
    } else {
        "ok"
    };
    let event = json!({
        "type": "tool_result",
        "tool": tool,
        "result": result,
        "call_id": call_id(req_id, turn, index),
        "turn": turn,
        "index": index,
        "status": status
    });
    event_line(event)
}

fn call_id(req_id: &str, turn: usize, index: usize) -> String {
    format!("{}:{}:{}", req_id, turn, index)
}

fn event_line(value: Value) -> String {
    let mut line = serde_json::to_string(&value)
        .unwrap_or_else(|_| r#"{"type":"progress","message":"工具事件序列化失败"}"#.to_string());
    line.push('\n');
    line
}

fn action_preview(action: &Value) -> Value {
    let tool = tool_name(action);
    let mut out = Map::new();
    insert_string_field(&mut out, action, "path");
    insert_string_field(&mut out, action, "reason");

    match tool.as_str() {
        "write_file" => {
            if let Some(content) = action.get("content").and_then(Value::as_str) {
                out.insert("content_chars".to_string(), json!(content.chars().count()));
            }
        }
        "run_command" => {
            insert_string_field(&mut out, action, "program");
            if let Some(args) = action.get("args").and_then(Value::as_array) {
                out.insert(
                    "args".to_string(),
                    redact_value("args", &Value::Array(args.clone())),
                );
            }
            insert_string_field(&mut out, action, "command");
        }
        _ => {
            for (key, value) in action.as_object().into_iter().flatten() {
                if key == "tool" || key == "content" || out.contains_key(key) {
                    continue;
                }
                out.insert(key.clone(), redact_value(key, value));
            }
        }
    }

    Value::Object(out)
}

fn insert_string_field(out: &mut Map<String, Value>, action: &Value, key: &str) {
    if let Some(value) = action
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        out.insert(key.to_string(), redact_string(key, value));
    }
}

fn redact_value(key: &str, value: &Value) -> Value {
    match value {
        Value::String(text) => redact_string(key, text),
        Value::Array(items) => Value::Array(
            items
                .iter()
                .map(|item| redact_value(key, item))
                .collect::<Vec<_>>(),
        ),
        Value::Object(map) => Value::Object(
            map.iter()
                .map(|(child_key, child_value)| {
                    (child_key.clone(), redact_value(child_key, child_value))
                })
                .collect(),
        ),
        _ => value.clone(),
    }
}

fn redact_string(key: &str, value: &str) -> Value {
    if is_secret_key(key) {
        Value::String("[redacted]".to_string())
    } else {
        Value::String(truncate_chars(value, 500))
    }
}

fn is_secret_key(key: &str) -> bool {
    let key = key.to_ascii_lowercase();
    [
        "token",
        "secret",
        "password",
        "api_key",
        "apikey",
        "credential",
    ]
    .iter()
    .any(|needle| key.contains(needle))
}

#[cfg(test)]
mod tests {
    use super::{tool_call_chunk, tool_result_chunk};
    use serde_json::{json, Value};

    #[test]
    fn tool_call_event_hides_write_content_and_secrets() {
        let line = tool_call_chunk(
            "req",
            2,
            3,
            &json!({
                "tool": "write_file",
                "path": "src/main.rs",
                "content": "secret body",
                "api_key": "should-not-render"
            }),
        );
        let event: Value = serde_json::from_str(line.trim()).unwrap();
        assert_eq!(event["type"], "tool_call");
        assert_eq!(event["tool"], "write_file");
        assert_eq!(event["call_id"], "req:2:3");
        assert_eq!(event["args"]["path"], "src/main.rs");
        assert_eq!(event["args"]["content_chars"], 11);
        assert!(event["args"].get("content").is_none());
        assert!(event["args"].get("api_key").is_none());
    }

    #[test]
    fn run_command_preview_keeps_structured_command() {
        let line = tool_call_chunk(
            "req",
            1,
            1,
            &json!({
                "tool": "run_command",
                "program": "git",
                "args": ["status", "--short"],
                "reason": "inspect state"
            }),
        );
        let event: Value = serde_json::from_str(line.trim()).unwrap();
        assert_eq!(event["args"]["program"], "git");
        assert_eq!(event["args"]["args"][0], "status");
        assert_eq!(event["args"]["reason"], "inspect state");
    }

    #[test]
    fn tool_result_event_marks_guard_errors() {
        let line = tool_result_chunk("req", 1, 2, "run_command", "error: denied");
        let event: Value = serde_json::from_str(line.trim()).unwrap();
        assert_eq!(event["type"], "tool_result");
        assert_eq!(event["tool"], "run_command");
        assert_eq!(event["status"], "error");
    }
}
