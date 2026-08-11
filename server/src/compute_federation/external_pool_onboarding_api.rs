//! Authenticated owner/admin HTTP entry for external-pool Provider onboarding.

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

use super::external_pool_onboarding_service::{
    self as service, ApplyExternalPoolOnboardingBody, CancelExternalPoolOnboardingBody,
    ReviewExternalPoolOnboardingBody, SubmitExternalPoolOnboardingBody,
};

#[cfg(test)]
#[path = "external_pool_onboarding_api_tests.rs"]
mod tests;

pub(crate) fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/api/me/compute/external-pool-onboarding-requests",
            get(list_owner_requests).post(submit_request),
        )
        .route(
            "/api/me/compute/external-pool-onboarding-requests/:request_id",
            get(get_owner_request),
        )
        .route(
            "/api/me/compute/external-pool-onboarding-requests/:request_id/cancel",
            post(cancel_owner_request),
        )
        .route(
            "/api/me/compute/external-pool-onboarding-requests/:request_id/preflight",
            get(preflight_owner_request),
        )
        .route(
            "/api/admin/compute/external-pool-onboarding-requests",
            get(list_admin_requests),
        )
        .route(
            "/api/admin/compute/external-pool-onboarding-requests/:request_id",
            get(get_admin_request),
        )
        .route(
            "/api/admin/compute/external-pool-onboarding-requests/:request_id/preflight",
            get(preflight_admin_request),
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

#[derive(Debug, Deserialize)]
struct ListQuery {
    status: Option<String>,
    #[serde(default = "default_limit")]
    limit: usize,
}

async fn list_owner_requests(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(query): Query<ListQuery>,
) -> Response {
    let owner_user_id = match authenticated_user(&state, &headers) {
        Ok(value) => value,
        Err(response) => return response,
    };
    onboarding_response(
        service::list_for_owner(
            &state.store,
            &owner_user_id,
            query.status.as_deref(),
            query.limit,
        )
        .map(|items| json!({"onboarding_requests": items})),
    )
}

async fn get_owner_request(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(request_id): Path<String>,
) -> Response {
    let owner_user_id = match authenticated_user(&state, &headers) {
        Ok(value) => value,
        Err(response) => return response,
    };
    onboarding_response(service::get_for_owner(
        &state.store,
        &owner_user_id,
        &request_id,
    ))
}

async fn cancel_owner_request(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(request_id): Path<String>,
    Json(body): Json<CancelExternalPoolOnboardingBody>,
) -> Response {
    let owner_user_id = match authenticated_user(&state, &headers) {
        Ok(value) => value,
        Err(response) => return response,
    };
    onboarding_response(service::cancel_for_owner(
        &state.store,
        &owner_user_id,
        &request_id,
        body,
    ))
}

async fn preflight_owner_request(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(request_id): Path<String>,
) -> Response {
    let owner_user_id = match authenticated_user(&state, &headers) {
        Ok(value) => value,
        Err(response) => return response,
    };
    onboarding_response(service::preflight_for_owner(
        &state.store,
        &owner_user_id,
        &request_id,
    ))
}

async fn list_admin_requests(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(query): Query<ListQuery>,
) -> Response {
    if let Err(response) = platform_admin(&state, &headers) {
        return response;
    }
    onboarding_response(
        service::list_for_admin(&state.store, query.status.as_deref(), query.limit)
            .map(|items| json!({"onboarding_requests": items})),
    )
}

async fn get_admin_request(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(request_id): Path<String>,
) -> Response {
    if let Err(response) = platform_admin(&state, &headers) {
        return response;
    }
    onboarding_response(service::get_for_admin(&state.store, &request_id))
}

async fn preflight_admin_request(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(request_id): Path<String>,
) -> Response {
    let admin_user_id = match platform_admin(&state, &headers) {
        Ok(value) => value,
        Err(response) => return response,
    };
    onboarding_response(service::preflight_for_admin(
        &state.store,
        &admin_user_id,
        &request_id,
    ))
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
            "只有平台管理员可以管理 external-pool onboarding",
        ));
    }
    Ok(user.id)
}

fn default_limit() -> usize {
    50
}

fn onboarding_response<T: serde::Serialize>(result: anyhow::Result<T>) -> Response {
    match result {
        Ok(value) => Json(value).into_response(),
        Err(error) => json_error(StatusCode::BAD_REQUEST, error),
    }
}
