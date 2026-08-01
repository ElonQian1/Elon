//! 分布式节点 REST API。
//!
//! 路由（注册在 router.rs）：
//! - `GET  /api/nodes`                 列出所有在线节点（需登录）
//! - `GET  /api/nodes/models`          当前用户自有或节点所有者显式共享的在线模型
//! - `POST /api/nodes/chat`            向节点 LLM 发起对话（需登录，同步阻塞返回）
//! - `GET  /api/me/nodes`              本用户自己的节点列表
//! - `POST /api/me/nodes/register`     注册一个新 PC 节点，获取 agent_id 和 secret
//! - `GET  /api/me/node-balance`       查询本用户作为节点提供者的积分余额
//! - `GET  /api/me/node-transactions`  查询最近积分流水（最多 50 条）

use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use homecli_proto::NodeStorageProfile;
use std::{collections::HashMap, sync::Arc};

pub use crate::node_register_api::register_node;
use crate::{
    admin,
    node_runtime::{clean_string, display_node_name, short_node_id, supports_project_cli},
    project_auth::auth_from_headers,
    types::AppState,
};
use serde::{Deserialize, Serialize};
mod compute_sharing;
mod my_nodes;
mod payouts;
mod public_dev;
mod public_dev_smoke;
mod responses;
mod runtime_response;
mod usage;
pub use compute_sharing::{get_my_node_compute_sharing, update_my_node_compute_sharing};
pub use my_nodes::my_nodes;
use payouts::node_payout_min_fen;
pub use payouts::{cancel_node_payout, create_node_payout, my_node_payouts};
pub use public_dev::{admin_public_dev_handshake, update_my_node_sharing};
use public_dev::{public_dev_handshake_state, public_dev_handshake_value};
pub use public_dev_smoke::{
    admin_owner_codex_smoke_post, admin_public_dev_mutual_smoke_get,
    admin_public_dev_mutual_smoke_post,
};
use responses::PublicNodeResponse;
use runtime_response::{
    capacity_for_response, hardware_for_response, hardware_summary, project_counts_for_user,
    runtime_route_flags,
};
pub use usage::my_node_usage;

pub(crate) fn storage_can_cross_pc(storage: &NodeStorageProfile) -> bool {
    storage
        .git_base_url
        .as_ref()
        .map(|value| !value.trim().is_empty())
        .unwrap_or(false)
        || storage.relay_git_url_enabled
}

mod list_nodes;
pub use list_nodes::list_nodes;

// ── /api/nodes/models ────────────────────────────────────────────────────────

#[derive(Serialize)]
struct AvailableModelsResp {
    models: Vec<homecli_proto::ModelCapability>,
}

/// GET /api/nodes/models — 当前用户自有或所有者显式共享的在线 LLM 模型。
/// 未登录调用者只能看到明确开启共享且仍有额度的模型。
pub async fn list_available_models(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let viewer_user_id = auth_from_headers(&state, &headers)
        .ok()
        .map(|user| user.id)
        .unwrap_or_default();
    let mut models = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for node in state.node_registry.list_online().await {
        for model in node.models {
            let status = crate::node_compute_sharing::status(
                &state.store,
                &viewer_user_id,
                &node.owner_user_id,
                &node.node_id,
                &model.model_id,
            );
            if status.is_ok_and(|status| status.available) && seen.insert(model.model_id.clone()) {
                models.push(model);
            }
        }
    }
    Json(AvailableModelsResp { models }).into_response()
}

// ── /api/me/node-balance ──────────────────────────────────────────────────────

#[derive(Serialize)]
struct NodeBalanceResp {
    /// 当前可用余额（与 Android/Web 约定的字段名）
    balance: f64,
    /// 当前可用余额（人民币分），用于资金对账。
    balance_fen: i64,
    /// 累计历史总收益
    lifetime_earned: f64,
    /// 累计历史总收益（人民币分）。
    lifetime_earned_fen: i64,
    /// 已申请、等待运营处理的提现金额。
    pending_payouts: f64,
    /// 已申请、等待运营处理的提现金额（人民币分）。
    pending_payout_fen: i64,
    /// 最低提现金额（分），由后台配置。
    payout_min_fen: i64,
    /// 节点提供者分账比例 × 1000（800 = 80%）
    provider_revenue_share_x1000: i64,
    /// 节点提供者分账比例百分比，便于前端展示。
    provider_revenue_share_percent: f64,
}

/// GET /api/me/node-balance — 本用户作为节点提供者的积分余额
pub async fn my_node_balance(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let user = match auth_from_headers(&state, &headers) {
        Ok(u) => u,
        Err(e) => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({"error": e.to_string()})),
            )
                .into_response()
        }
    };

    let balance_fen = match state.store.get_node_balance_fen(&user.id) {
        Ok(v) => v,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": e.to_string()})),
            )
                .into_response()
        }
    };
    let balance = balance_fen as f64 / 100.0;
    let lifetime_earned_fen = state.store.get_lifetime_earned_fen(&user.id).unwrap_or(0);
    let lifetime_earned = lifetime_earned_fen as f64 / 100.0;
    let pending_payout_fen = state
        .store
        .get_pending_node_payout_total_fen(&user.id)
        .unwrap_or(0);
    let pending_payouts = pending_payout_fen as f64 / 100.0;
    let payout_min_fen = node_payout_min_fen(&state);
    let provider_revenue_share_x1000 =
        crate::node_router::provider_revenue_share_x1000(&state.store);
    Json(NodeBalanceResp {
        balance,
        balance_fen,
        lifetime_earned,
        lifetime_earned_fen,
        pending_payouts,
        pending_payout_fen,
        payout_min_fen,
        provider_revenue_share_x1000,
        provider_revenue_share_percent: provider_revenue_share_x1000 as f64 / 10.0,
    })
    .into_response()
}

// ── /api/me/node-transactions ─────────────────────────────────────────────────

/// GET /api/me/node-transactions — 最近 50 条积分流水
pub async fn my_node_transactions(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let user = match auth_from_headers(&state, &headers) {
        Ok(u) => u,
        Err(e) => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({"error": e.to_string()})),
            )
                .into_response()
        }
    };

    match state.store.list_node_transactions(&user.id, 50) {
        Ok(txs) => Json(serde_json::json!({ "transactions": txs })).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

#[derive(Deserialize)]
pub struct NodeChatRequest {
    /// 可选目标节点 ID；为空时自动匹配在线节点。
    pub node_id: Option<String>,
    /// 目标模型 ID，如 "llama3:8b"（需匹配在线节点已上报的 model_id）
    pub model_id: String,
    /// OpenAI 格式的 messages 数组
    pub messages: Vec<serde_json::Value>,
    /// 最大生成 token 数（可选）
    pub max_tokens: Option<u32>,
}

#[derive(Serialize)]
pub struct NodeChatResponse {
    /// 模型生成的完整文本
    pub content: String,
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub model_id: String,
    pub node_id: String,
}

/// POST /api/nodes/chat — 向节点 LLM 发起对话（同步等待完整回复）
///
/// 调用流程：
/// 1. 认证用户
/// 2. 通过 NodeRegistry 找到支持该模型的在线节点
/// 3. 经 WebSocket 隧道发送推理请求，收集所有流式块
/// 4. 完成后触发后台积分结算
/// 5. 返回完整文本 + token 统计
pub async fn chat_with_node(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(req): Json<NodeChatRequest>,
) -> impl IntoResponse {
    let user = match auth_from_headers(&state, &headers) {
        Ok(u) => u,
        Err(e) => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({"error": e.to_string()})),
            )
                .into_response();
        }
    };

    if req.model_id.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "model_id 不能为空"})),
        )
            .into_response();
    }
    if req.messages.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "messages 不能为空"})),
        )
            .into_response();
    }
    if let Err(msg) = crate::billing::check_can_call(&state.store, &user.id) {
        return (
            StatusCode::PAYMENT_REQUIRED,
            Json(serde_json::json!({"error": msg})),
        )
            .into_response();
    }

    let max_output_tokens = req.max_tokens.unwrap_or(1024) as i64;
    let req_id = uuid::Uuid::new_v4().to_string();
    let accounting_key = format!("node_llm:{req_id}");
    let node_reserve_fen = crate::billing::estimate_cost_for_tokens(
        &state.store,
        &req.model_id,
        0,
        0,
        max_output_tokens,
    )
    .max(crate::billing::configured_reservation_fen(
        &state.store,
        "billing_node_llm_min_reservation_fen",
        1,
    ));
    if let Err(msg) = crate::billing::reserve_trusted_call(
        &state.store,
        &user.id,
        &accounting_key,
        "node_llm",
        "server_node_llm",
        Some(&req.model_id),
        node_reserve_fen,
    ) {
        return (
            StatusCode::PAYMENT_REQUIRED,
            Json(serde_json::json!({"error": msg})),
        )
            .into_response();
    }

    // 找节点、发起请求。预授权成功后才派发，避免余额不足用户先消耗节点算力。
    let (req_id, node_id, provider_user_id, mut rx) =
        match crate::node_router::dispatch_to_node_with_req_id(
            &state,
            req_id,
            &user.id,
            &req.model_id,
            req.node_id.as_deref(),
            req.messages,
            req.max_tokens,
        )
        .await
        {
            Ok(t) => t,
            Err(e) => {
                crate::billing::release_trusted_call(
                    &state.store,
                    &user.id,
                    &accounting_key,
                    "released_error",
                );
                return (
                    StatusCode::SERVICE_UNAVAILABLE,
                    Json(serde_json::json!({"error": e.to_string()})),
                )
                    .into_response();
            }
        };

    // 收集流式块
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
                    &state,
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
                    &user.id,
                    &accounting_key,
                    "released_error",
                );
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({"error": message})),
                )
                    .into_response();
            }
            _ => {}
        }
    }

    // 后台积分结算（不阻塞响应）
    let price = state
        .node_registry
        .get_node_model_price(&node_id, &req.model_id)
        .await
        .unwrap_or(1.0);
    crate::node_router::settle_after_stream(
        &state,
        &user.id,
        Some(&req_id),
        Some(&provider_user_id),
        &node_id,
        &req.model_id,
        prompt_tokens,
        completion_tokens,
        price,
    );

    Json(NodeChatResponse {
        content,
        prompt_tokens,
        completion_tokens,
        model_id: req.model_id,
        node_id,
    })
    .into_response()
}

// ── /api/node-agent/version ───────────────────────────────────────────────────

/// GET /api/node-agent/version — 返回最新 node-agent 发布版本信息
/// 不需要登录（node-agent 启动时就需要查询）
pub async fn node_agent_version(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let version_file = state
        .data_dir
        .join("downloads")
        .join("node-agent-version.json");
    match tokio::fs::read(&version_file).await {
        Ok(bytes) => axum::response::Response::builder()
            .status(StatusCode::OK)
            .header("content-type", "application/json")
            .body(axum::body::Body::from(bytes))
            .unwrap_or_else(|_| axum::response::Response::default()),
        Err(_) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "no version info available"})),
        )
            .into_response(),
    }
}

/// GET /api/node-agent/download/windows — 下载最新 Windows 客户端 exe
/// 不需要登录（执行文件不含敏感信息）
pub async fn download_node_agent_windows(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    download_node_agent_binary(state, "elon-pc-node.exe", "一龙开发平台.exe").await
}

/// GET /api/node-agent/download/windows-client — 下载 Windows 客户端包
/// 不需要登录（压缩包不含敏感信息，首次登录在本机管理页完成）
pub async fn download_node_agent_windows_client(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    download_node_agent_binary(
        state,
        "elon-node-agent-windows.zip",
        "elon-node-agent-windows.zip",
    )
    .await
}

/// GET /api/node-agent/download/windows-installer — 下载 Windows 单文件安装程序
/// ZIP 继续保留给既有客户端自动更新，浏览器首次安装使用此入口。
pub async fn download_node_agent_windows_installer(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    download_node_agent_binary(
        state,
        "elon-node-agent-windows-setup.exe",
        "Elon-Windows-Setup.exe",
    )
    .await
}

/// GET /api/node-agent/download/linux — 下载最新 Linux 可执行文件
/// 不需要登录（执行文件不含敏感信息）
pub async fn download_node_agent_linux(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    download_node_agent_binary(state, "elon-pc-node", "elon-pc-node").await
}

pub(crate) async fn download_node_agent_binary(
    state: Arc<AppState>,
    file_name: &'static str,
    download_name: &'static str,
) -> axum::response::Response {
    let exe_path = state.data_dir.join("downloads").join(file_name);
    match tokio::fs::read(&exe_path).await {
        Ok(bytes) => axum::response::Response::builder()
            .status(StatusCode::OK)
            .header("content-type", download_content_type(download_name))
            .header(
                "content-disposition",
                format!("attachment; filename=\"{}\"", download_name),
            )
            .body(axum::body::Body::from(bytes))
            .unwrap_or_else(|_| axum::response::Response::default()),
        Err(_) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "node-agent binary not available"})),
        )
            .into_response(),
    }
}

fn download_content_type(download_name: &str) -> &'static str {
    if download_name.ends_with(".zip") {
        "application/zip"
    } else {
        "application/octet-stream"
    }
}

/// POST /api/admin/nodes/push-update — 广播更新指令给所有在线节点（管理员功能）
pub async fn push_node_update(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if !admin::check_auth(&headers, &state.admin_token) {
        return (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({"error":"admin token required"})),
        )
            .into_response();
    }
    let version_file = state
        .data_dir
        .join("downloads")
        .join("node-agent-version.json");
    let release = tokio::fs::read_to_string(&version_file)
        .await
        .ok()
        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok());
    let version = release.as_ref().and_then(node_update_release_identity);
    let git_sha = release
        .as_ref()
        .and_then(|value| value["gitSha"].as_str().map(str::to_string));
    let count = state
        .agent_manager
        .broadcast_update_client(version.clone(), None)
        .await;
    match public_dev_handshake_value(&state).await {
        Ok(report) => Json(serde_json::json!({
            "ok": true,
            "broadcast_to": count,
            "version": version,
            "gitSha": git_sha,
            "message": format!("{count} 个在线节点已收到更新指令"),
            "public_dev_handshake": report,
        }))
        .into_response(),
        Err(e) => Json(serde_json::json!({
            "ok": true,
            "broadcast_to": count,
            "version": version,
            "gitSha": git_sha,
            "message": format!("{count} 个在线节点已收到更新指令"),
            "public_dev_handshake_error": e.to_string(),
        }))
        .into_response(),
    }
}

fn node_update_release_identity(value: &serde_json::Value) -> Option<String> {
    let version = value["version"].as_str()?.trim();
    if version.is_empty() {
        return None;
    }
    let git_sha = value["gitSha"].as_str().map(str::trim).unwrap_or_default();
    Some(if git_sha.is_empty() {
        version.to_string()
    } else {
        format!("{version}+{git_sha}")
    })
}

#[cfg(test)]
mod node_update_tests {
    use super::node_update_release_identity;

    #[test]
    fn broadcast_target_carries_the_full_immutable_release_identity() {
        let release = serde_json::json!({
            "version": "0.3.69",
            "gitSha": "b03b77295f00"
        });
        assert_eq!(
            node_update_release_identity(&release).as_deref(),
            Some("0.3.69+b03b77295f00")
        );
        assert_eq!(
            node_update_release_identity(&serde_json::json!({"version":"0.3.69"})).as_deref(),
            Some("0.3.69")
        );
    }
}
