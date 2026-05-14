use axum::{extract::State, Json};
use std::sync::Arc;
use crate::types::{AppState, WsMessage};
use tokio::sync::mpsc::UnboundedSender;

/// 健康检查
pub async fn health() -> &'static str {
    "OK"
}

/// REST 接口（APK 也可以用这个，不需要 WebSocket）
#[derive(serde::Deserialize)]
pub struct ChatRequest {
    pub message: String,
}

#[derive(serde::Serialize)]
pub struct ChatResponse {
    pub reply: String,
    pub apk_url: Option<String>,
}

pub async fn chat(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ChatRequest>,
) -> Json<ChatResponse> {
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<String>();

    let state_clone = state.clone();
    let msg = req.message.clone();

    tokio::spawn(async move {
        crate::agent::run(&msg, &state_clone, tx).await;
    });

    let mut final_reply = String::new();
    let mut apk_url = None;

    while let Some(raw) = rx.recv().await {
        if let Ok(ws_msg) = serde_json::from_str::<serde_json::Value>(&raw) {
            match ws_msg.get("type").and_then(|t| t.as_str()) {
                Some("done") => {
                    final_reply = ws_msg["message"].as_str().unwrap_or("完成").to_string();
                    apk_url = ws_msg["apk_url"].as_str().map(|s| s.to_string());
                    break;
                }
                Some("error") => {
                    final_reply = ws_msg["message"].as_str().unwrap_or("发生错误").to_string();
                    break;
                }
                _ => {}
            }
        }
    }

    Json(ChatResponse { reply: final_reply, apk_url })
}
