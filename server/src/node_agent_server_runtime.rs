// server/src/node_agent_server_runtime.rs

use crate::{
    node_agent_runtime_approval::{
        requires_tool_approval, wait_for_tool_approval, ApprovalOutcome,
    },
    node_agent_runtime_events::{
        runtime_status_chunk, tool_approval_checkpoint, tool_approval_decision_chunk,
        tool_approval_id, tool_approval_required_chunk_with_diff_and_checkpoint, tool_call_chunk,
        tool_name, tool_result_chunk,
    },
    node_agent_task_journal::TaskJournal,
    node_agent_tool_approval::ToolApprovalState,
    node_agent_tool_guard::ToolGuard,
};
use anyhow::{anyhow, Context, Result};
use serde_json::{json, Value};
use std::{future::Future, time::Duration};
use tokio::sync::{mpsc, watch};
use tokio_tungstenite::tungstenite::Message;

const MAX_TOOL_RESULT_CHARS: usize = 24_000;
const MAX_RUNTIME_HTTP_BODY_BYTES: usize = 1024 * 1024;

#[derive(Clone)]
pub(crate) struct ServerRuntimeConfig {
    pub server_url: String,
    pub user_token: Option<String>,
}

#[derive(Clone)]
pub(crate) struct ApiRuntimeConfig {
    pub api_base: String,
    pub api_key: String,
    pub model: String,
}

pub(crate) struct ServerRuntimeRunResult {
    pub exit_ok: bool,
    pub error: Option<String>,
    pub model: Option<String>,
    pub prompt_tokens: Option<u64>,
    pub completion_tokens: Option<u64>,
    pub total_tokens: Option<u64>,
}

pub(crate) struct RuntimePromptOptions<'a> {
    pub req_id: &'a str,
    pub cwd: Option<&'a str>,
    pub runtime_permission: Option<&'a str>,
    pub prompt: &'a str,
    pub approval_state: Option<ToolApprovalState>,
    pub cancel_rx: watch::Receiver<bool>,
    pub out_tx: mpsc::UnboundedSender<Message>,
    pub task_journal: Option<TaskJournal>,
}

pub(crate) async fn run_server_runtime_prompt(
    config: ServerRuntimeConfig,
    options: RuntimePromptOptions<'_>,
) -> ServerRuntimeRunResult {
    match run_server_runtime_inner(config, options).await {
        Ok(result) => result,
        Err(error) => ServerRuntimeRunResult {
            exit_ok: false,
            error: Some(error.to_string()),
            model: Some("server-runtime".to_string()),
            prompt_tokens: None,
            completion_tokens: None,
            total_tokens: None,
        },
    }
}

pub(crate) async fn run_api_runtime_prompt(
    options: RuntimePromptOptions<'_>,
) -> ServerRuntimeRunResult {
    let Some(config) = api_runtime_config_from_env() else {
        return ServerRuntimeRunResult {
            exit_ok: false,
            error: Some(
                "api-runtime 缺少本机 API key 或模型；请设置 ELON_AGENT_API_KEY/OPENAI_API_KEY 和 ELON_AGENT_MODEL/OPENAI_MODEL"
                    .to_string(),
            ),
            model: Some("api-runtime".to_string()),
            prompt_tokens: None,
            completion_tokens: None,
            total_tokens: None,
        };
    };
    match run_api_runtime_inner(config, options).await {
        Ok(result) => result,
        Err(error) => ServerRuntimeRunResult {
            exit_ok: false,
            error: Some(error.to_string()),
            model: Some("api-runtime".to_string()),
            prompt_tokens: None,
            completion_tokens: None,
            total_tokens: None,
        },
    }
}

pub(crate) fn api_runtime_config_from_env() -> Option<ApiRuntimeConfig> {
    api_runtime_config_from_lookup(|name| std::env::var(name).ok())
}

fn api_runtime_config_from_lookup(
    lookup: impl Fn(&str) -> Option<String>,
) -> Option<ApiRuntimeConfig> {
    let api_key = first_value(
        &lookup,
        &["ELON_AGENT_API_KEY", "OPENAI_API_KEY", "HUNYUAN_API_KEY"],
    )?;
    let api_base = first_value(
        &lookup,
        &[
            "ELON_AGENT_API_BASE",
            "OPENAI_API_BASE",
            "OPENAI_BASE_URL",
            "HUNYUAN_API_BASE",
        ],
    )
    .unwrap_or_else(|| "https://api.openai.com/v1".to_string());
    let model = first_value(
        &lookup,
        &["ELON_AGENT_MODEL", "OPENAI_MODEL", "HUNYUAN_MODEL"],
    )?;
    Some(ApiRuntimeConfig {
        api_base: api_base.trim_end_matches('/').to_string(),
        api_key,
        model,
    })
}

async fn run_server_runtime_inner(
    config: ServerRuntimeConfig,
    options: RuntimePromptOptions<'_>,
) -> Result<ServerRuntimeRunResult> {
    let RuntimePromptOptions {
        req_id,
        cwd,
        runtime_permission,
        prompt,
        approval_state,
        cancel_rx,
        out_tx,
        task_journal,
    } = options;
    let token = config
        .user_token
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| anyhow!("server-runtime 需要先在 Win 客户端登录账号"))?;
    let workspace = resolve_workspace(cwd)?;
    let guard = ToolGuard::new(workspace, runtime_permission);
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(150))
        .build()
        .unwrap_or_default();
    let server_url = config.server_url.clone();
    let token = token.to_string();
    run_runtime_loop(
        RuntimeLoopOptions {
            req_id,
            label: "server-runtime",
            guard,
            prompt,
            approval_state,
            cancel_rx,
            out_tx,
            task_journal,
            initial_model: Some("server-runtime".to_string()),
        },
        move |messages| {
            let client = client.clone();
            let server_url = server_url.clone();
            let token = token.clone();
            async move { call_server_runtime(&client, &server_url, &token, &messages).await }
        },
    )
    .await
}

async fn run_api_runtime_inner(
    config: ApiRuntimeConfig,
    options: RuntimePromptOptions<'_>,
) -> Result<ServerRuntimeRunResult> {
    let RuntimePromptOptions {
        req_id,
        cwd,
        runtime_permission,
        prompt,
        approval_state,
        cancel_rx,
        out_tx,
        task_journal,
    } = options;
    let workspace = resolve_workspace(cwd)?;
    let guard = ToolGuard::new(workspace, runtime_permission);
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(150))
        .no_proxy() // 绕过本机代理（代理可能停止或不稳定，混元 API 支持直连）
        .build()
        .unwrap_or_default();
    let initial_model = Some(config.model.clone());
    run_runtime_loop(
        RuntimeLoopOptions {
            req_id,
            label: "api-runtime",
            guard,
            prompt,
            approval_state,
            cancel_rx,
            out_tx,
            task_journal,
            initial_model,
        },
        move |messages| {
            let client = client.clone();
            let config = config.clone();
            async move { call_api_runtime(&client, &config, &messages).await }
        },
    )
    .await
}

mod runtime_loop;
use self::runtime_loop::{RuntimeLoopOptions, run_runtime_loop};

#[cfg(test)]
mod runtime_test;
pub(crate) mod utils;

use self::utils::*;
