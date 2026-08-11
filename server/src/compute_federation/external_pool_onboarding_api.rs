//! Authenticated owner/admin HTTP entry for external-pool Provider onboarding.

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

use super::external_pool_onboarding_service::{
    self as service, ApplyExternalPoolOnboardingBody, ReviewExternalPoolOnboardingBody,
    SubmitExternalPoolOnboardingBody,
};

#[cfg(test)]
#[path = "external_pool_onboarding_api_tests.rs"]
mod tests;

pub(crate) fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/api/me/compute/external-pool-onboarding-requests",
            post(submit_request),
        )
        .route(
            "/api/admin/compute/external-pool-onboarding-requests/:request_id/review",
            post(review_request),
        )
        .route(
            "/api/admin/compute/external-pool-onboarding-requests/:request_id/application",
            post(apply_request),
        )
}

async fn submit_request(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<SubmitExternalPoolOnboardingBody>,
) -> Response {
    let owner_user_id = match authenticated_user(&state, &headers) {
        Ok(value) => value,
        Err(response) => return response,
    };
    onboarding_response(service::submit_for_owner(
        &state.store,
        &owner_user_id,
        body,
    ))
}

async fn review_request(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(request_id): Path<String>,
    Json(body): Json<ReviewExternalPoolOnboardingBody>,
) -> Response {
    let admin_user_id = match platform_admin(&state, &headers) {
        Ok(value) => value,
        Err(response) => return response,
    };
    onboarding_response(service::review_for_admin(
        &state.store,
        &admin_user_id,
        &request_id,
        body,
    ))
}

async fn apply_request(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(request_id): Path<String>,
    Json(body): Json<ApplyExternalPoolOnboardingBody>,
) -> Response {
    let admin_user_id = match platform_admin(&state, &headers) {
        Ok(value) => value,
        Err(response) => return response,
    };
    onboarding_response(service::apply_for_admin(
        &state.store,
        &admin_user_id,
        &request_id,
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
            "只有平台管理员可以复核或应用 external-pool onboarding",
        ));
    }
    Ok(user.id)
}

fn onboarding_response<T: serde::Serialize>(result: anyhow::Result<T>) -> Response {
    match result {
        Ok(value) => Json(value).into_response(),
        Err(error) => json_error(StatusCode::BAD_REQUEST, error),
    }
}
