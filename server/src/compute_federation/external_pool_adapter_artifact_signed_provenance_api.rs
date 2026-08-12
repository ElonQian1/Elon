//! Authenticated administrator API for exact Artifact signed provenance.

use std::sync::Arc;

use axum::{
    extract::{rejection::JsonRejection, Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};

use crate::{
    project_auth::{auth_from_headers, json_error},
    types::AppState,
};

use super::external_pool_adapter_artifact_signed_provenance_service::{
    self as service, ArtifactSignatureChallengeBody, ArtifactSignedProvenanceServiceError,
    RecordArtifactSignedProvenanceBody,
};

pub(crate) fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/api/admin/compute/external-pool-adapter-release-admissions/:admission_id/artifact-signed-provenance/challenge",
            post(challenge),
        )
        .route(
            "/api/admin/compute/external-pool-adapter-release-admissions/:admission_id/artifact-signed-provenance",
            get(currentness).post(record),
        )
}

async fn challenge(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(admission_id): Path<String>,
    payload: Result<Json<ArtifactSignatureChallengeBody>, JsonRejection>,
) -> Response {
    if let Err(response) = platform_admin(&state, &headers) {
        return response;
    }
    let body = match json_body(payload) {
        Ok(value) => value,
        Err(response) => return response,
    };
    match service::challenge_for_admin(&state, &admission_id, body).await {
        Ok(receipt) => Json(receipt).into_response(),
        Err(error) => error_response(error),
    }
}

async fn record(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(admission_id): Path<String>,
    payload: Result<Json<RecordArtifactSignedProvenanceBody>, JsonRejection>,
) -> Response {
    let admin_user_id = match platform_admin(&state, &headers) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let body = match json_body(payload) {
        Ok(value) => value,
        Err(response) => return response,
    };
    match service::record_for_admin(&state, &admin_user_id, &admission_id, body).await {
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

fn json_body<T>(payload: Result<Json<T>, JsonRejection>) -> Result<T, Response> {
    payload
        .map(|Json(value)| value)
        .map_err(|error| json_error(StatusCode::BAD_REQUEST, error))
}

fn error_response(error: ArtifactSignedProvenanceServiceError) -> Response {
    let status = match &error {
        ArtifactSignedProvenanceServiceError::NotFound => StatusCode::NOT_FOUND,
        ArtifactSignedProvenanceServiceError::Invalid(_) => StatusCode::BAD_REQUEST,
        ArtifactSignedProvenanceServiceError::Conflict(_) => StatusCode::CONFLICT,
        ArtifactSignedProvenanceServiceError::Filesystem(error) => match error {
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
            "只有平台管理员可以验证 external-pool Adapter Artifact 签名来源",
        ));
    }
    Ok(user.id)
}
