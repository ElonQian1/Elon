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
    compute_federation_offer_draft_model::{
        CreateMyComputeOfferDraftRequest, RevokeMyComputeOfferDraftRequest,
    },
    compute_federation_offer_service,
    project_auth::{auth_from_headers, json_error},
    types::AppState,
};

pub(crate) fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/api/me/compute/providers/:provider_id/capacity-pools/:pool_id/offers",
            get(list_offers).post(create_offer_draft),
        )
        .route(
            "/api/me/compute/providers/:provider_id/capacity-pools/:pool_id/offers/:offer_id",
            get(get_offer),
        )
        .route(
            "/api/me/compute/providers/:provider_id/capacity-pools/:pool_id/offers/:offer_id/revoke",
            post(revoke_offer_draft),
        )
}

#[derive(Debug, Deserialize)]
struct ListQuery {
    #[serde(default = "default_limit")]
    limit: usize,
}

async fn create_offer_draft(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((provider_id, pool_id)): Path<(String, String)>,
    Json(request): Json<CreateMyComputeOfferDraftRequest>,
) -> Response {
    let user_id = match authenticated_user(&state, &headers) {
        Ok(value) => value,
        Err(response) => return response,
    };
    offer_response(compute_federation_offer_service::create_draft_for_user(
        &state.store,
        &user_id,
        &provider_id,
        &pool_id,
        request,
    ))
}

async fn get_offer(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((provider_id, pool_id, offer_id)): Path<(String, String, String)>,
) -> Response {
    let user_id = match authenticated_user(&state, &headers) {
        Ok(value) => value,
        Err(response) => return response,
    };
    offer_response(compute_federation_offer_service::get_for_user(
        &state.store,
        &user_id,
        &provider_id,
        &pool_id,
        &offer_id,
    ))
}

async fn list_offers(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((provider_id, pool_id)): Path<(String, String)>,
    Query(query): Query<ListQuery>,
) -> Response {
    let user_id = match authenticated_user(&state, &headers) {
        Ok(value) => value,
        Err(response) => return response,
    };
    offer_response(
        compute_federation_offer_service::list_for_user(
            &state.store,
            &user_id,
            &provider_id,
            &pool_id,
            query.limit,
        )
        .map(|offers| json!({"offers":offers})),
    )
}

async fn revoke_offer_draft(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((provider_id, pool_id, offer_id)): Path<(String, String, String)>,
    Json(request): Json<RevokeMyComputeOfferDraftRequest>,
) -> Response {
    let user_id = match authenticated_user(&state, &headers) {
        Ok(value) => value,
        Err(response) => return response,
    };
    offer_response(compute_federation_offer_service::revoke_draft_for_user(
        &state.store,
        &user_id,
        &provider_id,
        &pool_id,
        &offer_id,
        request,
    ))
}

fn authenticated_user(state: &AppState, headers: &HeaderMap) -> Result<String, Response> {
    auth_from_headers(state, headers)
        .map(|user| user.id)
        .map_err(|_| json_error(StatusCode::UNAUTHORIZED, "未登录"))
}

fn offer_response<T: serde::Serialize>(result: anyhow::Result<T>) -> Response {
    match result {
        Ok(value) => Json(value).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(json!({"error":format!("{error:#}")})),
        )
            .into_response(),
    }
}

fn default_limit() -> usize {
    20
}
