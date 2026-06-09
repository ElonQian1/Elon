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

use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use homecli_proto::ModelCapability;
use sha2::Digest as _;
use std::{collections::HashMap, sync::Arc};

use crate::{project_auth::auth_from_headers, types::AppState};
use serde::{Deserialize, Serialize};

// ── /api/nodes ────────────────────────────────────────────────────────────────

/// GET /api/nodes — 列出所有用户可发现的在线节点（含在线状态）
/// 需要有效用户 token（不要求管理员权限，普通用户可见）
pub async fn list_nodes(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if let Err(e) = auth_from_headers(&state, &headers) {
        return (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response();
    }
    let nodes = state.node_registry.list_online().await;
    let nodes: Vec<_> = nodes
        .into_iter()
        .map(|node| {
            let short_id = short_node_id(&node.node_id);
            let device_name = clean_string(node.device_name.as_deref());
            let display_name = display_node_name("", device_name.as_deref(), &short_id);
            PublicNodeResponse {
                agent_id: node.node_id.clone(),
                node_id: node.node_id,
                owner_user_id: node.owner_user_id,
                device_name,
                display_name,
                short_id,
                models: node.models,
                tts_worker_url: node.tts_worker_url,
                connected_at: node.connected_at,
                online: node.online,
            }
        })
        .collect();
    Json(serde_json::json!({ "nodes": nodes })).into_response()
}

// ── /api/nodes/models ────────────────────────────────────────────────────────

#[derive(Serialize)]
struct AvailableModelsResp {
    models: Vec<homecli_proto::ModelCapability>,
}

/// GET /api/nodes/models — 当前可用的 LLM 模型列表（无需登录）
pub async fn list_available_models(State(state): State<Arc<AppState>>) -> impl IntoResponse {
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
    let random_suffix = uuid::Uuid::new_v4()
        .to_string()
        .replace('-', "")
        .chars()
        .take(8)
        .collect::<String>();
    let agent_id = format!(
        "node-{}-{}",
        &user.id.chars().take(6).collect::<String>(),
        random_suffix
    );
    let agent_secret = uuid::Uuid::new_v4().to_string().replace('-', "")
        + &uuid::Uuid::new_v4().to_string().replace('-', "");

    // 存储 secret 的 SHA-256 hash
    let secret_hash = hex::encode(sha2::Sha256::digest(agent_secret.as_bytes()));
    if let Err(e) =
        state
            .store
            .create_node_credential(&agent_id, &secret_hash, &user.id, req.label.as_deref())
    {
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
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": e.to_string()})),
            )
                .into_response()
        }
    };
    let lifetime_earned = state.store.get_lifetime_earned(&user.id).unwrap_or(0.0);
    Json(NodeBalanceResp {
        balance,
        lifetime_earned,
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

/// GET /api/me/nodes — 本用户自己的节点列表（含在线状态）
pub async fn my_nodes(State(state): State<Arc<AppState>>, headers: HeaderMap) -> impl IntoResponse {
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

    let credentials = match state.store.list_node_credentials(&user.id) {
        Ok(nodes) => nodes,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": e.to_string()})),
            )
                .into_response()
        }
    };
    let mut online_by_id: HashMap<_, _> = state
        .node_registry
        .list_by_owner(&user.id)
        .await
        .into_iter()
        .map(|node| (node.node_id.clone(), node))
        .collect();
    let cli_by_id: HashMap<_, _> = state
        .agent_manager
        .list()
        .await
        .into_iter()
        .map(|agent| (agent.agent_id.clone(), agent))
        .collect();
    let project_counts = match state.store.list_projects_for_user(&user.id) {
        Ok(projects) => projects
            .into_iter()
            .filter_map(|project| project.node_id)
            .fold(HashMap::<String, i64>::new(), |mut counts, node_id| {
                *counts.entry(node_id).or_insert(0) += 1;
                counts
            }),
        Err(e) => {
            tracing::warn!(user_id = %user.id, error = %e, "failed to count user projects per node");
            HashMap::new()
        }
    };

    let mut nodes = Vec::new();
    for credential in credentials {
        let node_id = credential.agent_id.clone();
        let online = online_by_id.remove(&node_id);
        let cli_agent = cli_by_id.get(&node_id);
        let short_id = short_node_id(&node_id);
        let label = credential.label.trim().to_string();
        let device_name = online
            .as_ref()
            .and_then(|node| clean_string(node.device_name.as_deref()))
            .or_else(|| cli_agent.and_then(|agent| clean_string(agent.device_name.as_deref())))
            .or_else(|| clean_string(credential.device_name.as_deref()));
        let display_label = if label == node_id { "" } else { &label };
        let display_name = display_node_name(display_label, device_name.as_deref(), &short_id);
        let models = online
            .as_ref()
            .map(|node| node.models.clone())
            .unwrap_or_default();
        let allowed_clis = cli_agent
            .map(|agent| agent.allowed_clis.clone())
            .unwrap_or_default();
        let connected_at = online
            .as_ref()
            .map(|node| node.connected_at)
            .unwrap_or_else(|| cli_agent.map(|agent| agent.connected_at).unwrap_or(0));
        let is_online =
            online.as_ref().map(|node| node.online).unwrap_or(false) || cli_agent.is_some();
        nodes.push(MyNodeResponse {
            agent_id: node_id.clone(),
            node_id,
            owner_user_id: credential.owner_user_id,
            label,
            device_name,
            display_name,
            short_id,
            models,
            allowed_clis: allowed_clis.clone(),
            cli_project_ready: supports_project_cli(&allowed_clis),
            project_count: project_counts
                .get(&credential.agent_id)
                .copied()
                .unwrap_or(0),
            connected_at,
            created_at: credential.created_at,
            online: is_online,
        });
    }

    for node in online_by_id.into_values() {
        let node_id = node.node_id.clone();
        let short_id = short_node_id(&node.node_id);
        let device_name = clean_string(node.device_name.as_deref());
        let display_name = display_node_name("", device_name.as_deref(), &short_id);
        nodes.push(MyNodeResponse {
            agent_id: node_id.clone(),
            node_id: node_id.clone(),
            owner_user_id: node.owner_user_id,
            label: String::new(),
            device_name,
            display_name,
            short_id,
            models: node.models,
            allowed_clis: Vec::new(),
            cli_project_ready: false,
            project_count: project_counts.get(&node_id).copied().unwrap_or(0),
            connected_at: node.connected_at,
            created_at: String::new(),
            online: node.online,
        });
    }

    Json(serde_json::json!({ "nodes": nodes })).into_response()
}

#[derive(Serialize)]
struct PublicNodeResponse {
    agent_id: String,
    node_id: String,
    owner_user_id: String,
    device_name: Option<String>,
    display_name: String,
    short_id: String,
    models: Vec<ModelCapability>,
    tts_worker_url: Option<String>,
    connected_at: u64,
    online: bool,
}

#[derive(Serialize)]
struct MyNodeResponse {
    agent_id: String,
    node_id: String,
    owner_user_id: String,
    label: String,
    device_name: Option<String>,
    display_name: String,
    short_id: String,
    models: Vec<ModelCapability>,
    allowed_clis: Vec<String>,
    cli_project_ready: bool,
    project_count: i64,
    connected_at: u64,
    created_at: String,
    online: bool,
}

fn supports_project_cli(allowed_clis: &[String]) -> bool {
    allowed_clis
        .iter()
        .any(|cli| cli.eq_ignore_ascii_case("copilot") || cli.eq_ignore_ascii_case("codex"))
}

fn clean_string(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn display_node_name(label: &str, device_name: Option<&str>, short_id: &str) -> String {
    clean_string(Some(label))
        .or_else(|| clean_string(device_name))
        .unwrap_or_else(|| short_id.to_string())
}

fn short_node_id(id: &str) -> String {
    let chars: Vec<char> = id.chars().collect();
    if chars.len() > 16 {
        let tail: String = chars[chars.len() - 14..].iter().collect();
        format!("...{tail}")
    } else {
        id.to_string()
    }
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
    if let Err(msg) = crate::billing::check_can_call(&state.store, &user.id) {
        return (
            StatusCode::PAYMENT_REQUIRED,
            Json(serde_json::json!({"error": msg})),
        )
            .into_response();
    }

    let max_output_tokens = req.max_tokens.unwrap_or(1024) as i64;
    // 找节点、发起请求
    let (req_id, node_id, mut rx) = match crate::node_router::dispatch_to_node(
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
        &format!("node_llm:{req_id}"),
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
    crate::node_router::settle_after_stream(
        &state,
        &user.id,
        Some(&req_id),
        Some(&owner),
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

/// GET /api/node-agent/version — 返回最新 node-agent Windows exe 的版本信息
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

/// GET /api/node-agent/download/windows — 下载最新 Windows exe
/// 不需要登录（执行文件不含敏感信息）
pub async fn download_node_agent_windows(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    download_node_agent_binary(state, "elon-node-agent.exe", "elon-node-agent.exe").await
}

/// GET /api/node-agent/download/linux — 下载最新 Linux 可执行文件
/// 不需要登录（执行文件不含敏感信息）
pub async fn download_node_agent_linux(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    download_node_agent_binary(state, "elon-node-agent", "elon-node-agent").await
}

async fn download_node_agent_binary(
    state: Arc<AppState>,
    file_name: &'static str,
    download_name: &'static str,
) -> axum::response::Response {
    let exe_path = state.data_dir.join("downloads").join(file_name);
    match tokio::fs::read(&exe_path).await {
        Ok(bytes) => axum::response::Response::builder()
            .status(StatusCode::OK)
            .header("content-type", "application/octet-stream")
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
