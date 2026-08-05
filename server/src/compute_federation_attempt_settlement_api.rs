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
    compute_federation_attempt_settlement_service::{self, SettleComputeAttemptBody},
    project_auth::{auth_from_headers, json_error},
    types::AppState,
};

pub(crate) fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/api/admin/compute/attempt-leases/:lease_id/settlement-receipt",
            post(settle_attempt),
        )
        .route(
            "/api/me/compute/attempt-leases/:lease_id/settlement-receipt",
            get(get_settlement),
        )
        .route(
            "/api/admin/compute/attempt-finalizations/pending-settlement-receipt",
            get(list_pending_settlements),
        )
        .route(
            "/api/me/compute/settlements/history",
            get(list_consumer_settlement_history),
        )
        .route(
            "/api/me/compute/providers/:provider_id/settlements/history",
            get(list_provider_settlement_history),
        )
        .route(
            "/api/admin/compute/settlements/history",
            get(list_admin_settlement_history),
        )
}

#[derive(Debug, Deserialize)]
struct ListQuery {
    #[serde(default = "default_limit")]
    limit: usize,
}

fn default_limit() -> usize {
    50
}

async fn settle_attempt(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(lease_id): Path<String>,
    Json(body): Json<SettleComputeAttemptBody>,
) -> Response {
    let admin_user_id = match platform_admin(&state, &headers) {
        Ok(user_id) => user_id,
        Err(response) => return response,
    };
    settlement_response(
        compute_federation_attempt_settlement_service::settle_for_platform_admin(
            &state.store,
            &admin_user_id,
            &lease_id,
            body,
        ),
    )
}

async fn get_settlement(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(lease_id): Path<String>,
) -> Response {
    let user_id = match authenticated_user(&state, &headers) {
        Ok(user_id) => user_id,
        Err(response) => return response,
    };
    settlement_response(
        compute_federation_attempt_settlement_service::get_for_attempt_participant(
            &state.store,
            &user_id,
            &lease_id,
        ),
    )
}

async fn list_pending_settlements(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(query): Query<ListQuery>,
) -> Response {
    if let Err(response) = platform_admin(&state, &headers) {
        return response;
    }
    settlement_response(
        compute_federation_attempt_settlement_service::list_pending_for_platform_admin(
            &state.store,
            query.limit,
        )
        .map(|candidates| json!({"settlement_candidates":candidates})),
    )
}

async fn list_consumer_settlement_history(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(query): Query<ListQuery>,
) -> Response {
    let user_id = match authenticated_user(&state, &headers) {
        Ok(user_id) => user_id,
        Err(response) => return response,
    };
    settlement_response(
        compute_federation_attempt_settlement_service::list_history_for_consumer(
            &state.store,
            &user_id,
            query.limit,
        )
        .map(|history| json!({"settlement_history":history})),
    )
}

async fn list_provider_settlement_history(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(provider_id): Path<String>,
    Query(query): Query<ListQuery>,
) -> Response {
    let user_id = match authenticated_user(&state, &headers) {
        Ok(user_id) => user_id,
        Err(response) => return response,
    };
    settlement_response(
        compute_federation_attempt_settlement_service::list_history_for_provider_owner(
            &state.store,
            &user_id,
            &provider_id,
            query.limit,
        )
        .map(|history| json!({"settlement_history":history})),
    )
}

async fn list_admin_settlement_history(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(query): Query<ListQuery>,
) -> Response {
    if let Err(response) = platform_admin(&state, &headers) {
        return response;
    }
    settlement_response(
        compute_federation_attempt_settlement_service::list_history_for_platform_admin(
            &state.store,
            query.limit,
        )
        .map(|history| json!({"settlement_history":history})),
    )
}

fn authenticated_user(state: &AppState, headers: &HeaderMap) -> Result<String, Response> {
    auth_from_headers(state, headers)
        .map(|user| user.id)
        .map_err(|_| json_error(StatusCode::UNAUTHORIZED, "未登录"))
}

fn platform_admin(state: &AppState, headers: &HeaderMap) -> Result<String, Response> {
    let user = auth_from_headers(state, headers)
        .map_err(|error| json_error(StatusCode::UNAUTHORIZED, error))?;
    if !matches!(user.role.as_str(), "admin" | "owner") {
        return Err(json_error(
            StatusCode::FORBIDDEN,
            "只有平台管理员可以结算算力 Attempt",
        ));
    }
    Ok(user.id)
}

fn settlement_response<T: serde::Serialize>(result: anyhow::Result<T>) -> Response {
    match result {
        Ok(value) => Json(value).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(json!({"error":format!("{error:#}")})),
        )
            .into_response(),
    }
}
