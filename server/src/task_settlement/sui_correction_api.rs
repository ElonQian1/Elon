use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::Response,
    routing::{get, post},
    Json, Router,
};
use std::sync::Arc;

use crate::{
    project_auth::{can_edit, json_error},
    types::AppState,
};

use super::{
    api::{project_caller, service_response},
    model::PrepareSuiProjectionPackageRequest,
    sui_correction_projection_service as service,
};

pub(super) fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/api/projects/:project_id/economy/corrections/:correction_id/sui-projections",
            post(prepare),
        )
        .route(
            "/api/projects/:project_id/economy/sui-correction-projections",
            get(list),
        )
        .route(
            "/api/projects/:project_id/economy/sui-correction-projections/:projection_id",
            get(detail),
        )
        .route(
            "/api/projects/:project_id/economy/sui-correction-projections/:projection_id/verify",
            post(verify),
        )
}

async fn prepare(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, correction_id)): Path<(String, String)>,
    Json(request): Json<PrepareSuiProjectionPackageRequest>,
) -> Response {
    let (user_id, role) = match project_caller(&state, &headers, &project_id) {
        Ok(caller) => caller,
        Err(response) => return response,
    };
    if !can_edit(&role) {
        return json_error(
            StatusCode::FORBIDDEN,
            "只有项目编辑者可以准备 Sui 纠正投影包",
        );
    }
    service_response(service::prepare(
        &state.store,
        &project_id,
        &correction_id,
        &user_id,
        &request.target_network,
    ))
}

async fn list(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
) -> Response {
    if let Err(response) = project_caller(&state, &headers, &project_id) {
        return response;
    }
    service_response(service::list(&state.store, &project_id))
}

async fn detail(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, projection_id)): Path<(String, String)>,
) -> Response {
    if let Err(response) = project_caller(&state, &headers, &project_id) {
        return response;
    }
    service_response(service::detail(&state.store, &project_id, &projection_id))
}

async fn verify(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, projection_id)): Path<(String, String)>,
) -> Response {
    let (_, role) = match project_caller(&state, &headers, &project_id) {
        Ok(caller) => caller,
        Err(response) => return response,
    };
    if !can_edit(&role) {
        return json_error(
            StatusCode::FORBIDDEN,
            "只有项目编辑者可以复核 Sui 纠正投影包",
        );
    }
    service_response(service::verify(&state.store, &project_id, &projection_id))
}
