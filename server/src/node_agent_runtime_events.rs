// server/src/node_agent_runtime_events.rs

use crate::{
    node_agent_tool_approval::TOOL_APPROVAL_TIMEOUT_SECS, node_agent_tool_guard::truncate_chars,
};
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
        "call_id": action_call_id(req_id, turn, index, Some(action)),
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
    action: Option<&Value>,
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
        "call_id": action_call_id(req_id, turn, index, action),
        "turn": turn,
        "index": index,
        "status": status
    });
    event_line(event)
}

pub(crate) fn runtime_status_chunk(
    req_id: &str,
    turn: usize,
    label: &str,
    phase: &str,
    message: &str,
) -> String {
    let event = json!({
        "type": "runtime_status",
        "schema": "elon.routebc.runtime_status.v1",
        "req_id": req_id,
        "runtime": label,
        "phase": phase,
        "message": truncate_chars(message, 1_000),
        "turn": turn,
        "status": phase_status(phase)
    });
    event_line(event)
}

pub(crate) fn runtime_summary_chunk(
    req_id: &str,
    label: &str,
    turn: usize,
    status: &str,
    total_tools: usize,
    failed_tools: usize,
    message: &str,
) -> String {
    let event = json!({
        "type": "runtime_summary",
        "schema": "elon.routebc.runtime_summary.v1",
        "req_id": req_id,
        "runtime": label,
        "turn": turn,
        "status": status,
        "total_tools": total_tools,
        "failed_tools": failed_tools,
        "message": truncate_chars(message, 1_000),
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
    tool_approval_required_chunk_with_diff(
        req_id,
        turn,
        index,
        approval_id,
        action,
        diff_preview(action),
    )
}

pub(crate) fn tool_approval_required_chunk_with_diff(
    req_id: &str,
    turn: usize,
    index: usize,
    approval_id: &str,
    action: &Value,
    diff: Value,
) -> String {
    tool_approval_required_chunk_inner(req_id, turn, index, approval_id, action, diff, None)
}

pub(crate) fn tool_approval_required_chunk_with_diff_and_checkpoint(
    req_id: &str,
    turn: usize,
    index: usize,
    approval_id: &str,
    action: &Value,
    diff: Value,
    checkpoint: Value,
) -> String {
    tool_approval_required_chunk_inner(
        req_id,
        turn,
        index,
        approval_id,
        action,
        diff,
        Some(checkpoint),
    )
}

pub(crate) fn tool_approval_checkpoint(
    action: &Value,
    diff: &Value,
    registered_at_ms: u128,
    expires_at_ms: u128,
) -> Value {
    let diff_fingerprint = diff_fingerprint(diff);
    json!({
        "schema": "elon.routebc.tool_approval_checkpoint.v1",
        "registered_at_ms": registered_at_ms,
        "expires_at_ms": expires_at_ms,
        "timeout_secs": TOOL_APPROVAL_TIMEOUT_SECS,
        "action_sha256": sha256_json(action),
        "diff_sha256": sha256_json(&diff_fingerprint),
        "diff_fingerprint": diff_fingerprint,
        "restart_recovery": {
            "supported": false,
            "next_action": "continue_from_snapshot",
            "reason": "审批请求已持久化安全指纹，但节点重启后仍需重新校验任务、工作区和工具请求后才能开放续批。"
        },
        "revalidate_before_execute": [
            "approval_id",
            "task_id",
            "workspace_path",
            "action_sha256",
            "diff_sha256",
            "tool_guard_policy"
        ]
    })
}

fn tool_approval_required_chunk_inner(
    req_id: &str,
    turn: usize,
    index: usize,
    approval_id: &str,
    action: &Value,
    diff: Value,
    checkpoint: Option<Value>,
) -> String {
    let tool = tool_name(action);
    let mut event = json!({
        "type": "tool_approval_required",
        "schema": "elon.routebc.tool_approval.v1",
        "req_id": req_id,
        "approval_id": approval_id,
        "tool": tool,
        "risk": tool_risk(&tool),
        "args": action_preview(action),
        "diff": diff,
        "call_id": action_call_id(req_id, turn, index, Some(action)),
        "turn": turn,
        "index": index,
        "status": "pending"
    });
    if let Some(checkpoint) = checkpoint {
        event["approval_checkpoint"] = checkpoint;
    }
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
    action: Option<&Value>,
) -> String {
    let event = json!({
        "type": "tool_approval_decision",
        "schema": "elon.routebc.tool_approval.v1",
        "req_id": req_id,
        "approval_id": approval_id,
        "tool": tool,
        "decision": decision,
        "call_id": action_call_id(req_id, turn, index, action),
        "turn": turn,
        "index": index,
        "status": status
    });
    event_line(event)
}

fn call_id(req_id: &str, turn: usize, index: usize) -> String {
    format!("{}:{}:{}", req_id, turn, index)
}

fn action_call_id(req_id: &str, turn: usize, index: usize, action: Option<&Value>) -> String {
    action
        .and_then(|value| {
            value
                .get("tool_call_id")
                .or_else(|| value.get("_tool_call_id"))
        })
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty() && value.chars().count() <= 200)
        .map(str::to_string)
        .unwrap_or_else(|| call_id(req_id, turn, index))
}

fn phase_status(phase: &str) -> &'static str {
    match phase {
        "completed" => "ok",
        "canceled" => "canceled",
        "failed" => "error",
        "waiting_approval" => "pending",
        _ => "running",
    }
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
                if key == "tool"
                    || key == "content"
                    || key == "tool_call_id"
                    || key == "_tool_call_id"
                    || out.contains_key(key)
                {
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

fn diff_fingerprint(diff: &Value) -> Value {
    match diff {
        Value::Object(map) => {
            let mut out = Map::new();
            for (key, value) in map {
                if key == "preview" {
                    out.insert("preview_removed".to_string(), Value::Bool(true));
                    continue;
                }
                out.insert(key.clone(), value.clone());
            }
            Value::Object(out)
        }
        _ => diff.clone(),
    }
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

fn sha256_json(value: &Value) -> String {
    let bytes = serde_json::to_vec(value).unwrap_or_default();
    let mut hasher = Sha256::new();
    hasher.update(bytes);
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
#[path = "node_agent_runtime_events_tests.rs"]
mod tests;
