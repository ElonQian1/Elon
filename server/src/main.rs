use anyhow::Result;
use axum::{
    body::Body,
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Path, State,
    },
    http::{header, StatusCode},
    response::IntoResponse,
    routing::{delete, get, post},
    Router,
};
use dotenvy::dotenv;
use std::{net::SocketAddr, sync::Arc};
use tower_http::cors::{Any, CorsLayer};
use tracing::info;

mod admin;
mod agent;
mod api;
mod tools;
mod types;
mod user_api;

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
        // APK 下载
        .route("/download/{user_id}/{filename}", get(download_apk))
        // 管理后台：web 页面
        .route("/admin", get(admin::admin_page))
        // 管理后台：REST API
        .route("/api/admin/agents", get(admin::list_agents).post(admin::upsert_agent))
        .route("/api/admin/agents/:name", delete(admin::delete_agent))
        .route("/api/admin/agents/:name/key", get(admin::get_agent_key))
        .route("/api/admin/default/:name", post(admin::set_default_agent))
        .route("/api/admin/users", get(admin::list_users))
        // 用户端：AI 代理配置（APK 使用，无需管理员权限）
        .route("/api/user/:user_id/agent", get(user_api::get_user_agent).put(user_api::set_user_agent))
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

/// APK 下载端点 — GET /download/:user_id/:filename
async fn download_apk(
    Path((user_id, filename)): Path<(String, String)>,
    State(state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    // 安全检查：文件名不允许路径穿越
    if filename.contains("..") || filename.contains('/') || filename.contains('\\') {
        return Err((StatusCode::BAD_REQUEST, "非法文件名".into()));
    }
    if !filename.ends_with(".apk") {
        return Err((StatusCode::BAD_REQUEST, "仅允许下载 APK 文件".into()));
    }

    let workspace = types::get_user_workspace(&state.workspace_root, &user_id);

    // 在常见的输出目录中查找 APK
    let candidates = [
        workspace.join("app/build/outputs/apk/debug").join(&filename),
        workspace.join("app/build/outputs/apk/release").join(&filename),
    ];

    let apk_path = candidates
        .iter()
        .find(|p| p.exists())
        .ok_or_else(|| (StatusCode::NOT_FOUND, format!("APK 文件 {} 不存在", filename)))?;

    let data = tokio::fs::read(apk_path).await.map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("读取文件失败: {}", e))
    })?;

    let response = axum::response::Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/vnd.android.package-archive")
        .header(
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{}\"", filename),
        )
        .body(Body::from(data))
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(response)
}
