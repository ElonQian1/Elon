use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::post,
    Json, Router,
};

use crate::{
    compute_federation_broker_service::{self, FinishMyComputeRequest, ReserveMyComputeRequest},
    project_auth::{auth_from_headers, json_error},
    store::ComputeBrokerFinishAction,
    types::AppState,
};

pub(crate) fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/me/compute/reservations", post(reserve))
        .route(
            "/api/me/compute/reservations/:reservation_id/release",
            post(release),
        )
        .route(
            "/api/me/compute/reservations/:reservation_id/expire",
            post(expire),
        )
}

async fn reserve(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(request): Json<ReserveMyComputeRequest>,
) -> Response {
    let user_id = match authenticated_user(&state, &headers) {
        Ok(user_id) => user_id,
        Err(response) => return response,
    };
    broker_response(compute_federation_broker_service::reserve_for_user(
        &state, &user_id, request,
    ))
}

async fn release(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(reservation_id): Path<String>,
    Json(request): Json<FinishMyComputeRequest>,
) -> Response {
    finish(
        &state,
        &headers,
        reservation_id,
        ComputeBrokerFinishAction::Release,
        request,
    )
}

async fn expire(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(reservation_id): Path<String>,
    Json(request): Json<FinishMyComputeRequest>,
) -> Response {
    finish(
        &state,
        &headers,
        reservation_id,
        ComputeBrokerFinishAction::Expire,
        request,
    )
}

fn finish(
    state: &AppState,
    headers: &HeaderMap,
    reservation_id: String,
    action: ComputeBrokerFinishAction,
    request: FinishMyComputeRequest,
) -> Response {
    let user_id = match authenticated_user(state, headers) {
        Ok(user_id) => user_id,
        Err(response) => return response,
    };
    broker_response(compute_federation_broker_service::finish_for_user(
        state,
        &user_id,
        reservation_id,
        action,
        request,
    ))
}

fn authenticated_user(state: &AppState, headers: &HeaderMap) -> Result<String, Response> {
    auth_from_headers(state, headers)
        .map(|user| user.id)
        .map_err(|_| json_error(StatusCode::UNAUTHORIZED, "未登录"))
}

fn broker_response<T: serde::Serialize>(result: anyhow::Result<T>) -> Response {
    match result {
        Ok(value) => Json(value).into_response(),
        Err(error) => json_error(StatusCode::CONFLICT, format!("{error:#}")),
    }
}
