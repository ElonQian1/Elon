use anyhow::{anyhow, Result};
use serde_json::{json, Value};
use std::sync::Arc;

use crate::types::{AgentConfig, AppState};
use crate::agent_fallback::{is_retryable_agent_error, server_api_agents_in_fallback_order};
use crate::agent_llm_call::friendly_ai_api_error;

use super::helpers::extract_content;
use super::{MAX_OUTPUT_TOKENS};

pub(super) async fn call_route_c_model(
    state: &Arc<AppState>,
    requested_agent: Option<&str>,
    messages: &[Value],
) -> Result<(String, String, String, bool)> {
    let agents = candidate_agents(state, requested_agent).await?;
    let mut last_retryable_error = None;
    for (index, agent) in agents.iter().enumerate() {
        match send_chat_completion(state, agent, messages).await {
            Ok(response) => {
                let content = extract_content(&response).unwrap_or_else(|| {
                    "{\"reply\":\"我已收到问题，但模型没有返回可展示的文本。\",\"done\":true,\"actions\":[]}"
                        .to_string()
                });
                return Ok((content, agent.name.clone(), agent.model.clone(), index > 0));
            }
            Err(error) => {
                let message = error.to_string();
                let has_next = index + 1 < agents.len();
                if !is_retryable_agent_error(&message) || !has_next {
                    return Err(anyhow!(message));
                }
                last_retryable_error = Some(message);
            }
        }
    }
    Err(anyhow!(
        "{}",
        last_retryable_error.unwrap_or_else(|| "未配置可用 server_api_key AI 代理".to_string())
    ))
}

pub(super) async fn candidate_agents(
    state: &Arc<AppState>,
    requested_agent: Option<&str>,
) -> Result<Vec<AgentConfig>> {
    let agents = server_api_agents_in_fallback_order(state).await;
    if agents.is_empty() {
        return Err(anyhow!("未配置可用 server_api_key AI 代理"));
    }
    let Some(requested) = requested_agent
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return Ok(agents);
    };
    let Some(index) = agents
        .iter()
        .position(|agent| agent.name.eq_ignore_ascii_case(requested))
    else {
        return Err(anyhow!("请求的 AI 代理未开放给 Route C SDK MVP"));
    };
    let mut ordered = vec![agents[index].clone()];
    ordered.extend(agents.into_iter().enumerate().filter_map(|(i, agent)| {
        if i == index {
            None
        } else {
            Some(agent)
        }
    }));
    Ok(ordered)
}

pub(super) async fn send_chat_completion(
    state: &Arc<AppState>,
    agent: &AgentConfig,
    messages: &[Value],
) -> Result<Value> {
    let url = format!("{}/chat/completions", agent.api_base.trim_end_matches('/'));
    let body = json!({
        "model": agent.model,
        "messages": messages,
        "stream": false,
        "temperature": 0.2,
        "max_tokens": MAX_OUTPUT_TOKENS,
    });
    let response = state
        .http_client
        .post(url)
        .bearer_auth(&agent.api_key)
        .json(&body)
        .send()
        .await
        .map_err(|error| {
            if error.is_timeout() {
                anyhow!("AI 请求超时，请稍后重试")
            } else {
                anyhow!("AI 请求失败: {error}")
            }
        })?;
    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        return Err(anyhow!("{}", friendly_ai_api_error(status, &text)));
    }
    Ok(response.json::<Value>().await?)
}
