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
    open_commerce_portability_adoption_model::{
        ApplyConsumerPortabilityPreferencesRequest, RollbackConsumerPortabilityAdoptionRequest,
    },
    open_commerce_portability_adoption_service,
    open_commerce_service::OpenCommerceActor,
    project_auth::{auth_from_headers, json_error, project_access},
    types::AppState,
};

pub(crate) fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/api/projects/:project_id/open-commerce/consumer-portability-imports/:import_id/adoption-plan",
            get(adoption_plan),
        )
        .route(
            "/api/projects/:project_id/open-commerce/consumer-portability-imports/:import_id/adopt-preferences",
            axum::routing::post(apply_preferences),
        )
        .route(
            "/api/projects/:project_id/open-commerce/consumer-portability-adoptions",
            get(list_adoptions),
        )
        .route(
            "/api/projects/:project_id/open-commerce/consumer-portability-adoptions/:adoption_id/rollback",
            axum::routing::post(rollback_adoption),
        )
}

async fn adoption_plan(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, import_id)): Path<(String, String)>,
) -> Response {
    with_caller(&state, &headers, &project_id, |actor| {
        open_commerce_portability_adoption_service::adoption_plan(
            &state.store,
            &project_id,
            &import_id,
            &actor,
        )
    })
}

async fn apply_preferences(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, import_id)): Path<(String, String)>,
    Json(request): Json<ApplyConsumerPortabilityPreferencesRequest>,
) -> Response {
    with_caller(&state, &headers, &project_id, |actor| {
        open_commerce_portability_adoption_service::apply_preferences(
            &state.store,
            &project_id,
            &import_id,
            &actor,
            request,
        )
    })
}

async fn list_adoptions(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
) -> Response {
    with_caller(&state, &headers, &project_id, |actor| {
        open_commerce_portability_adoption_service::list_adoptions(
            &state.store,
            &project_id,
            &actor,
            100,
        )
        .map(|adoptions| {
            json!({"schema":"open_commerce.consumer_portability_adoptions.v1","adoptions":adoptions})
        })
    })
}

async fn rollback_adoption(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, adoption_id)): Path<(String, String)>,
    Json(request): Json<RollbackConsumerPortabilityAdoptionRequest>,
) -> Response {
    with_caller(&state, &headers, &project_id, |actor| {
        open_commerce_portability_adoption_service::rollback_adoption(
            &state.store,
            &project_id,
            &adoption_id,
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
