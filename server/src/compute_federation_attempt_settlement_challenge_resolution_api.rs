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
    compute_federation_attempt_settlement_challenge_resolution_service::{
        self, ResolveComputeSettlementChallengeBody, WithdrawComputeSettlementChallengeBody,
    },
    project_auth::{auth_from_headers, json_error},
    types::AppState,
};

pub(crate) fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/api/me/compute/attempt-leases/:lease_id/settlement-challenge/withdrawal",
            post(withdraw_challenge),
        )
        .route(
            "/api/me/compute/attempt-leases/:lease_id/settlement-challenge/resolution",
            get(get_participant_resolution),
        )
        .route(
            "/api/admin/compute/attempt-leases/:lease_id/settlement-challenge/resolution",
            get(get_admin_resolution).post(resolve_challenge),
        )
        .route(
            "/api/me/compute/settlement-challenges/open",
            get(list_consumer_open_challenges),
        )
        .route(
            "/api/admin/compute/settlement-challenges/open",
            get(list_admin_open_challenges),
        )
        .route(
            "/api/me/compute/settlement-challenges/history",
            get(list_consumer_challenge_history),
        )
        .route(
            "/api/admin/compute/settlement-challenges/history",
            get(list_admin_challenge_history),
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

async fn withdraw_challenge(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(lease_id): Path<String>,
    Json(body): Json<WithdrawComputeSettlementChallengeBody>,
) -> Response {
    let user_id = match authenticated_user(&state, &headers) {
        Ok(user_id) => user_id,
        Err(response) => return response,
    };
    resolution_response(
        compute_federation_attempt_settlement_challenge_resolution_service::withdraw_for_consumer(
            &state.store,
            &user_id,
            &lease_id,
            body,
        ),
    )
}

async fn resolve_challenge(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(lease_id): Path<String>,
    Json(body): Json<ResolveComputeSettlementChallengeBody>,
) -> Response {
    let admin_user_id = match platform_admin(&state, &headers) {
        Ok(user_id) => user_id,
        Err(response) => return response,
    };
    resolution_response(
        compute_federation_attempt_settlement_challenge_resolution_service::resolve_for_platform_admin(
            &state.store,
            &admin_user_id,
            &lease_id,
            body,
        ),
    )
}

async fn get_participant_resolution(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(lease_id): Path<String>,
) -> Response {
    let user_id = match authenticated_user(&state, &headers) {
        Ok(user_id) => user_id,
        Err(response) => return response,
    };
    resolution_response(
        compute_federation_attempt_settlement_challenge_resolution_service::get_for_attempt_participant(
            &state.store,
            &user_id,
            &lease_id,
        ),
    )
}

async fn get_admin_resolution(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(lease_id): Path<String>,
) -> Response {
    if let Err(response) = platform_admin(&state, &headers) {
        return response;
    }
    resolution_response(
        compute_federation_attempt_settlement_challenge_resolution_service::get_for_platform_admin(
            &state.store,
            &lease_id,
        ),
    )
}

async fn list_consumer_open_challenges(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(query): Query<ListQuery>,
) -> Response {
    let user_id = match authenticated_user(&state, &headers) {
        Ok(user_id) => user_id,
        Err(response) => return response,
    };
    resolution_response(
        compute_federation_attempt_settlement_challenge_resolution_service::list_open_for_consumer(
            &state.store,
            &user_id,
            query.limit,
        )
        .map(|challenges| json!({"challenge_candidates":challenges})),
    )
}

async fn list_admin_open_challenges(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(query): Query<ListQuery>,
) -> Response {
    if let Err(response) = platform_admin(&state, &headers) {
        return response;
    }
    resolution_response(
        compute_federation_attempt_settlement_challenge_resolution_service::list_open_for_platform_admin(
            &state.store,
            query.limit,
        )
        .map(|challenges| json!({"challenge_candidates":challenges})),
    )
}

async fn list_consumer_challenge_history(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(query): Query<ListQuery>,
) -> Response {
    let user_id = match authenticated_user(&state, &headers) {
        Ok(user_id) => user_id,
        Err(response) => return response,
    };
    resolution_response(
        compute_federation_attempt_settlement_challenge_resolution_service::list_history_for_consumer(
            &state.store,
            &user_id,
            query.limit,
        )
        .map(|history| json!({"challenge_history":history})),
    )
}

async fn list_admin_challenge_history(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(query): Query<ListQuery>,
) -> Response {
    if let Err(response) = platform_admin(&state, &headers) {
        return response;
    }
    resolution_response(
        compute_federation_attempt_settlement_challenge_resolution_service::list_history_for_platform_admin(
            &state.store,
            query.limit,
        )
        .map(|history| json!({"challenge_history":history})),
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
            "只有平台管理员可以裁决算力结算挑战",
        ));
    }
    Ok(user.id)
}

fn resolution_response<T: serde::Serialize>(result: anyhow::Result<T>) -> Response {
    match result {
        Ok(value) => Json(value).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(json!({"error":format!("{error:#}")})),
        )
            .into_response(),
    }
}
