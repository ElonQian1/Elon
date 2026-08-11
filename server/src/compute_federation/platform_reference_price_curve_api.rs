//! Authenticated administrator HTTP entry for governed platform fallback price curves.

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

use super::platform_reference_price_curve_service::{
    self as service, ApplyPlatformReferencePriceCurveBody, ReviewPlatformReferencePriceCurveBody,
    SubmitPlatformReferencePriceCurveBody,
};

#[cfg(test)]
#[path = "platform_reference_price_curve_api_tests.rs"]
mod tests;

pub(crate) fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/api/admin/compute/platform-reference-price-curves",
            get(list_batches).post(submit_batch),
        )
        .route(
            "/api/admin/compute/platform-reference-price-curves/:batch_id",
            get(get_batch),
        )
        .route(
            "/api/admin/compute/platform-reference-price-curves/:batch_id/preflight",
            get(preflight_batch),
        )
        .route(
            "/api/admin/compute/platform-reference-price-curves/:batch_id/review",
            post(review_batch),
        )
        .route(
            "/api/admin/compute/platform-reference-price-curves/:batch_id/application",
            post(apply_batch),
        )
}

#[derive(Debug, Deserialize)]
struct ListQuery {
    status: Option<String>,
    #[serde(default = "default_limit")]
    limit: usize,
}

async fn list_batches(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(query): Query<ListQuery>,
) -> Response {
    if let Err(response) = platform_admin(&state, &headers) {
        return response;
    }
    curve_response(
        service::list_for_admin(&state.store, query.status.as_deref(), query.limit)
            .map(|items| json!({"reference_curve_batches": items})),
    )
}

async fn get_batch(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(batch_id): Path<String>,
) -> Response {
    if let Err(response) = platform_admin(&state, &headers) {
        return response;
    }
    curve_response(service::get_for_admin(&state.store, &batch_id))
}

async fn preflight_batch(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(batch_id): Path<String>,
) -> Response {
    let admin_user_id = match platform_admin(&state, &headers) {
        Ok(value) => value,
        Err(response) => return response,
    };
    curve_response(service::preflight_for_admin(
        &state.store,
        &admin_user_id,
        &batch_id,
    ))
}

async fn submit_batch(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<SubmitPlatformReferencePriceCurveBody>,
) -> Response {
    let admin_user_id = match platform_admin(&state, &headers) {
        Ok(value) => value,
        Err(response) => return response,
    };
    curve_response(service::submit_for_admin(
        &state.store,
        &admin_user_id,
        body,
    ))
}

async fn review_batch(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(batch_id): Path<String>,
    Json(body): Json<ReviewPlatformReferencePriceCurveBody>,
) -> Response {
    let admin_user_id = match platform_admin(&state, &headers) {
        Ok(value) => value,
        Err(response) => return response,
    };
    curve_response(service::review_for_admin(
        &state.store,
        &admin_user_id,
        &batch_id,
        body,
    ))
}

async fn apply_batch(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(batch_id): Path<String>,
    Json(body): Json<ApplyPlatformReferencePriceCurveBody>,
) -> Response {
    let admin_user_id = match platform_admin(&state, &headers) {
        Ok(value) => value,
        Err(response) => return response,
    };
    curve_response(service::apply_for_admin(
        &state.store,
        &admin_user_id,
        &batch_id,
        body,
    ))
}

fn platform_admin(state: &AppState, headers: &HeaderMap) -> Result<String, Response> {
    let user = auth_from_headers(state, headers)
        .map_err(|error| json_error(StatusCode::UNAUTHORIZED, error))?;
    if !matches!(user.role.as_str(), "admin" | "owner") {
        return Err(json_error(
            StatusCode::FORBIDDEN,
            "只有平台管理员可以管理参考价格回退曲线",
        ));
    }
    Ok(user.id)
}

fn default_limit() -> usize {
    50
}

fn curve_response<T: serde::Serialize>(result: anyhow::Result<T>) -> Response {
    match result {
        Ok(value) => Json(value).into_response(),
        Err(error) => json_error(StatusCode::BAD_REQUEST, error),
    }
}
