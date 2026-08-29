//! Upstream SSE forwarding for the home chat stream.
//!
//! This module owns transport-level concerns only: it keeps the provider
//! stream from inheriting the ordinary request deadline, forwards deltas, and
//! accepts providers that omit the final newline. Conversation persistence and
//! user-facing recovery remain in `lm_chat.rs`.

use std::time::Duration;

use futures::StreamExt;
use serde_json::Value;
use tokio::sync::mpsc;

use crate::lm_chat_stream_support::send_stream_event;

pub(crate) const IDLE_TIMEOUT: Duration = Duration::from_secs(300);

pub(crate) struct ForwardOutcome {
    pub(crate) reply: String,
    pub(crate) client_connected: bool,
    pub(crate) failure: Option<ForwardFailure>,
}

pub(crate) enum ForwardFailure {
    Idle,
    Read(String),
}

pub(crate) async fn forward(
    response: reqwest::Response,
    tx: &mpsc::Sender<String>,
) -> ForwardOutcome {
    let mut upstream = response.bytes_stream();
    let mut buffer = String::new();
    let mut reply = String::new();

    loop {
        let next_chunk = match tokio::time::timeout(IDLE_TIMEOUT, upstream.next()).await {
            Ok(chunk) => chunk,
            Err(_) => {
                return ForwardOutcome {
                    reply,
                    client_connected: true,
                    failure: Some(ForwardFailure::Idle),
                };
            }
        };
        let Some(chunk) = next_chunk else { break };
        let chunk = match chunk {
            Ok(chunk) => chunk,
            Err(error) => {
                return ForwardOutcome {
                    reply,
                    client_connected: true,
                    failure: Some(ForwardFailure::Read(error.to_string())),
                };
            }
        };
        buffer.push_str(&String::from_utf8_lossy(&chunk));
        while let Some(position) = buffer.find('\n') {
            let line = buffer[..position].trim_end_matches('\r').to_string();
            buffer.drain(..=position);
            let Some(data) = line.strip_prefix("data:").map(str::trim) else {
                continue;
            };
            if data == "[DONE]" || data.is_empty() {
                continue;
            }
            if !forward_payload(data, &mut reply, tx).await {
                return ForwardOutcome {
                    reply,
                    client_connected: false,
                    failure: None,
                };
            }
        }
    }

    // A few OpenAI-compatible gateways close immediately after the final JSON
    // object and omit its trailing newline.
    if let Some(data) = buffer
        .trim()
        .strip_prefix("data:")
        .map(str::trim)
        .filter(|data| *data != "[DONE]" && !data.is_empty())
    {
        if !forward_payload(data, &mut reply, tx).await {
            return ForwardOutcome {
                reply,
                client_connected: false,
                failure: None,
            };
        }
    }

    ForwardOutcome {
        reply,
        client_connected: true,
        failure: None,
    }
}

async fn forward_payload(data: &str, reply: &mut String, tx: &mpsc::Sender<String>) -> bool {
    let Some(delta) = extract_delta(data) else {
        return true;
    };
    reply.push_str(&delta);
    send_stream_event(
        tx,
        serde_json::json!({
            "type": "delta",
            "content": delta,
        }),
    )
    .await
}

fn extract_delta(data: &str) -> Option<String> {
    let payload = serde_json::from_str::<Value>(data).ok()?;
    let delta = payload["choices"][0]["delta"]["content"]
        .as_str()
        .or_else(|| payload["choices"][0]["message"]["content"].as_str())
        .unwrap_or("");
    (!delta.is_empty()).then(|| delta.to_string())
}

#[cfg(test)]
mod tests {
    use super::extract_delta;

    #[test]
    fn extracts_incremental_delta_content() {
        assert_eq!(
            extract_delta(r#"{"choices":[{"delta":{"content":"你好"}}]}"#),
            Some("你好".to_string())
        );
    }

    #[test]
    fn accepts_non_stream_message_content() {
        assert_eq!(
            extract_delta(r#"{"choices":[{"message":{"content":"完成"}}]}"#),
            Some("完成".to_string())
        );
    }

    #[test]
    fn ignores_malformed_or_empty_payloads() {
        assert_eq!(extract_delta("not-json"), None);
        assert_eq!(
            extract_delta(r#"{"choices":[{"delta":{"role":"assistant"}}]}"#),
            None
        );
    }
}
