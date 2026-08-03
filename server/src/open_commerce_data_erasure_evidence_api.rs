use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use std::sync::Arc;

use crate::{
    open_commerce_data_erasure_evidence_model::CreateDataErasureEvidenceRequest,
    open_commerce_data_erasure_evidence_service,
    open_commerce_service::OpenCommerceActor,
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
            "/api/projects/:project_id/open-commerce/consumer-data-erasure-evidence",
            get(list_consumer_evidence),
        )
        .route(
            "/api/projects/:project_id/open-commerce/merchants/:merchant_id/consumer-data-erasure-evidence",
            get(list_merchant_evidence),
        )
        .route(
            "/api/projects/:project_id/open-commerce/merchants/:merchant_id/consumer-data-requests/:request_id/evidence",
            post(create_merchant_evidence),
        )
}

async fn list_consumer_evidence(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
    Query(query): Query<EvidenceQuery>,
) -> Response {
    let caller = match project_caller(&state, &headers, &project_id) {
        Ok(value) => value,
        Err(response) => return response,
    };
    service_response(
        open_commerce_data_erasure_evidence_service::list_consumer_evidence(
            &state.store,
            &project_id,
            &actor(&caller),
            query.limit,
        ),
    )
}

async fn list_merchant_evidence(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, merchant_id)): Path<(String, String)>,
    Query(query): Query<EvidenceQuery>,
) -> Response {
    let caller = match project_caller(&state, &headers, &project_id) {
        Ok(value) => value,
        Err(response) => return response,
    };
    service_response(
        open_commerce_data_erasure_evidence_service::list_merchant_evidence(
            &state.store,
            &project_id,
            &merchant_id,
            &actor(&caller),
            query.limit,
        ),
    )
}

async fn create_merchant_evidence(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, merchant_id, request_id)): Path<(String, String, String)>,
    Json(request): Json<CreateDataErasureEvidenceRequest>,
) -> Response {
    let caller = match project_caller(&state, &headers, &project_id) {
        Ok(value) => value,
        Err(response) => return response,
    };
    service_response(
        open_commerce_data_erasure_evidence_service::create_merchant_evidence(
            &state.store,
            &project_id,
            &merchant_id,
            &request_id,
            &actor(&caller),
            request,
        ),
    )
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

fn default_limit() -> usize {
    200
}
