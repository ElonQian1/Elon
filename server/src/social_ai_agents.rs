//! social_ai_agents.rs - social chat model selection and fallback.
//!
//! 群聊 AI 是面向用户的实时能力，不能因为默认 provider 单点不可用就直接失败。
//! 这里仅在模型供应/接口类错误时切换备用代理；用户余额、封禁、计费系统错误仍直接返回。

use anyhow::{anyhow, Result};
use serde_json::Value;
use std::sync::Arc;
use tracing::warn;

use crate::store::social_ai_messages::requests::work::GroupWorkAiOptions;
use crate::{
    agent_fallback::{
        call_chat_llm_with_default_fallback_options, server_api_agents_in_fallback_order,
    },
    types::{AgentConfig, AppState},
};

pub(crate) async fn resolve_group_work_agent(
    state: &Arc<AppState>,
    options: &GroupWorkAiOptions,
) -> Result<AgentConfig> {
    options.validate()?;
    choose_group_work_agent(social_agents_in_fallback_order(state).await, options)
}

fn choose_group_work_agent(
    agents: Vec<AgentConfig>,
    options: &GroupWorkAiOptions,
) -> Result<AgentConfig> {
    agents
        .into_iter()
        .find(|agent| {
            options
                .agent
                .as_ref()
                .map_or(true, |name| name == &agent.name)
        })
        .ok_or_else(|| anyhow!("所选群聊模型暂不可用，请重新选择；尚未发送 AI 请求"))
}

pub(crate) async fn call_group_configured_llm(
    state: &Arc<AppState>,
    messages: &[Value],
    user_id: &str,
    feature: &str,
    options: Option<&GroupWorkAiOptions>,
) -> Result<Value> {
    let Some(options) = options else {
        return call_social_chat_llm_with_fallback(state, messages, user_id, feature).await;
    };
    let agent = resolve_group_work_agent(state, options).await?;
    let (response, _, _) = call_chat_llm_with_default_fallback_options(
        state,
        &agent,
        options.allow_fallback,
        messages,
        user_id,
        feature,
        0.8,
        700,
    )
    .await?;
    Ok(response)
}

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

#[cfg(test)]
mod group_tests {
    use super::*;
    fn agent(name: &str) -> AgentConfig {
        AgentConfig {
            name: name.into(),
            api_base: "https://example.invalid/v1".into(),
            api_key: String::new(),
            model: format!("model-{name}"),
            embedding_model: None,
            usage_mode: None,
        }
    }
    #[test]
    fn configured_group_agent_is_used_instead_of_first_default() {
        let options = GroupWorkAiOptions {
            agent: Some("b".into()),
            allow_fallback: false,
        };
        let chosen = choose_group_work_agent(vec![agent("a"), agent("b")], &options).unwrap();
        assert_eq!(chosen.model, "model-b");
        assert!(!options.allow_fallback);
    }
    #[test]
    fn missing_explicit_agent_does_not_silently_select_default() {
        let options = GroupWorkAiOptions {
            agent: Some("removed".into()),
            allow_fallback: true,
        };
        assert!(choose_group_work_agent(vec![agent("a")], &options).is_err());
        assert_eq!(
            choose_group_work_agent(vec![agent("a")], &GroupWorkAiOptions::default())
                .unwrap()
                .name,
            "a"
        );
    }
    #[test]
    fn group_options_cannot_inject_a_provider_url_or_credential() {
        assert!(serde_json::from_value::<GroupWorkAiOptions>(
            serde_json::json!({"agent":"a","api_base":"https://example.invalid"})
        )
        .is_err());
    }
}
