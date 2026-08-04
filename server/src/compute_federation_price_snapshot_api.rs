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
    compute_federation_price_snapshot_model::PublishMyComputePriceSnapshotRequest,
    compute_federation_price_snapshot_service,
    project_auth::{auth_from_headers, json_error},
    types::AppState,
};

pub(crate) fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/api/me/compute/providers/:provider_id/capacity-pools/:pool_id/offers/:offer_id/price-snapshots",
            get(list_price_snapshots).post(publish_price_snapshot),
        )
        .route(
            "/api/me/compute/providers/:provider_id/capacity-pools/:pool_id/offers/:offer_id/price-snapshots/:snapshot_id",
            get(get_price_snapshot),
        )
}

#[derive(Debug, Deserialize)]
struct ListQuery {
    #[serde(default = "default_limit")]
    limit: usize,
}

async fn publish_price_snapshot(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((provider_id, pool_id, offer_id)): Path<(String, String, String)>,
    Json(request): Json<PublishMyComputePriceSnapshotRequest>,
) -> Response {
    let user_id = match authenticated_user(&state, &headers) {
        Ok(value) => value,
        Err(response) => return response,
    };
    snapshot_response(compute_federation_price_snapshot_service::publish_for_user(
        &state.store,
        &user_id,
        &provider_id,
        &pool_id,
        &offer_id,
        request,
    ))
}

async fn get_price_snapshot(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((provider_id, pool_id, offer_id, snapshot_id)): Path<(String, String, String, String)>,
) -> Response {
    let user_id = match authenticated_user(&state, &headers) {
        Ok(value) => value,
        Err(response) => return response,
    };
    snapshot_response(compute_federation_price_snapshot_service::get_for_user(
        &state.store,
        &user_id,
        &provider_id,
        &pool_id,
        &offer_id,
        &snapshot_id,
    ))
}

async fn list_price_snapshots(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((provider_id, pool_id, offer_id)): Path<(String, String, String)>,
    Query(query): Query<ListQuery>,
) -> Response {
    let user_id = match authenticated_user(&state, &headers) {
        Ok(value) => value,
        Err(response) => return response,
    };
    snapshot_response(
        compute_federation_price_snapshot_service::list_for_user(
            &state.store,
            &user_id,
            &provider_id,
            &pool_id,
            &offer_id,
            query.limit,
        )
        .map(|snapshots| json!({"snapshots":snapshots})),
    )
}

fn authenticated_user(state: &AppState, headers: &HeaderMap) -> Result<String, Response> {
    auth_from_headers(state, headers)
        .map(|user| user.id)
        .map_err(|_| json_error(StatusCode::UNAUTHORIZED, "未登录"))
}

fn snapshot_response<T: serde::Serialize>(result: anyhow::Result<T>) -> Response {
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
