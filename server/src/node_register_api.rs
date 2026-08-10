use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use sha2::Digest as _;
use std::sync::Arc;

use crate::{
    project_auth::{auth_from_headers, bearer_token},
    store::node_credentials::{LegacyNodeRegistrationOutcome, LegacyNodeRegistrationRequest},
    types::AppState,
};

#[derive(Deserialize)]
pub struct RegisterNodeRequest {
    /// 用户给这个节点起的名字，如 "我的游戏 PC"
    pub label: Option<String>,
    /// PC 系统设备名，仅用于展示和旧节点压缩。
    pub device_name: Option<String>,
    /// Win 端安装实例 ID。相同账号 + 相同 install_id 必须复用同一个 agent_id。
    pub install_id: Option<String>,
    /// 已有节点 ID（重新登录时带上，服务器验证后复用，避免换 ID）
    pub existing_agent_id: Option<String>,
    /// 已有节点 secret（与 existing_agent_id 配套，用于验证所有权）
    pub existing_secret: Option<String>,
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

pub(crate) fn legacy_node_cloud_ws_url() -> String {
    format!(
        "ws://{}",
        std::env::var("ELON_PUBLIC_HOST").unwrap_or_else(|_| "43.139.149.158:8080".to_string())
    )
}

fn normalize_register_field(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

/// POST /api/me/nodes/register — 为当前用户生成一个新的 PC 节点凭证
///
/// 若请求中携带了 `existing_agent_id + existing_secret`，且它们属于当前用户，
/// 则只刷新 secret、保留原 agent_id（续约模式）；
/// 否则优先按 `install_id` 复用同一台 Win 端安装实例，最后才生成全新 agent_id。
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
    let current_bearer_token = bearer_token(&headers)
        .filter(|token| state.owner_token.as_deref() != Some(*token))
        .map(str::to_string);

    let new_secret = uuid::Uuid::new_v4().to_string().replace('-', "")
        + &uuid::Uuid::new_v4().to_string().replace('-', "");
    let new_secret_hash = hex::encode(sha2::Sha256::digest(new_secret.as_bytes()));

    let existing_agent_id = req
        .existing_agent_id
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let existing_secret_hash = req
        .existing_secret
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|secret| hex::encode(sha2::Sha256::digest(secret.as_bytes())));
    let install_id = normalize_register_field(req.install_id.as_deref());
    let device_name = normalize_register_field(req.device_name.as_deref());
    let label = normalize_register_field(req.label.as_deref());

    let random_suffix = uuid::Uuid::new_v4()
        .to_string()
        .replace('-', "")
        .chars()
        .take(8)
        .collect::<String>();
    let proposed_agent_id = format!(
        "node-{}-{}",
        &user.id.chars().take(6).collect::<String>(),
        random_suffix
    );
    let request = LegacyNodeRegistrationRequest::new(
        &user.id,
        &proposed_agent_id,
        &new_secret_hash,
        existing_agent_id,
        existing_secret_hash.as_deref(),
        install_id,
        label,
        device_name,
    );
    let request = match current_bearer_token.as_deref() {
        Some(bearer_token) => request.with_current_bearer_token(bearer_token),
        None => request,
    };
    let outcome = state
        .agent_manager
        .run_legacy_registration_and_close_process_session(&state, || {
            state
                .store
                .register_or_renew_legacy_node_credential(request)
        })
        .await;
    match outcome {
        Ok(LegacyNodeRegistrationOutcome::Renewed { agent_id })
        | Ok(LegacyNodeRegistrationOutcome::Created { agent_id }) => Json(RegisterNodeResponse {
            agent_id,
            agent_secret: new_secret,
            cloud_ws_url: legacy_node_cloud_ws_url(),
            owner_user_id: user.id,
        })
        .into_response(),
        Ok(LegacyNodeRegistrationOutcome::EndpointAuthorityRequired { endpoint_authority }) => (
            StatusCode::CONFLICT,
            Json(serde_json::json!({
                "error": "NODE_ENDPOINT_AUTHORITY_REQUIRED",
                "endpoint_authority": endpoint_authority,
            })),
        )
            .into_response(),
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("节点注册失败: {error}")})),
        )
            .into_response(),
    }
}
