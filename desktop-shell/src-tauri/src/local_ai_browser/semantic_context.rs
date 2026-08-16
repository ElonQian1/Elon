#[path = "semantic_context/chatgpt_window.rs"]
mod chatgpt_window;

use serde_json::Value;
use std::{
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};
use tauri::Url;

const GOOGLE_HISTORY_LIMIT: usize = 32;
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
    let previous_messages = messages(previous);
    let Some(current_user) = incoming_messages
        .iter()
        .rev()
        .find(|message| role(message) == Some("user"))
        .cloned()
    else {
        replace_messages(&mut incoming, previous_messages, base_window_start);
        return incoming;
    };
    let previous_user_index = previous_messages
        .iter()
        .rposition(|message| role(message) == Some("user"));
    let same_turn = previous_user_index.is_some_and(|index| {
        normalized_text(&previous_messages[index]) == normalized_text(&current_user)
    });
    let mut merged = if same_turn {
        previous_messages[..previous_user_index.unwrap_or_default()].to_vec()
    } else {
        previous_messages.clone()
    };
    let current_assistant = incoming_messages
        .iter()
        .rev()
        .find(|message| role(message) == Some("assistant"))
        .cloned();
    let previous_assistant = same_turn.then(|| {
        previous_messages[previous_user_index.unwrap_or_default() + 1..]
            .iter()
            .rev()
            .find(|message| role(message) == Some("assistant"))
            .cloned()
    });
    merged.push(current_user);
    if let Some(assistant) = current_assistant.or_else(|| previous_assistant.flatten()) {
        merged.push(assistant);
    }
    replace_messages(&mut incoming, merged, base_window_start);
    incoming
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
