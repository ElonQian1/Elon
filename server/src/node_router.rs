//! 节点 LLM 请求路由器。
//!
//! 接收来自用户的 LLM 推理请求，寻找在线节点并通过 WS 隧道发送，
//! 流式转发推理结果给调用方，最后触发 token 积分结算。

use anyhow::{anyhow, Result};
use homecli_proto::AgentToServer;
use tokio::sync::mpsc;

use crate::types::AppState;
use std::sync::Arc;

/// 向节点发起 LLM 流式推理请求，返回一个 receiver，
/// 流中依次输出 `LlmStreamChunk`、最终 `LlmStreamEnd` 或 `LlmStreamError`。
pub async fn dispatch_to_node(
    state: &Arc<AppState>,
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
        .dispatch_llm_stream(&node_id, model_id.to_string(), messages, max_tokens)
        .await?;

    Ok((req_id, node_id, rx))
}

/// 推理完成后，根据 `LlmStreamEnd` 中的 token 统计执行积分结算。
/// `provider_user_id` 是节点属主的用户 ID（从 NodeRegistry 查到）。
pub fn settle_after_stream(
    state: &Arc<AppState>,
    consumer_user_id: &str,
    provider_user_id: &str,
    node_id: &str,
    model_id: &str,
    prompt_tokens: u32,
    completion_tokens: u32,
    price_per_1k: f64,
) {
    // 在后台线程结算，不阻塞 WS 响应
    let state = state.clone();
    let consumer = consumer_user_id.to_string();
    let provider = provider_user_id.to_string();
    let node = node_id.to_string();
    let model = model_id.to_string();

    tokio::spawn(async move {
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
