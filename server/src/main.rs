use anyhow::Result;
use dotenvy::dotenv;
use std::{net::SocketAddr, sync::Arc};
use tracing::info;

mod admin;
mod agent;
mod ai_cli;
mod ai_cli_prompts;
mod api;
mod app_update;
mod codex_health;
mod codex_stream;
mod homecli_agent;
mod image_generation;
mod intent_router;
mod peer_relay;
mod project_api;
mod project_attachments;
mod project_auth;
mod project_chat_reply;
mod project_conversation_workspace;
mod project_git;
mod project_keys;
mod project_mobile;
mod project_ws_protocol;
mod router;
mod server_trace;
mod store;
mod store_schema;
mod tools;
mod types;
mod user_api;
mod web;

pub use types::AppState;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info,elon_server=debug".into()),
        )
        .init();

    let state = Arc::new(AppState::new()?);
    codex_health::spawn_codex_network_monitor(state.clone());

    // 服务启动时：将上次运行中的任务标记为已中断
    let interrupted = state.store.mark_interrupted_running_ws_tasks().unwrap_or(0);
    if interrupted > 0 {
        info!("{} 个进行中的任务因服务器重启被标记为已中断", interrupted);
    }
    let interrupted_tasks = state.store.mark_interrupted_running_tasks().unwrap_or(0);
    if interrupted_tasks > 0 {
        info!(
            "{} 个数据库运行中任务因服务器重启被标记为已中断",
            interrupted_tasks
        );
    }

    let app = router::build_app(state);

    let addr: SocketAddr = std::env::var("LISTEN_ADDR")
        .unwrap_or_else(|_| "0.0.0.0:8080".into())
        .parse()?;

    info!("elon server listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
