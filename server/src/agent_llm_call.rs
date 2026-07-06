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
        chat_completion_body, looks_like_node_casual_chat_failure,
        should_retry_without_json_response_mode,
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
    fn node_casual_chat_failure_text_falls_back_to_cloud_llm() {
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
    if status.as_u16() == 402
        || lower.contains("free_quota_exhausted")
        || lower.contains("payment required")
        || lower.contains("endpoint is inactive")
    {
        return "当前 AI 模型额度已用尽或接口不可用，请切换可用模型，或联系管理员补充额度后重试"
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

/// 根据工具名和参数，调用对应的工具函数
pub(crate) fn execute_tool(
    state: &Arc<AppState>,
    workspace: &std::path::Path,
    agent: &AgentConfig,
    tool_name: &str,
    args: &Value,
    user_id: &str,
    trace_id: Option<&str>,
) -> Result<String> {
    if crate::context_compiler::agent_rag_context::is_rag_tool(tool_name) {
        return crate::context_compiler::agent_rag_context::execute_rag_tool(
            &state.data_dir,
            workspace,
            Some(agent),
            tool_name,
            args,
            trace_id,
        );
    }

    match tool_name {
        "init_project" => {
            let project_type = args["project_type"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("缺少 project_type 参数"))?;
            tools::init_project(workspace, project_type)
        }
        "read_file" => {
            let path = args["path"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("缺少 path 参数"))?;
            tools::read_file(workspace, path)
        }
        "write_file" => {
            let path = args["path"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("缺少 path 参数"))?;
            let content = args["content"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("缺少 content 参数"))?;
            tools::write_file(workspace, path, content)
        }
        "apply_patch" => {
            let patch = args["patch"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("缺少 patch 参数"))?;
            let check_only = args["check_only"].as_bool().unwrap_or(false);
            tools::apply_patch(workspace, patch, check_only)
        }
        "list_dir" => {
            let path = args["path"].as_str().unwrap_or(".");
            tools::list_dir(workspace, path)
        }
        "run_shell" => {
            let command = args["command"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("缺少 command 参数"))?;
            tools::run_shell(workspace, command)
        }
        "git_commit" => {
            let message = args["message"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("缺少 message 参数"))?;
            tools::git_commit(workspace, message)
        }
        "build_project" => {
            let target = args["target"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("缺少 target 参数"))?;
            let quota: i64 = std::env::var("DAILY_BUILD_QUOTA")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(10);
            if quota > 0 {
                state
                    .store
                    .check_and_increment_build_quota(user_id, quota)?;
            }
            tools::build_project(workspace, target, user_id)
        }
        _ => Err(anyhow::anyhow!("未知工具: {}", tool_name)),
    }
}

/// 尝试将对话路由到在线 PC 节点 LLM。
///
/// 当且仅当 `model` 名称与在线节点上报的 `model_id` 完全匹配时路由；
/// 否则返回 `None`（调用方应降级到云端 LLM）。
///
/// 返回 `Some((content, node_id, model_id))` 或 `None`。
pub(crate) async fn try_casual_chat_via_node(
    state: &Arc<AppState>,
    model: &str,
    messages: &[Value],
    user_id: &str,
) -> Option<(String, String, String)> {
    if let Err(msg) = crate::billing::check_can_call(&state.store, user_id) {
        tracing::warn!(user_id, model, "节点推理被余额/配额拦截: {}", msg);
        return None;
    }

    // 如果没有在线节点支持该模型，立即返回 None
    let node_id = state.node_registry.find_node_for_model(model).await?;

    let req_id = uuid::Uuid::new_v4().to_string();
    let accounting_key = format!("node_llm:{req_id}");
    let node_reserve_fen =
        crate::billing::estimate_cost_for_tokens(&state.store, model, 0, 0, 1024).max(
            crate::billing::configured_reservation_fen(
                &state.store,
                "billing_node_llm_min_reservation_fen",
                1,
            ),
        );
    if let Err(msg) = crate::billing::reserve_trusted_call(
        &state.store,
        user_id,
        &accounting_key,
        "node_llm",
        "server_node_llm",
        Some(model),
        node_reserve_fen,
    ) {
        tracing::warn!(user_id, model, "节点推理预授权失败: {}", msg);
        return None;
    }

    let dispatch = crate::node_router::dispatch_to_node_with_req_id(
        state,
        req_id,
        user_id,
        model,
        None,
        messages.to_vec(),
        Some(1024),
    )
    .await;

    let (req_id, actual_node_id, mut rx) = match dispatch {
        Ok(t) => t,
        Err(e) => {
            crate::billing::release_trusted_call(
                &state.store,
                user_id,
                &accounting_key,
                "released_error",
            );
            tracing::warn!("节点路由失败，降级到云端 LLM: {e}");
            return None;
        }
    };

    let mut content = String::new();
    let mut prompt_tokens: u32 = 0;
    let mut completion_tokens: u32 = 0;

    while let Some(msg) = rx.recv().await {
        match msg {
            homecli_proto::AgentToServer::LlmStreamChunk { delta, .. } => {
                content.push_str(&delta);
            }
            homecli_proto::AgentToServer::LlmStreamEnd {
                prompt_tokens: pt,
                completion_tokens: ct,
                ..
            } => {
                prompt_tokens = pt;
                completion_tokens = ct;
                break;
            }
            homecli_proto::AgentToServer::LlmStreamError { message, .. } => {
                crate::node_router::finish_node_compute_run(
                    state,
                    &accounting_key,
                    crate::store::NodeComputeRunFinish {
                        provider_user_id: None,
                        status: "failed",
                        prompt_tokens: prompt_tokens as i64,
                        completion_tokens: completion_tokens as i64,
                        billed_cost_rmb_fen: 0,
                        provider_earned_fen: 0,
                        settlement_status: None,
                        error_message: Some(&message),
                    },
                );
                crate::billing::release_trusted_call(
                    &state.store,
                    user_id,
                    &accounting_key,
                    "released_error",
                );
                tracing::warn!("节点推理错误，降级到云端 LLM: {message}");
                return None;
            }
            _ => {}
        }
    }

    if looks_like_node_casual_chat_failure(&content) {
        crate::node_router::finish_node_compute_run(
            state,
            &accounting_key,
            crate::store::NodeComputeRunFinish {
                provider_user_id: None,
                status: "failed",
                prompt_tokens: prompt_tokens as i64,
                completion_tokens: completion_tokens as i64,
                billed_cost_rmb_fen: 0,
                provider_earned_fen: 0,
                settlement_status: None,
                error_message: Some(content.trim()),
            },
        );
        crate::billing::release_trusted_call(
            &state.store,
            user_id,
            &accounting_key,
            "released_error",
        );
        tracing::warn!("节点普通聊天返回执行失败文本，降级到云端 LLM: {content}");
        return None;
    }

    if content.is_empty() {
        crate::node_router::finish_node_compute_run(
            state,
            &accounting_key,
            crate::store::NodeComputeRunFinish {
                provider_user_id: None,
                status: "released_no_usage",
                prompt_tokens: prompt_tokens as i64,
                completion_tokens: completion_tokens as i64,
                billed_cost_rmb_fen: 0,
                provider_earned_fen: 0,
                settlement_status: None,
                error_message: Some("empty node LLM response"),
            },
        );
        crate::billing::release_trusted_call(
            &state.store,
            user_id,
            &accounting_key,
            "released_no_usage",
        );
        return None;
    }

    // 后台结算积分
    let price = state
        .node_registry
        .get_node_model_price(&actual_node_id, model)
        .await
        .unwrap_or(1.0);
    let owner = state
        .node_registry
        .get_node_owner(&actual_node_id)
        .await
        .unwrap_or_default();
    crate::node_router::settle_after_stream(
        state,
        user_id,
        Some(&req_id),
        Some(&owner),
        &actual_node_id,
        model,
        prompt_tokens,
        completion_tokens,
        price,
    );

    let _ = node_id; // used above for find check
    Some((content, actual_node_id, model.to_string()))
}

fn looks_like_node_casual_chat_failure(content: &str) -> bool {
    let normalized = content.trim().to_ascii_lowercase();
    if normalized.is_empty() {
        return false;
    }
    normalized.contains("pc cli 执行失败")
        || normalized.contains("调用 server-runtime 失败")
        || normalized.contains("server-runtime failed")
        || (normalized.contains("server-runtime") && normalized.contains("失败"))
}
