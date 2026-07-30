use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, patch},
    Json, Router,
};
use std::sync::Arc;

use crate::{
    project_auth::{auth_from_headers, can_edit, json_error, project_access},
    types::AppState,
};

use super::{model::UpdateTaskEconomyProjectSettingRequest, service};

pub(crate) fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/api/projects/:project_id/economy/overview",
            get(project_overview),
        )
        .route(
            "/api/projects/:project_id/economy/settings",
            patch(update_project_setting),
        )
        .route(
            "/api/projects/:project_id/economy/settlements/:receipt_id",
            get(settlement_detail),
        )
        .route(
            "/api/projects/:project_id/economy/settlements/:receipt_id/sui-envelope",
            get(sui_envelope),
        )
}

async fn project_overview(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
) -> Response {
    if let Err(response) = project_caller(&state, &headers, &project_id) {
        return response;
    }
    service_response(service::overview(&state.store, &project_id))
}

async fn update_project_setting(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
    Json(request): Json<UpdateTaskEconomyProjectSettingRequest>,
) -> Response {
    let (user_id, role) = match project_caller(&state, &headers, &project_id) {
        Ok(caller) => caller,
        Err(response) => return response,
    };
    if !can_edit(&role) {
        return json_error(StatusCode::FORBIDDEN, "只有项目编辑者可以修改影子经济设置");
    }
    service_response(state.store.set_task_economy_project_enabled(
        &project_id,
        &user_id,
        request.enabled,
    ))
}

async fn settlement_detail(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, receipt_id)): Path<(String, String)>,
) -> Response {
    if let Err(response) = project_caller(&state, &headers, &project_id) {
        return response;
    }
    service_response(service::receipt_detail(
        &state.store,
        &project_id,
        &receipt_id,
    ))
}

async fn sui_envelope(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, receipt_id)): Path<(String, String)>,
) -> Response {
    if let Err(response) = project_caller(&state, &headers, &project_id) {
        return response;
    }
    service_response(service::sui_envelope(
        &state.store,
        &project_id,
        &receipt_id,
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

fn service_response<T: serde::Serialize>(result: anyhow::Result<T>) -> Response {
    match result {
        Ok(value) => Json(value).into_response(),
        Err(error) => {
            let message = format!("{error:#}");
            let status = if message.contains("权限") {
                StatusCode::FORBIDDEN
            } else if message.contains("不存在") {
                StatusCode::NOT_FOUND
            } else if message.contains("冲突")
                || message.contains("幂等")
                || message.contains("不能")
            {
                StatusCode::CONFLICT
            } else {
                StatusCode::BAD_REQUEST
            };
            json_error(status, message)
        }
    }
}
