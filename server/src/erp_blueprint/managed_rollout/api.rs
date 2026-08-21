use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use serde::Deserialize;

use crate::{
    project_auth::{auth_from_headers, can_edit, json_error, project_access},
    types::AppState,
};

use super::{
    model::CreateManagedRolloutPlanRequest,
    service::{create_plan, get_plan, list_plans},
};

#[derive(Debug, Deserialize)]
struct PlanListQuery {
    #[serde(default = "default_limit")]
    limit: usize,
}

pub(crate) fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/api/projects/:project_id/erp/instances/:instance_id/managed-rollouts",
            get(list).post(create),
        )
        .route(
            "/api/projects/:project_id/erp/instances/:instance_id/managed-rollouts/:rollout_id",
            get(detail),
        )
}

async fn create(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, instance_id)): Path<(String, String)>,
    Json(request): Json<CreateManagedRolloutPlanRequest>,
) -> Response {
    let user = match authenticate(&state, &headers, &project_id, true) {
        Ok(user) => user,
        Err(response) => return response,
    };
    respond(create_plan(
        &state.store,
        &project_id,
        &instance_id,
        &user.id,
        request,
    ))
}

async fn list(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, instance_id)): Path<(String, String)>,
    Query(query): Query<PlanListQuery>,
) -> Response {
    if let Err(response) = authenticate(&state, &headers, &project_id, false) {
        return response;
    }
    respond(list_plans(
        &state.store,
        &project_id,
        &instance_id,
        query.limit,
    ))
}

async fn detail(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, instance_id, rollout_id)): Path<(String, String, String)>,
) -> Response {
    if let Err(response) = authenticate(&state, &headers, &project_id, false) {
        return response;
    }
    respond(get_plan(
        &state.store,
        &project_id,
        &instance_id,
        &rollout_id,
    ))
}

fn authenticate(
    state: &AppState,
    headers: &HeaderMap,
    project_id: &str,
    write: bool,
) -> Result<crate::store::PublicUser, Response> {
    let user = auth_from_headers(state, headers)
        .map_err(|error| json_error(StatusCode::UNAUTHORIZED, error.to_string()))?;
    let access = project_access(state, &user.id, project_id)
        .map_err(|error| json_error(StatusCode::FORBIDDEN, error.to_string()))?;
    if write && !can_edit(&access.role) {
        return Err(json_error(StatusCode::FORBIDDEN, "当前项目只有查看权限"));
    }
    Ok(user)
}

fn respond<T: serde::Serialize>(result: anyhow::Result<T>) -> Response {
    match result {
        Ok(value) => Json(value).into_response(),
        Err(error) => json_error(StatusCode::BAD_REQUEST, error.to_string()),
    }
}

fn default_limit() -> usize {
    20
}
