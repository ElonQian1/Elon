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
    open_commerce_adapter_claim_model::{
        ClaimAdapterHandoffRequest, CompleteAdapterHandoffClaimRequest,
        ReleaseAdapterHandoffClaimRequest, RenewAdapterHandoffClaimRequest,
        ResumeAdapterHandoffClaimRequest,
    },
    open_commerce_adapter_claim_service,
    open_commerce_model::normalize_app_id,
    open_commerce_service::OpenCommerceActor,
    project_auth::{auth_from_headers, json_error, project_access},
    types::AppState,
};

pub(crate) fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/api/projects/:project_id/open-commerce/adapter-handoff-claims",
            get(list_claims),
        )
        .route(
            "/api/projects/:project_id/open-commerce/adapter-handoff-claims/:claim_id/resume",
            post(resume_retry),
        )
        .route(
            "/api/open-commerce/adapter/business-handoff-claims",
            post(claim_next),
        )
        .route(
            "/api/open-commerce/adapter/business-handoff-claims/:claim_id/complete",
            post(complete_claim),
        )
        .route(
            "/api/open-commerce/adapter/business-handoff-claims/:claim_id/release",
            post(release_claim),
        )
        .route(
            "/api/open-commerce/adapter/business-handoff-claims/:claim_id/renew",
            post(renew_claim),
        )
}

#[derive(Debug, Deserialize)]
struct ListQuery {
    #[serde(default = "default_limit")]
    limit: usize,
}

async fn list_claims(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
    Query(query): Query<ListQuery>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(value) => value,
        Err(error) => return json_error(StatusCode::UNAUTHORIZED, error),
    };
    if let Err(error) = project_access(&state, &user.id, &project_id) {
        return json_error(StatusCode::FORBIDDEN, error);
    }
    service_response(open_commerce_adapter_claim_service::list_claims(
        &state.store,
        &project_id,
        query.limit,
    ))
}

async fn resume_retry(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, claim_id)): Path<(String, String)>,
    Json(request): Json<ResumeAdapterHandoffClaimRequest>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(value) => value,
        Err(error) => return json_error(StatusCode::UNAUTHORIZED, error),
    };
    let access = match project_access(&state, &user.id, &project_id) {
        Ok(value) => value,
        Err(error) => return json_error(StatusCode::FORBIDDEN, error),
    };
    let raw_app_id = headers
        .get("x-elon-app-id")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("pc-web");
    let app_id = match normalize_app_id(raw_app_id) {
        Ok(value) => value,
        Err(error) => return json_error(StatusCode::BAD_REQUEST, error),
    };
    service_response(open_commerce_adapter_claim_service::resume_retry(
        &state.store,
        &project_id,
        &claim_id,
        request,
        &OpenCommerceActor {
            user_id: &user.id,
            app_id: &app_id,
            project_role: Some(&access.role),
        },
    ))
}

async fn claim_next(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(request): Json<ClaimAdapterHandoffRequest>,
) -> Response {
    let credential = match authenticate(&state, &headers) {
        Ok(value) => value,
        Err(response) => return response,
    };
    service_response(open_commerce_adapter_claim_service::claim_next(
        &state.store,
        &credential,
        request.lease_seconds,
    ))
}

async fn complete_claim(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(claim_id): Path<String>,
    Json(request): Json<CompleteAdapterHandoffClaimRequest>,
) -> Response {
    let credential = match authenticate(&state, &headers) {
        Ok(value) => value,
        Err(response) => return response,
    };
    service_response(open_commerce_adapter_claim_service::complete_claim(
        &state.store,
        &credential,
        &claim_id,
        request,
    ))
}

async fn release_claim(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(claim_id): Path<String>,
    Json(request): Json<ReleaseAdapterHandoffClaimRequest>,
) -> Response {
    let credential = match authenticate(&state, &headers) {
        Ok(value) => value,
        Err(response) => return response,
    };
    service_response(open_commerce_adapter_claim_service::release_claim(
        &state.store,
        &credential,
        &claim_id,
        request,
    ))
}

async fn renew_claim(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(claim_id): Path<String>,
    Json(request): Json<RenewAdapterHandoffClaimRequest>,
) -> Response {
    let credential = match authenticate(&state, &headers) {
        Ok(value) => value,
        Err(response) => return response,
    };
    service_response(open_commerce_adapter_claim_service::renew_claim(
        &state.store,
        &credential,
        &claim_id,
        request,
    ))
}

fn authenticate(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<crate::open_commerce_adapter_model::OpenCommerceAdapterCredential, Response> {
    let token = bearer_token(headers)
        .ok_or_else(|| json_error(StatusCode::UNAUTHORIZED, "缺少适配器 Bearer 凭据"))?;
    state
        .store
        .authenticate_open_commerce_adapter_credential(token)
        .map_err(|error| json_error(StatusCode::UNAUTHORIZED, format!("{error:#}")))
}

fn bearer_token(headers: &HeaderMap) -> Option<&str> {
    headers
        .get("authorization")?
        .to_str()
        .ok()?
        .strip_prefix("Bearer ")
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn service_response<T: serde::Serialize>(result: anyhow::Result<T>) -> Response {
    match result {
        Ok(value) => Json(value).into_response(),
        Err(error) => {
            let message = format!("{error:#}");
            let status = if message.contains("不存在") {
                StatusCode::NOT_FOUND
            } else if message.contains("租约无效") || message.contains("并发冲突") {
                StatusCode::CONFLICT
            } else if message.contains("未获得") || message.contains("只有项目编辑者") {
                StatusCode::FORBIDDEN
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
