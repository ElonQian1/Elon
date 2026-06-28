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
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use homecli_proto::{
    ModelCapability, NodeDevRuntimeProfile, NodeHardwareProfile, NodeStorageProfile,
};
use sha2::Digest as _;
use std::{collections::HashMap, sync::Arc};

use crate::{
    node_runtime::{
        clean_string, display_node_name, short_node_id, supports_project_cli, user_node_runtimes,
        NodeRuntime,
    },
    pc_node_capacity::{assess_pc_node_capacity, PcNodeCapacity},
    project_auth::auth_from_headers,
    store::CreateNodePayout,
    types::AppState,
};
use serde::{Deserialize, Serialize};

fn storage_can_cross_pc(storage: &NodeStorageProfile) -> bool {
    storage
        .git_base_url
        .as_ref()
        .map(|value| !value.trim().is_empty())
        .unwrap_or(false)
        || storage.relay_git_url_enabled
}

// ── /api/nodes ────────────────────────────────────────────────────────────────

/// GET /api/nodes — 列出所有用户可发现的在线节点（含在线状态）
/// 需要有效用户 token（不要求管理员权限，普通用户可见）
pub async fn list_nodes(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let user = match auth_from_headers(&state, &headers) {
        Ok(user) => user,
        Err(e) => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({"error": e.to_string()})),
            )
                .into_response()
        }
    };
    let project_counts = project_counts_for_user(&state, &user.id);
    let mut cli_by_id: HashMap<_, _> = state
        .agent_manager
        .list()
        .await
        .into_iter()
        .map(|agent| (agent.agent_id.clone(), agent))
        .collect();

    let mut nodes = Vec::new();
    for node in state.node_registry.list_online().await {
        let node_id = node.node_id.clone();
        let cli_agent = cli_by_id.remove(&node_id);
        let allowed_clis = cli_agent
            .as_ref()
            .map(|agent| agent.allowed_clis.clone())
            .unwrap_or_default();
        let dev_runtime = node.dev_runtime.clone().or_else(|| {
            cli_agent
                .as_ref()
                .and_then(|agent| agent.dev_runtime.clone())
        });
        let cli_project_ready = supports_project_cli(&allowed_clis);
        let workspace_provision_ready = dev_runtime
            .as_ref()
            .map(|runtime| runtime.workspace_provision_ready)
            .unwrap_or(cli_project_ready);
        let ai_cli_ready = dev_runtime
            .as_ref()
            .map(|runtime| runtime.ai_cli_ready)
            .unwrap_or(cli_project_ready);
        let (route_a_ready, api_runtime_ready, server_runtime_ready) =
            runtime_route_flags(dev_runtime.as_ref(), cli_project_ready);
        let short_id = short_node_id(&node_id);
        let device_name = clean_string(node.device_name.as_deref()).or_else(|| {
            cli_agent
                .as_ref()
                .and_then(|agent| clean_string(agent.device_name.as_deref()))
        });
        let display_name = display_node_name("", device_name.as_deref(), &short_id);
        let project_count = state
            .store
            .count_active_pc_projects_for_node(&node_id)
            .unwrap_or_else(|_| project_counts.get(&node_id).copied().unwrap_or(0));
        let capacity = capacity_for_response(
            &state,
            &node_id,
            &node.owner_user_id,
            "",
            device_name.as_deref(),
            &display_name,
            node.online || cli_agent.is_some(),
            cli_agent.is_some(),
            &allowed_clis,
            dev_runtime.clone(),
            project_count,
        );
        let hardware = hardware_for_response(&state, &node_id, node.hardware);
        let hardware_summary = hardware_summary(hardware.as_ref());
        nodes.push(PublicNodeResponse {
            agent_id: node_id.clone(),
            node_id: node_id.clone(),
            owner_user_id: node.owner_user_id,
            device_name,
            hardware,
            hardware_summary,
            storage: node.storage.clone(),
            dev_runtime,
            storage_ready: node
                .storage
                .as_ref()
                .map(|storage| storage.enabled)
                .unwrap_or(false),
            storage_repo_url_configured: node
                .storage
                .as_ref()
                .map(storage_can_cross_pc)
                .unwrap_or(false),
            display_name,
            short_id,
            models: node.models,
            allowed_clis: allowed_clis.clone(),
            cli_project_ready,
            workspace_provision_ready,
            ai_cli_ready,
            route_a_ready,
            api_runtime_ready,
            server_runtime_ready,
            project_count,
            project_limit: capacity.project_limit,
            project_slots_remaining: capacity.project_slots_remaining,
            disk_free_bytes: capacity.disk_free_bytes,
            can_accept_project: capacity.can_accept_project,
            capacity_label: capacity.label,
            capacity_tone: capacity.tone,
            capacity_warnings: capacity.warnings,
            tts_worker_url: node.tts_worker_url,
            connected_at: node.connected_at,
            online: node.online || cli_agent.is_some(),
        });
    }

    for agent in cli_by_id.into_values() {
        let node_id = agent.agent_id.clone();
        let allowed_clis = agent.allowed_clis.clone();
        let dev_runtime = agent.dev_runtime.clone();
        let cli_project_ready = supports_project_cli(&allowed_clis);
        let workspace_provision_ready = dev_runtime
            .as_ref()
            .map(|runtime| runtime.workspace_provision_ready)
            .unwrap_or(cli_project_ready);
        let ai_cli_ready = dev_runtime
            .as_ref()
            .map(|runtime| runtime.ai_cli_ready)
            .unwrap_or(cli_project_ready);
        let (route_a_ready, api_runtime_ready, server_runtime_ready) =
            runtime_route_flags(dev_runtime.as_ref(), cli_project_ready);
        let short_id = short_node_id(&node_id);
        let device_name = clean_string(agent.device_name.as_deref());
        let display_name = display_node_name("", device_name.as_deref(), &short_id);
        let owner_user_id = state
            .store
            .get_node_credential_owner(&node_id)
            .ok()
            .flatten()
            .unwrap_or_default();
        let project_count = state
            .store
            .count_active_pc_projects_for_node(&node_id)
            .unwrap_or_else(|_| project_counts.get(&node_id).copied().unwrap_or(0));
        let capacity = capacity_for_response(
            &state,
            &node_id,
            &owner_user_id,
            "",
            device_name.as_deref(),
            &display_name,
            true,
            true,
            &allowed_clis,
            dev_runtime.clone(),
            project_count,
        );
        let hardware = hardware_for_response(&state, &node_id, agent.hardware);
        let hardware_summary = hardware_summary(hardware.as_ref());
        nodes.push(PublicNodeResponse {
            agent_id: node_id.clone(),
            node_id: node_id.clone(),
            owner_user_id,
            device_name,
            hardware,
            hardware_summary,
            storage: agent.storage.clone(),
            dev_runtime,
            storage_ready: agent
                .storage
                .as_ref()
                .map(|storage| storage.enabled)
                .unwrap_or(false),
            storage_repo_url_configured: agent
                .storage
                .as_ref()
                .map(storage_can_cross_pc)
                .unwrap_or(false),
            display_name,
            short_id,
            models: Vec::new(),
            allowed_clis: allowed_clis.clone(),
            cli_project_ready,
            workspace_provision_ready,
            ai_cli_ready,
            route_a_ready,
            api_runtime_ready,
            server_runtime_ready,
            project_count,
            project_limit: capacity.project_limit,
            project_slots_remaining: capacity.project_slots_remaining,
            disk_free_bytes: capacity.disk_free_bytes,
            can_accept_project: capacity.can_accept_project,
            capacity_label: capacity.label,
            capacity_tone: capacity.tone,
            capacity_warnings: capacity.warnings,
            tts_worker_url: None,
            connected_at: agent.connected_at,
            online: true,
        });
    }

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

// ── /api/me/node-payouts ─────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct CreateNodePayoutBody {
    pub amount_fen: Option<i64>,
    pub amount_credits: Option<f64>,
    pub payout_method: String,
    pub payout_account: String,
    pub contact: Option<String>,
}

/// GET /api/me/node-payouts — 当前用户的节点收益提现申请。
pub async fn my_node_payouts(
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

    match state.store.list_node_payout_requests(&user.id, 50) {
        Ok(payouts) => Json(serde_json::json!({ "payouts": payouts })).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

/// POST /api/me/node-payouts — 申请提现，申请成功后立即冻结节点可用余额。
pub async fn create_node_payout(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<CreateNodePayoutBody>,
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
    let amount_fen = match payout_amount_fen(&body) {
        Some(amount) => amount,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "提现金额必须大于 0"})),
            )
                .into_response()
        }
    };
    let min_fen = node_payout_min_fen(&state);
    if amount_fen < min_fen {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": format!("提现金额不能低于 ¥{:.2}", min_fen as f64 / 100.0),
                "payout_min_fen": min_fen
            })),
        )
            .into_response();
    }

    match state.store.create_node_payout_request(CreateNodePayout {
        provider_user_id: &user.id,
        amount_fen,
        payout_method: &body.payout_method,
        payout_account: &body.payout_account,
        contact: body.contact.as_deref(),
    }) {
        Ok(payout) => {
            let balance = state.store.get_node_balance(&user.id).unwrap_or(0.0);
            Json(serde_json::json!({ "ok": true, "payout": payout, "balance": balance }))
                .into_response()
        }
        Err(e) => node_payout_error_response(e),
    }
}

/// POST /api/me/node-payouts/:payout_id/cancel — 取消待处理提现并退回冻结余额。
pub async fn cancel_node_payout(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(payout_id): Path<String>,
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
    match state.store.cancel_node_payout_request(&user.id, &payout_id) {
        Ok(payout) => {
            let balance = state.store.get_node_balance(&user.id).unwrap_or(0.0);
            Json(serde_json::json!({ "ok": true, "payout": payout, "balance": balance }))
                .into_response()
        }
        Err(e) => node_payout_error_response(e),
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

    let runtimes = match user_node_runtimes(&state, &user.id).await {
        Ok(nodes) => nodes,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": e.to_string()})),
            )
                .into_response()
        }
    };

    let nodes = runtimes
        .into_iter()
        .map(|node| {
            let cli_project_ready = node.cli_project_ready();
            let workspace_provision_ready = node.workspace_provision_ready();
            let ai_cli_ready = node
                .dev_runtime
                .as_ref()
                .map(|runtime| runtime.ai_cli_ready)
                .unwrap_or(cli_project_ready);
            let (route_a_ready, api_runtime_ready, server_runtime_ready) =
                runtime_route_flags(node.dev_runtime.as_ref(), cli_project_ready);
            let global_project_count = state
                .store
                .count_active_pc_projects_for_node(&node.node_id)
                .unwrap_or(node.project_count);
            let mut capacity_node = node.clone();
            capacity_node.project_count = global_project_count;
            let latest_snapshot = state
                .store
                .latest_workspace_health_snapshot_for_node(&node.node_id)
                .ok()
                .flatten();
            let capacity = assess_pc_node_capacity(&capacity_node, latest_snapshot.as_ref());
            let storage = node.storage.clone();
            let storage_ready = node.storage_ready();
            let storage_repo_url_configured =
                storage.as_ref().map(storage_can_cross_pc).unwrap_or(false);
            let hardware = hardware_for_response(&state, &node.node_id, node.hardware);
            let hardware_summary = hardware_summary(hardware.as_ref());
            MyNodeResponse {
                agent_id: node.node_id.clone(),
                node_id: node.node_id,
                owner_user_id: node.owner_user_id,
                label: node.label,
                device_name: node.device_name,
                hardware,
                hardware_summary,
                storage,
                dev_runtime: node.dev_runtime,
                storage_ready,
                storage_repo_url_configured,
                display_name: node.display_name,
                short_id: node.short_id,
                models: node.models,
                allowed_clis: node.allowed_clis,
                allowed_cwds: node.allowed_cwds,
                cli_project_ready,
                workspace_provision_ready,
                ai_cli_ready,
                route_a_ready,
                api_runtime_ready,
                server_runtime_ready,
                project_count: capacity.project_count,
                project_limit: capacity.project_limit,
                project_slots_remaining: capacity.project_slots_remaining,
                disk_free_bytes: capacity.disk_free_bytes,
                can_accept_project: capacity.can_accept_project,
                capacity_label: capacity.label,
                capacity_tone: capacity.tone,
                capacity_warnings: capacity.warnings,
                connected_at: node.connected_at,
                created_at: node.created_at,
                online: node.online,
                registry_online: node.registry_online,
                cli_connected: node.cli_connected,
            }
        })
        .collect::<Vec<_>>();

    Json(serde_json::json!({ "nodes": nodes })).into_response()
}

#[derive(Serialize)]
struct PublicNodeResponse {
    agent_id: String,
    node_id: String,
    owner_user_id: String,
    device_name: Option<String>,
    hardware: Option<NodeHardwareProfile>,
    hardware_summary: String,
    storage: Option<NodeStorageProfile>,
    dev_runtime: Option<NodeDevRuntimeProfile>,
    storage_ready: bool,
    storage_repo_url_configured: bool,
    display_name: String,
    short_id: String,
    models: Vec<ModelCapability>,
    allowed_clis: Vec<String>,
    cli_project_ready: bool,
    workspace_provision_ready: bool,
    ai_cli_ready: bool,
    route_a_ready: bool,
    api_runtime_ready: bool,
    server_runtime_ready: bool,
    project_count: i64,
    project_limit: i64,
    project_slots_remaining: i64,
    disk_free_bytes: Option<u64>,
    can_accept_project: bool,
    capacity_label: String,
    capacity_tone: String,
    capacity_warnings: Vec<String>,
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
    hardware: Option<NodeHardwareProfile>,
    hardware_summary: String,
    storage: Option<NodeStorageProfile>,
    dev_runtime: Option<NodeDevRuntimeProfile>,
    storage_ready: bool,
    storage_repo_url_configured: bool,
    display_name: String,
    short_id: String,
    models: Vec<ModelCapability>,
    allowed_clis: Vec<String>,
    allowed_cwds: Vec<String>,
    cli_project_ready: bool,
    workspace_provision_ready: bool,
    ai_cli_ready: bool,
    route_a_ready: bool,
    api_runtime_ready: bool,
    server_runtime_ready: bool,
    project_count: i64,
    project_limit: i64,
    project_slots_remaining: i64,
    disk_free_bytes: Option<u64>,
    can_accept_project: bool,
    capacity_label: String,
    capacity_tone: String,
    capacity_warnings: Vec<String>,
    connected_at: u64,
    created_at: String,
    online: bool,
    registry_online: bool,
    cli_connected: bool,
}

fn runtime_route_flags(
    runtime: Option<&NodeDevRuntimeProfile>,
    legacy_cli_ready: bool,
) -> (bool, bool, bool) {
    runtime
        .map(|runtime| {
            (
                runtime.route_a_ready,
                runtime.api_runtime_ready,
                runtime.server_runtime_ready,
            )
        })
        .unwrap_or((legacy_cli_ready, false, false))
}

fn project_counts_for_user(state: &AppState, user_id: &str) -> HashMap<String, i64> {
    match state.store.list_projects_for_user(user_id) {
        Ok(projects) => projects
            .into_iter()
            .filter_map(|project| project.node_id)
            .fold(HashMap::<String, i64>::new(), |mut counts, node_id| {
                *counts.entry(node_id).or_insert(0) += 1;
                counts
            }),
        Err(e) => {
            tracing::warn!(user_id = %user_id, error = %e, "failed to count user projects per node");
            HashMap::new()
        }
    }
}

fn capacity_for_response(
    state: &AppState,
    node_id: &str,
    owner_user_id: &str,
    label: &str,
    device_name: Option<&str>,
    display_name: &str,
    online: bool,
    cli_connected: bool,
    allowed_clis: &[String],
    dev_runtime: Option<NodeDevRuntimeProfile>,
    project_count: i64,
) -> PcNodeCapacity {
    let latest_snapshot = state
        .store
        .latest_workspace_health_snapshot_for_node(node_id)
        .ok()
        .flatten();
    let runtime = NodeRuntime {
        node_id: node_id.to_string(),
        owner_user_id: owner_user_id.to_string(),
        label: label.to_string(),
        device_name: device_name.map(ToOwned::to_owned),
        hardware: None,
        storage: None,
        dev_runtime,
        display_name: display_name.to_string(),
        short_id: short_node_id(node_id),
        models: Vec::new(),
        allowed_clis: allowed_clis.to_vec(),
        allowed_cwds: Vec::new(),
        connected_at: 0,
        created_at: String::new(),
        online,
        registry_online: online,
        cli_connected,
        project_count,
    };
    assess_pc_node_capacity(&runtime, latest_snapshot.as_ref())
}

fn hardware_for_response(
    state: &AppState,
    node_id: &str,
    live: Option<NodeHardwareProfile>,
) -> Option<NodeHardwareProfile> {
    live.or_else(|| {
        state
            .store
            .get_node_hardware_snapshot(node_id)
            .ok()
            .flatten()
            .map(|snapshot| snapshot.hardware)
    })
}

fn hardware_summary(profile: Option<&NodeHardwareProfile>) -> String {
    let Some(profile) = profile else {
        return "硬件未知".to_string();
    };
    let mut parts = Vec::new();
    if !profile.gpu_names.is_empty() {
        parts.push(format!("GPU {}", profile.gpu_names.join(" / ")));
    }
    if let Some(bytes) = profile.gpu_memory_total_bytes.and_then(format_bytes) {
        parts.push(format!("显存 {bytes}"));
    }
    if let Some(bytes) = profile.memory_total_bytes.and_then(format_bytes) {
        parts.push(format!("内存 {bytes}"));
    }
    if let Some(cores) = profile.cpu_cores.filter(|cores| *cores > 0) {
        parts.push(format!("CPU {cores} 核"));
    } else if let Some(cpu) = profile
        .cpu_brand
        .as_deref()
        .filter(|value| !value.trim().is_empty())
    {
        parts.push(cpu.trim().to_string());
    }
    if parts.is_empty() {
        "硬件未知".to_string()
    } else {
        parts.join(" · ")
    }
}

fn format_bytes(bytes: u64) -> Option<String> {
    if bytes == 0 {
        return None;
    }
    let units = ["B", "KB", "MB", "GB", "TB"];
    let mut value = bytes as f64;
    let mut idx = 0usize;
    while value >= 1024.0 && idx < units.len() - 1 {
        value /= 1024.0;
        idx += 1;
    }
    Some(if idx >= 3 {
        format!("{value:.1} {}", units[idx])
    } else {
        format!("{} {}", value.round() as u64, units[idx])
    })
}

fn payout_amount_fen(body: &CreateNodePayoutBody) -> Option<i64> {
    if let Some(amount_fen) = body.amount_fen {
        return (amount_fen > 0).then_some(amount_fen);
    }
    let credits = body.amount_credits?;
    if !credits.is_finite() || credits <= 0.0 {
        return None;
    }
    Some((credits * 100.0).round() as i64)
}

fn node_payout_min_fen(state: &AppState) -> i64 {
    state
        .store
        .billing_get_config("node_payout_min_fen")
        .ok()
        .flatten()
        .and_then(|value| value.parse::<i64>().ok())
        .unwrap_or(100)
        .max(1)
}

fn node_payout_error_response(err: anyhow::Error) -> axum::response::Response {
    let msg = err.to_string();
    let status = if msg.contains("不存在") {
        StatusCode::NOT_FOUND
    } else if msg.contains("余额不足")
        || msg.contains("待处理")
        || msg.contains("不能为空")
        || msg.contains("必须大于")
    {
        StatusCode::BAD_REQUEST
    } else {
        StatusCode::INTERNAL_SERVER_ERROR
    };
    (
        status,
        Json(serde_json::json!({
            "error": msg
        })),
    )
        .into_response()
}

// ── /api/nodes/chat ───────────────────────────────────────────────────────────

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
    let (req_id, node_id, mut rx) = match crate::node_router::dispatch_to_node_with_req_id(
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

/// GET /api/node-agent/download/linux — 下载最新 Linux 可执行文件
/// 不需要登录（执行文件不含敏感信息）
pub async fn download_node_agent_linux(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    download_node_agent_binary(state, "elon-pc-node", "elon-pc-node").await
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
    let presented = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .unwrap_or("");
    if presented.is_empty() || presented != state.admin_token {
        return (StatusCode::UNAUTHORIZED, Json(serde_json::json!({"error":"admin token required"}))).into_response();
    }
    let version_file = state.data_dir.join("downloads").join("node-agent-version.json");
    let version = tokio::fs::read_to_string(&version_file).await
        .ok()
        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
        .and_then(|v| v["version"].as_str().map(str::to_string));
    let count = state.agent_manager.broadcast_update_client(version.clone(), None).await;
    Json(serde_json::json!({
        "ok": true,
        "broadcast_to": count,
        "version": version,
        "message": format!("{count} 个在线节点已收到更新指令"),
    })).into_response()
}
