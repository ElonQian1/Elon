//! 分布式节点 REST API。
//!
//! 路由（注册在 router.rs）：
//! - `GET  /api/nodes`                 列出所有在线节点（需登录）
//! - `GET  /api/nodes/models`          所有在线节点中可用模型列表（无需登录，供 APK 模型选择用）
//! - `POST /api/nodes/chat`            向节点 LLM 发起对话（需登录，同步阻塞返回）
//! - `GET  /api/me/nodes`              本用户自己的节点列表
//! - `POST /api/me/nodes/register`     注册一个新 PC 节点，获取 agent_id 和 secret
//! - `GET  /api/me/node-balance`       查询本用户作为节点提供者的积分余额
//! - `GET  /api/me/node-transactions`  查询最近积分流水（最多 50 条）

use axum::{extract::State, http::{HeaderMap, StatusCode}, response::IntoResponse, Json};
use sha2::Digest as _;
use std::sync::Arc;

use crate::{project_auth::auth_from_headers, types::AppState};
use serde::{Deserialize, Serialize};

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

// ── /api/me/nodes/register ────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct RegisterNodeRequest {
    /// 用户给这个节点起的名字，如 "我的游戏 PC"
    pub label: Option<String>,
}

#[derive(Serialize)]
pub struct RegisterNodeResponse {
    /// 分配给节点的 agent_id，配置到 NODE_AGENT_ID 环境变量
    pub agent_id: String,
    /// 明文 secret（只在注册时返回一次，不存储明文）
    pub agent_secret: String,
    /// 节点应连接的服务器 WebSocket 地址
    pub cloud_ws_url: String,
    /// 对应的 owner user_id（节点需要配置到 NODE_OWNER_USER_ID）
    pub owner_user_id: String,
}

/// POST /api/me/nodes/register — 为当前用户生成一个新的 PC 节点凭证
///
/// 生成随机 agent_id + secret，将 secret 的 SHA-256 hash 存入 DB，
/// 明文 secret 只在此响应中返回一次，用户需立即保存到节点的环境变量。
pub async fn register_node(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(req): Json<RegisterNodeRequest>,
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

    // 生成随机 agent_id 和 secret
    let random_suffix = uuid::Uuid::new_v4().to_string().replace('-', "").chars().take(8).collect::<String>();
    let agent_id = format!("node-{}-{}", &user.id.chars().take(6).collect::<String>(), random_suffix);
    let agent_secret = uuid::Uuid::new_v4().to_string().replace('-', "")
        + &uuid::Uuid::new_v4().to_string().replace('-', "");

    // 存储 secret 的 SHA-256 hash
    let secret_hash = hex::encode(sha2::Sha256::digest(agent_secret.as_bytes()));
    if let Err(e) = state.store.create_node_credential(
        &agent_id,
        &secret_hash,
        &user.id,
        req.label.as_deref(),
    ) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("创建凭证失败: {e}")})),
        )
            .into_response();
    }

    let cloud_ws_url = format!(
        "ws://{}",
        std::env::var("ELON_PUBLIC_HOST").unwrap_or_else(|_| "43.139.149.158:8080".to_string())
    );

    Json(RegisterNodeResponse {
        agent_id,
        agent_secret,
        cloud_ws_url,
        owner_user_id: user.id,
    })
    .into_response()
}

// ── /api/me/node-balance ──────────────────────────────────────────────────────

#[derive(Serialize)]
struct NodeBalanceResp {
    /// 当前可用余额（与 Android/Web 约定的字段名）
    balance: f64,
    /// 累计历史总收益
    lifetime_earned: f64,
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

    let balance = match state.store.get_node_balance(&user.id) {
        Ok(v) => v,
        Err(e) => return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        ).into_response(),
    };
    let lifetime_earned = state.store.get_lifetime_earned(&user.id).unwrap_or(0.0);
    Json(NodeBalanceResp { balance, lifetime_earned }).into_response()
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

// ── /api/nodes/chat ───────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct NodeChatRequest {
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

    // 找节点、发起请求
    let (_req_id, node_id, mut rx) = match crate::node_router::dispatch_to_node(
        &state,
        &req.model_id,
        req.messages,
        req.max_tokens,
    )
    .await
    {
        Ok(t) => t,
        Err(e) => {
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
    let owner = state
        .node_registry
        .get_node_owner(&node_id)
        .await
        .unwrap_or_default();
    if !owner.is_empty() {
        crate::node_router::settle_after_stream(
            &state,
            &user.id,
            &owner,
            &node_id,
            &req.model_id,
            prompt_tokens,
            completion_tokens,
            price,
        );
    }

    Json(NodeChatResponse {
        content,
        prompt_tokens,
        completion_tokens,
        model_id: req.model_id,
        node_id,
    })
    .into_response()
}
