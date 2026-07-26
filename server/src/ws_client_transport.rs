//! Shared client-side WebSocket transport helpers.
//!
//! This module is for `tokio-tungstenite` client connections. Keep it separate
//! from `ws_transport`, which owns Axum server-side WebSocket frames.

use anyhow::{anyhow, Result};
use serde::Serialize;
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message;

pub fn text_message(payload: impl Into<String>) -> Message {
    Message::text(payload.into())
}

pub fn json_text_message<T: Serialize>(payload: &T) -> Result<Message> {
    Ok(text_message(serde_json::to_string(payload)?))
}

pub fn try_send_json<T: Serialize>(
    sender: &mpsc::UnboundedSender<Message>,
    payload: &T,
) -> Result<()> {
    let frame = json_text_message(payload)?;
    sender
        .send(frame)
        .map_err(|_| anyhow!("websocket client writer closed"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::ser::Error as _;

    #[derive(Serialize)]
    struct ClientEvent<'a> {
        #[serde(rename = "type")]
        kind: &'a str,
        req_id: &'a str,
    }

    #[test]
    fn client_text_frame_uses_tungstenite_constructor() {
        let frame = text_message("hello");

        let Message::Text(text) = frame else {
            panic!("expected text frame");
        };
        assert_eq!(text.to_string(), "hello");
    }

    #[test]
    fn json_text_message_serializes_payload() {
        let frame = json_text_message(&ClientEvent {
            kind: "pong",
            req_id: "req_1",
        })
        .expect("payload should serialize");

        let Message::Text(text) = frame else {
            panic!("expected text frame");
        };
        let value: serde_json::Value =
            serde_json::from_str(&text).expect("frame should contain json");
        assert_eq!(value["type"], "pong");
        assert_eq!(value["req_id"], "req_1");
    }

    #[test]
    fn json_text_message_returns_serializer_errors() {
        struct BrokenPayload;

        impl Serialize for BrokenPayload {
            fn serialize<S>(&self, _serializer: S) -> std::result::Result<S::Ok, S::Error>
            where
                S: serde::Serializer,
            {
                Err(S::Error::custom("intentional failure"))
            }
        }

        assert!(json_text_message(&BrokenPayload).is_err());
    }

    #[test]
    fn try_send_json_reports_closed_writer() {
        let (tx, rx) = mpsc::unbounded_channel();
        drop(rx);

        let result = try_send_json(
            &tx,
            &ClientEvent {
                kind: "pong",
                req_id: "req_1",
            },
        );

        assert!(result.is_err());
    }
}
