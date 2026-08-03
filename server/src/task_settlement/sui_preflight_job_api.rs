use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::Response,
    routing::{get, post},
    Json, Router,
};
use std::sync::Arc;

use crate::{project_auth::json_error, types::AppState};

use super::{
    api::{project_caller, service_response},
    sui_preflight_api::bearer_token,
    sui_preflight_job_model::{
        CancelSuiPreflightJobRequest, ClaimSuiPreflightJobRequest, CompleteSuiPreflightJobRequest,
        QueueSuiPreflightJobRequest, ReleaseSuiPreflightJobRequest, RenewSuiPreflightJobRequest,
    },
    sui_preflight_job_service as service,
};

pub(super) fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/api/projects/:project_id/economy/sui-preflight-jobs",
            get(list_jobs).post(queue_job),
        )
        .route(
            "/api/projects/:project_id/economy/sui-preflight-jobs/:job_id/cancel",
            post(cancel_job),
        )
        .route("/api/economy/sui-preflight/jobs/claim", post(claim_job))
        .route(
            "/api/economy/sui-preflight/jobs/:job_id/renew",
            post(renew_job),
        )
        .route(
            "/api/economy/sui-preflight/jobs/:job_id/release",
            post(release_job),
        )
        .route(
            "/api/economy/sui-preflight/jobs/:job_id/complete",
            post(complete_job),
        )
}

async fn list_jobs(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
) -> Response {
    if let Err(response) = project_caller(&state, &headers, &project_id) {
        return response;
    }
    service_response(service::list(&state.store, &project_id))
}

async fn queue_job(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
    Json(request): Json<QueueSuiPreflightJobRequest>,
) -> Response {
    let (user_id, role) = match project_caller(&state, &headers, &project_id) {
        Ok(caller) => caller,
        Err(response) => return response,
    };
    service_response(service::queue(
        &state.store,
        &project_id,
        &user_id,
        &role,
        &request,
    ))
}

async fn cancel_job(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, job_id)): Path<(String, String)>,
    Json(request): Json<CancelSuiPreflightJobRequest>,
) -> Response {
    let (_, role) = match project_caller(&state, &headers, &project_id) {
        Ok(caller) => caller,
        Err(response) => return response,
    };
    service_response(service::cancel(
        &state.store,
        &project_id,
        &job_id,
        &role,
        &request,
    ))
}

async fn claim_job(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(request): Json<ClaimSuiPreflightJobRequest>,
) -> Response {
    let adapter = match machine_adapter(&state, &headers) {
        Ok(value) => value,
        Err(response) => return response,
    };
    service_response(service::claim_next(&state.store, &adapter, &request))
}

async fn renew_job(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(job_id): Path<String>,
    Json(request): Json<RenewSuiPreflightJobRequest>,
) -> Response {
    let adapter = match machine_adapter(&state, &headers) {
        Ok(value) => value,
        Err(response) => return response,
    };
    service_response(service::renew(&state.store, &adapter, &job_id, &request))
}

async fn release_job(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(job_id): Path<String>,
    Json(request): Json<ReleaseSuiPreflightJobRequest>,
) -> Response {
    let adapter = match machine_adapter(&state, &headers) {
        Ok(value) => value,
        Err(response) => return response,
    };
    service_response(service::release(&state.store, &adapter, &job_id, &request))
}

async fn complete_job(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(job_id): Path<String>,
    Json(request): Json<CompleteSuiPreflightJobRequest>,
) -> Response {
    let adapter = match machine_adapter(&state, &headers) {
        Ok(value) => value,
        Err(response) => return response,
    };
    service_response(service::complete(&state.store, &adapter, &job_id, &request))
}

fn machine_adapter(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<super::sui_preflight_model::SuiPreflightAdapter, Response> {
    let token = bearer_token(headers)
        .ok_or_else(|| json_error(StatusCode::UNAUTHORIZED, "缺少 Sui 预检适配器 Bearer 凭据"))?;
    state
        .store
        .authenticate_task_sui_preflight_adapter(token)
        .map_err(|error| json_error(StatusCode::UNAUTHORIZED, format!("{error:#}")))
}
