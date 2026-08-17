#[path = "semantic_context/chatgpt_window.rs"]
mod chatgpt_window;

use serde_json::Value;
use std::{
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};
use tauri::Url;

const GOOGLE_HISTORY_LIMIT: usize = 80;
static NEXT_CONVERSATION_NONCE: AtomicU64 = AtomicU64::new(1);

pub(super) fn page_context_key(provider_id: &str, raw_url: &str) -> Option<String> {
    let mut url = raw_url.parse::<Url>().ok()?;
    if url.scheme() != "https"
        || !url.username().is_empty()
        || url.password().is_some()
        || url.port().is_some_and(|port| port != 443)
    {
        return None;
    }
    match provider_id {
        "chatgpt" if url.host_str() == Some("chatgpt.com") => {
            url.set_query(None);
            url.set_fragment(None);
        }
        "google-ai-mode" if matches!(url.host_str(), Some("google.com" | "www.google.com")) => {
            url.set_fragment(None);
        }
        _ => return None,
    }
    Some(opaque_id(url.as_str()))
}

pub(super) fn target_context_key(provider_id: &str, target: &str) -> Option<String> {
    let url = match provider_id {
        "chatgpt" if target.starts_with('/') => format!("https://chatgpt.com{target}"),
        _ if target.starts_with("https://") => target.to_string(),
        _ => return None,
    };
    page_context_key(provider_id, &url)
}

pub(super) fn opaque_id(value: &str) -> String {
    let hash = value
        .as_bytes()
        .iter()
        .fold(0xcbf29ce484222325_u64, |hash, byte| {
            (hash ^ u64::from(*byte)).wrapping_mul(0x100000001b3)
        });
    format!("{hash:016x}")
}

pub(super) fn fresh_conversation_id(provider_id: &str) -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    let nonce = NEXT_CONVERSATION_NONCE.fetch_add(1, Ordering::Relaxed);
    opaque_id(&format!("{provider_id}:{now}:{nonce}"))
}

pub(super) fn is_new_conversation_surface(provider_id: &str, raw_url: &str) -> bool {
    let Ok(url) = raw_url.parse::<Url>() else {
        return false;
    };
    match provider_id {
        "chatgpt" => url.host_str() == Some("chatgpt.com") && url.path() == "/",
        "google-ai-mode" => {
            matches!(url.host_str(), Some("google.com" | "www.google.com"))
                && matches!(url.path(), "/aimode" | "/")
        }
        _ => false,
    }
}

pub(super) fn has_same_last_user(previous: Option<&Value>, incoming: &Value) -> bool {
    let previous = previous.and_then(last_user_text);
    let incoming = last_user_text(incoming);
    previous.is_some() && previous == incoming
}

pub(super) fn has_last_user_text(snapshot: &Value, expected: &str) -> bool {
    let expected = expected.split_whitespace().collect::<Vec<_>>().join(" ");
    last_user_text(snapshot).as_deref() == Some(expected.as_str())
}

pub(super) fn merge_message_snapshot(
    provider_id: &str,
    previous: Option<&Value>,
    mut incoming: Value,
    same_conversation: bool,
) -> Value {
    if provider_id == "chatgpt" {
        return chatgpt_window::merge(previous, incoming, same_conversation);
    }
    if provider_id != "google-ai-mode" {
        return incoming;
    }
    sanitize_google_snapshot(&mut incoming);
    let base_window_start = previous
        .and_then(|snapshot| snapshot.get("messageWindowStart"))
        .and_then(Value::as_u64)
        .unwrap_or_default() as usize;
    let incoming_messages = messages(&incoming);
    if !same_conversation {
        replace_messages(&mut incoming, incoming_messages, base_window_start);
        return incoming;
    }
    let Some(previous) = previous else {
        replace_messages(&mut incoming, incoming_messages, base_window_start);
        return incoming;
    };
    let previous_messages = messages(previous)
        .into_iter()
        .filter(|message| !google_page_chrome_message(message))
        .collect::<Vec<_>>();
    if incoming_messages.is_empty() {
        replace_messages(&mut incoming, previous_messages, base_window_start);
        return incoming;
    }
    let merged = merge_google_turns(previous_messages, incoming_messages);
    replace_messages(&mut incoming, merged, base_window_start);
    incoming
}

pub(super) fn sanitize_google_snapshot(snapshot: &mut Value) {
    let window_start = snapshot
        .get("messageWindowStart")
        .and_then(Value::as_u64)
        .unwrap_or_default();
    let Some(messages) = snapshot.get_mut("messages").and_then(Value::as_array_mut) else {
        return;
    };
    messages.retain(|message| !google_page_chrome_message(message));
    let observed = window_start.saturating_add(messages.len() as u64);
    if let Some(snapshot) = snapshot.as_object_mut() {
        snapshot.insert("observedMessageCount".to_string(), Value::from(observed));
    }
}

fn google_page_chrome_message(message: &Value) -> bool {
    if role(message) != Some("assistant") {
        return false;
    }
    let text = normalized_text(message).to_lowercase();
    if text.is_empty() || text.chars().count() > 1_600 {
        return false;
    }
    let signed_out = [
        "您已退出账号",
        "您已退出帐号",
        "若要访问历史记录",
        "you are signed out",
        "you're signed out",
        "sign in to access",
        "sign in to view",
    ]
    .iter()
    .any(|needle| text.contains(needle));
    let chrome_signals = [
        "打开边栏",
        "关闭边栏",
        "open sidebar",
        "close sidebar",
        "新话题",
        "新对话",
        "new chat",
        "new conversation",
        "共享的公开链接",
        "分享公开链接",
        "public link",
        "ai 模式历史记录",
        "ai mode history",
        "搜索消息串",
        "search chats",
    ]
    .iter()
    .filter(|needle| text.contains(**needle))
    .count();
    (signed_out && chrome_signals >= 1) || chrome_signals >= 4
}

fn merge_google_turns(previous: Vec<Value>, incoming: Vec<Value>) -> Vec<Value> {
    let mut merged = message_turns(previous);
    for incoming_turn in message_turns(incoming) {
        let user_text = incoming_turn.first().map(normalized_text).unwrap_or_default();
        if user_text.is_empty() {
            continue;
        }
        if let Some(index) = merged.iter().position(|turn| {
            turn.first().map(normalized_text).as_deref() == Some(user_text.as_str())
        }) {
            if incoming_turn.iter().any(|message| role(message) == Some("assistant")) {
                merged[index] = incoming_turn;
            }
        } else {
            merged.push(incoming_turn);
        }
    }
    merged.into_iter().flatten().collect()
}

fn message_turns(source: Vec<Value>) -> Vec<Vec<Value>> {
    let mut turns = Vec::<Vec<Value>>::new();
    for message in source {
        if role(&message) == Some("user") {
            turns.push(vec![message]);
        } else if let Some(turn) = turns.last_mut() {
            turn.push(message);
        }
    }
    turns
}

pub(super) fn has_visible_messages(snapshot: &Value) -> bool {
    snapshot
        .get("messages")
        .and_then(Value::as_array)
        .is_some_and(|messages| !messages.is_empty())
}

pub(super) fn has_completed_assistant(snapshot: &Value) -> bool {
    snapshot
        .get("messages")
        .and_then(Value::as_array)
        .is_some_and(|messages| {
            messages.iter().rev().any(|message| {
                role(message) == Some("assistant")
                    && message.get("state").and_then(Value::as_str) != Some("streaming")
            })
        })
}

fn messages(snapshot: &Value) -> Vec<Value> {
    snapshot
        .get("messages")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
}

fn role(message: &Value) -> Option<&str> {
    message.get("role").and_then(Value::as_str)
}

fn normalized_text(message: &Value) -> String {
    message
        .get("content")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|part| {
            matches!(
                part.get("type").and_then(Value::as_str),
                Some("text" | "markdown")
            )
            .then(|| part.get("text").and_then(Value::as_str))
            .flatten()
        })
        .collect::<Vec<_>>()
        .join("\n")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn last_user_text(snapshot: &Value) -> Option<String> {
    snapshot
        .get("messages")
        .and_then(Value::as_array)
        .and_then(|messages| {
            messages
                .iter()
                .rev()
                .find(|message| role(message) == Some("user"))
        })
        .map(normalized_text)
}

fn replace_messages(snapshot: &mut Value, source: Vec<Value>, base_window_start: usize) {
    let source_len = source.len();
    let dropped = source_len.saturating_sub(GOOGLE_HISTORY_LIMIT);
    let mut bounded = source.into_iter().skip(dropped).collect::<Vec<_>>();
    for (index, message) in bounded.iter_mut().enumerate() {
        if let Some(message) = message.as_object_mut() {
            let role = message
                .get("role")
                .and_then(Value::as_str)
                .unwrap_or("message")
                .to_string();
            message.insert(
                "id".to_string(),
                Value::String(format!(
                    "google-message-{}-{role}",
                    base_window_start + dropped + index
                )),
            );
        }
    }
    if let Some(snapshot) = snapshot.as_object_mut() {
        snapshot.insert("messages".to_string(), Value::Array(bounded));
        snapshot.insert(
            "messageWindowStart".to_string(),
            Value::from((base_window_start + dropped) as u64),
        );
        snapshot.insert(
            "observedMessageCount".to_string(),
            Value::from((base_window_start + source_len) as u64),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn page_context_key_keeps_google_queries_private_but_distinct() {
        let first = page_context_key(
            "google-ai-mode",
            "https://www.google.com/search?q=secret-one",
        )
        .unwrap();
        let second = page_context_key(
            "google-ai-mode",
            "https://www.google.com/search?q=secret-two",
        )
        .unwrap();
        assert_ne!(first, second);
        assert_eq!(first.len(), 16);
        assert!(!first.contains("secret"));
    }

    #[test]
    fn google_merge_replaces_streaming_copy_of_the_same_turn() {
        let previous = json!({"messages":[
            {"role":"user","content":[{"type":"text","text":"one"}]},
            {"role":"assistant","state":"streaming","content":[{"type":"text","text":"partial"}]}
        ]});
        let incoming = json!({"messages":[
            {"role":"user","content":[{"type":"text","text":"one"}]},
            {"role":"assistant","state":"completed","content":[{"type":"text","text":"complete"}]}
        ]});
        let merged = merge_message_snapshot("google-ai-mode", Some(&previous), incoming, true);
        assert_eq!(merged["messages"].as_array().unwrap().len(), 2);
        assert_eq!(merged["messages"][1]["content"][0]["text"], "complete");
    }

    #[test]
    fn google_partial_prefix_refresh_keeps_cached_followups() {
        let previous = json!({"messages":[
            {"role":"user","content":[{"type":"text","text":"first"}]},
            {"role":"assistant","content":[{"type":"text","text":"first answer"}]},
            {"role":"user","content":[{"type":"text","text":"second"}]},
            {"role":"assistant","content":[{"type":"text","text":"second answer"}]}
        ]});
        let incoming = json!({"messages":[
            {"role":"user","content":[{"type":"text","text":"first"}]},
            {"role":"assistant","content":[{"type":"text","text":"first answer"}]}
        ]});

        let merged = merge_message_snapshot("google-ai-mode", Some(&previous), incoming, true);

        assert_eq!(merged["messages"].as_array().unwrap().len(), 4);
        assert_eq!(merged["messages"][2]["content"][0]["text"], "second");
    }

    #[test]
    fn google_full_dom_refresh_replaces_matching_cached_window_without_duplicates() {
        let previous = json!({"messages":[
            {"role":"user","content":[{"type":"text","text":"first"}]},
            {"role":"assistant","content":[{"type":"text","text":"old first answer"}]},
            {"role":"user","content":[{"type":"text","text":"second"}]},
            {"role":"assistant","content":[{"type":"text","text":"old second answer"}]}
        ]});
        let incoming = json!({"messages":[
            {"role":"user","content":[{"type":"text","text":"first"}]},
            {"role":"assistant","content":[{"type":"text","text":"fresh first answer"}]},
            {"role":"user","content":[{"type":"text","text":"second"}]},
            {"role":"assistant","content":[{"type":"text","text":"fresh second answer"}]}
        ]});

        let merged = merge_message_snapshot("google-ai-mode", Some(&previous), incoming, true);

        assert_eq!(merged["messages"].as_array().unwrap().len(), 4);
        assert_eq!(
            merged["messages"][3]["content"][0]["text"],
            "fresh second answer"
        );
    }

    #[test]
    fn google_page_chrome_answer_is_removed_from_a_cached_snapshot() {
        let mut snapshot = json!({"messages":[
            {"role":"user","content":[{"type":"text","text":"hello"}]},
            {"role":"assistant","content":[{"type":"text","text":"打开边栏 新话题 共享的公开链接 AI 模式历史记录 您已退出账号 若要访问历史记录，请登录您的账号"}]}
        ]});

        sanitize_google_snapshot(&mut snapshot);

        assert_eq!(snapshot["messages"].as_array().unwrap().len(), 1);
        assert_eq!(snapshot["messages"][0]["role"], "user");
    }

    #[test]
    fn chatgpt_rolling_window_keeps_the_overlapping_conversation_prefix() {
        let previous = json!({
            "messageWindowStart": 0,
            "observedMessageCount": 4,
            "messages": [
                {"id":"u1","role":"user"},
                {"id":"a1","role":"assistant"},
                {"id":"u2","role":"user"},
                {"id":"a2","role":"assistant"}
            ]
        });
        let incoming = json!({
            "messageWindowStart": 2,
            "observedMessageCount": 6,
            "messages": [
                {"id":"u2","role":"user"},
                {"id":"a2","role":"assistant"},
                {"id":"u3","role":"user"},
                {"id":"a3","role":"assistant"}
            ]
        });
        let merged = merge_message_snapshot("chatgpt", Some(&previous), incoming, true);
        let messages = merged["messages"].as_array().unwrap();
        assert_eq!(messages.len(), 6);
        assert_eq!(messages[0]["id"], "u1");
        assert_eq!(messages[5]["id"], "a3");
        assert_eq!(merged["messageWindowStart"], 0);
        assert_eq!(merged["observedMessageCount"], 6);
    }

    #[test]
    fn chatgpt_temporary_short_snapshot_does_not_erase_later_turns() {
        let previous = json!({
            "messageWindowStart": 0,
            "observedMessageCount": 4,
            "messages": [
                {"id":"u1","role":"user"},
                {"id":"a1","role":"assistant"},
                {"id":"u2","role":"user"},
                {"id":"a2","role":"assistant"}
            ]
        });
        let incoming = json!({
            "messageWindowStart": 0,
            "observedMessageCount": 2,
            "messages": [
                {"id":"u1","role":"user"},
                {"id":"a1","role":"assistant"}
            ]
        });
        let merged = merge_message_snapshot("chatgpt", Some(&previous), incoming, true);
        assert_eq!(merged["messages"].as_array().unwrap().len(), 4);
        assert_eq!(merged["observedMessageCount"], 4);
    }

    #[test]
    fn chatgpt_new_conversation_never_inherits_the_previous_window() {
        let previous = json!({"messages":[{"id":"old","role":"assistant"}]});
        let incoming = json!({"messages":[{"id":"new","role":"assistant"}]});
        let merged = merge_message_snapshot("chatgpt", Some(&previous), incoming, false);
        assert_eq!(merged["messages"].as_array().unwrap().len(), 1);
        assert_eq!(merged["messages"][0]["id"], "new");
    }
}
