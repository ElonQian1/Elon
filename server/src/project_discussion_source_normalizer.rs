//! Provider-neutral normalization for exported conversation sources.

use anyhow::Result;
use serde_json::Value;
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NormalizedConversation {
    pub body: String,
    pub format: String,
    pub message_count: usize,
    pub source_id: String,
    pub content_revision: String,
}

#[derive(Debug, Clone)]
struct ConversationMessage {
    role: String,
    text: String,
    order: f64,
}

pub(crate) fn normalize_conversation(raw: &str) -> Result<NormalizedConversation> {
    let raw = raw.trim_start_matches('\u{feff}').trim();
    let parsed = serde_json::from_str::<Value>(raw).ok();
    let messages = parsed.as_ref().map(extract_messages).unwrap_or_default();
    let (body, format, message_count) = if messages.is_empty() {
        let format = if raw.lines().any(|line| line.trim_start().starts_with('#')) {
            "markdown"
        } else {
            "text"
        };
        (raw.to_string(), format.to_string(), 0)
    } else {
        (
            messages_to_markdown(&messages),
            "conversation_json".to_string(),
            messages.len(),
        )
    };
    let content_revision = digest(&body);
    Ok(NormalizedConversation {
        source_id: format!("conversation-{}", &content_revision[..16]),
        body,
        format,
        message_count,
        content_revision,
    })
}

fn extract_messages(value: &Value) -> Vec<ConversationMessage> {
    if let Some(mapping) = value.get("mapping").and_then(Value::as_object) {
        let mut messages = mapping
            .values()
            .filter_map(|node| node.get("message"))
            .filter_map(message_from_value)
            .collect::<Vec<_>>();
        messages.sort_by(|left, right| {
            left.order
                .partial_cmp(&right.order)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        return messages;
    }
    let candidates = value
        .as_array()
        .or_else(|| value.get("messages").and_then(Value::as_array))
        .or_else(|| value.get("conversation").and_then(Value::as_array))
        .or_else(|| value.get("chat").and_then(Value::as_array));
    candidates
        .into_iter()
        .flatten()
        .enumerate()
        .filter_map(|(index, item)| {
            let mut message = message_from_value(item)?;
            if message.order == 0.0 {
                message.order = index as f64;
            }
            Some(message)
        })
        .collect()
}

fn message_from_value(value: &Value) -> Option<ConversationMessage> {
    let role = value
        .get("role")
        .and_then(Value::as_str)
        .or_else(|| value.pointer("/author/role").and_then(Value::as_str))
        .unwrap_or("unknown")
        .trim()
        .to_ascii_lowercase();
    let text = content_text(value.get("content").unwrap_or(value))
        .trim()
        .to_string();
    if text.is_empty() {
        return None;
    }
    let order = value
        .get("create_time")
        .and_then(Value::as_f64)
        .or_else(|| value.get("timestamp").and_then(Value::as_f64))
        .unwrap_or(0.0);
    Some(ConversationMessage { role, text, order })
}

fn content_text(value: &Value) -> String {
    if let Some(text) = value.as_str() {
        return text.to_string();
    }
    if let Some(text) = value.get("text").and_then(Value::as_str) {
        return text.to_string();
    }
    if let Some(parts) = value.get("parts").and_then(Value::as_array) {
        return parts
            .iter()
            .map(content_text)
            .filter(|part| !part.trim().is_empty())
            .collect::<Vec<_>>()
            .join("\n\n");
    }
    if let Some(items) = value.as_array() {
        return items
            .iter()
            .map(content_text)
            .filter(|part| !part.trim().is_empty())
            .collect::<Vec<_>>()
            .join("\n\n");
    }
    String::new()
}

fn messages_to_markdown(messages: &[ConversationMessage]) -> String {
    let mut output = String::new();
    for (index, message) in messages.iter().enumerate() {
        let turn = format!("turn-{:04}", index + 1);
        output.push_str(&format!(
            "<a id=\"{turn}\"></a>\n## {turn} · {}\n\n{}\n\n",
            role_label(&message.role),
            message.text.trim()
        ));
    }
    output.trim().to_string()
}

fn role_label(role: &str) -> &str {
    match role {
        "user" => "用户",
        "assistant" => "AI",
        "system" => "系统",
        "developer" => "开发者",
        "tool" => "工具",
        _ => "未知角色",
    }
}

fn digest(value: &str) -> String {
    format!("{:x}", Sha256::digest(value.as_bytes()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_referenced_chatgpt_conversation_shape() {
        let raw = r#"{"conversation":[
          {"role":"user","content":[{"content_type":"text","text":"为什么开放？"}]},
          {"role":"assistant","content":[{"content_type":"text","text":"因为需要跨应用连接。"}]}
        ]}"#;
        let normalized = normalize_conversation(raw).unwrap();
        assert_eq!(normalized.format, "conversation_json");
        assert_eq!(normalized.message_count, 2);
        assert!(normalized.body.contains("turn-0001 · 用户"));
        assert!(normalized.body.contains("turn-0002 · AI"));
        assert!(normalized.source_id.starts_with("conversation-"));
    }

    #[test]
    fn supports_official_chatgpt_mapping_shape() {
        let raw = r#"{"mapping":{
          "b":{"message":{"author":{"role":"assistant"},"create_time":2,"content":{"parts":["回答"]}}},
          "a":{"message":{"author":{"role":"user"},"create_time":1,"content":{"parts":["问题"]}}}
        }}"#;
        let normalized = normalize_conversation(raw).unwrap();
        assert!(normalized.body.find("问题").unwrap() < normalized.body.find("回答").unwrap());
    }

    #[test]
    fn keeps_markdown_without_rewriting_meaning() {
        let normalized = normalize_conversation("# 标题\n\n正文").unwrap();
        assert_eq!(normalized.format, "markdown");
        assert_eq!(normalized.message_count, 0);
        assert_eq!(normalized.body, "# 标题\n\n正文");
    }
}
