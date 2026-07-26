//! Shared WebSocket transport helpers.
//!
//! Protocol modules define the application payloads; this module owns the
//! low-level frame construction and common JSON text-frame inspection.

use axum::extract::ws::Message;
use futures::{Sink, SinkExt};
use serde::Serialize;

pub const MESSAGE_TYPE_FIELD: &str = "type";
pub const SERIALIZE_ERROR_PAYLOAD: &str =
    r#"{"type":"error","code":"serialize","message":"serialize failed"}"#;

pub fn text_message(payload: impl Into<String>) -> Message {
    Message::Text(payload.into())
}

pub fn json_text_payload<T: Serialize>(payload: &T) -> String {
    serde_json::to_string(payload).unwrap_or_else(|_| SERIALIZE_ERROR_PAYLOAD.to_string())
}

pub fn json_text_message<T: Serialize>(payload: &T) -> Message {
    text_message(json_text_payload(payload))
}

pub fn try_json_text_message<T: Serialize>(payload: &T) -> Result<Message, serde_json::Error> {
    serde_json::to_string(payload).map(text_message)
}

#[derive(Debug, PartialEq, Eq)]
pub enum WsIncoming {
    Text(String),
    Binary(Vec<u8>),
    Continue,
    Closed(WsCloseReason),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WsCloseReason {
    PeerClosed,
    ReadError,
    ReaderEnded,
    PongWriteFailed,
    WriteFailed,
    ClientControlClose,
}

impl WsCloseReason {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::PeerClosed => "peer_closed",
            Self::ReadError => "read_error",
            Self::ReaderEnded => "reader_ended",
            Self::PongWriteFailed => "pong_write_failed",
            Self::WriteFailed => "write_failed",
            Self::ClientControlClose => "client_control_close",
        }
    }
}

pub async fn send_text<S>(sender: &mut S, payload: impl Into<String>) -> bool
where
    S: Sink<Message> + Unpin,
{
    sender.send(text_message(payload)).await.is_ok()
}

pub async fn send_json<S, T>(sender: &mut S, payload: &T) -> bool
where
    S: Sink<Message> + Unpin,
    T: Serialize,
{
    sender.send(json_text_message(payload)).await.is_ok()
}

pub async fn receive_data_or_control<S>(
    incoming: Option<Result<Message, axum::Error>>,
    sender: &mut S,
) -> WsIncoming
where
    S: Sink<Message> + Unpin,
{
    match incoming {
        Some(Ok(Message::Text(text))) => WsIncoming::Text(text.to_string()),
        Some(Ok(Message::Binary(bytes))) => WsIncoming::Binary(bytes),
        Some(Ok(Message::Ping(payload))) => {
            if sender.send(Message::Pong(payload)).await.is_ok() {
                WsIncoming::Continue
            } else {
                WsIncoming::Closed(WsCloseReason::PongWriteFailed)
            }
        }
        Some(Ok(Message::Pong(_))) => WsIncoming::Continue,
        Some(Ok(Message::Close(_))) => WsIncoming::Closed(WsCloseReason::PeerClosed),
        Some(Err(_)) => WsIncoming::Closed(WsCloseReason::ReadError),
        None => WsIncoming::Closed(WsCloseReason::ReaderEnded),
    }
}

pub fn text_message_type(raw: &str) -> Option<String> {
    serde_json::from_str::<serde_json::Value>(raw)
        .ok()
        .and_then(|value| {
            value
                .get(MESSAGE_TYPE_FIELD)
                .and_then(|kind| kind.as_str())
                .map(str::to_string)
        })
}

pub fn text_message_type_is(raw: &str, expected: &str) -> bool {
    text_message_type(raw).as_deref() == Some(expected)
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::{channel::mpsc, StreamExt};
    use serde::ser::{Error as _, SerializeStruct};

    #[test]
    fn text_frame_uses_axum_text_constructor() {
        let frame = text_message("hello");

        let Message::Text(text) = frame else {
            panic!("expected text frame");
        };
        assert_eq!(text.to_string(), "hello");
    }

    #[test]
    fn extracts_type_from_json_text_frame() {
        assert_eq!(
            text_message_type(r#"{"type":"typing","toUserId":"u2"}"#).as_deref(),
            Some("typing")
        );
        assert!(text_message_type_is(
            r#"{"type":"project_task_completed"}"#,
            "project_task_completed"
        ));
    }

    #[test]
    fn rejects_invalid_or_untyped_text_frame() {
        assert_eq!(text_message_type("not json"), None);
        assert_eq!(text_message_type(r#"{"message":"missing type"}"#), None);
        assert_eq!(text_message_type(r#"{"type":42}"#), None);
    }

    #[test]
    fn json_payload_falls_back_to_protocol_error() {
        struct BrokenPayload;

        impl Serialize for BrokenPayload {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde::Serializer,
            {
                let mut state = serializer.serialize_struct("BrokenPayload", 1)?;
                state.serialize_field("broken", &AlwaysFails)?;
                state.end()
            }
        }

        struct AlwaysFails;

        impl Serialize for AlwaysFails {
            fn serialize<S>(&self, _serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde::Serializer,
            {
                Err(S::Error::custom("intentional failure"))
            }
        }

        assert_eq!(json_text_payload(&BrokenPayload), SERIALIZE_ERROR_PAYLOAD);
        let Message::Text(text) = json_text_message(&BrokenPayload) else {
            panic!("expected text frame");
        };
        assert_eq!(text.to_string(), SERIALIZE_ERROR_PAYLOAD);
    }

    #[test]
    fn fallible_json_text_message_returns_serializer_errors() {
        struct BrokenPayload;

        impl Serialize for BrokenPayload {
            fn serialize<S>(&self, _serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde::Serializer,
            {
                Err(S::Error::custom("intentional failure"))
            }
        }

        assert!(try_json_text_message(&BrokenPayload).is_err());
    }

    #[tokio::test]
    async fn receive_data_or_control_returns_text_and_binary_data() {
        let (mut tx, _rx) = mpsc::unbounded();

        assert_eq!(
            receive_data_or_control(Some(Ok(Message::Text("hello".into()))), &mut tx).await,
            WsIncoming::Text("hello".into())
        );
        assert_eq!(
            receive_data_or_control(Some(Ok(Message::Binary(vec![1, 2, 3]))), &mut tx).await,
            WsIncoming::Binary(vec![1, 2, 3])
        );
    }

    #[tokio::test]
    async fn receive_data_or_control_replies_to_ping() {
        let (mut tx, mut rx) = mpsc::unbounded();

        let result = receive_data_or_control(Some(Ok(Message::Ping(vec![9, 8]))), &mut tx).await;

        assert_eq!(result, WsIncoming::Continue);
        let Some(Message::Pong(payload)) = rx.next().await else {
            panic!("expected pong frame");
        };
        assert_eq!(payload, vec![9, 8]);
    }

    #[tokio::test]
    async fn receive_data_or_control_reports_closed_states() {
        let (mut tx, rx) = mpsc::unbounded::<Message>();
        drop(rx);

        assert_eq!(
            receive_data_or_control(Some(Ok(Message::Pong(vec![1]))), &mut tx).await,
            WsIncoming::Continue
        );
        assert_eq!(
            receive_data_or_control(None, &mut tx).await,
            WsIncoming::Closed(WsCloseReason::ReaderEnded)
        );
        assert_eq!(
            receive_data_or_control(Some(Ok(Message::Close(None))), &mut tx).await,
            WsIncoming::Closed(WsCloseReason::PeerClosed)
        );
        assert_eq!(
            receive_data_or_control(Some(Ok(Message::Ping(vec![1]))), &mut tx).await,
            WsIncoming::Closed(WsCloseReason::PongWriteFailed)
        );
    }

    #[test]
    fn ws_close_reason_labels_are_stable() {
        let cases = [
            (WsCloseReason::PeerClosed, "peer_closed"),
            (WsCloseReason::ReadError, "read_error"),
            (WsCloseReason::ReaderEnded, "reader_ended"),
            (WsCloseReason::PongWriteFailed, "pong_write_failed"),
            (WsCloseReason::WriteFailed, "write_failed"),
            (WsCloseReason::ClientControlClose, "client_control_close"),
        ];

        for (reason, expected) in cases {
            assert_eq!(reason.as_str(), expected);
        }
    }
}
