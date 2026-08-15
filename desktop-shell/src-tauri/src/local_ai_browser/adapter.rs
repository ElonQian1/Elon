use serde_json::{json, Map, Value};

use super::{adapter_content, chatgpt_adapter_bootstrap::ADAPTER_VERSION};

const MAX_EVENT_BYTES: usize = 512 * 1024;
const MAX_MESSAGES: usize = 80;
const MAX_DRAFT_CHARS: usize = 20_000;
const MAX_OPTIONS: usize = 100;
const MAX_PROJECTS: usize = 40;

pub struct SanitizedAdapterEvent {
    pub kind: String,
    pub payload: Value,
}

pub fn sanitize_event(raw: &str) -> Result<SanitizedAdapterEvent, String> {
    if raw.len() > MAX_EVENT_BYTES {
        return Err("ChatGPT 可见语义事件过大，已拒绝。".to_string());
    }
    let value: Value = serde_json::from_str(raw).map_err(|_| "ChatGPT 语义事件格式无效。")?;
    if value.get("schema").and_then(Value::as_str) == Some("yilong.ai.ui.v1") {
        if value.get("providerId").and_then(Value::as_str) != Some("chatgpt") {
            return Err("ChatGPT 语义事件厂商标识无效。".to_string());
        }
        if value.get("adapterVersion").and_then(Value::as_u64) != Some(u64::from(ADAPTER_VERSION)) {
            return Err("ChatGPT 语义适配器版本无效。".to_string());
        }
        if !value
            .get("documentToken")
            .and_then(Value::as_str)
            .is_some_and(valid_document_token)
        {
            return Err("ChatGPT 页面文档令牌无效。".to_string());
        }
        let event = value
            .get("event")
            .and_then(Value::as_object)
            .ok_or_else(|| "ChatGPT 语义事件缺少 event。".to_string())?;
        return sanitize_protocol_event(event);
    }

    match value.get("type").and_then(Value::as_str) {
        Some("command_result") => Ok(SanitizedAdapterEvent {
            kind: "command_result".to_string(),
            payload: json!({
                "type": "command_result",
                "action": clean_string(value.get("action"), 48),
                "ok": value.get("ok").and_then(Value::as_bool).unwrap_or(false),
                "detail": clean_string(value.get("detail"), 240),
                "requestId": sanitize_request_id(value.get("requestId")),
            }),
        }),
        Some("browser_diagnostic") => Ok(SanitizedAdapterEvent {
            kind: "browser_diagnostic".to_string(),
            payload: json!({
                "type": "browser_diagnostic",
                "kind": clean_identifier(value.get("kind"), 48),
                "detail": clean_string(value.get("detail"), 240),
                "url": sanitize_chatgpt_url(value.get("url")),
            }),
        }),
        _ => Err("不支持的 ChatGPT 本地浏览器事件。".to_string()),
    }
}

fn valid_document_token(value: &str) -> bool {
    value.len() >= 7
        && value.len() <= 84
        && value.starts_with("doc_")
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
}

fn sanitize_protocol_event(event: &Map<String, Value>) -> Result<SanitizedAdapterEvent, String> {
    let kind = event
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let payload = match kind {
        "adapter_ready" => json!({
            "type": kind,
            "capabilities": clean_identifiers(event.get("capabilities"), 40),
        }),
        "message_snapshot" => json!({
            "type": kind,
            "title": clean_string(event.get("title"), 160),
            "url": sanitize_chatgpt_url(event.get("url")),
            "draft": clean_string(event.get("draft"), MAX_DRAFT_CHARS),
            "messages": sanitize_messages(event.get("messages")),
            "observedMessageCount": bounded_u64(event.get("observedMessageCount"), 0, 1_000_000),
            "messageWindowStart": bounded_u64(event.get("messageWindowStart"), 0, 1_000_000),
            "authenticated": event.get("authenticated").and_then(Value::as_bool).unwrap_or(false),
            "pageKind": sanitize_page_kind(event.get("pageKind")),
            "loginRequired": event.get("loginRequired").and_then(Value::as_bool).unwrap_or(false),
            "composerReady": event.get("composerReady").and_then(Value::as_bool).unwrap_or(false),
            "streaming": event.get("streaming").and_then(Value::as_bool).unwrap_or(false),
            "currentModel": clean_string(event.get("currentModel"), 80),
            "attachments": sanitize_attachments(event.get("attachments")),
            "dictationActive": event.get("dictationActive").and_then(Value::as_bool).unwrap_or(false),
            "capabilities": clean_identifiers(event.get("capabilities"), 40),
        }),
        "conversation_snapshot" => json!({
            "type": kind,
            "conversations": sanitize_conversations(event.get("conversations")),
            "projects": sanitize_projects(event.get("projects")),
        }),
        "composer_controls_snapshot" => json!({
            "type": kind,
            "section": clean_identifier(event.get("section"), 24),
            "currentModel": clean_string(event.get("currentModel"), 80),
            "options": sanitize_options(event.get("options")),
        }),
        "navigation_snapshot" => json!({
            "type": kind,
            "features": sanitize_features(event.get("features")),
        }),
        "ui_manifest_snapshot" => json!({
            "type": kind,
            "version": bounded_u64(event.get("version"), 1, 8),
            "pageKind": sanitize_page_kind(event.get("pageKind")),
            "title": clean_string(event.get("title"), 160),
            "compatibility": sanitize_compatibility(event.get("compatibility")),
            "controls": sanitize_ui_controls(event.get("controls")),
            "discoveredControlCount": bounded_u64(event.get("discoveredControlCount"), 0, 10_000),
            "controlsTruncated": event.get("controlsTruncated").and_then(Value::as_bool).unwrap_or(false),
        }),
        "web_touch_request" => json!({
            "type": kind,
            "purpose": clean_identifier(event.get("purpose"), 48),
            "controlId": clean_string(event.get("controlId"), 72),
        }),
        _ => return Err("不支持的 ChatGPT 可见语义事件类型。".to_string()),
    };
    Ok(SanitizedAdapterEvent {
        kind: kind.to_string(),
        payload,
    })
}

fn sanitize_messages(value: Option<&Value>) -> Vec<Value> {
    value
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .take(MAX_MESSAGES)
        .filter_map(|message| {
            let message = message.as_object()?;
            let role = message.get("role")?.as_str()?;
            if !matches!(role, "user" | "assistant") {
                return None;
            }
            let content = adapter_content::sanitize_parts(message.get("content"));
            if content.is_empty() {
                return None;
            }
            Some(json!({
                "id": clean_string(message.get("id"), 160),
                "role": role,
                "state": match message.get("state").and_then(Value::as_str) {
                    Some("streaming") => "streaming",
                    _ => "completed",
                },
                "content": content,
            }))
        })
        .collect()
}

fn sanitize_attachments(value: Option<&Value>) -> Vec<Value> {
    value
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .take(10)
        .filter_map(|item| {
            let item = item.as_object()?;
            let id = clean_string(item.get("id"), 64);
            let name = clean_string(item.get("name"), 180);
            if !id.starts_with("attachment_") || name.is_empty() {
                return None;
            }
            Some(json!({
                "id": id,
                "name": name,
                "state": match item.get("state").and_then(Value::as_str) {
                    Some("uploading") => "uploading",
                    Some("error") => "error",
                    _ => "ready",
                },
                "removable": item.get("removable").and_then(Value::as_bool).unwrap_or(false),
            }))
        })
        .collect()
}

fn sanitize_features(value: Option<&Value>) -> Vec<Value> {
    value
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .take(60)
        .filter_map(|item| {
            let item = item.as_object()?;
            let id = clean_string(item.get("id"), 64);
            let label = clean_string(item.get("label"), 120);
            if !id.starts_with("feature_") || label.is_empty() {
                return None;
            }
            Some(json!({
                "id": id,
                "label": label,
                "kind": clean_identifier(item.get("kind"), 32),
                "selected": item.get("selected").and_then(Value::as_bool).unwrap_or(false),
            }))
        })
        .collect()
}

fn sanitize_ui_controls(value: Option<&Value>) -> Vec<Value> {
    value
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .take(512)
        .filter_map(|item| {
            let item = item.as_object()?;
            let id = clean_string(item.get("id"), 72);
            let label = clean_string(item.get("label"), 160);
            let region = clean_identifier(item.get("region"), 24);
            if !id.starts_with("control_")
                || label.is_empty()
                || !matches!(
                    region.as_str(),
                    "header" | "suggestions" | "composer" | "overlay" | "message" | "content"
                )
            {
                return None;
            }
            Some(json!({
                "id": id,
                "semantic": clean_identifier(item.get("semantic"), 48),
                "label": label,
                "region": region,
                "role": clean_identifier(item.get("role"), 32),
                "enabled": item.get("enabled").and_then(Value::as_bool).unwrap_or(true),
                "selected": item.get("selected").and_then(Value::as_bool).unwrap_or(false),
            }))
        })
        .collect()
}

fn sanitize_compatibility(value: Option<&Value>) -> &'static str {
    match value.and_then(Value::as_str) {
        Some("healthy") => "healthy",
        Some("fallback_required") => "fallback_required",
        _ => "partial",
    }
}

fn sanitize_conversations(value: Option<&Value>) -> Vec<Value> {
    value
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .take(MAX_OPTIONS)
        .filter_map(|item| {
            let item = item.as_object()?;
            let path = clean_string(item.get("path"), 256);
            if !is_safe_conversation_path(&path) {
                return None;
            }
            let project_id = clean_string(item.get("projectId"), 164);
            let project_path = clean_string(item.get("projectPath"), 256);
            Some(json!({
                "id": clean_string(item.get("id"), 160),
                "title": clean_string(item.get("title"), 160),
                "path": path,
                "active": item.get("active").and_then(Value::as_bool).unwrap_or(false),
                "groupLabel": clean_string(item.get("groupLabel"), 80),
                "projectId": is_safe_project_id(&project_id).then_some(project_id),
                "projectTitle": clean_string(item.get("projectTitle"), 160),
                "projectPath": is_safe_project_path(&project_path).then_some(project_path),
                "activityDates": sanitize_activity_dates(item.get("activityDates")),
            }))
        })
        .collect()
}

fn sanitize_projects(value: Option<&Value>) -> Vec<Value> {
    value
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .take(MAX_PROJECTS)
        .filter_map(|item| {
            let item = item.as_object()?;
            let id = clean_string(item.get("id"), 164);
            let path = clean_string(item.get("path"), 256);
            let title = clean_string(item.get("title"), 160);
            if !is_safe_project_id(&id) || !is_safe_project_path(&path) || title.is_empty() {
                return None;
            }
            Some(json!({
                "id": id,
                "title": title,
                "path": path,
                "active": item.get("active").and_then(Value::as_bool).unwrap_or(false),
            }))
        })
        .collect()
}

fn sanitize_activity_dates(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .take(10)
        .filter_map(Value::as_str)
        .filter(|value| is_iso_date(value))
        .map(str::to_string)
        .collect()
}

fn is_safe_conversation_path(path: &str) -> bool {
    let segments = path
        .strip_prefix('/')
        .map(|value| value.split('/').collect::<Vec<_>>());
    match segments.as_deref() {
        Some(["c", conversation_id]) => is_safe_route_id(conversation_id, 160),
        Some(["g", project_id, "c", conversation_id]) => {
            is_safe_project_id(project_id) && is_safe_route_id(conversation_id, 160)
        }
        _ => false,
    }
}

fn is_safe_project_path(path: &str) -> bool {
    let segments = path
        .strip_prefix('/')
        .map(|value| value.split('/').collect::<Vec<_>>());
    match segments.as_deref() {
        Some(["g", project_id]) | Some(["g", project_id, "project"]) => {
            is_safe_project_id(project_id)
        }
        _ => false,
    }
}

fn is_safe_project_id(value: &str) -> bool {
    value
        .strip_prefix("g-p-")
        .is_some_and(|suffix| is_safe_route_id(suffix, 160))
}

fn is_safe_route_id(value: &str, max: usize) -> bool {
    !value.is_empty()
        && value.len() <= max
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
}

fn is_iso_date(value: &str) -> bool {
    value.len() == 10
        && value.bytes().enumerate().all(|(index, byte)| match index {
            4 | 7 => byte == b'-',
            _ => byte.is_ascii_digit(),
        })
}

fn sanitize_options(value: Option<&Value>) -> Vec<Value> {
    value
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .take(30)
        .filter_map(|item| {
            let item = item.as_object()?;
            Some(json!({
                "id": clean_identifier(item.get("id"), 64),
                "label": clean_string(item.get("label"), 120),
                "selected": item.get("selected").and_then(Value::as_bool).unwrap_or(false),
                "kind": clean_identifier(item.get("kind"), 32),
                "semantic": clean_identifier(item.get("semantic"), 32),
                "opensSubmenu": item.get("opensSubmenu").and_then(Value::as_bool).unwrap_or(false),
            }))
        })
        .collect()
}

fn clean_identifiers(value: Option<&Value>, max: usize) -> Vec<String> {
    value
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .take(max)
        .map(|item| clean_identifier(Some(item), 48))
        .filter(|item| !item.is_empty())
        .collect()
}

fn clean_identifier(value: Option<&Value>, max: usize) -> String {
    clean_string(value, max)
        .chars()
        .filter(|character| {
            character.is_ascii_lowercase() || character.is_ascii_digit() || *character == '_'
        })
        .take(max)
        .collect()
}

fn sanitize_request_id(value: Option<&Value>) -> Option<String> {
    let request_id = clean_string(value, 36);
    (request_id.len() >= 5
        && request_id.starts_with("mcp_")
        && request_id.len() <= 36
        && request_id
            .bytes()
            .skip(4)
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit()))
    .then_some(request_id)
}

fn bounded_u64(value: Option<&Value>, default: u64, max: u64) -> u64 {
    value.and_then(Value::as_u64).unwrap_or(default).min(max)
}

fn sanitize_page_kind(value: Option<&Value>) -> &'static str {
    match value.and_then(Value::as_str) {
        Some("auth") => "auth",
        Some("conversation") => "conversation",
        Some("home") => "home",
        Some("feature") => "feature",
        _ => "unknown",
    }
}

fn clean_string(value: Option<&Value>, max: usize) -> String {
    value
        .and_then(Value::as_str)
        .unwrap_or_default()
        .chars()
        .filter(|character| !character.is_control() || matches!(character, '\n' | '\t'))
        .take(max)
        .collect::<String>()
        .trim()
        .to_string()
}

fn sanitize_chatgpt_url(value: Option<&Value>) -> String {
    let Some(raw) = value.and_then(Value::as_str) else {
        return String::new();
    };
    let Ok(url) = raw.parse::<tauri::Url>() else {
        return String::new();
    };
    if url.scheme() != "https" || url.host_str() != Some("chatgpt.com") {
        return String::new();
    }
    format!("https://chatgpt.com{}", url.path())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn semantic_snapshot_drops_queries_and_unsupported_content() {
        let raw = serde_json::to_string(&json!({
            "schema": "yilong.ai.ui.v1",
            "adapterVersion": ADAPTER_VERSION,
            "documentToken": "doc_win_contract",
            "providerId": "chatgpt",
            "event": {
                "type": "message_snapshot",
                "url": "https://chatgpt.com/c/test?token=secret",
                "pageKind": "auth",
                "loginRequired": true,
                "draft": "hello",
                "messages": [{
                    "id": "m1",
                    "role": "assistant",
                    "state": "completed",
                    "content": [
                        {"type": "text", "text": "visible"},
                        {"type": "cookie", "text": "secret"}
                    ]
                }]
            }
        }))
        .unwrap();
        let event = sanitize_event(&raw).unwrap();
        assert_eq!(event.kind, "message_snapshot");
        assert_eq!(event.payload["url"], "https://chatgpt.com/c/test");
        assert_eq!(event.payload["pageKind"], "auth");
        assert_eq!(event.payload["loginRequired"], true);
        assert!(!event.payload.to_string().contains("secret"));
        assert!(event.payload.to_string().contains("visible"));
    }

    #[test]
    fn oversized_and_unknown_events_are_rejected() {
        assert!(sanitize_event(&"x".repeat(MAX_EVENT_BYTES + 1)).is_err());
        assert!(sanitize_event(r#"{"type":"cookie_dump"}"#).is_err());
    }

    #[test]
    fn conversation_directory_keeps_safe_projects_and_project_chats() {
        let raw = serde_json::to_string(&json!({
            "schema": "yilong.ai.ui.v1",
            "adapterVersion": ADAPTER_VERSION,
            "documentToken": "doc_win_contract",
            "providerId": "chatgpt",
            "event": {
                "type": "conversation_snapshot",
                "projects": [
                    {"id": "g-p-roadmap", "title": "路线图", "path": "/g/g-p-roadmap/project"},
                    {"id": "bad", "title": "丢弃", "path": "https://example.com"}
                ],
                "conversations": [
                    {
                        "id": "chat-1",
                        "title": "规划",
                        "path": "/g/g-p-roadmap/c/chat-1",
                        "projectId": "g-p-roadmap",
                        "projectTitle": "路线图",
                        "projectPath": "/g/g-p-roadmap/project",
                        "groupLabel": "已置顶",
                        "activityDates": ["2026-08-14", "not-a-date"]
                    },
                    {"id": "bad", "title": "丢弃", "path": "/g/../../secret"}
                ]
            }
        }))
        .unwrap();
        let event = sanitize_event(&raw).unwrap();
        assert_eq!(event.payload["projects"].as_array().unwrap().len(), 1);
        assert_eq!(event.payload["conversations"].as_array().unwrap().len(), 1);
        assert_eq!(
            event.payload["conversations"][0]["activityDates"],
            json!(["2026-08-14"])
        );
    }

    #[test]
    fn protocol_events_require_current_adapter_metadata_shape() {
        let event = json!({
            "schema": "yilong.ai.ui.v1",
            "providerId": "chatgpt",
            "adapterVersion": ADAPTER_VERSION,
            "documentToken": "doc_win_contract",
            "event": { "type": "adapter_ready", "capabilities": [] }
        });
        assert!(sanitize_event(&event.to_string()).is_ok());

        let mut wrong_version = event.clone();
        wrong_version["adapterVersion"] = json!(ADAPTER_VERSION + 1);
        assert!(sanitize_event(&wrong_version.to_string()).is_err());

        let mut missing_token = event;
        missing_token
            .as_object_mut()
            .unwrap()
            .remove("documentToken");
        assert!(sanitize_event(&missing_token.to_string()).is_err());
    }
}
