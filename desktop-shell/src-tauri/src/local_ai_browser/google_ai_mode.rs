use serde_json::{json, Map, Value};

use super::adapter::SanitizedAdapterEvent;

const MAX_EVENT_BYTES: usize = 512 * 1024;
const MAX_MESSAGES: usize = 12;
const MAX_MESSAGE_CHARS: usize = 40_000;
const MAX_DRAFT_CHARS: usize = 20_000;

pub fn initialization_script() -> String {
    let adapter = include_str!("google_ai_mode_adapter.js");
    format!(
        r#"
(function () {{
  'use strict';
  if (window.__elonWinGoogleAiModeBootstrap) return;
  window.__elonWinGoogleAiModeBootstrap = true;

  function allowedOrigin() {{
    return location.origin === 'https://google.com' || location.origin === 'https://www.google.com';
  }}

  function invoke(payload) {{
    if (!allowedOrigin()) return;
    var internalInvoke = window.__TAURI_INTERNALS__ && window.__TAURI_INTERNALS__.invoke;
    var publicInvoke = window.__TAURI__ && window.__TAURI__.core && window.__TAURI__.core.invoke;
    var call = internalInvoke || publicInvoke;
    if (typeof call === 'function') {{
      Promise.resolve(call('publish_local_ai_web_event', {{ payload: String(payload || '') }})).catch(function () {{}});
    }}
  }}

  window.elonGoogleAiModeNative = Object.freeze({{ postMessage: invoke }});

  function diagnostic(kind, detail) {{
    invoke(JSON.stringify({{
      type: 'browser_diagnostic',
      kind: String(kind || '').slice(0, 48),
      detail: String(detail || '').slice(0, 240),
      url: location.origin + location.pathname
    }}));
  }}

  window.addEventListener('error', function (event) {{
    diagnostic('page_error', event && event.message ? event.message : 'Google AI 模式页面脚本加载失败。');
  }});
  window.addEventListener('unhandledrejection', function () {{
    diagnostic('promise_rejection', 'Google AI 模式页面尚未完成初始化，可显示官方窗口确认。');
  }});

  function start() {{
    if (!allowedOrigin()) return;
    window.setTimeout(function () {{
      if (!document.querySelector('main, [role="main"], form')) {{
        diagnostic('page_not_ready', 'Google AI 模式页面尚未就绪；地区、语言或账号可能暂未开放。');
      }}
    }}, 9000);
    {adapter}
  }}

  if (document.documentElement) start();
  else document.addEventListener('DOMContentLoaded', start, {{ once: true }});
}})();
"#
    )
}

pub fn sanitize_event(raw: &str) -> Result<SanitizedAdapterEvent, String> {
    if raw.len() > MAX_EVENT_BYTES {
        return Err("Google AI 模式可见语义事件过大，已拒绝。".to_string());
    }
    let value: Value = serde_json::from_str(raw).map_err(|_| "Google AI 模式语义事件格式无效。")?;
    if value.get("schema").and_then(Value::as_str) == Some("yilong.ai.ui.v1") {
        if value.get("providerId").and_then(Value::as_str) != Some("google-ai-mode") {
            return Err("Google AI 模式语义事件厂商标识无效。".to_string());
        }
        let event = value
            .get("event")
            .and_then(Value::as_object)
            .ok_or_else(|| "Google AI 模式语义事件缺少 event。".to_string())?;
        return sanitize_protocol_event(event);
    }

    match value.get("type").and_then(Value::as_str) {
        Some("command_result") => Ok(SanitizedAdapterEvent {
            kind: "command_result".to_string(),
            payload: json!({
                "type": "command_result",
                "action": clean_identifier(value.get("action"), 48),
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
                "url": sanitize_google_url(value.get("url")),
            }),
        }),
        _ => Err("不支持的 Google AI 模式本地浏览器事件。".to_string()),
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
            "capabilities": clean_identifiers(event.get("capabilities"), 20),
        }),
        "message_snapshot" => json!({
            "type": kind,
            "title": clean_string(event.get("title"), 160),
            "url": sanitize_google_url(event.get("url")),
            "draft": clean_string(event.get("draft"), MAX_DRAFT_CHARS),
            "messages": sanitize_messages(event.get("messages")),
            "authenticated": event.get("authenticated").and_then(Value::as_bool).unwrap_or(false),
            "composerReady": event.get("composerReady").and_then(Value::as_bool).unwrap_or(false),
            "streaming": event.get("streaming").and_then(Value::as_bool).unwrap_or(false),
            "currentModel": clean_string(event.get("currentModel"), 80),
            "capabilities": clean_identifiers(event.get("capabilities"), 20),
        }),
        _ => return Err("不支持的 Google AI 模式可见语义事件类型。".to_string()),
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
                .take(24)
                .filter_map(sanitize_content_part)
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

fn sanitize_content_part(part: &Value) -> Option<Value> {
    let part = part.as_object()?;
    match part.get("type")?.as_str()? {
        "text" => {
            let text = clean_string(part.get("text"), MAX_MESSAGE_CHARS);
            (!text.is_empty()).then(|| json!({"type": "text", "text": text}))
        }
        "citation" => {
            let url = sanitize_public_url(part.get("url"));
            (!url.is_empty()).then(|| {
                json!({
                    "type": "citation",
                    "title": clean_string(part.get("title"), 160),
                    "url": url,
                })
            })
        }
        _ => None,
    }
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

fn sanitize_google_url(value: Option<&Value>) -> String {
    sanitize_url(value, true)
}

fn sanitize_public_url(value: Option<&Value>) -> String {
    sanitize_url(value, false)
}

fn sanitize_url(value: Option<&Value>, google_only: bool) -> String {
    let Some(raw) = value.and_then(Value::as_str) else {
        return String::new();
    };
    let Ok(url) = raw.parse::<tauri::Url>() else {
        return String::new();
    };
    if url.scheme() != "https"
        || !url.username().is_empty()
        || url.password().is_some()
        || url.port().is_some_and(|port| port != 443)
    {
        return String::new();
    }
    let Some(host) = url.host_str() else {
        return String::new();
    };
    if google_only && !matches!(host, "google.com" | "www.google.com") {
        return String::new();
    }
    format!("https://{host}{}", url.path())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_keeps_visible_text_and_public_citations_only() {
        let raw = serde_json::to_string(&json!({
            "schema": "yilong.ai.ui.v1",
            "providerId": "google-ai-mode",
            "event": {
                "type": "message_snapshot",
                "url": "https://www.google.com/search?udm=50&q=private",
                "messages": [{
                    "id": "answer",
                    "role": "assistant",
                    "content": [
                        {"type": "text", "text": "visible answer"},
                        {"type": "citation", "title": "Rust", "url": "https://www.rust-lang.org/learn?tracking=secret"},
                        {"type": "credential", "text": "secret"}
                    ]
                }]
            }
        }))
        .unwrap();
        let event = sanitize_event(&raw).unwrap();
        assert_eq!(event.kind, "message_snapshot");
        assert_eq!(event.payload["url"], "https://www.google.com/search");
        assert!(event.payload.to_string().contains("visible answer"));
        assert!(event
            .payload
            .to_string()
            .contains("https://www.rust-lang.org/learn"));
        assert!(!event.payload.to_string().contains("secret"));
    }

    #[test]
    fn rejects_wrong_provider_and_oversized_events() {
        assert!(sanitize_event(&"x".repeat(MAX_EVENT_BYTES + 1)).is_err());
        assert!(sanitize_event(
            r#"{"schema":"yilong.ai.ui.v1","providerId":"chatgpt","event":{}}"#
        )
        .is_err());
    }
}
