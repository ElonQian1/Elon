//! LLM API 调用与工具执行（从 agent.rs 抽出）。
//!
//! 这里只关注：
//! - OpenAI 兼容 /chat/completions 接口的两种调用形态（带 tools / 普通对话）
//! - LLM 错误信息的中文化
//! - 工具名 → tools::* 的派发
//!
//! 让 agent.rs 只保留路由、Agent 选择和高层编排。

use anyhow::Result;
use serde_json::{json, Value};
use std::sync::Arc;

use crate::{
    agent_prompts::tool_definitions,
    tools,
    types::{AgentConfig, AppState},
};
/// 调用 LLM API（OpenAI 兼容接口，带工具定义）
///
/// `user_id`  触发本次调用的用户 ID，用于 token 用量统计。
/// `feature`  功能标签，例如 "agent_tool" / "chat"，用于用量分类。
pub(crate) async fn call_llm(
    state: &Arc<AppState>,
    agent: &AgentConfig,
    messages: &[Value],
    user_id: &str,
    feature: &str,
) -> Result<Value> {
    ensure_api_call_allowed(state, agent, user_id)?;
    let url = format!("{}/chat/completions", agent.api_base);

    let body = json!({
        "model": agent.model,
        "messages": messages,
        "tools": tool_definitions(),
        "tool_choice": "auto",
    });

    // GitHub Copilot 直连 API 需要额外的 editor 标识 header
    let is_copilot_direct = agent.api_base.contains("githubcopilot.com");
    let integration_id =
        std::env::var("COPILOT_INTEGRATION_ID").unwrap_or_else(|_| "vscode-chat".into());

    let mut req = state
        .http_client
        .post(&url)
        .bearer_auth(&agent.api_key)
        .json(&body);
    if is_copilot_direct {
        req = req
            .header("editor-version", "vscode/1.99.0")
            .header("editor-plugin-version", "copilot-chat/0.26.0")
            .header("Copilot-Integration-Id", integration_id);
    }
    let resp = req.send().await.map_err(|e| {
        if e.is_timeout() {
            anyhow::anyhow!("AI 请求超时，请检查代理地址、密钥或稍后重试")
        } else {
            anyhow::anyhow!("AI 请求失败: {}", e)
        }
    })?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await?;
        return Err(anyhow::anyhow!("{}", friendly_ai_api_error(status, &text)));
    }

    let response: Value = resp.json().await?;
    crate::token_usage_api::record_api_usage(
        &state.store,
        &response,
        user_id,
        feature,
        &agent.model,
        agent.usage_mode(),
    );
    Ok(response)
}

/// 调用 LLM API（普通对话，不带工具）
///
/// `user_id`  触发本次调用的用户 ID，用于 token 用量统计。
/// `feature`  功能标签，例如 "chat" / "social_ai" / "speech_translate"。
pub(crate) async fn call_chat_llm(
    state: &Arc<AppState>,
    agent: &AgentConfig,
    messages: &[Value],
    user_id: &str,
    feature: &str,
) -> Result<Value> {
    call_chat_llm_with_options(state, agent, messages, user_id, feature, 0.8, 700).await
}

/// 调用 LLM API（普通对话，不带工具），允许调用方控制生成参数。
pub(crate) async fn call_chat_llm_with_options(
    state: &Arc<AppState>,
    agent: &AgentConfig,
    messages: &[Value],
    user_id: &str,
    feature: &str,
    temperature: f64,
    max_tokens: usize,
) -> Result<Value> {
    call_chat_llm_with_response_format(
        state,
        agent,
        messages,
        user_id,
        feature,
        temperature,
        max_tokens,
        false,
    )
    .await
}

/// 调用 LLM API（普通对话），要求 OpenAI 兼容模型尽量返回 JSON 对象。
///
/// 部分 OpenAI-compatible 网关不支持 `response_format`，遇到兼容性错误时会自动降级
/// 为普通对话请求；认证、额度、限流等真实错误不会被重试掩盖。
pub(crate) async fn call_chat_llm_with_json_response_mode(
    state: &Arc<AppState>,
    agent: &AgentConfig,
    messages: &[Value],
    user_id: &str,
    feature: &str,
    temperature: f64,
    max_tokens: usize,
) -> Result<Value> {
    call_chat_llm_with_response_format(
        state,
        agent,
        messages,
        user_id,
        feature,
        temperature,
        max_tokens,
        true,
    )
    .await
}

async fn call_chat_llm_with_response_format(
    state: &Arc<AppState>,
    agent: &AgentConfig,
    messages: &[Value],
    user_id: &str,
    feature: &str,
    temperature: f64,
    max_tokens: usize,
    json_response_mode: bool,
) -> Result<Value> {
    ensure_api_call_allowed(state, agent, user_id)?;
    let url = format!("{}/chat/completions", agent.api_base);
    let body = chat_completion_body(agent, messages, temperature, max_tokens, json_response_mode);
    let resp = send_chat_completion_request(state, agent, &url, &body).await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await?;
        if json_response_mode && should_retry_without_json_response_mode(status, &text) {
            let fallback_body =
                chat_completion_body(agent, messages, temperature, max_tokens, false);
            let fallback = send_chat_completion_request(state, agent, &url, &fallback_body).await?;
            if !fallback.status().is_success() {
                let fallback_status = fallback.status();
                let fallback_text = fallback.text().await?;
                return Err(anyhow::anyhow!(
                    "{}",
                    friendly_ai_api_error(fallback_status, &fallback_text)
                ));
            }
            let response: Value = fallback.json().await?;
            crate::token_usage_api::record_api_usage(
                &state.store,
                &response,
                user_id,
                feature,
                &agent.model,
                agent.usage_mode(),
            );
            return Ok(response);
        }
        return Err(anyhow::anyhow!("{}", friendly_ai_api_error(status, &text)));
    }

    let response: Value = resp.json().await?;
    crate::token_usage_api::record_api_usage(
        &state.store,
        &response,
        user_id,
        feature,
        &agent.model,
        agent.usage_mode(),
    );
    Ok(response)
}

fn chat_completion_body(
    agent: &AgentConfig,
    messages: &[Value],
    temperature: f64,
    max_tokens: usize,
    json_response_mode: bool,
) -> Value {
    let mut body = json!({
        "model": agent.model,
        "messages": messages,
        "stream": false,
        "temperature": temperature,
        "max_tokens": max_tokens,
    });
    if json_response_mode {
        body["response_format"] = json!({ "type": "json_object" });
    }
    body
}

async fn send_chat_completion_request(
    state: &Arc<AppState>,
    agent: &AgentConfig,
    url: &str,
    body: &Value,
) -> Result<reqwest::Response> {
    let is_copilot_direct = agent.api_base.contains("githubcopilot.com");
    let integration_id =
        std::env::var("COPILOT_INTEGRATION_ID").unwrap_or_else(|_| "vscode-chat".into());

    let mut req = state
        .http_client
        .post(url)
        .bearer_auth(&agent.api_key)
        .json(body);
    if is_copilot_direct {
        req = req
            .header("editor-version", "vscode/1.99.0")
            .header("editor-plugin-version", "copilot-chat/0.26.0")
            .header("Copilot-Integration-Id", integration_id);
    }

    req.send().await.map_err(|e| {
        if e.is_timeout() {
            anyhow::anyhow!("AI 请求超时，请检查代理地址、密钥或稍后重试")
        } else {
            anyhow::anyhow!("AI 请求失败: {}", e)
        }
    })
}

fn should_retry_without_json_response_mode(status: reqwest::StatusCode, body: &str) -> bool {
    if !matches!(
        status,
        reqwest::StatusCode::BAD_REQUEST
            | reqwest::StatusCode::UNPROCESSABLE_ENTITY
            | reqwest::StatusCode::NOT_IMPLEMENTED
    ) {
        return false;
    }
    let lower = body.to_ascii_lowercase();
    lower.contains("response_format")
        || lower.contains("json_object")
        || lower.contains("json mode")
        || lower.contains("unsupported parameter")
        || lower.contains("unknown parameter")
        || lower.contains("unrecognized request argument")
}

#[cfg(test)]
mod tests {
    use super::{
        chat_completion_body, friendly_ai_api_error, should_retry_without_json_response_mode,
    };
    use crate::types::AgentConfig;
    use serde_json::json;

    #[test]
    fn chat_completion_body_can_request_json_object_response() {
        let agent = test_agent();
        let messages = vec![json!({"role": "user", "content": "Return JSON"})];

        let body = chat_completion_body(&agent, &messages, 0.2, 3000, true);
        assert_eq!(body["model"], "gpt-test");
        assert_eq!(body["temperature"], 0.2);
        assert_eq!(body["max_tokens"], 3000);
        assert_eq!(body["response_format"]["type"], "json_object");
        assert_eq!(body["messages"][0]["content"], "Return JSON");

        let fallback = chat_completion_body(&agent, &messages, 0.2, 3000, false);
        assert!(fallback.get("response_format").is_none());
    }

    #[test]
    fn json_response_fallback_only_handles_compatibility_errors() {
        assert!(should_retry_without_json_response_mode(
            reqwest::StatusCode::BAD_REQUEST,
            "Unrecognized request argument supplied: response_format"
        ));
        assert!(should_retry_without_json_response_mode(
            reqwest::StatusCode::UNPROCESSABLE_ENTITY,
            "json_object is not supported"
        ));
        assert!(!should_retry_without_json_response_mode(
            reqwest::StatusCode::UNAUTHORIZED,
            "invalid api key"
        ));
        assert!(!should_retry_without_json_response_mode(
            reqwest::StatusCode::TOO_MANY_REQUESTS,
            "rate limit"
        ));
    }

    #[test]
    fn friendly_error_hides_deprecated_model_json() {
        let message = friendly_ai_api_error(
            reqwest::StatusCode::BAD_REQUEST,
            r#"{"error":{"message":"该模型已下线，请根据迁移指南前往TokenHub平台体验最新模型服务。","code":"2030"}}"#,
        );

        assert!(message.contains("模型已下线"));
        assert!(message.contains("TokenHub"));
        assert!(!message.contains("{\"error\""));
    }

    #[test]
    fn friendly_error_explains_tokenhub_postpaid_quota() {
        let message = friendly_ai_api_error(
            reqwest::StatusCode::PAYMENT_REQUIRED,
            "The free trial quota for the service has been exhausted and postpaid billing is not enabled.",
        );

        assert!(message.contains("后付费"));
        assert!(!message.contains("free trial quota"));
    }

    #[test]
    fn node_casual_chat_failure_text_falls_back_to_cloud_llm() {
        use crate::agent_llm_tools::looks_like_node_casual_chat_failure;
        assert!(looks_like_node_casual_chat_failure(
            "PC CLI 执行失败: 调用 server-runtime 失败"
        ));
        assert!(looks_like_node_casual_chat_failure(
            "server-runtime failed before producing a reply"
        ));
        assert!(!looks_like_node_casual_chat_failure(
            "我在，状态正常。你可以继续说。"
        ));
    }

    fn test_agent() -> AgentConfig {
        AgentConfig {
            name: "test".to_string(),
            api_base: "https://example.test/v1".to_string(),
            api_key: "sk-test".to_string(),
            model: "gpt-test".to_string(),
            embedding_model: None,
            usage_mode: None,
        }
    }
}

fn ensure_api_call_allowed(
    state: &Arc<AppState>,
    agent: &AgentConfig,
    user_id: &str,
) -> Result<()> {
    if agent.usage_mode() == "user_api_key_proxy" {
        return Ok(());
    }
    if let Err(msg) = crate::billing::check_can_call(&state.store, user_id) {
        return Err(anyhow::anyhow!("{}", msg));
    }
    Ok(())
}

pub(crate) fn friendly_ai_api_error(status: reqwest::StatusCode, body: &str) -> String {
    let lower = body.to_lowercase();
    if lower.contains("该模型已下线")
        || lower.contains("模型已下线")
        || lower.contains("\"code\":\"2030\"")
        || lower.contains("\"code\":2030")
        || lower.contains("model has been discontinued")
        || lower.contains("model is discontinued")
    {
        return "当前 AI 模型已下线，请管理员迁移到 TokenHub 的可用模型或切换其他可用模型通道"
            .into();
    }
    if lower.contains("model or service id")
        && (lower.contains("does not exist") || lower.contains("not exist"))
    {
        return "当前 AI 模型配置不存在，请管理员检查 TokenHub 服务 ID 或切换其他可用模型通道"
            .into();
    }
    if status.as_u16() == 402
        || lower.contains("free_quota_exhausted")
        || lower.contains("free trial quota")
        || lower.contains("postpaid billing is not enabled")
        || lower.contains("payment required")
        || lower.contains("endpoint is inactive")
    {
        return "当前 AI 模型额度已用尽或未开启后付费，请切换可用模型，或联系管理员补充额度后重试"
            .into();
    }
    if status.as_u16() == 401 || lower.contains("unauthorized") || lower.contains("invalid api key")
    {
        return "当前 AI 模型密钥无效或权限不足，请检查 AI 设置或切换可用模型".into();
    }
    if status.as_u16() == 429 || lower.contains("rate limit") || lower.contains("too many requests")
    {
        return "当前 AI 模型请求过于频繁，请稍后重试或切换可用模型".into();
    }
    if status.as_u16() >= 500 {
        return "AI 服务暂时不可用，请稍后重试".into();
    }

    let compact = body.lines().collect::<Vec<_>>().join(" ");
    let visible = compact.chars().take(120).collect::<String>();
    if visible.trim().is_empty() {
        format!("AI 服务返回错误 {}", status)
    } else {
        format!("AI 服务返回错误 {}：{}", status, visible)
    }
}

// 向后兼容：execute_tool 和 try_casual_chat_via_node 已移至 agent_llm_tools
pub(crate) use crate::agent_llm_tools::{execute_tool, try_casual_chat_via_node};
