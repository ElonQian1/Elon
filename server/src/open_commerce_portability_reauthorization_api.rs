use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use serde_json::json;
use std::sync::Arc;

use crate::{
    open_commerce_portability_reauthorization_model::{
        CreatePortabilityReauthorizationRequest, CreatePortabilityRelationshipMappingRequest,
    },
    open_commerce_portability_reauthorization_service,
    open_commerce_service::OpenCommerceActor,
    project_auth::{auth_from_headers, json_error, project_access},
    types::AppState,
};

pub(crate) fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/api/projects/:project_id/open-commerce/portability-relationship-mappings",
            get(list_mappings).post(create_mapping),
        )
        .route(
            "/api/projects/:project_id/open-commerce/portability-relationship-mappings/:mapping_id/revoke",
            axum::routing::post(revoke_mapping),
        )
        .route(
            "/api/projects/:project_id/open-commerce/portability-relationship-mappings/:mapping_id/reauthorize",
            axum::routing::post(create_reauthorization),
        )
}

async fn list_mappings(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
) -> Response {
    with_caller(&state, &headers, &project_id, |actor| {
        open_commerce_portability_reauthorization_service::list_mappings(
            &state.store,
            &project_id,
            &actor,
            100,
        )
        .map(|mappings| {
            json!({"schema":"open_commerce.portability_relationship_mappings.v1","mappings":mappings})
        })
    })
}

async fn create_mapping(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
    Json(request): Json<CreatePortabilityRelationshipMappingRequest>,
) -> Response {
    with_caller(&state, &headers, &project_id, |actor| {
        open_commerce_portability_reauthorization_service::create_mapping(
            &state.store,
            &project_id,
            &actor,
            request,
        )
    })
}

async fn revoke_mapping(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, mapping_id)): Path<(String, String)>,
) -> Response {
    with_caller(&state, &headers, &project_id, |actor| {
        open_commerce_portability_reauthorization_service::revoke_mapping(
            &state.store,
            &project_id,
            &mapping_id,
            &actor,
        )
    })
}

async fn create_reauthorization(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, mapping_id)): Path<(String, String)>,
    Json(request): Json<CreatePortabilityReauthorizationRequest>,
) -> Response {
    with_caller(&state, &headers, &project_id, |actor| {
        open_commerce_portability_reauthorization_service::create_reauthorization(
            &state.store,
            &project_id,
            &mapping_id,
            &actor,
            request,
        )
    })
}

fn with_caller<T: serde::Serialize>(
    state: &AppState,
    headers: &HeaderMap,
    project_id: &str,
    operation: impl FnOnce(OpenCommerceActor<'_>) -> anyhow::Result<T>,
) -> Response {
    let user = match auth_from_headers(state, headers) {
        Ok(value) => value,
        Err(error) => return json_error(StatusCode::UNAUTHORIZED, error),
    };
    let access = match project_access(state, &user.id, project_id) {
        Ok(value) => value,
        Err(error) => return json_error(StatusCode::FORBIDDEN, error),
    };
    match operation(OpenCommerceActor {
        user_id: &user.id,
        app_id: "pc-web",
        project_role: Some(&access.role),
    }) {
        Ok(value) => Json(value).into_response(),
        Err(error) => json_error(StatusCode::BAD_REQUEST, error.to_string()),
    }
}
