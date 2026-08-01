// server/src/agent_llm_tools.rs
//! LLM 工具调用，从 agent_llm_call.rs 提取。
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

    let (req_id, actual_node_id, provider_user_id, mut rx) = match dispatch {
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
    crate::node_router::settle_after_stream(
        state,
        user_id,
        Some(&req_id),
        Some(&provider_user_id),
        &actual_node_id,
        model,
        prompt_tokens,
        completion_tokens,
        price,
    );

    let _ = node_id; // used above for find check
    Some((content, actual_node_id, model.to_string()))
}

pub(crate) fn looks_like_node_casual_chat_failure(content: &str) -> bool {
    let normalized = content.trim().to_ascii_lowercase();
    if normalized.is_empty() {
        return false;
    }
    normalized.contains("pc cli 执行失败")
        || normalized.contains("调用 server-runtime 失败")
        || normalized.contains("server-runtime failed")
        || (normalized.contains("server-runtime") && normalized.contains("失败"))
}
