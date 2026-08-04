use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde_json::json;

use crate::{
    compute_federation_attempt_service::{self, ActivateMyComputeAttemptRequest},
    project_auth::{auth_from_headers, json_error},
    types::AppState,
};

pub(crate) fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/api/me/compute/providers/:provider_id/attempt-activations",
            post(activate),
        )
        .route(
            "/api/me/compute/attempt-leases/:lease_id/activation",
            get(get_activation),
        )
}

async fn activate(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(provider_id): Path<String>,
    Json(request): Json<ActivateMyComputeAttemptRequest>,
) -> Response {
    let user_id = match authenticated_user(&state, &headers) {
        Ok(user_id) => user_id,
        Err(response) => return response,
    };
    attempt_response(
        compute_federation_attempt_service::activate_for_provider_owner(
            &state.store,
            &user_id,
            &provider_id,
            request,
        ),
    )
}

async fn get_activation(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(lease_id): Path<String>,
) -> Response {
    let user_id = match authenticated_user(&state, &headers) {
        Ok(user_id) => user_id,
        Err(response) => return response,
    };
    attempt_response(compute_federation_attempt_service::get_for_participant(
        &state.store,
        &user_id,
        &lease_id,
    ))
}

fn authenticated_user(state: &AppState, headers: &HeaderMap) -> Result<String, Response> {
    auth_from_headers(state, headers)
        .map(|user| user.id)
        .map_err(|_| json_error(StatusCode::UNAUTHORIZED, "未登录"))
}

fn attempt_response<T: serde::Serialize>(result: anyhow::Result<T>) -> Response {
    match result {
        Ok(value) => Json(value).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(json!({"error":format!("{error:#}")})),
        )
            .into_response(),
    }
}
