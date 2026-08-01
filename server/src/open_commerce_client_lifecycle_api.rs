//! HTTP surface for developer App lifecycle and requester authorization state.

use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::Serialize;
use serde_json::json;
use std::sync::Arc;

use crate::{
    open_commerce_client_lifecycle_service as lifecycle,
    open_commerce_service::OpenCommerceActor,
    project_auth::{auth_from_headers, json_error, project_access},
    types::AppState,
};

pub(crate) fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/api/projects/:project_id/open-commerce/developer-apps/:app_record_id/disable",
            post(disable_app),
        )
        .route(
            "/api/projects/:project_id/open-commerce/developer-apps/:app_record_id/reactivate",
            post(reactivate_app),
        )
        .route(
            "/api/projects/:project_id/open-commerce/outbound-authorization-requests",
            get(list_outbound_requests),
        )
        .route(
            "/api/projects/:project_id/open-commerce/outbound-authorization-requests/:request_id/cancel",
            post(cancel_outbound_request),
        )
}

async fn disable_app(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, app_record_id)): Path<(String, String)>,
) -> Response {
    let caller = match project_caller(&state, &headers, &project_id) {
        Ok(value) => value,
        Err(response) => return response,
    };
    service_response(lifecycle::disable_app(
        &state.store,
        &project_id,
        &app_record_id,
        &actor(&caller),
    ))
}

async fn reactivate_app(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, app_record_id)): Path<(String, String)>,
) -> Response {
    let caller = match project_caller(&state, &headers, &project_id) {
        Ok(value) => value,
        Err(response) => return response,
    };
    service_response(lifecycle::reactivate_app(
        &state.store,
        &project_id,
        &app_record_id,
        &actor(&caller),
    ))
}

async fn list_outbound_requests(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
) -> Response {
    if let Err(response) = project_caller(&state, &headers, &project_id) {
        return response;
    }
    service_response(lifecycle::list_outbound_requests(
        &state.store,
        &project_id,
    )
    .map(|requests| {
        json!({"schema":"open_commerce.outbound_authorization_requests.v1","requests":requests})
    }))
}

async fn cancel_outbound_request(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, request_id)): Path<(String, String)>,
) -> Response {
    let caller = match project_caller(&state, &headers, &project_id) {
        Ok(value) => value,
        Err(response) => return response,
    };
    service_response(lifecycle::cancel_outbound_request(
        &state.store,
        &project_id,
        &request_id,
        &actor(&caller),
    ))
}

fn actor<'a>(caller: &'a (String, String)) -> OpenCommerceActor<'a> {
    OpenCommerceActor {
        user_id: &caller.0,
        app_id: "pc-web",
        project_role: Some(&caller.1),
    }
}

fn project_caller(
    state: &AppState,
    headers: &HeaderMap,
    project_id: &str,
) -> Result<(String, String), Response> {
    let user = auth_from_headers(state, headers)
        .map_err(|error| json_error(StatusCode::UNAUTHORIZED, error))?;
    let access = project_access(state, &user.id, project_id)
        .map_err(|error| json_error(StatusCode::FORBIDDEN, error))?;
    Ok((user.id, access.role))
}

fn service_response<T: Serialize>(result: anyhow::Result<T>) -> Response {
    match result {
        Ok(value) => Json(value).into_response(),
        Err(error) => json_error(StatusCode::BAD_REQUEST, error.to_string()),
    }
}
