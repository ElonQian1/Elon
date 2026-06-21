// server/src/node_agent_runtime_events.rs

use crate::node_agent_tool_guard::truncate_chars;
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};

const MAX_EVENT_RESULT_CHARS: usize = 6_000;
const MAX_DIFF_PREVIEW_CHARS: usize = 4_000;

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

pub(crate) fn tool_approval_id(turn: usize, index: usize) -> String {
    format!("tap_{turn}_{index}")
}

pub(crate) fn tool_approval_required_chunk(
    req_id: &str,
    turn: usize,
    index: usize,
    approval_id: &str,
    action: &Value,
) -> String {
    let tool = tool_name(action);
    let event = json!({
        "type": "tool_approval_required",
        "schema": "elon.routebc.tool_approval.v1",
        "req_id": req_id,
        "approval_id": approval_id,
        "tool": tool,
        "risk": tool_risk(&tool),
        "args": action_preview(action),
        "diff": diff_preview(action),
        "call_id": call_id(req_id, turn, index),
        "turn": turn,
        "index": index,
        "status": "pending"
    });
    event_line(event)
}

pub(crate) fn tool_approval_decision_chunk(
    req_id: &str,
    turn: usize,
    index: usize,
    approval_id: &str,
    tool: &str,
    decision: &str,
    status: &str,
) -> String {
    let event = json!({
        "type": "tool_approval_decision",
        "schema": "elon.routebc.tool_approval.v1",
        "req_id": req_id,
        "approval_id": approval_id,
        "tool": tool,
        "decision": decision,
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
        "apply_patch" => {
            if let Some(patch) = action.get("patch").and_then(Value::as_str) {
                out.insert("patch_chars".to_string(), json!(patch.chars().count()));
                out.insert("patch_sha256".to_string(), json!(sha256_hex(patch)));
                out.insert("files".to_string(), json!(patch_touched_files(patch)));
            }
            if let Some(check_only) = action.get("check_only").and_then(Value::as_bool) {
                out.insert("check_only".to_string(), json!(check_only));
            }
            insert_string_field(&mut out, action, "reason");
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

fn diff_preview(action: &Value) -> Value {
    if tool_name(action) != "apply_patch" {
        return Value::Null;
    }
    let Some(patch) = action.get("patch").and_then(Value::as_str) else {
        return Value::Null;
    };
    let preview = truncate_chars(patch, MAX_DIFF_PREVIEW_CHARS);
    json!({
        "format": "unified",
        "preview": preview,
        "truncated": patch.chars().count() > MAX_DIFF_PREVIEW_CHARS,
        "files": patch_touched_files(patch)
    })
}

fn tool_risk(tool: &str) -> &'static str {
    match tool {
        "run_command" => "command",
        "write_file" | "apply_patch" => "write",
        _ => "read",
    }
}

fn sha256_hex(text: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    hex::encode(hasher.finalize())
}

fn patch_touched_files(patch: &str) -> Vec<String> {
    let mut files = Vec::new();
    for line in patch.lines() {
        let Some(path) = line
            .strip_prefix("+++ b/")
            .or_else(|| line.strip_prefix("--- a/"))
        else {
            continue;
        };
        let path = path.trim();
        if path == "/dev/null" || path.is_empty() || files.iter().any(|item| item == path) {
            continue;
        }
        files.push(path.to_string());
        if files.len() >= 20 {
            break;
        }
    }
    files
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
    use super::{tool_approval_required_chunk, tool_call_chunk, tool_result_chunk};
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

    #[test]
    fn apply_patch_preview_uses_summary_and_diff_preview() {
        let line = tool_approval_required_chunk(
            "req",
            1,
            1,
            "tap_1_1",
            &json!({
                "tool": "apply_patch",
                "patch": "diff --git a/src/main.rs b/src/main.rs\n--- a/src/main.rs\n+++ b/src/main.rs\n@@ -1 +1 @@\n-old\n+new\n",
                "check_only": false
            }),
        );
        let event: Value = serde_json::from_str(line.trim()).unwrap();
        assert_eq!(event["type"], "tool_approval_required");
        assert_eq!(event["approval_id"], "tap_1_1");
        assert_eq!(event["args"]["files"][0], "src/main.rs");
        assert_eq!(event["diff"]["files"][0], "src/main.rs");
        assert!(event["args"]["patch_sha256"].as_str().unwrap().len() >= 64);
        assert!(event["args"].get("patch").is_none());
    }
}
