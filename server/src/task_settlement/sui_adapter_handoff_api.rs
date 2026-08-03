use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::Response,
    routing::post,
    Router,
};
use std::sync::Arc;

use crate::{
    project_auth::{can_edit, json_error},
    types::AppState,
};

use super::{
    api::{project_caller, service_response},
    sui_adapter_handoff_service,
};

pub(super) fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/api/projects/:project_id/economy/sui-projections/:projection_id/adapter-handoff",
            post(standard_handoff),
        )
        .route(
            "/api/projects/:project_id/economy/sui-correction-projections/:projection_id/adapter-handoff",
            post(correction_handoff),
        )
}

async fn standard_handoff(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, projection_id)): Path<(String, String)>,
) -> Response {
    let (_, role) = match project_caller(&state, &headers, &project_id) {
        Ok(value) => value,
        Err(response) => return response,
    };
    if !can_edit(&role) {
        return json_error(
            StatusCode::FORBIDDEN,
            "只有项目编辑者可以导出 Sui 适配器交接包",
        );
    }
    service_response(sui_adapter_handoff_service::standard(
        &state.store,
        &project_id,
        &projection_id,
    ))
}

async fn correction_handoff(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, projection_id)): Path<(String, String)>,
) -> Response {
    let (_, role) = match project_caller(&state, &headers, &project_id) {
        Ok(value) => value,
        Err(response) => return response,
    };
    if !can_edit(&role) {
        return json_error(
            StatusCode::FORBIDDEN,
            "只有项目编辑者可以导出 Sui 纠正适配器交接包",
        );
    }
    service_response(sui_adapter_handoff_service::correction(
        &state.store,
        &project_id,
        &projection_id,
    ))
}
