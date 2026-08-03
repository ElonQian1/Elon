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
    sui_preflight_model::{
        ConfirmSuiPreflightAdapterChangeRequest, CreateSuiPreflightAdapterRequest,
        RecordSuiPreflightReportRequest, RotateSuiPreflightAdapterRequest,
    },
    sui_preflight_service as service,
};

pub(super) fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/api/projects/:project_id/economy/sui-preflight-adapters",
            get(list_adapters).post(create_adapter),
        )
        .route(
            "/api/projects/:project_id/economy/sui-preflight-adapters/:adapter_id/rotate",
            post(rotate_adapter),
        )
        .route(
            "/api/projects/:project_id/economy/sui-preflight-adapters/:adapter_id/disable",
            post(disable_adapter),
        )
        .route(
            "/api/projects/:project_id/economy/sui-preflight-reports",
            get(list_reports),
        )
        .route(
            "/api/economy/sui-preflight/reports",
            post(record_machine_report),
        )
}

async fn list_adapters(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
) -> Response {
    if let Err(response) = project_caller(&state, &headers, &project_id) {
        return response;
    }
    service_response(service::list_adapters(&state.store, &project_id))
}

async fn create_adapter(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
    Json(request): Json<CreateSuiPreflightAdapterRequest>,
) -> Response {
    let (user_id, role) = match project_caller(&state, &headers, &project_id) {
        Ok(caller) => caller,
        Err(response) => return response,
    };
    if !request.confirmed_by_user {
        return json_error(
            StatusCode::BAD_REQUEST,
            "签发 Sui 预检适配器凭据前必须取得用户明确确认",
        );
    }
    service_response(service::create_adapter(
        &state.store,
        &project_id,
        &user_id,
        &role,
        &request,
    ))
}

async fn rotate_adapter(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, adapter_id)): Path<(String, String)>,
    Json(request): Json<RotateSuiPreflightAdapterRequest>,
) -> Response {
    let (_, role) = match project_caller(&state, &headers, &project_id) {
        Ok(caller) => caller,
        Err(response) => return response,
    };
    if !request.confirmed_by_user {
        return json_error(
            StatusCode::BAD_REQUEST,
            "轮换 Sui 预检适配器凭据前必须取得用户明确确认",
        );
    }
    service_response(service::rotate_adapter(
        &state.store,
        &project_id,
        &adapter_id,
        request.expires_in_days,
        &role,
    ))
}

async fn disable_adapter(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, adapter_id)): Path<(String, String)>,
    Json(request): Json<ConfirmSuiPreflightAdapterChangeRequest>,
) -> Response {
    let (_, role) = match project_caller(&state, &headers, &project_id) {
        Ok(caller) => caller,
        Err(response) => return response,
    };
    if !request.confirmed_by_user {
        return json_error(
            StatusCode::BAD_REQUEST,
            "停用 Sui 预检适配器前必须取得用户明确确认",
        );
    }
    service_response(service::disable_adapter(
        &state.store,
        &project_id,
        &adapter_id,
        &role,
    ))
}

async fn list_reports(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
) -> Response {
    if let Err(response) = project_caller(&state, &headers, &project_id) {
        return response;
    }
    service_response(service::list_reports(&state.store, &project_id))
}

async fn record_machine_report(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(request): Json<RecordSuiPreflightReportRequest>,
) -> Response {
    let token = match bearer_token(&headers) {
        Some(value) => value,
        None => return json_error(StatusCode::UNAUTHORIZED, "缺少 Sui 预检适配器 Bearer 凭据"),
    };
    let adapter = match state.store.authenticate_task_sui_preflight_adapter(token) {
        Ok(value) => value,
        Err(error) => return json_error(StatusCode::UNAUTHORIZED, format!("{error:#}")),
    };
    service_response(service::record_report(&state.store, &adapter, &request))
}

pub(super) fn bearer_token(headers: &HeaderMap) -> Option<&str> {
    headers
        .get("authorization")?
        .to_str()
        .ok()?
        .strip_prefix("Bearer ")
        .map(str::trim)
        .filter(|value| !value.is_empty())
}
