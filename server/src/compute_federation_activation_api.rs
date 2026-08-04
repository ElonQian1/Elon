use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use serde_json::json;

use crate::{
    compute_federation_activation_service::{
        self, CancelMyComputeActivationEvidenceRequest, ReviewComputeActivationEvidenceRequestBody,
        SubmitMyComputeActivationEvidenceRequest,
    },
    project_auth::{auth_from_headers, json_error},
    types::AppState,
};

pub(crate) fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/api/me/compute/providers/:provider_id/capacity-pools/:pool_id/activation-evidence-requests",
            get(list_my_requests).post(submit_my_request),
        )
        .route(
            "/api/me/compute/providers/:provider_id/capacity-pools/:pool_id/activation-evidence-requests/:request_id",
            get(get_my_request),
        )
        .route(
            "/api/me/compute/providers/:provider_id/capacity-pools/:pool_id/activation-evidence-requests/:request_id/cancel",
            post(cancel_my_request),
        )
        .route(
            "/api/me/compute/providers/:provider_id/capacity-pools/:pool_id/activation-evidence-requests/:request_id/preflight",
            get(preflight_my_request),
        )
        .route(
            "/api/admin/compute/activation-evidence-requests",
            get(list_reviewable_requests),
        )
        .route(
            "/api/admin/compute/activation-evidence-requests/:request_id/review",
            post(review_request),
        )
        .route(
            "/api/admin/compute/activation-evidence-requests/:request_id/preflight",
            get(preflight_review_request),
        )
}

#[derive(Debug, Deserialize)]
struct ListQuery {
    #[serde(default = "default_limit")]
    limit: usize,
}

#[derive(Debug, Deserialize)]
struct ReviewQueueQuery {
    #[serde(default = "default_review_status")]
    status: String,
    #[serde(default = "default_limit")]
    limit: usize,
}

async fn submit_my_request(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((provider_id, pool_id)): Path<(String, String)>,
    Json(request): Json<SubmitMyComputeActivationEvidenceRequest>,
) -> Response {
    let user_id = match authenticated_user(&state, &headers) {
        Ok(value) => value,
        Err(response) => return response,
    };
    activation_response(compute_federation_activation_service::submit_for_user(
        &state.store,
        &user_id,
        &provider_id,
        &pool_id,
        request,
    ))
}

async fn list_my_requests(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((provider_id, pool_id)): Path<(String, String)>,
    Query(query): Query<ListQuery>,
) -> Response {
    let user_id = match authenticated_user(&state, &headers) {
        Ok(value) => value,
        Err(response) => return response,
    };
    activation_response(
        compute_federation_activation_service::list_for_user(
            &state.store,
            &user_id,
            &provider_id,
            &pool_id,
            query.limit,
        )
        .map(|items| json!({"activation_evidence_requests":items})),
    )
}

async fn get_my_request(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((provider_id, pool_id, request_id)): Path<(String, String, String)>,
) -> Response {
    let user_id = match authenticated_user(&state, &headers) {
        Ok(value) => value,
        Err(response) => return response,
    };
    activation_response(compute_federation_activation_service::get_for_user(
        &state.store,
        &user_id,
        &provider_id,
        &pool_id,
        &request_id,
    ))
}

async fn cancel_my_request(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((provider_id, pool_id, request_id)): Path<(String, String, String)>,
    Json(request): Json<CancelMyComputeActivationEvidenceRequest>,
) -> Response {
    let user_id = match authenticated_user(&state, &headers) {
        Ok(value) => value,
        Err(response) => return response,
    };
    activation_response(compute_federation_activation_service::cancel_for_user(
        &state.store,
        &user_id,
        &provider_id,
        &pool_id,
        &request_id,
        request,
    ))
}

async fn preflight_my_request(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((provider_id, pool_id, request_id)): Path<(String, String, String)>,
) -> Response {
    let user_id = match authenticated_user(&state, &headers) {
        Ok(value) => value,
        Err(response) => return response,
    };
    activation_response(compute_federation_activation_service::preflight_for_user(
        &state.store,
        &user_id,
        &provider_id,
        &pool_id,
        &request_id,
    ))
}

async fn list_reviewable_requests(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(query): Query<ReviewQueueQuery>,
) -> Response {
    if let Err(response) = platform_admin(&state, &headers) {
        return response;
    }
    activation_response(
        compute_federation_activation_service::list_for_review(
            &state.store,
            &query.status,
            query.limit,
        )
        .map(|items| json!({"activation_evidence_requests":items})),
    )
}

async fn review_request(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(request_id): Path<String>,
    Json(request): Json<ReviewComputeActivationEvidenceRequestBody>,
) -> Response {
    let reviewer_user_id = match platform_admin(&state, &headers) {
        Ok(value) => value,
        Err(response) => return response,
    };
    activation_response(compute_federation_activation_service::review(
        &state.store,
        &reviewer_user_id,
        &request_id,
        request,
    ))
}

async fn preflight_review_request(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(request_id): Path<String>,
) -> Response {
    if let Err(response) = platform_admin(&state, &headers) {
        return response;
    }
    activation_response(compute_federation_activation_service::preflight_for_review(
        &state.store,
        &request_id,
    ))
}

fn authenticated_user(state: &AppState, headers: &HeaderMap) -> Result<String, Response> {
    auth_from_headers(state, headers)
        .map(|user| user.id)
        .map_err(|error| json_error(StatusCode::UNAUTHORIZED, error))
}

fn platform_admin(state: &AppState, headers: &HeaderMap) -> Result<String, Response> {
    let user = auth_from_headers(state, headers)
        .map_err(|error| json_error(StatusCode::UNAUTHORIZED, error))?;
    if !matches!(user.role.as_str(), "admin" | "owner") {
        return Err(json_error(
            StatusCode::FORBIDDEN,
            "只有平台管理员可以审核算力激活证据申请",
        ));
    }
    Ok(user.id)
}

fn activation_response<T: serde::Serialize>(result: anyhow::Result<T>) -> Response {
    match result {
        Ok(value) => Json(value).into_response(),
        Err(error) => json_error(StatusCode::BAD_REQUEST, error),
    }
}

fn default_limit() -> usize {
    20
}

fn default_review_status() -> String {
    "submitted".to_string()
}
