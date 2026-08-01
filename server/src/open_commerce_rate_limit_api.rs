//! HTTP management surface for merchant-controlled open-commerce rate limits.

use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{patch, put},
    Json, Router,
};

use crate::{
    open_commerce_model::normalize_app_id,
    open_commerce_rate_limit_model::{
        OpenCommerceRateLimitExceeded, SetOpenCommerceRateLimitEnabledRequest,
        UpsertOpenCommerceRateLimitRequest,
    },
    open_commerce_rate_limit_service,
    project_auth::{auth_from_headers, json_error, project_access},
    types::AppState,
};

const DEFAULT_HTTP_APP_ID: &str = "pc-web";

struct ProjectCaller {
    user_id: String,
    role: String,
    app_id: String,
}

pub(crate) fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/api/projects/:project_id/open-commerce/rate-limits",
            put(upsert_policy),
        )
        .route(
            "/api/projects/:project_id/open-commerce/rate-limits/:policy_id/enabled",
            patch(set_policy_enabled),
        )
}

async fn upsert_policy(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
    Json(request): Json<UpsertOpenCommerceRateLimitRequest>,
) -> Response {
    let caller = match project_caller(&state, &headers, &project_id) {
        Ok(value) => value,
        Err(response) => return response,
    };
    service_response(open_commerce_rate_limit_service::upsert_policy(
        &state.store,
        &project_id,
        &caller.user_id,
        &caller.app_id,
        &caller.role,
        request,
    ))
}

async fn set_policy_enabled(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, policy_id)): Path<(String, String)>,
    Json(request): Json<SetOpenCommerceRateLimitEnabledRequest>,
) -> Response {
    let caller = match project_caller(&state, &headers, &project_id) {
        Ok(value) => value,
        Err(response) => return response,
    };
    service_response(open_commerce_rate_limit_service::set_policy_enabled(
        &state.store,
        &project_id,
        &policy_id,
        &caller.user_id,
        &caller.app_id,
        &caller.role,
        request,
    ))
}

fn project_caller(
    state: &AppState,
    headers: &HeaderMap,
    project_id: &str,
) -> Result<ProjectCaller, Response> {
    let user = auth_from_headers(state, headers)
        .map_err(|error| json_error(StatusCode::UNAUTHORIZED, error))?;
    let access = project_access(state, &user.id, project_id)
        .map_err(|error| json_error(StatusCode::FORBIDDEN, error))?;
    let app_id = headers
        .get("x-elon-app-id")
        .and_then(|value| value.to_str().ok())
        .unwrap_or(DEFAULT_HTTP_APP_ID);
    let app_id =
        normalize_app_id(app_id).map_err(|error| json_error(StatusCode::BAD_REQUEST, error))?;
    Ok(ProjectCaller {
        user_id: user.id,
        role: access.role,
        app_id,
    })
}

fn service_response<T: serde::Serialize>(result: anyhow::Result<T>) -> Response {
    match result {
        Ok(value) => Json(value).into_response(),
        Err(error) => service_error(error),
    }
}

fn service_error(error: anyhow::Error) -> Response {
    let rate_limited = error.is::<OpenCommerceRateLimitExceeded>();
    let message = format!("{error:#}");
    let status = if rate_limited {
        StatusCode::TOO_MANY_REQUESTS
    } else if message.contains("权限") {
        StatusCode::FORBIDDEN
    } else if message.contains("不存在") {
        StatusCode::NOT_FOUND
    } else {
        StatusCode::BAD_REQUEST
    };
    json_error(status, message)
}
