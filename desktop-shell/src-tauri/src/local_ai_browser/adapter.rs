use serde_json::{json, Map, Value};

const MAX_EVENT_BYTES: usize = 512 * 1024;
const MAX_MESSAGES: usize = 80;
const MAX_MESSAGE_CHARS: usize = 40_000;
const MAX_DRAFT_CHARS: usize = 20_000;
const MAX_OPTIONS: usize = 100;

pub struct SanitizedAdapterEvent {
    pub kind: String,
    pub payload: Value,
}

pub fn initialization_script() -> String {
    let adapters = [
        include_str!(
            "../../../../android/app/src/main/assets/chatgpt_web_adapter_conversations.js"
        ),
        include_str!("../../../../android/app/src/main/assets/chatgpt_web_adapter_messages.js"),
        include_str!("../../../../android/app/src/main/assets/chatgpt_web_adapter_composer.js"),
        include_str!("../../../../android/app/src/main/assets/chatgpt_web_adapter.js"),
    ]
    .join("\n");

    format!(
        r#"
(function () {{
  'use strict';
  if (window.__elonWinChatGptBootstrap) return;
  window.__elonWinChatGptBootstrap = true;

  function invoke(payload) {{
    if (location.origin !== 'https://chatgpt.com') return;
    var internalInvoke = window.__TAURI_INTERNALS__ && window.__TAURI_INTERNALS__.invoke;
    var publicInvoke = window.__TAURI__ && window.__TAURI__.core && window.__TAURI__.core.invoke;
    var call = internalInvoke || publicInvoke;
    if (typeof call === 'function') {{
      Promise.resolve(call('publish_local_ai_web_event', {{ payload: String(payload || '') }})).catch(function () {{}});
    }}
  }}

  window.elonChatGptNative = Object.freeze({{ postMessage: invoke }});

  function diagnostic(kind, detail) {{
    invoke(JSON.stringify({{
      type: 'browser_diagnostic',
      kind: String(kind || '').slice(0, 48),
      detail: String(detail || '').slice(0, 240),
      url: location.origin + location.pathname
    }}));
  }}

  window.addEventListener('error', function (event) {{
    diagnostic('page_error', event && event.message ? event.message : 'ChatGPT 页面脚本加载失败。');
  }});
  window.addEventListener('unhandledrejection', function () {{
    diagnostic('promise_rejection', 'ChatGPT 页面尚未完成初始化，可尝试刷新或用系统浏览器继续。');
  }});

  function start() {{
    if (location.origin !== 'https://chatgpt.com') return;
    window.setTimeout(function () {{
      var text = String(document.body && document.body.innerText || '').trim();
      if (!text && !document.querySelector('iframe')) {{
        diagnostic('blank_page', 'ChatGPT 页面保持空白，请刷新；若仍失败，可在系统浏览器完成登录。');
      }}
    }}, 9000);
    {adapters}
  }}

  if (document.documentElement) start();
  else document.addEventListener('DOMContentLoaded', start, {{ once: true }});
}})();
"#
    )
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
            "authenticated": event.get("authenticated").and_then(Value::as_bool).unwrap_or(false),
            "composerReady": event.get("composerReady").and_then(Value::as_bool).unwrap_or(false),
            "streaming": event.get("streaming").and_then(Value::as_bool).unwrap_or(false),
            "currentModel": clean_string(event.get("currentModel"), 80),
            "capabilities": clean_identifiers(event.get("capabilities"), 40),
        }),
        "conversation_snapshot" => json!({
            "type": kind,
            "conversations": sanitize_conversations(event.get("conversations")),
        }),
        "composer_controls_snapshot" => json!({
            "type": kind,
            "section": clean_identifier(event.get("section"), 24),
            "currentModel": clean_string(event.get("currentModel"), 80),
            "options": sanitize_options(event.get("options")),
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
            let content = message
                .get("content")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .take(20)
                .filter_map(|part| {
                    let part = part.as_object()?;
                    let kind = part.get("type")?.as_str()?;
                    if !matches!(kind, "text" | "markdown") {
                        return None;
                    }
                    let text = clean_string(part.get("text"), MAX_MESSAGE_CHARS);
                    (!text.is_empty()).then(|| json!({"type": "text", "text": text}))
                })
                .collect::<Vec<_>>();
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

fn sanitize_conversations(value: Option<&Value>) -> Vec<Value> {
    value
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .take(MAX_OPTIONS)
        .filter_map(|item| {
            let item = item.as_object()?;
            let path = clean_string(item.get("path"), 256);
            if !path.starts_with("/c/") || path.contains('?') || path.contains('#') {
                return None;
            }
            Some(json!({
                "id": clean_string(item.get("id"), 160),
                "title": clean_string(item.get("title"), 160),
                "path": path,
                "active": item.get("active").and_then(Value::as_bool).unwrap_or(false),
            }))
        })
        .collect()
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
            "providerId": "chatgpt",
            "event": {
                "type": "message_snapshot",
                "url": "https://chatgpt.com/c/test?token=secret",
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
        assert!(!event.payload.to_string().contains("secret"));
        assert!(event.payload.to_string().contains("visible"));
    }

    #[test]
    fn oversized_and_unknown_events_are_rejected() {
        assert!(sanitize_event(&"x".repeat(MAX_EVENT_BYTES + 1)).is_err());
        assert!(sanitize_event(r#"{"type":"cookie_dump"}"#).is_err());
    }
}
