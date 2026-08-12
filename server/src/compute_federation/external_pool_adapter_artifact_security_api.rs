//! Authenticated administrator API for V233 artifact security receipts.

use axum::{
    extract::{rejection::JsonRejection, Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use std::sync::Arc;

use super::external_pool_adapter_artifact_security_service::{
    self as service, ArtifactSecurityServiceError, ScanArtifactSecurityBody,
};
use crate::{
    project_auth::{auth_from_headers, json_error},
    types::AppState,
};

pub(crate) fn routes() -> Router<Arc<AppState>> {
    Router::new().route(
        "/api/admin/compute/external-pool-adapter-release-admissions/:admission_id/artifact-security",
        get(currentness).post(scan),
    )
}

async fn scan(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(admission_id): Path<String>,
    payload: Result<Json<ScanArtifactSecurityBody>, JsonRejection>,
) -> Response {
    let admin = match platform_admin(&state, &headers) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let body = match payload {
        Ok(Json(value)) => value,
        Err(error) => return json_error(StatusCode::BAD_REQUEST, error),
    };
    match service::scan_for_admin(&state, &admin, &admission_id, body).await {
        Ok(receipt) => {
            let status = if receipt.replayed {
                StatusCode::OK
            } else {
                StatusCode::CREATED
            };
            (status, Json(receipt)).into_response()
        }
        Err(error) => error_response(error),
    }
}

async fn currentness(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(admission_id): Path<String>,
) -> Response {
    if let Err(response) = platform_admin(&state, &headers) {
        return response;
    }
    match service::currentness_for_admin(&state, &admission_id).await {
        Ok(receipt) => Json(receipt).into_response(),
        Err(error) => error_response(error),
    }
}

fn error_response(error: ArtifactSecurityServiceError) -> Response {
    let status = match &error {
        ArtifactSecurityServiceError::NotFound => StatusCode::NOT_FOUND,
        ArtifactSecurityServiceError::Rejected(_) => StatusCode::UNPROCESSABLE_ENTITY,
        ArtifactSecurityServiceError::Conflict(_) => StatusCode::CONFLICT,
        ArtifactSecurityServiceError::Task(_) => StatusCode::INTERNAL_SERVER_ERROR,
        ArtifactSecurityServiceError::Filesystem(error) => match error {
            super::external_pool_adapter_artifact_source::ExternalPoolAdapterArtifactSourceFsError::BlobMissing
            | super::external_pool_adapter_artifact_source::ExternalPoolAdapterArtifactSourceFsError::BlobDrift
            | super::external_pool_adapter_artifact_source::ExternalPoolAdapterArtifactSourceFsError::UnsafeTarget => StatusCode::CONFLICT,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        },
    };
    json_error(status, error)
}

fn platform_admin(state: &AppState, headers: &HeaderMap) -> Result<String, Response> {
    let user = auth_from_headers(state, headers)
        .map_err(|error| json_error(StatusCode::UNAUTHORIZED, error))?;
    if !matches!(user.role.as_str(), "admin" | "owner") {
        return Err(json_error(
            StatusCode::FORBIDDEN,
            "只有平台管理员可以扫描 external-pool Adapter Artifact",
        ));
    }
    Ok(user.id)
}
