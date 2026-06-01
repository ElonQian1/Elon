//! 分布式节点 REST API。
//!
//! 路由（注册在 router.rs）：
//! - `GET /api/nodes`                 列出所有在线节点（需登录）
//! - `GET /api/nodes/models`          所有在线节点中可用模型列表（无需登录，供 APK 模型选择用）
//! - `GET /api/me/node-balance`       查询本用户作为节点提供者的积分余额
//! - `GET /api/me/node-transactions`  查询最近积分流水（最多 50 条）

use axum::{extract::State, http::{HeaderMap, StatusCode}, response::IntoResponse, Json};
use std::sync::Arc;

use crate::{project_auth::auth_from_headers, types::AppState};
use serde::Serialize;

// ── /api/nodes ────────────────────────────────────────────────────────────────

/// GET /api/nodes — 列出所有已知节点（含在线状态）
/// 需要有效用户 token（不要求管理员权限，普通用户可见）
pub async fn list_nodes(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if let Err(e) = auth_from_headers(&state, &headers) {
        return (StatusCode::UNAUTHORIZED, Json(serde_json::json!({"error": e.to_string()}))).into_response();
    }
    let nodes = state.node_registry.list_online().await;
    Json(serde_json::json!({ "nodes": nodes })).into_response()
}

// ── /api/nodes/models ────────────────────────────────────────────────────────

#[derive(Serialize)]
struct AvailableModelsResp {
    models: Vec<homecli_proto::ModelCapability>,
}

/// GET /api/nodes/models — 当前可用的 LLM 模型列表（无需登录）
pub async fn list_available_models(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let models = state.node_registry.available_models().await;
    Json(AvailableModelsResp { models }).into_response()
}

// ── /api/me/node-balance ──────────────────────────────────────────────────────

#[derive(Serialize)]
struct NodeBalanceResp {
    user_id: String,
    credits: f64,
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

    match state.store.get_node_balance(&user.id) {
        Ok(credits) => Json(NodeBalanceResp {
            user_id: user.id,
            credits,
        })
        .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
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

/// GET /api/me/nodes — 本用户自己的节点列表（含在线状态）
pub async fn my_nodes(
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

    let nodes = state.node_registry.list_by_owner(&user.id).await;
    Json(serde_json::json!({ "nodes": nodes })).into_response()
}
