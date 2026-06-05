//! 方案 B：连接 OpenAI Realtime Transcription WebSocket，把 PCM 转成文字。
//!
//! 协议参考：https://platform.openai.com/docs/guides/realtime-transcription
//! - 上行：`input_audio_buffer.append`（base64 PCM）+ `input_audio_buffer.commit`
//! - 下行：`conversation.item.input_audio_transcription.delta` / `.completed`
//!
//! 本模块只暴露一个 [`RealtimeTranscriber`]，不直接与 axum WebSocket 耦合，便于复用。

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

use crate::voice_config::RealtimeTranscribeConfig;

/// 服务端向调用方推送的转写事件。
#[derive(Debug, Clone)]
pub enum TranscriptEvent {
    Delta(String),
    Final(String),
    Error(String),
    Closed,
}

/// 一个活跃的 OpenAI Realtime 转写会话。
///
/// 调用方：
/// - 通过 [`append_pcm`] 不断推 PCM 数据
/// - 通过 [`commit`] 触发一次完整转写
/// - 通过 `event_rx` 接收 delta/final/error
pub struct RealtimeTranscriber {
    tx_audio: mpsc::UnboundedSender<UplinkCommand>,
    pub event_rx: mpsc::UnboundedReceiver<TranscriptEvent>,
}

enum UplinkCommand {
    Audio(Vec<u8>),
    Commit,
    Close,
}

impl RealtimeTranscriber {
    pub async fn connect(cfg: &RealtimeTranscribeConfig) -> Result<Self> {
        let api_key = cfg
            .read_api_key()
            .ok_or_else(|| anyhow!("缺少环境变量 {}", cfg.api_key_env))?;

        let mut request = cfg
            .ws_url
            .as_str()
            .into_client_request()
            .context("构造 Realtime WebSocket 请求失败")?;
        request.headers_mut().insert(
            "Authorization",
            HeaderValue::from_str(&format!("Bearer {api_key}"))?,
        );
        request
            .headers_mut()
            .insert("OpenAI-Beta", HeaderValue::from_static("realtime=v1"));

        let (ws, _resp) = connect_async(request)
            .await
            .context("连接 OpenAI Realtime WebSocket 失败")?;
        let (mut write, mut read) = ws.split();

        // 发送 transcription session 配置
        let session_update = json!({
            "type": "transcription_session.update",
            "session": {
                "input_audio_format": "pcm16",
                "input_audio_transcription": {
                    "model": cfg.model,
                    "language": cfg.language,
                },
                "turn_detection": null,
            }
        });
        write
            .send(TgMessage::Text(session_update.to_string()))
            .await
            .context("发送 session.update 失败")?;

        let (tx_audio, mut rx_audio) = mpsc::unbounded_channel::<UplinkCommand>();
        let (tx_event, event_rx) = mpsc::unbounded_channel::<TranscriptEvent>();

        // 上行任务
        tokio::spawn(async move {
            while let Some(cmd) = rx_audio.recv().await {
                let msg = match cmd {
                    UplinkCommand::Audio(pcm) => {
                        let b64 = general_purpose::STANDARD.encode(&pcm);
                        TgMessage::Text(
                            json!({
                                "type": "input_audio_buffer.append",
                                "audio": b64,
                            })
                            .to_string(),
                        )
                    }
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

        // 下行任务：解析转写事件
        let tx_event_recv = tx_event.clone();
        tokio::spawn(async move {
            while let Some(item) = read.next().await {
                let Ok(TgMessage::Text(text)) = item else {
                    continue;
                };
                let Ok(v) = serde_json::from_str::<Value>(&text) else {
                    continue;
                };
                let kind = v.get("type").and_then(Value::as_str).unwrap_or("");
                match kind {
                    k if k.ends_with("transcription.delta") => {
                        if let Some(delta) = v.get("delta").and_then(Value::as_str) {
                            let _ = tx_event_recv.send(TranscriptEvent::Delta(delta.to_string()));
                        }
                    }
                    k if k.ends_with("transcription.completed") => {
                        if let Some(text) = v.get("transcript").and_then(Value::as_str) {
                            let _ = tx_event_recv.send(TranscriptEvent::Final(text.to_string()));
                        }
                    }
                    "error" => {
                        let msg = v
                            .get("error")
                            .and_then(|e| e.get("message"))
                            .and_then(Value::as_str)
                            .unwrap_or("unknown")
                            .to_string();
                        let _ = tx_event_recv.send(TranscriptEvent::Error(msg));
                    }
                    _ => {}
                }
            }
            let _ = tx_event_recv.send(TranscriptEvent::Closed);
        });

        Ok(Self { tx_audio, event_rx })
    }

    pub fn append_pcm(&self, pcm: Vec<u8>) -> Result<()> {
        self.tx_audio
            .send(UplinkCommand::Audio(pcm))
            .map_err(|_| anyhow!("Realtime 上行通道已关闭"))
    }

    pub fn commit(&self) -> Result<()> {
        self.tx_audio
            .send(UplinkCommand::Commit)
            .map_err(|_| anyhow!("Realtime 上行通道已关闭"))
    }

    pub fn close(&self) {
        let _ = self.tx_audio.send(UplinkCommand::Close);
    }
}
