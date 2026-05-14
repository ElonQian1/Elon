use anyhow::Result;
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use dotenvy::dotenv;
use std::{net::SocketAddr, sync::Arc};
use tokio::sync::RwLock;
use tower_http::cors::{Any, CorsLayer};
use tracing::info;

mod agent;
mod api;
mod tools;
mod types;

pub use types::AppState;

#[tokio::main]
async fn main() -> Result<()> {
    // 加载 .env 文件
    dotenv().ok();

    // 初始化日志
    tracing_subscriber::fmt()
        .with_env_filter(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info,elon_server=debug".into()),
        )
        .init();

    let state = Arc::new(AppState::new()?);

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        // 健康检查
        .route("/health", get(api::health))
        // WebSocket 实时对话（APK 主要使用这个）
        .route("/ws", get(ws_handler))
        // REST API（备用）
        .route("/api/chat", post(api::chat))
        .layer(cors)
        .with_state(state);

    let addr: SocketAddr = std::env::var("LISTEN_ADDR")
        .unwrap_or_else(|_| "0.0.0.0:8080".into())
        .parse()?;

    info!("一龙服务器启动，监听 {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_ws(socket, state))
}

async fn handle_ws(socket: WebSocket, state: Arc<AppState>) {
    use futures::{SinkExt, StreamExt};
    let (mut sender, mut receiver) = socket.split();

    info!("新的 WebSocket 连接");

    while let Some(Ok(msg)) = receiver.next().await {
        if let Message::Text(text) = msg {
            let state_clone = state.clone();

            // 逐步把 AI 代理的进度推送回客户端
            let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<String>();

            tokio::spawn(async move {
                // 消息格式支持两种：
                // 1. 纯文本：直接传给默认代理
                // 2. JSON: { "user_id": "alice", "message": "...", "agent": "deepseek" }
                let (user_id, content, agent_name) = parse_ws_message(&text);
                agent::run(&user_id, &content, agent_name.as_deref(), &state_clone, tx).await;
            });

            while let Some(progress) = rx.recv().await {
                if sender.send(Message::Text(progress)).await.is_err() {
                    break;
                }
            }
        }
    }

    info!("WebSocket 连接断开");
}

/// 解析 WebSocket 消息，返回 (user_id, content, agent_name)
fn parse_ws_message(raw: &str) -> (String, String, Option<String>) {
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(raw) {
        let user_id = v["user_id"].as_str().unwrap_or("default").to_string();
        let content = v["message"].as_str().unwrap_or(raw).to_string();
        let agent   = v["agent"].as_str().map(|s| s.to_lowercase());
        (user_id, content, agent)
    } else {
        ("default".to_string(), raw.to_string(), None)
    }
}
