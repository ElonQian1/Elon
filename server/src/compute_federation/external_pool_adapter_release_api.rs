//! Authenticated platform-administrator API for staging external-pool Adapter releases.

use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use serde_json::json;

use crate::{
    project_auth::{auth_from_headers, json_error},
    types::AppState,
};

use super::external_pool_adapter_release_service::{
    self as service, ReviewExternalPoolAdapterReleaseBody, StageExternalPoolAdapterReleaseBody,
    SubmitExternalPoolAdapterReleaseBody,
};

#[cfg(test)]
#[path = "external_pool_adapter_release_api_tests.rs"]
mod tests;

pub(crate) fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/api/admin/compute/external-pool-adapter-releases",
            get(list_releases).post(submit_release),
        )
        .route(
            "/api/admin/compute/external-pool-adapter-releases/:request_id",
            get(get_release),
        )
        .route(
            "/api/admin/compute/external-pool-adapter-releases/:request_id/preflight",
            get(preflight_release),
        )
        .route(
            "/api/admin/compute/external-pool-adapter-releases/:request_id/review",
            post(review_release),
        )
        .route(
            "/api/admin/compute/external-pool-adapter-releases/:request_id/stage",
            post(stage_release),
        )
        .merge(super::external_pool_adapter_artifact_source_api::routes())
        .merge(super::external_pool_adapter_artifact_signing_key_api::routes())
        .merge(super::external_pool_adapter_artifact_signed_provenance_api::routes())
        .merge(super::external_pool_adapter_artifact_package_api::routes())
        .merge(super::external_pool_adapter_artifact_security_api::routes())
        .merge(super::external_pool_adapter_artifact_vulnerability_report_api::routes())
        .merge(super::external_pool_adapter_artifact_sandbox_conformance_api::routes())
        .merge(super::external_pool_adapter_scanner_key_api::routes())
        .merge(super::external_pool_adapter_sandbox_verifier_key_api::routes())
        .merge(super::external_pool_adapter_credential_verifier_api::routes())
        .merge(super::external_pool_adapter_credential_verifier_key_api::routes())
        .merge(super::external_pool_adapter_credential_verification_api::routes())
        .merge(super::external_pool_adapter_adoption_api::routes())
        .merge(super::external_pool_adapter_installation_api::routes())
        .merge(super::external_pool_adapter_release_lifecycle_api::routes())
}

#[derive(Debug, Deserialize)]
struct ListQuery {
    status: Option<String>,
    #[serde(default = "default_limit")]
    limit: usize,
}

async fn list_releases(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(query): Query<ListQuery>,
) -> Response {
    if let Err(response) = platform_admin(&state, &headers) {
        return response;
    }
    release_response(
        service::list_for_admin(&state.store, query.status.as_deref(), query.limit)
            .map(|items| json!({"adapter_release_requests": items})),
    )
}

async fn get_release(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(request_id): Path<String>,
) -> Response {
    if let Err(response) = platform_admin(&state, &headers) {
        return response;
    }
    release_response(service::get_for_admin(&state.store, &request_id))
}

async fn preflight_release(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(request_id): Path<String>,
) -> Response {
    let admin_user_id = match platform_admin(&state, &headers) {
        Ok(value) => value,
        Err(response) => return response,
    };
    release_response(service::preflight_for_admin(
        &state.store,
        &admin_user_id,
        &request_id,
    ))
}

async fn submit_release(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<SubmitExternalPoolAdapterReleaseBody>,
) -> Response {
    let admin_user_id = match platform_admin(&state, &headers) {
        Ok(value) => value,
        Err(response) => return response,
    };
    release_response(service::submit_for_admin(
        &state.store,
        &admin_user_id,
        body,
    ))
}

async fn review_release(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(request_id): Path<String>,
    Json(body): Json<ReviewExternalPoolAdapterReleaseBody>,
) -> Response {
    let admin_user_id = match platform_admin(&state, &headers) {
        Ok(value) => value,
        Err(response) => return response,
    };
    release_response(service::review_for_admin(
        &state.store,
        &admin_user_id,
        &request_id,
        body,
    ))
}

async fn stage_release(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(request_id): Path<String>,
    Json(body): Json<StageExternalPoolAdapterReleaseBody>,
) -> Response {
    let admin_user_id = match platform_admin(&state, &headers) {
        Ok(value) => value,
        Err(response) => return response,
    };
    release_response(service::stage_for_admin(
        &state.store,
        &admin_user_id,
        &request_id,
        body,
    ))
}

fn platform_admin(state: &AppState, headers: &HeaderMap) -> Result<String, Response> {
    let user = auth_from_headers(state, headers)
        .map_err(|error| json_error(StatusCode::UNAUTHORIZED, error))?;
    if !matches!(user.role.as_str(), "admin" | "owner") {
        return Err(json_error(
            StatusCode::FORBIDDEN,
            "只有平台管理员可以管理 external-pool Adapter release",
        ));
    }
    Ok(user.id)
}

fn default_limit() -> usize {
    50
}

fn release_response<T: serde::Serialize>(result: anyhow::Result<T>) -> Response {
    match result {
        Ok(value) => Json(value).into_response(),
        Err(error) => json_error(StatusCode::BAD_REQUEST, error),
    }
}
