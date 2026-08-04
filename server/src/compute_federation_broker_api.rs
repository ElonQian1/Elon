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
    compute_federation_broker_service::{self, FinishMyComputeRequest, ReserveMyComputeRequest},
    project_auth::{auth_from_headers, json_error},
    store::ComputeBrokerFinishAction,
    types::AppState,
};

pub(crate) fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/me/compute/jobs", get(list_jobs))
        .route("/api/me/compute/jobs/:job_id", get(get_job))
        .route(
            "/api/me/compute/reservations",
            get(list_reservations).post(reserve),
        )
        .route(
            "/api/me/compute/reservations/:reservation_id",
            get(get_reservation),
        )
        .route(
            "/api/me/compute/reservations/:reservation_id/release",
            post(release),
        )
        .route(
            "/api/me/compute/reservations/:reservation_id/expire",
            post(expire),
        )
}

#[derive(Debug, Deserialize)]
struct ListQuery {
    #[serde(default = "default_limit")]
    limit: usize,
}

async fn list_jobs(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(query): Query<ListQuery>,
) -> Response {
    let user_id = match authenticated_user(&state, &headers) {
        Ok(user_id) => user_id,
        Err(response) => return response,
    };
    broker_response(
        compute_federation_broker_service::list_jobs_for_user(
            &state.store,
            &user_id,
            None,
            query.limit,
        )
        .map(|jobs| json!({"jobs":jobs})),
    )
}

async fn get_job(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(job_id): Path<String>,
) -> Response {
    let user_id = match authenticated_user(&state, &headers) {
        Ok(user_id) => user_id,
        Err(response) => return response,
    };
    broker_response(compute_federation_broker_service::get_job_for_user(
        &state.store,
        &user_id,
        None,
        &job_id,
    ))
}

async fn list_reservations(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(query): Query<ListQuery>,
) -> Response {
    let user_id = match authenticated_user(&state, &headers) {
        Ok(user_id) => user_id,
        Err(response) => return response,
    };
    broker_response(
        compute_federation_broker_service::list_reservations_for_user(
            &state.store,
            &user_id,
            None,
            query.limit,
        )
        .map(|reservations| json!({"reservations":reservations})),
    )
}

async fn get_reservation(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(reservation_id): Path<String>,
) -> Response {
    let user_id = match authenticated_user(&state, &headers) {
        Ok(user_id) => user_id,
        Err(response) => return response,
    };
    broker_response(compute_federation_broker_service::get_reservation_for_user(
        &state.store,
        &user_id,
        None,
        &reservation_id,
    ))
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
        &state.store,
        &user_id,
        None,
        request,
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
        &state.store,
        &user_id,
        None,
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

fn default_limit() -> usize {
    20
}
