//! Authenticated HTTP surface for local Provider Capacity Commitments.

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
    project_auth::{auth_from_headers, json_error},
    types::AppState,
};

use super::capacity_commitment_service::{
    self as service, CancelCapacityCommitmentBody, CreateCapacityCommitmentBody,
    ExpireDueCapacityCommitmentsBody,
};

pub(crate) fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/api/me/compute/providers/:provider_id/capacity-pools/:pool_id/capacity-commitments",
            get(list_commitments).post(create_commitment),
        )
        .route(
            "/api/me/compute/providers/:provider_id/capacity-pools/:pool_id/capacity-commitments/:commitment_id",
            get(get_commitment),
        )
        .route(
            "/api/me/compute/providers/:provider_id/capacity-pools/:pool_id/capacity-commitments/:commitment_id/cancel",
            post(cancel_commitment),
        )
        .route(
            "/api/admin/compute/capacity-commitments/expire-due",
            post(expire_due),
        )
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ListQuery {
    status: Option<String>,
    #[serde(default = "default_limit")]
    limit: usize,
}

async fn create_commitment(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((provider_id, pool_id)): Path<(String, String)>,
    Json(body): Json<CreateCapacityCommitmentBody>,
) -> Response {
    let owner_account_id = match authenticated_user(&state, &headers) {
        Ok(value) => value,
        Err(response) => return response,
    };
    commitment_response(service::create_for_owner(
        &state.store,
        &owner_account_id,
        &provider_id,
        &pool_id,
        body,
    ))
}

async fn list_commitments(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((provider_id, pool_id)): Path<(String, String)>,
    Query(query): Query<ListQuery>,
) -> Response {
    let owner_account_id = match authenticated_user(&state, &headers) {
        Ok(value) => value,
        Err(response) => return response,
    };
    commitment_response(
        service::list_for_owner(
            &state.store,
            &owner_account_id,
            &provider_id,
            &pool_id,
            query.status.as_deref(),
            query.limit,
        )
        .map(|items| json!({"capacity_commitments": items})),
    )
}

async fn get_commitment(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((provider_id, pool_id, commitment_id)): Path<(String, String, String)>,
) -> Response {
    let owner_account_id = match authenticated_user(&state, &headers) {
        Ok(value) => value,
        Err(response) => return response,
    };
    commitment_response(service::get_for_owner(
        &state.store,
        &owner_account_id,
        &provider_id,
        &pool_id,
        &commitment_id,
    ))
}

async fn cancel_commitment(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((provider_id, pool_id, commitment_id)): Path<(String, String, String)>,
    Json(body): Json<CancelCapacityCommitmentBody>,
) -> Response {
    let owner_account_id = match authenticated_user(&state, &headers) {
        Ok(value) => value,
        Err(response) => return response,
    };
    commitment_response(service::cancel_for_owner(
        &state.store,
        &owner_account_id,
        &provider_id,
        &pool_id,
        &commitment_id,
        body,
    ))
}

async fn expire_due(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<ExpireDueCapacityCommitmentsBody>,
) -> Response {
    let admin_user_id = match platform_admin(&state, &headers) {
        Ok(value) => value,
        Err(response) => return response,
    };
    commitment_response(service::expire_due_for_admin(
        &state.store,
        &admin_user_id,
        body,
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
            "只有平台管理员可以执行容量承诺到期恢复",
        ));
    }
    Ok(user.id)
}

fn commitment_response<T: serde::Serialize>(result: anyhow::Result<T>) -> Response {
    match result {
        Ok(value) => Json(value).into_response(),
        Err(error) => json_error(StatusCode::BAD_REQUEST, error),
    }
}

fn default_limit() -> usize {
    20
}
