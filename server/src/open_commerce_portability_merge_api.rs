use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde_json::json;

use crate::{
    open_commerce_portability_merge_model::{
        ApplyConsumerPortabilityMergeRequest, CreateConsumerPortabilityMergePlanRequest,
        RollbackConsumerPortabilityMergeRequest,
    },
    open_commerce_portability_merge_service,
    open_commerce_service::OpenCommerceActor,
    project_auth::{auth_from_headers, json_error, project_access},
    types::AppState,
};

pub(crate) fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/api/projects/:project_id/open-commerce/consumer-portability-merge-plan",
            post(merge_plan),
        )
        .route(
            "/api/projects/:project_id/open-commerce/consumer-portability-merge-adoptions",
            get(list_merges).post(apply_merge),
        )
        .route(
            "/api/projects/:project_id/open-commerce/consumer-portability-merge-adoptions/:adoption_id/rollback",
            post(rollback_merge),
        )
}

async fn merge_plan(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
    Json(request): Json<CreateConsumerPortabilityMergePlanRequest>,
) -> Response {
    with_caller(&state, &headers, &project_id, |actor| {
        open_commerce_portability_merge_service::merge_plan(
            &state.store,
            &project_id,
            &actor,
            request,
        )
    })
}

async fn apply_merge(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
    Json(request): Json<ApplyConsumerPortabilityMergeRequest>,
) -> Response {
    with_caller(&state, &headers, &project_id, |actor| {
        open_commerce_portability_merge_service::apply_merge(
            &state.store,
            &project_id,
            &actor,
            request,
        )
    })
}

async fn list_merges(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
) -> Response {
    with_caller(&state, &headers, &project_id, |actor| {
        open_commerce_portability_merge_service::list_merges(&state.store, &project_id, &actor, 100)
            .map(|adoptions| {
                json!({
                    "schema": "open_commerce.consumer_portability_merge_adoptions.v1",
                    "adoptions": adoptions,
                })
            })
    })
}

async fn rollback_merge(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, adoption_id)): Path<(String, String)>,
    Json(request): Json<RollbackConsumerPortabilityMergeRequest>,
) -> Response {
    with_caller(&state, &headers, &project_id, |actor| {
        open_commerce_portability_merge_service::rollback_merge(
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
