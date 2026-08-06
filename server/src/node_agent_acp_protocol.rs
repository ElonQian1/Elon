//! Minimal ACP v1 message construction and parsing shared by CLI providers.

use serde_json::{json, Value};
use std::path::{Path, PathBuf};

use crate::node_agent_provider_auth_protocol::client_info;

pub(crate) const ACP_PROTOCOL_VERSION: i64 = 1;

pub(crate) fn initialize_request(id: i64) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": "initialize",
        "params": {
            "protocolVersion": ACP_PROTOCOL_VERSION,
            "clientCapabilities": {},
            "clientInfo": client_info()
        }
    })
}

pub(crate) fn authenticate_request(id: i64, method_id: &str) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": "authenticate",
        "params": {"methodId": method_id}
    })
}

pub(crate) fn new_session_request(id: i64, cwd: &str) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": "session/new",
        "params": {"cwd": cwd, "mcpServers": []}
    })
}

pub(crate) fn load_session_request(id: i64, session_id: &str, cwd: &str) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": "session/load",
        "params": {"sessionId": session_id, "cwd": cwd, "mcpServers": []}
    })
}

pub(crate) fn prompt_request(
    id: i64,
    session_id: &str,
    prompt: &str,
    attachments: &[PathBuf],
) -> Value {
    let mut blocks = vec![json!({"type": "text", "text": prompt})];
    blocks.extend(attachments.iter().filter_map(resource_link));
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": "session/prompt",
        "params": {"sessionId": session_id, "prompt": blocks}
    })
}

pub(crate) fn cancel_notification(session_id: &str) -> Value {
    json!({
        "jsonrpc": "2.0",
        "method": "session/cancel",
        "params": {"sessionId": session_id}
    })
}

pub(crate) fn response_result(message: &Value, expected_id: i64) -> Option<Result<Value, String>> {
    if message.get("id").and_then(Value::as_i64) != Some(expected_id) {
        return None;
    }
    if let Some(error) = message.get("error") {
        return Some(Err(error
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or("ACP agent returned an error")
            .to_string()));
    }
    Some(Ok(message.get("result").cloned().unwrap_or(Value::Null)))
}

pub(crate) fn session_id(result: &Value) -> Option<String> {
    result
        .get("sessionId")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty() && value.len() <= 512)
        .map(str::to_string)
}

pub(crate) fn load_session_supported(initialize: &Value) -> bool {
    initialize
        .pointer("/agentCapabilities/loadSession")
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

pub(crate) fn agent_info_label(initialize: &Value) -> Option<String> {
    initialize
        .pointer("/agentInfo/title")
        .or_else(|| initialize.pointer("/agentInfo/name"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.chars().take(120).collect())
}

pub(crate) fn agent_message_text(message: &Value) -> Option<&str> {
    if message.get("method").and_then(Value::as_str) != Some("session/update") {
        return None;
    }
    let update = message.pointer("/params/update")?;
    if update.get("sessionUpdate").and_then(Value::as_str) != Some("agent_message_chunk")
        || update.pointer("/content/type").and_then(Value::as_str) != Some("text")
    {
        return None;
    }
    update.pointer("/content/text").and_then(Value::as_str)
}

pub(crate) fn tool_call_descriptor(message: &Value) -> Option<(String, String)> {
    if message.get("method").and_then(Value::as_str) != Some("session/update") {
        return None;
    }
    let update = message.pointer("/params/update")?;
    if update.get("sessionUpdate").and_then(Value::as_str) != Some("tool_call") {
        return None;
    }
    let id = update.get("toolCallId")?.as_str()?.to_string();
    let kind = update
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or("other")
        .to_ascii_lowercase();
    Some((id, kind))
}

pub(crate) fn permission_response(
    message: &Value,
    tool_kind: Option<&str>,
    read_only: bool,
) -> Option<Value> {
    if message.get("method").and_then(Value::as_str) != Some("session/request_permission") {
        return None;
    }
    let id = message.get("id")?.clone();
    let options = message.pointer("/params/options")?.as_array()?;
    let safe_read = matches!(tool_kind, Some("read" | "search" | "think" | "fetch"));
    let want_allow = !read_only || safe_read;
    let selected = options.iter().find(|option| {
        let kind = option
            .get("kind")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if want_allow {
            matches!(kind, "allow_once" | "allow_always")
        } else {
            matches!(kind, "reject_once" | "reject_always")
        }
    });
    let outcome = selected
        .and_then(|option| option.get("optionId"))
        .and_then(Value::as_str)
        .map(|option_id| json!({"outcome": "selected", "optionId": option_id}))
        .unwrap_or_else(|| json!({"outcome": "cancelled"}));
    Some(json!({"jsonrpc": "2.0", "id": id, "result": {"outcome": outcome}}))
}

pub(crate) fn method_not_supported_response(message: &Value) -> Option<Value> {
    let id = message.get("id")?.clone();
    let method = message.get("method")?.as_str()?;
    Some(json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": {"code": -32601, "message": format!("ACP client method is not available: {method}")}
    }))
}

pub(crate) fn session_scope_key(extra_args: &[String]) -> Option<String> {
    extra_args
        .iter()
        .find_map(|arg| arg.strip_prefix("--session-id="))
        .map(str::trim)
        .filter(|value| !value.is_empty() && value.len() <= 256)
        .map(str::to_string)
}

pub(crate) fn attachment_paths(extra_args: &[String]) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    let mut index = 0;
    while index + 1 < extra_args.len() {
        if extra_args[index] == "--attachment" {
            paths.push(PathBuf::from(&extra_args[index + 1]));
            index += 2;
        } else {
            index += 1;
        }
    }
    paths
}

fn resource_link(path: &PathBuf) -> Option<Value> {
    let uri = reqwest::Url::from_file_path(path).ok()?.to_string();
    let name = path.file_name()?.to_string_lossy().to_string();
    let size = std::fs::metadata(path).ok().map(|metadata| metadata.len());
    let mime_type = mime_type(path);
    Some(json!({
        "type": "resource_link",
        "uri": uri,
        "name": name,
        "mimeType": mime_type,
        "size": size
    }))
}

fn mime_type(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase()
        .as_str()
    {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "pdf" => "application/pdf",
        "json" => "application/json",
        "md" => "text/markdown",
        "txt" => "text/plain",
        _ => "application/octet-stream",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_only_agent_text_chunks() {
        let message = json!({
            "method":"session/update",
            "params":{"update":{"sessionUpdate":"agent_message_chunk","content":{"type":"text","text":"hello"}}}
        });
        assert_eq!(agent_message_text(&message), Some("hello"));
        assert_eq!(
            agent_message_text(&json!({"method":"session/update"})),
            None
        );
    }

    #[test]
    fn read_only_permission_rejects_edit_and_allows_read() {
        let request = json!({
            "jsonrpc":"2.0","id":7,"method":"session/request_permission",
            "params":{"options":[
                {"optionId":"yes","kind":"allow_once"},
                {"optionId":"no","kind":"reject_once"}
            ]}
        });
        assert_eq!(
            permission_response(&request, Some("edit"), true)
                .unwrap()
                .pointer("/result/outcome/optionId")
                .and_then(Value::as_str),
            Some("no")
        );
        assert_eq!(
            permission_response(&request, Some("read"), true)
                .unwrap()
                .pointer("/result/outcome/optionId")
                .and_then(Value::as_str),
            Some("yes")
        );
    }

    #[test]
    fn prompt_uses_acp_v1_prompt_field() {
        let message = prompt_request(4, "session-a", "hello", &[]);
        assert_eq!(
            message.pointer("/params/prompt/0/text"),
            Some(&json!("hello"))
        );
        assert!(message.pointer("/params/content").is_none());
    }
}
