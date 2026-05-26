use anyhow::Result;
use dotenvy::dotenv;
use std::{net::SocketAddr, sync::Arc};
use tracing::info;

mod admin;
mod admin_html;
mod agent;
mod agent_api_loop;
mod agent_intent;
mod agent_llm_call;
mod agent_prompts;
mod agent_routing;
mod ai_cli;
mod ai_cli_chat;
mod ai_cli_environment;
mod ai_cli_intent_gate;
mod ai_cli_native_session;
mod ai_cli_output;
mod ai_cli_process;
mod ai_cli_prewarm;
mod ai_cli_prompts;
mod ai_cli_runner;
mod ai_cli_streaming;
mod ai_cli_trace;
mod api;
mod app_update;
mod cli_config;
mod codex_health;
mod codex_stream;
mod global_ws;
mod homecli_agent;
mod image_generation;
mod intent_router;
mod peer_relay;
mod project_api;
mod project_attachment_notes;
mod project_attachment_paths;
mod project_attachments;
mod project_auth;
mod project_chat;
mod project_chat_executor;
mod project_chat_reply;
mod project_completion;
mod project_conversation_workspace;
mod project_downloads;
mod project_git;
mod project_keys;
mod project_membership;
mod project_mobile;
mod project_prewarm;
mod project_store;
mod project_task_scheduler;
mod project_trace_events;
mod project_ws_job;
mod project_ws_protocol;
mod release_claim;
mod router;
mod server_trace;
mod source_hygiene;
mod store;
mod store_schema;
mod tools;
mod tools_apk;
mod tools_git;
mod types;
mod user_api;
mod web;
mod ws_message;

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
