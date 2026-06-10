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
    // 计费前置检查：若用户已开通预存计费且余额为 0，则拒绝调用
    if let Err(msg) = crate::billing::check_can_call(&state.store, user_id) {
        return Err(anyhow::anyhow!("{}", msg));
    }
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
    // 计费前置检查
    if let Err(msg) = crate::billing::check_can_call(&state.store, user_id) {
        return Err(anyhow::anyhow!("{}", msg));
    }
    let url = format!("{}/chat/completions", agent.api_base);

    let body = json!({
        "model": agent.model,
        "messages": messages,
        "stream": false,
        "temperature": 0.8,
        "max_tokens": 700,
    });

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
    );
    Ok(response)
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
    tool_name: &str,
    args: &Value,
    user_id: &str,
) -> Result<String> {
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

    if content.is_empty() {
        crate::node_router::finish_node_compute_run(
            state,
            &accounting_key,
            crate::store::NodeComputeRunFinish {
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
