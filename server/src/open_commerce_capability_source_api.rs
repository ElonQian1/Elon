use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::put,
    Json, Router,
};
use std::sync::Arc;

use crate::{
    open_commerce_capability_source_model::LinkCapabilitySourceRequest,
    open_commerce_capability_source_service,
    open_commerce_service::OpenCommerceActor,
    project_auth::{auth_from_headers, json_error, project_access},
    types::AppState,
};

pub(crate) fn routes() -> Router<Arc<AppState>> {
    Router::new().route(
        "/api/projects/:project_id/open-commerce/capabilities/:capability_id/source-link",
        put(link_source).delete(remove_source),
    )
}

async fn link_source(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, capability_id)): Path<(String, String)>,
    Json(request): Json<LinkCapabilitySourceRequest>,
) -> Response {
    let caller = match project_caller(&state, &headers, &project_id) {
        Ok(value) => value,
        Err(response) => return response,
    };
    service_response(open_commerce_capability_source_service::link_source(
        &state.store,
        &project_id,
        &capability_id,
        &actor(&caller),
        request,
    ))
}

async fn remove_source(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, capability_id)): Path<(String, String)>,
) -> Response {
    let caller = match project_caller(&state, &headers, &project_id) {
        Ok(value) => value,
        Err(response) => return response,
    };
    service_response(open_commerce_capability_source_service::remove_source(
        &state.store,
        &project_id,
        &capability_id,
        &actor(&caller),
    ))
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

fn actor<'a>(caller: &'a (String, String)) -> OpenCommerceActor<'a> {
    OpenCommerceActor {
        user_id: &caller.0,
        app_id: "pc-web",
        project_role: Some(&caller.1),
    }
}

fn service_response<T: serde::Serialize>(result: anyhow::Result<T>) -> Response {
    match result {
        Ok(value) => Json(value).into_response(),
        Err(error) => json_error(StatusCode::BAD_REQUEST, error.to_string()),
    }
}
