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
use homecli_proto::{NodeDevRuntimeProfile, NodeHardwareProfile, NodeStorageProfile};
use std::{collections::HashMap, sync::Arc};

pub use crate::node_register_api::register_node;
use crate::{
    admin,
    node_runtime::{
        clean_string, display_node_name, short_node_id, supports_project_cli, NodeRuntime,
    },
    pc_node_capacity::{assess_pc_node_capacity, PcNodeCapacity},
    project_auth::auth_from_headers,
    types::AppState,
};
use serde::{Deserialize, Serialize};
mod my_nodes;
mod payouts;
mod public_dev;
mod responses;
mod usage;
pub use my_nodes::my_nodes;
use payouts::node_payout_min_fen;
pub use payouts::{cancel_node_payout, create_node_payout, my_node_payouts};
pub use public_dev::{admin_public_dev_handshake, update_my_node_sharing};
use public_dev::{public_dev_handshake_state, public_dev_handshake_value};
use responses::PublicNodeResponse;
pub use usage::my_node_usage;
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
        let credential = state.store.get_node_credential(&node_id).ok().flatten();
        let agent_version = cli_agent.as_ref().map(|agent| agent.version.clone());
        let (public_dev_handshake_ready, public_dev_handshake_status) = public_dev_handshake_state(
            credential.as_ref(),
            node.online || cli_agent.is_some(),
            agent_version.as_deref(),
            &allowed_clis,
            dev_runtime.as_ref(),
        );
        nodes.push(PublicNodeResponse {
            agent_id: node_id.clone(),
            node_id: node_id.clone(),
            owner_user_id: node.owner_user_id,
            device_name,
            hardware,
            hardware_summary,
            storage: node.storage.clone(),
            dev_runtime,
            lifecycle: node
                .lifecycle
                .clone()
                .or_else(|| cli_agent.as_ref().and_then(|agent| agent.lifecycle.clone())),
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
            agent_version,
            public_dev_enabled: credential
                .as_ref()
                .map(|credential| credential.public_dev_enabled)
                .unwrap_or(false),
            public_dev_allowed_clis: credential
                .as_ref()
                .map(|credential| credential.public_dev_allowed_clis.clone())
                .unwrap_or_default(),
            public_dev_permission_level: credential
                .as_ref()
                .map(|credential| credential.public_dev_permission_level.clone())
                .unwrap_or_else(|| "project_write".to_string()),
            public_dev_handshake_ready,
            public_dev_handshake_status,
            last_handshake_at: credential
                .as_ref()
                .and_then(|credential| credential.last_handshake_at.clone()),
            last_handshake_agent_version: credential
                .as_ref()
                .and_then(|credential| credential.last_handshake_agent_version.clone()),
            last_handshake_allowed_clis: credential
                .as_ref()
                .map(|credential| credential.last_handshake_allowed_clis.clone())
                .unwrap_or_default(),
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
        let credential = state.store.get_node_credential(&node_id).ok().flatten();
        let agent_version = Some(agent.version.clone());
        let (public_dev_handshake_ready, public_dev_handshake_status) = public_dev_handshake_state(
            credential.as_ref(),
            true,
            agent_version.as_deref(),
            &allowed_clis,
            dev_runtime.as_ref(),
        );
        nodes.push(PublicNodeResponse {
            agent_id: node_id.clone(),
            node_id: node_id.clone(),
            owner_user_id,
            device_name,
            hardware,
            hardware_summary,
            storage: agent.storage.clone(),
            dev_runtime,
            lifecycle: agent.lifecycle.clone(),
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
            agent_version,
            public_dev_enabled: credential
                .as_ref()
                .map(|credential| credential.public_dev_enabled)
                .unwrap_or(false),
            public_dev_allowed_clis: credential
                .as_ref()
                .map(|credential| credential.public_dev_allowed_clis.clone())
                .unwrap_or_default(),
            public_dev_permission_level: credential
                .as_ref()
                .map(|credential| credential.public_dev_permission_level.clone())
                .unwrap_or_else(|| "project_write".to_string()),
            public_dev_handshake_ready,
            public_dev_handshake_status,
            last_handshake_at: credential
                .as_ref()
                .and_then(|credential| credential.last_handshake_at.clone()),
            last_handshake_agent_version: credential
                .as_ref()
                .and_then(|credential| credential.last_handshake_agent_version.clone()),
            last_handshake_allowed_clis: credential
                .as_ref()
                .map(|credential| credential.last_handshake_allowed_clis.clone())
                .unwrap_or_default(),
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
        install_id: None,
        public_dev_enabled: false,
        public_dev_allowed_clis: Vec::new(),
        public_dev_permission_level: "project_write".to_string(),
        last_handshake_at: None,
        last_handshake_agent_version: None,
        last_handshake_allowed_clis: Vec::new(),
        last_handshake_route_a_ready: false,
        last_handshake_api_runtime_ready: false,
        last_handshake_server_runtime_ready: false,
        last_handshake_ai_cli_ready: false,
        hardware: None,
        storage: None,
        dev_runtime,
        lifecycle: None,
        display_name: display_name.to_string(),
        short_id: short_node_id(node_id),
        models: Vec::new(),
        allowed_clis: allowed_clis.to_vec(),
        allowed_cwds: Vec::new(),
        agent_version: None,
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
    let version = tokio::fs::read_to_string(&version_file)
        .await
        .ok()
        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
        .and_then(|v| v["version"].as_str().map(str::to_string));
    let count = state
        .agent_manager
        .broadcast_update_client(version.clone(), None)
        .await;
    match public_dev_handshake_value(&state).await {
        Ok(report) => Json(serde_json::json!({
            "ok": true,
            "broadcast_to": count,
            "version": version,
            "message": format!("{count} 个在线节点已收到更新指令"),
            "public_dev_handshake": report,
        }))
        .into_response(),
        Err(e) => Json(serde_json::json!({
            "ok": true,
            "broadcast_to": count,
            "version": version,
            "message": format!("{count} 个在线节点已收到更新指令"),
            "public_dev_handshake_error": e.to_string(),
        }))
        .into_response(),
    }
}
