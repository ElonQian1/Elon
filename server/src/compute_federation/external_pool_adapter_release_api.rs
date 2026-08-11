//! Authenticated platform-administrator API for staging external-pool Adapter releases.

use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::post,
    Json, Router,
};

use crate::{
    project_auth::{auth_from_headers, json_error},
    types::AppState,
};

use super::external_pool_adapter_release_service::{
    self as service, ReviewExternalPoolAdapterReleaseBody, StageExternalPoolAdapterReleaseBody,
    SubmitExternalPoolAdapterReleaseBody,
};

#[cfg(test)]
#[path = "external_pool_adapter_release_api_tests.rs"]
mod tests;

pub(crate) fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/api/admin/compute/external-pool-adapter-releases",
            post(submit_release),
        )
        .route(
            "/api/admin/compute/external-pool-adapter-releases/:request_id/review",
            post(review_release),
        )
        .route(
            "/api/admin/compute/external-pool-adapter-releases/:request_id/stage",
            post(stage_release),
        )
}

async fn submit_release(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<SubmitExternalPoolAdapterReleaseBody>,
) -> Response {
    let admin_user_id = match platform_admin(&state, &headers) {
        Ok(value) => value,
        Err(response) => return response,
    };
    release_response(service::submit_for_admin(
        &state.store,
        &admin_user_id,
        body,
    ))
}

async fn review_release(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(request_id): Path<String>,
    Json(body): Json<ReviewExternalPoolAdapterReleaseBody>,
) -> Response {
    let admin_user_id = match platform_admin(&state, &headers) {
        Ok(value) => value,
        Err(response) => return response,
    };
    release_response(service::review_for_admin(
        &state.store,
        &admin_user_id,
        &request_id,
        body,
    ))
}

async fn stage_release(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(request_id): Path<String>,
    Json(body): Json<StageExternalPoolAdapterReleaseBody>,
) -> Response {
    let admin_user_id = match platform_admin(&state, &headers) {
        Ok(value) => value,
        Err(response) => return response,
    };
    release_response(service::stage_for_admin(
        &state.store,
        &admin_user_id,
        &request_id,
        body,
    ))
}

fn platform_admin(state: &AppState, headers: &HeaderMap) -> Result<String, Response> {
    let user = auth_from_headers(state, headers)
        .map_err(|error| json_error(StatusCode::UNAUTHORIZED, error))?;
    if !matches!(user.role.as_str(), "admin" | "owner") {
        return Err(json_error(
            StatusCode::FORBIDDEN,
            "只有平台管理员可以管理 external-pool Adapter release",
        ));
    }
    Ok(user.id)
}

fn release_response<T: serde::Serialize>(result: anyhow::Result<T>) -> Response {
    match result {
        Ok(value) => Json(value).into_response(),
        Err(error) => json_error(StatusCode::BAD_REQUEST, error),
    }
}
