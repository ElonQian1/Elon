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
    store::{
        NodeComputePluginSharingConsentRequest, NodeComputeSharingPolicy,
        UpdateNodeComputeSharingPolicy,
    },
    types::AppState,
};

#[derive(Debug, Deserialize)]
pub struct UpdateNodeComputeSharingRequest {
    pub enabled: Option<bool>,
    pub plugin_runtime_requested: Option<bool>,
    pub expected_plugin_policy_revision: Option<i64>,
    pub plugin_consent_request_id: Option<String>,
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
    let enabled = request.enabled.unwrap_or(current.enabled);
    let plugin_consent = match request.plugin_runtime_requested {
        Some(plugin_runtime_requested) => {
            let Some(expected_policy_revision) = request.expected_plugin_policy_revision else {
                return error_response(
                    StatusCode::BAD_REQUEST,
                    "显式更新算力插件意愿必须携带期望策略修订号",
                );
            };
            let Some(consent_request_id) = request.plugin_consent_request_id else {
                return error_response(
                    StatusCode::BAD_REQUEST,
                    "显式更新算力插件意愿必须携带同意请求编号",
                );
            };
            Some(NodeComputePluginSharingConsentRequest {
                plugin_runtime_requested,
                expected_policy_revision,
                consent_request_id,
            })
        }
        None => {
            if request.expected_plugin_policy_revision.is_some()
                || request.plugin_consent_request_id.is_some()
            {
                return error_response(
                    StatusCode::BAD_REQUEST,
                    "未更新算力插件意愿时不能单独携带同意修订字段",
                );
            }
            None
        }
    };
    let update = UpdateNodeComputeSharingPolicy {
        enabled,
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
    let outcome = match state
        .store
        .update_node_compute_sharing_policy_with_plugin_runtime(
            &user.id,
            &credential.agent_id,
            update,
            plugin_consent,
        ) {
        Ok(outcome) => outcome,
        Err(error) => {
            let message = error.to_string();
            let status = if message.contains("修订号已变化") || message.contains("请求编号不能改变")
            {
                StatusCode::CONFLICT
            } else {
                StatusCode::BAD_REQUEST
            };
            return error_response(status, message);
        }
    };
    if let Some(intent) = outcome.dispatch_intent {
        let notify_state = Arc::clone(&state);
        tokio::spawn(async move {
            crate::homecli_agent::dispatch_durable_compute_plugin_sharing_intent(
                &notify_state,
                &intent,
            )
            .await;
        });
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
    let plugin_runtime_control = match state
        .store
        .node_compute_plugin_sharing_control_summary(node_id)
    {
        Ok(summary) => summary,
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
        "plugin_runtime_control": plugin_runtime_control,
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
