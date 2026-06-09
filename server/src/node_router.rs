//! 节点 LLM 请求路由器。
//!
//! 接收来自用户的 LLM 推理请求，寻找在线节点并通过 WS 隧道发送，
//! 流式转发推理结果给调用方，最后触发 token 积分结算。

use anyhow::{anyhow, Result};
use homecli_proto::AgentToServer;
use tokio::sync::mpsc;

use crate::types::AppState;
use std::sync::Arc;

pub async fn dispatch_to_node_with_req_id(
    state: &Arc<AppState>,
    req_id: String,
    model_id: &str,
    messages: Vec<serde_json::Value>,
    max_tokens: Option<u32>,
) -> Result<(String, String, mpsc::UnboundedReceiver<AgentToServer>)> {
    // 找到一个支持该模型的在线节点
    let node_id = state
        .node_registry
        .find_node_for_model(model_id)
        .await
        .ok_or_else(|| anyhow!("没有在线节点支持模型 {model_id}"))?;

    let (req_id, rx) = state
        .agent_manager
        .dispatch_llm_stream_with_req_id(
            &node_id,
            req_id,
            model_id.to_string(),
            messages,
            max_tokens,
        )
        .await?;

    Ok((req_id, node_id, rx))
}

/// 推理完成后，根据 `LlmStreamEnd` 中的 token 统计执行消费扣费和节点积分结算。
/// `provider_user_id` 是节点属主的用户 ID（从 NodeRegistry 查到）；为空时仍记录消费者用量。
pub fn settle_after_stream(
    state: &Arc<AppState>,
    consumer_user_id: &str,
    compute_call_id: Option<&str>,
    provider_user_id: Option<&str>,
    node_id: &str,
    model_id: &str,
    prompt_tokens: u32,
    completion_tokens: u32,
    price_per_1k: f64,
) {
    // 在后台线程结算，不阻塞 WS 响应
    let state = state.clone();
    let consumer = consumer_user_id.to_string();
    let provider = provider_user_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);
    let accounting_key = compute_call_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| format!("node_llm:{value}"));
    let node = node_id.to_string();
    let model = model_id.to_string();

    tokio::spawn(async move {
        let usage = crate::cli_usage::CliTokenUsage {
            input_tokens: prompt_tokens as i64,
            cached_input_tokens: 0,
            output_tokens: completion_tokens as i64,
            reasoning_tokens: 0,
            total_tokens: (prompt_tokens + completion_tokens) as i64,
            model: Some(model.clone()),
        };
        let accounting_result = crate::token_usage_api::record_trusted_usage_with_key(
            &state.store,
            &consumer,
            "node_llm",
            "server_node_llm",
            Some(&model),
            &usage,
            accounting_key.as_deref(),
        );
        if accounting_result
            .as_ref()
            .map(|result| result.deduplicated)
            .unwrap_or(true)
        {
            tracing::warn!(
                consumer,
                node,
                model,
                "节点推理用量未新增扣费事件，跳过节点积分结算"
            );
            return;
        }

        let Some(provider) = provider else {
            tracing::warn!(
                consumer,
                node,
                model,
                "节点推理已记录消费者 token 用量，但缺少 provider owner，跳过节点积分结算"
            );
            return;
        };

        let params = crate::store::SettleParams {
            consumer_user_id: &consumer,
            provider_user_id: &provider,
            node_id: &node,
            model_id: &model,
            prompt_tokens,
            completion_tokens,
            price_per_1k_credits: price_per_1k,
            platform_fee_rate: 0.2, // 平台抽成 20%
        };
        match state.store.settle_node_inference(params) {
            Ok(tx) => tracing::debug!(
                "💰 节点积分结算: {} → {} | {}+{} tokens | {:.4} credits",
                consumer,
                provider,
                prompt_tokens,
                completion_tokens,
                tx.settled_credits
            ),
            Err(e) => tracing::error!("节点积分结算失败: {e}"),
        }
    });
}
