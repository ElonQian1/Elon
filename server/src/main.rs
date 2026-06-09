use anyhow::Result;
use dotenvy::dotenv;
use std::{net::SocketAddr, sync::Arc};
use tracing::info;

mod admin;
mod admin_html;
mod admin_quota;
mod admin_token_stats;
mod agent;
mod agent_api_loop;
mod agent_balloon;
mod agent_config;
mod agent_intent;
mod agent_llm_call;
mod agent_prompts;
mod agent_routing;
mod ai_cli;
mod api;
mod app_update;
mod auth_api;
mod billing;
mod billing_admin;
mod billing_api;
mod billing_events;
mod billing_pay;
mod chat_attachments;
mod cli_config;
mod cli_usage;
mod codex_health;
mod codex_stream;
mod compute_usage;
mod errors;
mod friend_api;
mod friend_events;
mod global_ws;
mod homecli_agent;
mod image_generation;
mod intent_router;
mod join_request_events;
mod lan_peer;
mod lm_chat;
mod node_api;
mod node_registry;
mod node_router;
mod pc_relay;
mod pc_relay_client;
mod pc_workspace_provisioner;
mod peer_relay;
mod presence_events;
mod project_api;
mod project_attachment_notes;
mod project_attachment_paths;
mod project_attachments;
mod project_auth;
mod project_channel_summary;
mod project_chat;
mod project_chat_executor;
mod project_chat_reply;
mod project_completion;
mod project_conversation_identity;
mod project_conversation_workspace;
mod project_deletion;
mod project_downloads;
mod project_events;
mod project_execution_mode;
mod project_git;
mod project_join_requests;
mod project_keys;
mod project_membership;
mod project_mobile;
mod project_prewarm;
mod project_space;
mod project_store;
mod project_task_scheduler;
mod project_trace_events;
mod project_workspace_provision;
mod project_ws_job;
mod project_ws_protocol;
mod project_ws_session;
mod read_receipt_events;
mod release_claim;
mod release_manager;
mod router;
mod server_trace;
mod social_ai;
mod social_ai_message_reply;
mod source_hygiene;
mod speech_translate;
mod store;
mod store_migrations;
mod store_schema;
mod token_usage_api;
mod tools;
mod tools_apk;
mod tools_exec;
mod tools_git;
mod types;
mod typing_events;
mod user_api;
mod user_archive_api;
mod user_memory_api;
mod user_memory_extract;
mod voice_asr_upload;
mod voice_audio_format;
mod voice_config;
mod voice_openai_realtime;
mod voice_openai_realtime_chat;
mod voice_protocol;
mod voice_pwcat;
mod voice_to_cli;
mod voice_tts_api;
mod voice_tts_catalog;
mod voice_tts_rewrite;
mod voice_tts_worker;
mod voice_whisper_local;
mod voice_whisper_rest;
mod voice_ws_realtime_chat;
mod voice_ws_transcribe;
mod voice_ws_virtual_mic;
mod web;
mod wechat_pay;
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
    // 本地模式：作为 agent 连回云端，实现 APK→云端→PC 全双工中继
    pc_relay_client::spawn_if_configured();

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

    // 定期清理：running 超过 10 分钟的任务自动标记为 failed
    // 防止 PC 节点断线但任务因异常未收到 CliDone 而永久卡住
    {
        let state_cleanup = state.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(120));
            loop {
                interval.tick().await;
                match state_cleanup.store.mark_stale_running_tasks(10 * 60) {
                    Ok(n) if n > 0 => {
                        info!("{n} 个超时 running 任务已自动标记为 failed")
                    }
                    Ok(_) => {}
                    Err(e) => tracing::warn!("stale task cleanup error: {e}"),
                }
            }
        });
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
