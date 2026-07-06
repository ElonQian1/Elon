//! 本机 TTS 合成（代理到本地 model-tts-worker）。
//! 从 node_agent_main.rs 拆分，保持行为不变。

use std::time::Duration;

use base64::Engine as _;
use homecli_proto::AgentToServer;

pub async fn run_tts_synthesis(
    req_id: String,
    worker_base_url: String,
    text: String,
    voice_id: Option<String>,
    emotion_id: Option<String>,
    intensity: Option<String>,
    provider: Option<String>,
) -> AgentToServer {
    let url = format!("{}/synthesize", worker_base_url.trim_end_matches('/'));
    let mut body = serde_json::json!({
        "text": text,
        "cacheVersion": "pc_relay_v1",
    });
    if let Some(v) = &voice_id {
        body["voiceId"] = serde_json::json!(v);
    }
    if let Some(e) = &emotion_id {
        body["emotionId"] = serde_json::json!(e);
    }
    if let Some(i) = &intensity {
        body["intensity"] = serde_json::json!(i);
    }
    if let Some(p) = &provider {
        body["provider"] = serde_json::json!(p);
    }

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(180))
        .build()
        .unwrap_or_default();

    let resp = match client.post(&url).json(&body).send().await {
        Ok(r) => r,
        Err(e) => {
            return AgentToServer::TtsSynthesizeError {
                req_id,
                message: format!("本机 TTS Worker 请求失败: {e}"),
            }
        }
    };
    if !resp.status().is_success() {
        let status = resp.status().as_u16();
        let msg = resp.text().await.unwrap_or_default();
        return AgentToServer::TtsSynthesizeError {
            req_id,
            message: format!("TTS Worker 返回 {status}: {msg}"),
        };
    }
    let content_type = resp
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("audio/wav")
        .to_string();
    let worker_voice = resp
        .headers()
        .get("x-elon-tts-worker-voice")
        .and_then(|v| v.to_str().ok())
        .map(str::to_string);

    if content_type.starts_with("application/json") {
        match resp.json::<serde_json::Value>().await {
            Ok(j) => {
                let b64 = j
                    .get("audioBase64")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let mime = j
                    .get("mime")
                    .and_then(|v| v.as_str())
                    .unwrap_or("audio/wav")
                    .to_string();
                AgentToServer::TtsSynthesizeResponse {
                    req_id,
                    audio_b64: b64,
                    mime,
                    worker_voice,
                }
            }
            Err(e) => AgentToServer::TtsSynthesizeError {
                req_id,
                message: format!("JSON 解析失败: {e}"),
            },
        }
    } else {
        match resp.bytes().await {
            Ok(bytes) => {
                let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
                AgentToServer::TtsSynthesizeResponse {
                    req_id,
                    audio_b64: b64,
                    mime: content_type,
                    worker_voice,
                }
            }
            Err(e) => AgentToServer::TtsSynthesizeError {
                req_id,
                message: format!("读取音频失败: {e}"),
            },
        }
    }
}


