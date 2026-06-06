//! OpenAI Realtime 语音到语音会话封装。
//!
//! Android ↔ 本服务：二进制 PCM16 帧。
//! 本服务 ↔ OpenAI：Realtime WebSocket JSON 事件，音频字段 base64。

use anyhow::{anyhow, Context, Result};
use base64::{engine::general_purpose, Engine as _};
use futures::{SinkExt, StreamExt};
use serde_json::{json, Value};
use tokio::sync::mpsc;
use tokio_tungstenite::{
    connect_async,
    tungstenite::{
        client::IntoClientRequest, http::header::HeaderValue, protocol::Message as TgMessage,
    },
};

use crate::voice_config::{RealtimeChatConfig, REALTIME_SAMPLE_RATE_HZ};

#[derive(Debug, Clone)]
pub enum RealtimeChatEvent {
    SessionUpdated,
    UserSpeechStarted,
    UserSpeechStopped,
    UserTranscriptDelta(String),
    UserTranscriptFinal(String),
    AiTranscriptDelta(String),
    AiTranscriptDone(String),
    AudioDelta(Vec<u8>),
    AudioDone,
    ResponseDone,
    Error(String),
    Closed,
}

pub struct RealtimeChatSession {
    tx_audio: mpsc::UnboundedSender<UplinkCommand>,
    pub event_rx: mpsc::UnboundedReceiver<RealtimeChatEvent>,
}

enum UplinkCommand {
    Audio(Vec<u8>),
    Commit,
    Close,
}

impl RealtimeChatSession {
    pub async fn connect(
        cfg: &RealtimeChatConfig,
        instructions: String,
        api_key: String,
    ) -> Result<Self> {
        let mut request = cfg
            .websocket_url()
            .as_str()
            .into_client_request()
            .context("构造 Realtime Chat WebSocket 请求失败")?;
        request.headers_mut().insert(
            "Authorization",
            HeaderValue::from_str(&format!("Bearer {api_key}"))?,
        );

        let (ws, _resp) = connect_async(request)
            .await
            .context("连接 OpenAI Realtime Chat WebSocket 失败")?;
        let (mut write, mut read) = ws.split();

        let session_update = json!({
            "type": "session.update",
            "session": {
                "type": "realtime",
                "model": cfg.model,
                "instructions": instructions,
                "output_modalities": ["audio"],
                "audio": {
                    "input": {
                        "format": {
                            "type": "audio/pcm",
                            "rate": REALTIME_SAMPLE_RATE_HZ,
                        },
                        "turn_detection": {
                            "type": "server_vad",
                            "threshold": 0.5,
                            "prefix_padding_ms": 300,
                            "silence_duration_ms": 500,
                            "create_response": true,
                            "interrupt_response": true,
                        }
                    },
                    "output": {
                        "format": {
                            "type": "audio/pcm",
                        },
                        "voice": cfg.voice,
                    }
                }
            }
        });
        write
            .send(TgMessage::Text(session_update.to_string()))
            .await
            .context("发送 Realtime Chat session.update 失败")?;

        let (tx_audio, mut rx_audio) = mpsc::unbounded_channel::<UplinkCommand>();
        let (tx_event, event_rx) = mpsc::unbounded_channel::<RealtimeChatEvent>();

        tokio::spawn(async move {
            while let Some(cmd) = rx_audio.recv().await {
                let msg = match cmd {
                    UplinkCommand::Audio(pcm) => TgMessage::Text(
                        json!({
                            "type": "input_audio_buffer.append",
                            "audio": general_purpose::STANDARD.encode(&pcm),
                        })
                        .to_string(),
                    ),
                    UplinkCommand::Commit => {
                        TgMessage::Text(json!({"type": "input_audio_buffer.commit"}).to_string())
                    }
                    UplinkCommand::Close => {
                        let _ = write.send(TgMessage::Close(None)).await;
                        break;
                    }
                };
                if write.send(msg).await.is_err() {
                    break;
                }
            }
        });

        let tx_event_recv = tx_event.clone();
        tokio::spawn(async move {
            while let Some(item) = read.next().await {
                let Ok(TgMessage::Text(text)) = item else {
                    continue;
                };
                let Ok(value) = serde_json::from_str::<Value>(&text) else {
                    continue;
                };
                if let Some(event) = parse_realtime_event(&value) {
                    let _ = tx_event_recv.send(event);
                }
            }
            let _ = tx_event_recv.send(RealtimeChatEvent::Closed);
        });

        Ok(Self { tx_audio, event_rx })
    }

    pub fn append_pcm(&self, pcm: Vec<u8>) -> Result<()> {
        self.tx_audio
            .send(UplinkCommand::Audio(pcm))
            .map_err(|_| anyhow!("Realtime Chat 上行通道已关闭"))
    }

    pub fn commit(&self) -> Result<()> {
        self.tx_audio
            .send(UplinkCommand::Commit)
            .map_err(|_| anyhow!("Realtime Chat 上行通道已关闭"))
    }

    pub fn close(&self) {
        let _ = self.tx_audio.send(UplinkCommand::Close);
    }
}

fn parse_realtime_event(value: &Value) -> Option<RealtimeChatEvent> {
    let kind = value.get("type").and_then(Value::as_str).unwrap_or("");
    match kind {
        "session.updated" => Some(RealtimeChatEvent::SessionUpdated),
        "input_audio_buffer.speech_started" => Some(RealtimeChatEvent::UserSpeechStarted),
        "input_audio_buffer.speech_stopped" => Some(RealtimeChatEvent::UserSpeechStopped),
        "conversation.item.input_audio_transcription.delta" => value
            .get("delta")
            .and_then(Value::as_str)
            .map(|text| RealtimeChatEvent::UserTranscriptDelta(text.to_string())),
        "conversation.item.input_audio_transcription.completed" => value
            .get("transcript")
            .and_then(Value::as_str)
            .map(|text| RealtimeChatEvent::UserTranscriptFinal(text.to_string())),
        "response.output_audio.delta" | "response.audio.delta" => value
            .get("delta")
            .and_then(Value::as_str)
            .and_then(|delta| general_purpose::STANDARD.decode(delta).ok())
            .map(RealtimeChatEvent::AudioDelta),
        "response.output_audio.done" | "response.audio.done" => Some(RealtimeChatEvent::AudioDone),
        "response.output_audio_transcript.delta" | "response.audio_transcript.delta" => value
            .get("delta")
            .and_then(Value::as_str)
            .map(|text| RealtimeChatEvent::AiTranscriptDelta(text.to_string())),
        "response.output_audio_transcript.done" | "response.audio_transcript.done" => {
            let text = value
                .get("transcript")
                .or_else(|| value.get("text"))
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            Some(RealtimeChatEvent::AiTranscriptDone(text))
        }
        "response.output_text.delta" => value
            .get("delta")
            .and_then(Value::as_str)
            .map(|text| RealtimeChatEvent::AiTranscriptDelta(text.to_string())),
        "response.output_text.done" => value
            .get("text")
            .and_then(Value::as_str)
            .map(|text| RealtimeChatEvent::AiTranscriptDone(text.to_string())),
        "response.done" => Some(RealtimeChatEvent::ResponseDone),
        "error" => {
            let message = value
                .get("error")
                .and_then(|e| e.get("message"))
                .or_else(|| value.get("message"))
                .and_then(Value::as_str)
                .unwrap_or("Realtime Chat unknown error")
                .to_string();
            Some(RealtimeChatEvent::Error(message))
        }
        _ => None,
    }
}
