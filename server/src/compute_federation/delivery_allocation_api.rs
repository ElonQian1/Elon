//! Authenticated HTTP surface for v228 whole-only DeliveryAllocation.

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

use super::delivery_allocation_service::{
    self as service, CreateDeliveryAllocationGrantBody, DeclineDeliveryAllocationGrantBody,
    ExerciseDeliveryAllocationGrantBody, ExpireDueDeliveryAllocationGrantsBody,
};

pub(crate) fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/api/me/compute/providers/:provider_id/capacity-pools/:pool_id/capacity-commitments/:commitment_id/delivery-allocation-grant",
            get(get_for_provider).post(create_for_provider),
        )
        .route(
            "/api/me/compute/delivery-allocation-grants",
            get(list_for_consumer),
        )
        .route(
            "/api/me/compute/delivery-allocation-grants/:grant_id",
            get(get_for_consumer),
        )
        .route(
            "/api/me/compute/delivery-allocation-grants/:grant_id/exercise",
            post(exercise_for_consumer),
        )
        .route(
            "/api/me/compute/delivery-allocation-grants/:grant_id/decline",
            post(decline_for_consumer),
        )
        .route(
            "/api/admin/compute/delivery-allocation-grants/expire-due",
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

async fn create_for_provider(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((provider_id, pool_id, commitment_id)): Path<(String, String, String)>,
    Json(body): Json<CreateDeliveryAllocationGrantBody>,
) -> Response {
    let actor_id = match authenticated_user(&state, &headers) {
        Ok(value) => value,
        Err(response) => return response,
    };
    allocation_response(service::create_for_provider_owner(
        &state.store,
        &actor_id,
        &provider_id,
        &pool_id,
        &commitment_id,
        body,
    ))
}

async fn get_for_provider(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((provider_id, pool_id, commitment_id)): Path<(String, String, String)>,
) -> Response {
    let actor_id = match authenticated_user(&state, &headers) {
        Ok(value) => value,
        Err(response) => return response,
    };
    allocation_response(service::get_for_provider_owner(
        &state.store,
        &actor_id,
        &provider_id,
        &pool_id,
        &commitment_id,
    ))
}

async fn list_for_consumer(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(query): Query<ListQuery>,
) -> Response {
    let actor_id = match authenticated_user(&state, &headers) {
        Ok(value) => value,
        Err(response) => return response,
    };
    allocation_response(
        service::list_for_consumer(
            &state.store,
            &actor_id,
            query.status.as_deref(),
            query.limit,
        )
        .map(|items| json!({"delivery_allocation_grants": items})),
    )
}

async fn get_for_consumer(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(grant_id): Path<String>,
) -> Response {
    let actor_id = match authenticated_user(&state, &headers) {
        Ok(value) => value,
        Err(response) => return response,
    };
    allocation_response(service::get_for_consumer(
        &state.store,
        &actor_id,
        &grant_id,
    ))
}

async fn exercise_for_consumer(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(grant_id): Path<String>,
    Json(body): Json<ExerciseDeliveryAllocationGrantBody>,
) -> Response {
    let actor_id = match authenticated_user(&state, &headers) {
        Ok(value) => value,
        Err(response) => return response,
    };
    allocation_response(service::exercise_for_consumer(
        &state.store,
        &actor_id,
        &grant_id,
        body,
    ))
}

async fn decline_for_consumer(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(grant_id): Path<String>,
    Json(body): Json<DeclineDeliveryAllocationGrantBody>,
) -> Response {
    let actor_id = match authenticated_user(&state, &headers) {
        Ok(value) => value,
        Err(response) => return response,
    };
    allocation_response(service::decline_for_consumer(
        &state.store,
        &actor_id,
        &grant_id,
        body,
    ))
}

async fn expire_due(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<ExpireDueDeliveryAllocationGrantsBody>,
) -> Response {
    let admin_user_id = match platform_admin(&state, &headers) {
        Ok(value) => value,
        Err(response) => return response,
    };
    allocation_response(service::expire_due_for_admin(
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
            "只有平台管理员可以执行交付授权到期恢复",
        ));
    }
    Ok(user.id)
}

fn allocation_response<T: serde::Serialize>(result: anyhow::Result<T>) -> Response {
    match result {
        Ok(value) => Json(value).into_response(),
        Err(error) => json_error(StatusCode::BAD_REQUEST, error),
    }
}

fn default_limit() -> usize {
    20
}
