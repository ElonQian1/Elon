use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use std::sync::Arc;

use crate::{
    project_auth::auth_from_headers,
    store::{NodeComputeSharingPolicy, UpdateNodeComputeSharingPolicy},
    types::AppState,
};

#[derive(Debug, Deserialize)]
pub struct UpdateNodeComputeSharingRequest {
    pub enabled: Option<bool>,
    pub allowed_model_ids: Option<Vec<String>>,
    pub max_concurrent_runs: Option<i64>,
    pub daily_token_limit: Option<i64>,
}

pub async fn get_my_node_compute_sharing(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(node_id): Path<String>,
) -> impl IntoResponse {
    let user = match auth_from_headers(&state, &headers) {
        Ok(user) => user,
        Err(error) => return error_response(StatusCode::UNAUTHORIZED, error.to_string()),
    };
    let credential = match owned_credential(&state, &user.id, &node_id) {
        Ok(credential) => credential,
        Err(response) => return response,
    };
    compute_sharing_response(&state, &credential.agent_id, &user.id).await
}

pub async fn update_my_node_compute_sharing(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(node_id): Path<String>,
    Json(request): Json<UpdateNodeComputeSharingRequest>,
) -> impl IntoResponse {
    let user = match auth_from_headers(&state, &headers) {
        Ok(user) => user,
        Err(error) => return error_response(StatusCode::UNAUTHORIZED, error.to_string()),
    };
    let credential = match owned_credential(&state, &user.id, &node_id) {
        Ok(credential) => credential,
        Err(response) => return response,
    };
    let current =
        match state
            .store
            .node_compute_sharing_status(&credential.agent_id, &user.id, None)
        {
            Ok(status) => status.policy,
            Err(error) => {
                return error_response(StatusCode::INTERNAL_SERVER_ERROR, error.to_string())
            }
        };
    let update = UpdateNodeComputeSharingPolicy {
        enabled: request.enabled.unwrap_or(current.enabled),
        allowed_model_ids: request
            .allowed_model_ids
            .unwrap_or(current.allowed_model_ids),
        max_concurrent_runs: request
            .max_concurrent_runs
            .unwrap_or(current.max_concurrent_runs),
        daily_token_limit: request
            .daily_token_limit
            .unwrap_or(current.daily_token_limit),
    };
    if let Err(error) =
        state
            .store
            .update_node_compute_sharing_policy(&user.id, &credential.agent_id, update)
    {
        return error_response(StatusCode::BAD_REQUEST, error.to_string());
    }
    compute_sharing_response(&state, &credential.agent_id, &user.id).await
}

async fn compute_sharing_response(
    state: &AppState,
    node_id: &str,
    owner_user_id: &str,
) -> axum::response::Response {
    let status = match state
        .store
        .node_compute_sharing_status(node_id, owner_user_id, None)
    {
        Ok(status) => status,
        Err(error) => return error_response(StatusCode::INTERNAL_SERVER_ERROR, error.to_string()),
    };
    let runtime_health = match state
        .store
        .node_compute_sharing_runtime_health(node_id, owner_user_id)
    {
        Ok(health) => health,
        Err(error) => return error_response(StatusCode::INTERNAL_SERVER_ERROR, error.to_string()),
    };
    let observed_models = state
        .node_registry
        .list_by_owner(owner_user_id)
        .await
        .into_iter()
        .find(|node| node.node_id == node_id)
        .map(|node| node.models)
        .unwrap_or_default();
    Json(serde_json::json!({
        "ok": true,
        "compute_sharing": status,
        "runtime_health": runtime_health,
        "observed_models": observed_models,
    }))
    .into_response()
}

fn owned_credential(
    state: &AppState,
    owner_user_id: &str,
    node_id: &str,
) -> Result<crate::store::NodeCredential, axum::response::Response> {
    match state.store.get_node_credential(node_id.trim()) {
        Ok(Some(credential)) if credential.owner_user_id == owner_user_id => Ok(credential),
        Ok(Some(_)) => Err(error_response(
            StatusCode::FORBIDDEN,
            "只能修改自己的节点算力共享策略",
        )),
        Ok(None) => Err(error_response(StatusCode::NOT_FOUND, "节点不存在")),
        Err(error) => Err(error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            error.to_string(),
        )),
    }
}

pub(super) fn disabled_status(
    node_id: &str,
    owner_user_id: &str,
) -> crate::store::NodeComputeSharingStatus {
    crate::store::NodeComputeSharingStatus {
        policy: NodeComputeSharingPolicy::disabled(node_id, owner_user_id),
        active_runs: 0,
        tokens_used_today: 0,
        tokens_reserved_today: 0,
        available: false,
        availability: "sharing_disabled".to_string(),
    }
}

fn error_response(status: StatusCode, message: impl Into<String>) -> axum::response::Response {
    (status, Json(serde_json::json!({ "error": message.into() }))).into_response()
}
