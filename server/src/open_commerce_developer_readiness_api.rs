use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use std::sync::Arc;

use crate::{
    open_commerce_developer_manifest_service, open_commerce_developer_readiness_service,
    open_commerce_service::OpenCommerceActor,
    project_auth::{auth_from_headers, json_error, project_access},
    types::AppState,
};

pub(crate) fn routes() -> Router<Arc<AppState>> {
    Router::new().route(
        "/api/projects/:project_id/open-commerce/developer-apps/:app_record_id/production-readiness",
        get(production_readiness),
    )
}

async fn production_readiness(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, app_record_id)): Path<(String, String)>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(value) => value,
        Err(error) => return json_error(StatusCode::UNAUTHORIZED, error),
    };
    let access = match project_access(&state, &user.id, &project_id) {
        Ok(value) => value,
        Err(error) => return json_error(StatusCode::FORBIDDEN, error),
    };
    let actor = OpenCommerceActor {
        user_id: &user.id,
        app_id: "pc-web",
        project_role: Some(&access.role),
    };
    let app = match open_commerce_developer_manifest_service::managed_app(
        &state.store,
        &project_id,
        &app_record_id,
        &actor,
    ) {
        Ok(value) => value,
        Err(error) => return json_error(StatusCode::FORBIDDEN, error),
    };
    match open_commerce_developer_readiness_service::readiness_summary(&state.store, &app) {
        Ok(value) => Json(value).into_response(),
        Err(error) => json_error(StatusCode::BAD_REQUEST, error),
    }
}
