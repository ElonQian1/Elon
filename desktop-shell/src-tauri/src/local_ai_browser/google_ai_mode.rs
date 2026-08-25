use serde_json::{json, Map, Value};

use super::{
    adapter::{self, SanitizedAdapterEvent},
    adapter_content, semantic_context, snapshot_cache,
};

const MAX_EVENT_BYTES: usize = 512 * 1024;
const MAX_MESSAGES: usize = 80;
const MAX_DRAFT_CHARS: usize = 20_000;
pub(super) const ADAPTER_VERSION: u32 = 37;

pub fn initialization_script() -> String {
    super::google_ai_mode_adapter_bootstrap::initialization_script(ADAPTER_VERSION)
}

pub fn sanitize_event(raw: &str) -> Result<SanitizedAdapterEvent, String> {
    if raw.len() > MAX_EVENT_BYTES {
        return Err("Google AI 模式可见语义事件过大，已拒绝。".to_string());
    }
    let value: Value = serde_json::from_str(raw).map_err(|_| "Google AI 模式语义事件格式无效。")?;
    if value.get("schema").and_then(Value::as_str) == Some("yilong.ai.ui.v1") {
        if value.get("providerId").and_then(Value::as_str) != Some("google_web") {
            return Err("Google AI 模式语义事件厂商标识无效。".to_string());
        }
        if value.get("adapterVersion").and_then(Value::as_u64) != Some(u64::from(ADAPTER_VERSION)) {
            return Err("Google AI 模式语义适配器版本无效。".to_string());
        }
        let event = value
            .get("event")
            .and_then(Value::as_object)
            .ok_or_else(|| "Google AI 模式语义事件缺少 event。".to_string())?;
        return sanitize_protocol_event(event);
    }

    match value.get("type").and_then(Value::as_str) {
        Some("command_result")
            if value.get("action").and_then(Value::as_str) == Some("dom_diagnostics") =>
        {
            Ok(SanitizedAdapterEvent {
                kind: "adapter_diagnostic".to_string(),
                payload: json!({
                    "type": "adapter_diagnostic",
                    "kind": "dom_diagnostics",
                }),
                page_context_key: None,
                restorable_url: None,
            })
        }
        Some("command_result") => Ok(SanitizedAdapterEvent {
            kind: "command_result".to_string(),
            payload: json!({
                "type": "command_result",
                "action": clean_identifier(value.get("action"), 48),
                "ok": value.get("ok").and_then(Value::as_bool).unwrap_or(false),
                "detail": clean_string(value.get("detail"), 240),
                "requestId": sanitize_request_id(value.get("requestId")),
            }),
            page_context_key: None,
            restorable_url: None,
        }),
        Some("browser_diagnostic") => Ok(SanitizedAdapterEvent {
            kind: "browser_diagnostic".to_string(),
            payload: json!({
                "type": "browser_diagnostic",
                "kind": clean_identifier(value.get("kind"), 48),
                "detail": clean_string(value.get("detail"), 240),
                "url": sanitize_google_url(value.get("url")),
            }),
            page_context_key: None,
            restorable_url: None,
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
            "observedMessageCount": bounded_u64(event.get("observedMessageCount"), 0, 1_000_000),
            "messageWindowStart": bounded_u64(event.get("messageWindowStart"), 0, 1_000_000),
            "authenticated": event.get("authenticated").and_then(Value::as_bool).unwrap_or(false),
            "pageKind": sanitize_page_kind(event.get("pageKind")),
            "loginRequired": event.get("loginRequired").and_then(Value::as_bool).unwrap_or(false),
            "composerReady": event.get("composerReady").and_then(Value::as_bool).unwrap_or(false),
            "streaming": event.get("streaming").and_then(Value::as_bool).unwrap_or(false),
            "currentModel": clean_string(event.get("currentModel"), 80),
            "capabilities": clean_identifiers(event.get("capabilities"), 20),
        }),
        "conversation_snapshot" => json!({
            "type": kind,
            "conversations": adapter::sanitize_conversations(event.get("conversations")),
            "projects": [],
            "collection": adapter::sanitize_conversation_collection(event.get("collection")),
        }),
        _ => return Err("不支持的 Google AI 模式可见语义事件类型。".to_string()),
    };
    let page_context_key = (kind == "message_snapshot")
        .then(|| {
            event
                .get("url")
                .and_then(Value::as_str)
                .and_then(|url| semantic_context::page_context_key("google-ai-mode", url))
        })
        .flatten();
    let restorable_url = (kind == "message_snapshot")
        .then(|| {
            event
                .get("url")
                .and_then(Value::as_str)
                .and_then(|url| snapshot_cache::normalize_restorable_url("google-ai-mode", url))
        })
        .flatten();
    Ok(SanitizedAdapterEvent {
        kind: kind.to_string(),
        payload,
        page_context_key,
        restorable_url,
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
            let content = adapter_content::sanitize_parts("google-ai-mode", message.get("content"));
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

fn bounded_u64(value: Option<&Value>, default: u64, max: u64) -> u64 {
    value.and_then(Value::as_u64).unwrap_or(default).min(max)
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

fn sanitize_page_kind(value: Option<&Value>) -> &'static str {
    match value.and_then(Value::as_str) {
        Some("conversation") => "conversation",
        Some("ai_mode") => "ai_mode",
        Some("unsupported") => "unsupported",
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

fn sanitize_google_url(value: Option<&Value>) -> String {
    sanitize_url(value, true)
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
            "adapterVersion": ADAPTER_VERSION,
            "providerId": "google_web",
            "event": {
                "type": "message_snapshot",
                "url": "https://www.google.com/search?udm=50&q=private",
                "pageKind": "ai_mode",
                "loginRequired": false,
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
        assert_eq!(event.payload["pageKind"], "ai_mode");
        assert_eq!(
            event.restorable_url.as_deref(),
            Some("https://www.google.com/search?udm=50&q=private")
        );
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

    #[test]
    fn desktop_bootstrap_reuses_the_android_google_adapter() {
        let script = initialization_script();
        let android_page_adapter = include_str!(
            "../../../../android/app/src/main/kotlin/com/elon/app/googleweb/GoogleWebPageAdapter.kt"
        );
        let android_assets = android_page_adapter
            .split("private val ADAPTER_ASSETS = listOf(")
            .nth(1)
            .and_then(|tail| tail.split("\n        )").next())
            .expect("Android Google adapter asset list should remain readable")
            .lines()
            .filter_map(|line| {
                let value = line.trim().trim_end_matches(',');
                value
                    .strip_prefix('"')
                    .and_then(|value| value.strip_suffix('"'))
            })
            .collect::<Vec<_>>();
        let android_version = android_page_adapter
            .split("const val ADAPTER_VERSION = ")
            .nth(1)
            .and_then(|tail| tail.lines().next())
            .and_then(|value| value.trim().parse::<u32>().ok())
            .expect("Android Google adapter version should remain readable");
        let win_assets = super::super::google_ai_mode_adapter_bootstrap::adapter_asset_names();

        assert_eq!(win_assets, android_assets);
        assert_eq!(ADAPTER_VERSION, android_version);
        assert!(script.contains(&format!(
            "window.__elonGoogleWebAdapterVersion = {ADAPTER_VERSION}"
        )));
        assert!(script.contains("window.__elonGoogleWebDocumentToken"));
        assert!(script.contains("window.__elonGoogleWebAnswerCandidatePolicy"));
        assert!(script.contains("window.__elonGoogleWebRichContent"));
        assert!(script.contains("window.__elonGoogleWebMessageExtractor"));
        assert!(script.contains("window.__elonGoogleWebComposerBridge"));
        assert!(script.contains("window.__elonGoogleWebSendPolicy"));
        assert!(script.contains("window.elonGoogleWebNative"));
        assert!(script.contains("window.__elonGoogleWebBridge"));
        assert!(script.contains("providerId: 'google_web'"));
        assert!(script.contains("function installAdapter()"));
        assert!(script.contains("function installWhenReady()"));
        assert!(script.contains("document.documentElement instanceof Node"));
        assert!(script.contains("DOMContentLoaded"));
        assert!(script.contains("adapter_bootstrap_failed"));
        assert!(script.contains("publish_local_ai_web_research_capture"));
        assert!(script.contains("endpointFamily: 'ai_rpc'"));
        assert!(script.contains("window.__elonGoogleWebPrivateResearchEnabled = true"));
        assert!(script.contains("window.__elonGoogleWebPrivateResponseTap"));
        assert!(script.contains("research_network_observation"));
        assert!(script.contains("google_win_private_conversation_bridge.js"));
        assert!(script.contains("__elonWinGooglePrivateConversationBridgeVersion"));
        assert!(script.contains("__elonGoogleWebPrivateReplyObserver"));
        assert!(script.contains("__elonGoogleWebPrivateThreadDirectory"));
        assert!(script.contains("__elonGoogleWebQueryPolicy"));
        let dom_ready = script
            .find("DOMContentLoaded")
            .expect("Win bootstrap should retain its DOM-ready semantic install");
        assert!(
            script
                .find("__elonGoogleWebPrivateThreadDirectory")
                .unwrap()
                < dom_ready
        );
        let private_tap = script
            .find("window.__elonGoogleWebPrivateResponseTap")
            .expect("Win bootstrap should install the APK private response tap");
        let full_capture = script
            .find("window.__elonWinWebResponseResearchCaptureVersion")
            .expect("Win bootstrap should retain the bounded full response capture");
        assert!(private_tap < full_capture);
        assert!(full_capture < dom_ready);
    }

    #[test]
    fn private_google_directory_is_sanitized_for_win_navigation() {
        let raw = serde_json::to_string(&json!({
            "schema": "yilong.ai.ui.v1",
            "adapterVersion": ADAPTER_VERSION,
            "providerId": "google_web",
            "event": {
                "type": "conversation_snapshot",
                "conversations": [{
                    "id": "thread_1234567890",
                    "title": "BTC 走势",
                    "path": "/c/thread_1234567890",
                    "providerUrl": "https://www.google.com/search?udm=50&csuir=secret"
                }, {
                    "id": "bad",
                    "title": "bad",
                    "path": "https://example.com/private"
                }],
                "projects": [{"id": "private", "path": "/private"}],
                "collection": {"observedCount": 2, "complete": false}
            }
        }))
        .unwrap();
        let event = sanitize_event(&raw).unwrap();
        assert_eq!(event.kind, "conversation_snapshot");
        assert_eq!(event.payload["conversations"].as_array().unwrap().len(), 1);
        assert_eq!(event.payload["conversations"][0]["path"], "/c/thread_1234567890");
        assert_eq!(event.payload["projects"].as_array().unwrap().len(), 0);
        assert!(!event.payload.to_string().contains("csuir"));
        assert_eq!(event.payload["collection"]["observedCount"], 2);
    }
}
