//! social_ai_agents.rs - social chat model selection and fallback.
//!
//! 群聊 AI 是面向用户的实时能力，不能因为默认 provider 单点不可用就直接失败。
//! 这里仅在模型供应/接口类错误时切换备用代理；用户余额、封禁、计费系统错误仍直接返回。

use anyhow::{anyhow, Result};
use serde_json::Value;
use std::sync::Arc;
use tracing::warn;

use crate::{
    agent_fallback::{
        call_chat_llm_with_default_fallback_options, server_api_agents_in_fallback_order,
    },
    types::{AgentConfig, AppState},
};

pub(crate) async fn resolve_social_agent(state: &Arc<AppState>) -> Result<AgentConfig> {
    social_agents_in_fallback_order(state)
        .await
        .into_iter()
        .next()
        .ok_or_else(|| anyhow!("未配置可用 AI 代理，请先在后台配置 API 代理"))
}

pub(crate) async fn call_social_chat_llm_with_fallback(
    state: &Arc<AppState>,
    messages: &[Value],
    user_id: &str,
    feature: &str,
) -> Result<Value> {
    call_social_chat_llm_with_fallback_options(state, messages, user_id, feature, 0.8, 700).await
}

pub(crate) async fn call_social_chat_llm_with_fallback_options(
    state: &Arc<AppState>,
    messages: &[Value],
    user_id: &str,
    feature: &str,
    temperature: f64,
    max_tokens: usize,
) -> Result<Value> {
    let agent = resolve_social_agent(state).await?;
    let (response, used_agent, used_fallback) = call_chat_llm_with_default_fallback_options(
        state,
        &agent,
        true,
        messages,
        user_id,
        feature,
        temperature,
        max_tokens,
    )
    .await?;
    if used_fallback {
        warn!(
            feature,
            preferred_agent = %agent.name,
            used_agent = %used_agent.name,
            model = %used_agent.model,
            "social AI agent failed, switched to fallback"
        );
    }
    Ok(response)
}

async fn social_agents_in_fallback_order(state: &Arc<AppState>) -> Vec<AgentConfig> {
    server_api_agents_in_fallback_order(state).await
}
