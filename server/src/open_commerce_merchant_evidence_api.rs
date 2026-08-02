use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use serde::Deserialize;
use std::sync::Arc;

use crate::{
    open_commerce_merchant_evidence_service,
    project_auth::{auth_from_headers, json_error, project_access},
    types::AppState,
};

#[derive(Debug, Deserialize)]
struct EvidenceQuery {
    #[serde(default = "default_limit")]
    limit: usize,
}

pub(crate) fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/api/projects/:project_id/open-commerce/merchants/:merchant_id/business-evidence",
            get(list_evidence),
        )
        .route(
            "/api/projects/:project_id/open-commerce/merchants/:merchant_id/business-evidence/:invocation_id",
            get(get_evidence),
        )
}

async fn list_evidence(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, merchant_id)): Path<(String, String)>,
    Query(query): Query<EvidenceQuery>,
) -> Response {
    if let Err(response) = authorize(&state, &headers, &project_id) {
        return response;
    }
    service_response(open_commerce_merchant_evidence_service::list_evidence(
        &state.store,
        &project_id,
        &merchant_id,
        query.limit,
    ))
}

async fn get_evidence(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, merchant_id, invocation_id)): Path<(String, String, String)>,
) -> Response {
    if let Err(response) = authorize(&state, &headers, &project_id) {
        return response;
    }
    service_response(open_commerce_merchant_evidence_service::get_evidence(
        &state.store,
        &project_id,
        &merchant_id,
        &invocation_id,
    ))
}

fn authorize(state: &AppState, headers: &HeaderMap, project_id: &str) -> Result<(), Response> {
    let user = auth_from_headers(state, headers)
        .map_err(|error| json_error(StatusCode::UNAUTHORIZED, error))?;
    project_access(state, &user.id, project_id)
        .map_err(|error| json_error(StatusCode::FORBIDDEN, error))?;
    Ok(())
}

fn service_response<T: serde::Serialize>(result: anyhow::Result<T>) -> Response {
    match result {
        Ok(value) => Json(value).into_response(),
        Err(error) => {
            let message = format!("{error:#}");
            let status = if message.contains("不存在") {
                StatusCode::NOT_FOUND
            } else {
                StatusCode::BAD_REQUEST
            };
            json_error(status, message)
        }
    }
}

fn default_limit() -> usize {
    50
}
