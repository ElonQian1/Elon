//! Shared AI agent fallback helpers.
//!
//! Default-user-facing chat should not fail just because the current default
//! provider is out of quota or temporarily unavailable. These helpers retry
//! other server-side API agents only for provider/interface errors; user
//! billing and account errors still stop immediately.

use anyhow::{anyhow, Result};
use serde_json::Value;
use std::sync::Arc;
use tracing::warn;

use crate::{
    agent_llm_call::call_chat_llm_with_options,
    types::{AgentConfig, AgentsConfig, AppState},
};

pub(crate) async fn call_chat_llm_with_default_fallback_options(
    state: &Arc<AppState>,
    preferred: &AgentConfig,
    allow_fallback: bool,
    messages: &[Value],
    user_id: &str,
    feature: &str,
    temperature: f64,
    max_tokens: usize,
) -> Result<(Value, AgentConfig, bool)> {
    let mut agents = vec![preferred.clone()];
    if allow_fallback {
        for agent in server_api_agents_in_fallback_order(state).await {
            if agent.name != preferred.name {
                agents.push(agent);
            }
        }
    }

    let mut last_retryable_error = None;
    for (index, agent) in agents.iter().enumerate() {
        match call_chat_llm_with_options(
            state,
            agent,
            messages,
            user_id,
            feature,
            temperature,
            max_tokens,
        )
        .await
        {
            Ok(response) => return Ok((response, agent.clone(), index > 0)),
            Err(error) => {
                let message = error.to_string();
                let has_next = index + 1 < agents.len();
                if !is_retryable_agent_error(&message) || !has_next {
                    return Err(anyhow!(message));
                }
                warn!(
                    feature,
                    agent = %agent.name,
                    model = %agent.model,
                    "AI agent failed, trying fallback: {}",
                    message
                );
                last_retryable_error = Some(message);
            }
        }
    }

    Err(anyhow!(
        "{}",
        last_retryable_error.unwrap_or_else(|| "未配置可用 AI 代理".to_string())
    ))
}

pub(crate) async fn server_api_agents_in_fallback_order(state: &Arc<AppState>) -> Vec<AgentConfig> {
    let config = state.agents_config.read().await;
    ordered_server_api_agents(&config)
}

pub(crate) fn ordered_server_api_agents(config: &AgentsConfig) -> Vec<AgentConfig> {
    let mut result = Vec::new();
    if let Some(default) = config
        .agents
        .get(&config.default_agent)
        .filter(|agent| server_api_agent_is_eligible(agent))
    {
        result.push(default.clone());
    }

    let mut names = config.agents.keys().cloned().collect::<Vec<_>>();
    names.sort();
    for name in names {
        if name == config.default_agent {
            continue;
        }
        if let Some(agent) = config
            .agents
            .get(&name)
            .filter(|agent| server_api_agent_is_eligible(agent))
        {
            result.push(agent.clone());
        }
    }
    result
}

fn server_api_agent_is_eligible(agent: &AgentConfig) -> bool {
    if agent.usage_mode() != "server_api_key" {
        return false;
    }
    let name = agent.name.to_ascii_lowercase();
    let base = agent.api_base.to_ascii_lowercase();
    !name.starts_with("copilot:") && !base.contains("api.githubcopilot.com")
}

pub(crate) fn is_retryable_agent_error(message: &str) -> bool {
    let lower = message.to_ascii_lowercase();
    if lower.contains("余额不足")
        || lower.contains("用户已被封禁")
        || lower.contains("token 用量已达上限")
        || lower.contains("计费系统暂时不可用")
    {
        return false;
    }

    lower.contains("当前 ai 模型额度已用尽")
        || lower.contains("当前 ai 模型密钥无效")
        || lower.contains("当前 ai 模型请求过于频繁")
        || lower.contains("ai 服务暂时不可用")
        || lower.contains("ai 服务返回错误")
        || lower.contains("ai 请求失败")
        || lower.contains("ai 请求超时")
        || lower.contains("endpoint is inactive")
        || lower.contains("free_quota_exhausted")
        || lower.contains("payment required")
        || lower.contains("rate limit")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn agent(name: &str, model: &str) -> AgentConfig {
        AgentConfig {
            name: name.to_string(),
            api_base: "https://example.invalid/v1".to_string(),
            api_key: "test-key".to_string(),
            model: model.to_string(),
            embedding_model: None,
            usage_mode: None,
        }
    }

    fn agent_with_usage(name: &str, model: &str, usage_mode: Option<&str>) -> AgentConfig {
        AgentConfig {
            usage_mode: usage_mode.map(str::to_string),
            ..agent(name, model)
        }
    }

    #[test]
    fn orders_default_agent_first_then_stable_names() {
        let mut agents = HashMap::new();
        agents.insert("zeta".to_string(), agent("zeta", "z"));
        agents.insert("default".to_string(), agent("default", "d"));
        agents.insert("alpha".to_string(), agent("alpha", "a"));
        let config = AgentsConfig {
            agents,
            default_agent: "default".to_string(),
        };

        let names = ordered_server_api_agents(&config)
            .into_iter()
            .map(|agent| agent.name)
            .collect::<Vec<_>>();
        assert_eq!(names, ["default", "alpha", "zeta"]);
    }

    #[test]
    fn orders_only_server_side_agents() {
        let mut agents = HashMap::new();
        agents.insert("default".to_string(), agent("default", "d"));
        agents.insert(
            "copilot:gpt-4o".to_string(),
            agent("copilot:gpt-4o", "gpt-4o"),
        );
        agents.insert(
            "user-proxy".to_string(),
            agent_with_usage("user-proxy", "u", Some("user_api_key_proxy")),
        );
        agents.insert("server-alt".to_string(), agent("server-alt", "s"));
        let config = AgentsConfig {
            agents,
            default_agent: "default".to_string(),
        };

        let names = ordered_server_api_agents(&config)
            .into_iter()
            .map(|agent| agent.name)
            .collect::<Vec<_>>();
        assert_eq!(names, ["default", "server-alt"]);
    }

    #[test]
    fn retryable_errors_exclude_user_billing_failures() {
        assert!(is_retryable_agent_error(
            "当前 AI 模型额度已用尽或接口不可用，请切换可用模型"
        ));
        assert!(is_retryable_agent_error("AI 请求超时，请稍后重试"));
        assert!(!is_retryable_agent_error(
            "余额不足（当前 0 分），请联系管理员充值后继续使用"
        ));
        assert!(!is_retryable_agent_error("用户已被封禁"));
    }
}
