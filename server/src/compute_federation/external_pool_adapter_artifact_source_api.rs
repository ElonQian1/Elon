//! Authenticated platform-administrator API for external-pool Adapter byte quarantine.

use std::sync::Arc;

use axum::{
    body::Body,
    extract::{Path, State},
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};

use crate::{
    project_auth::{auth_from_headers, json_error},
    store::ExternalPoolAdapterArtifactSourceReceipt,
    types::AppState,
};

use super::{
    external_pool_adapter_artifact_source::ExternalPoolAdapterArtifactSourceFsError as ArtifactFsError,
    external_pool_adapter_artifact_source_service::{
        self as service, ExternalPoolAdapterArtifactSourceServiceError,
        PutExternalPoolAdapterArtifactSource,
    },
};

pub(crate) fn routes() -> Router<Arc<AppState>> {
    Router::new().route(
        "/api/admin/compute/external-pool-adapter-release-admissions/:admission_id/artifact-source",
        get(get_artifact_source).put(put_artifact_source),
    )
}

async fn put_artifact_source(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(admission_id): Path<String>,
    body: Body,
) -> Response {
    let admin_user_id = match platform_admin(&state, &headers) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let artifact_headers = match parse_artifact_source_headers(&headers) {
        Ok(value) => value,
        Err((status, message)) => return json_error(status, message),
    };
    artifact_response(
        service::put_for_admin(
            &state,
            &admin_user_id,
            &admission_id,
            PutExternalPoolAdapterArtifactSource {
                idempotency_key: artifact_headers.idempotency_key,
                expected_admission_digest: artifact_headers.expected_admission_digest,
                intake_confirmation: artifact_headers.intake_confirmation,
                body,
            },
        )
        .await,
        true,
    )
}

async fn get_artifact_source(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(admission_id): Path<String>,
) -> Response {
    if let Err(response) = platform_admin(&state, &headers) {
        return response;
    }
    artifact_response(service::get_for_admin(&state, &admission_id).await, false)
}

struct ArtifactSourceHeaders {
    idempotency_key: String,
    expected_admission_digest: String,
    intake_confirmation: String,
}

fn parse_artifact_source_headers(
    headers: &HeaderMap,
) -> Result<ArtifactSourceHeaders, (StatusCode, &'static str)> {
    let content_type = required_single_header(
        headers,
        header::CONTENT_TYPE.as_str(),
        StatusCode::UNSUPPORTED_MEDIA_TYPE,
        "Content-Type must be application/octet-stream",
    )?;
    if content_type != "application/octet-stream" || headers.contains_key(header::CONTENT_ENCODING)
    {
        return Err((
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            "artifact source requires unencoded application/octet-stream",
        ));
    }
    let idempotency_key = required_single_header(
        headers,
        "idempotency-key",
        StatusCode::BAD_REQUEST,
        "Idempotency-Key is required exactly once",
    )?;
    if idempotency_key.is_empty()
        || idempotency_key.len() > 160
        || idempotency_key.trim() != idempotency_key
        || idempotency_key.chars().any(char::is_control)
    {
        return Err((StatusCode::BAD_REQUEST, "Idempotency-Key is invalid"));
    }
    let expected_admission_digest = required_single_header(
        headers,
        "x-elon-expected-admission-digest",
        StatusCode::CONFLICT,
        "X-Elon-Expected-Admission-Digest is required exactly once",
    )?;
    if !lowercase_sha256(expected_admission_digest) {
        return Err((
            StatusCode::CONFLICT,
            "X-Elon-Expected-Admission-Digest must be a lowercase SHA-256 digest",
        ));
    }
    let intake_confirmation = required_single_header(
        headers,
        "x-elon-artifact-source-confirmation",
        StatusCode::CONFLICT,
        "X-Elon-Artifact-Source-Confirmation is required exactly once",
    )?;
    if intake_confirmation != service::intake_confirmation() {
        return Err((
            StatusCode::CONFLICT,
            "X-Elon-Artifact-Source-Confirmation is not exact",
        ));
    }
    Ok(ArtifactSourceHeaders {
        idempotency_key: idempotency_key.to_string(),
        expected_admission_digest: expected_admission_digest.to_string(),
        intake_confirmation: intake_confirmation.to_string(),
    })
}

fn required_single_header<'a>(
    headers: &'a HeaderMap,
    name: &str,
    status: StatusCode,
    message: &'static str,
) -> Result<&'a str, (StatusCode, &'static str)> {
    let mut values = headers.get_all(name).iter();
    let value = values.next().ok_or((status, message))?;
    if values.next().is_some() {
        return Err((status, message));
    }
    value.to_str().map_err(|_| (status, message))
}

fn lowercase_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn artifact_response(
    result: Result<
        ExternalPoolAdapterArtifactSourceReceipt,
        ExternalPoolAdapterArtifactSourceServiceError,
    >,
    created_when_new: bool,
) -> Response {
    match result {
        Ok(receipt) => {
            let status = if created_when_new && !receipt.replayed {
                StatusCode::CREATED
            } else {
                StatusCode::OK
            };
            (status, Json(receipt)).into_response()
        }
        Err(error) => artifact_error_response(error),
    }
}

fn artifact_error_response(error: ExternalPoolAdapterArtifactSourceServiceError) -> Response {
    let status = match &error {
        ExternalPoolAdapterArtifactSourceServiceError::NotFound => StatusCode::NOT_FOUND,
        ExternalPoolAdapterArtifactSourceServiceError::Conflict(_) => StatusCode::CONFLICT,
        ExternalPoolAdapterArtifactSourceServiceError::Filesystem(fs_error) => match fs_error {
            ArtifactFsError::BodyRead(_) => StatusCode::BAD_REQUEST,
            ArtifactFsError::PayloadTooLarge => StatusCode::PAYLOAD_TOO_LARGE,
            ArtifactFsError::EmptyBody | ArtifactFsError::IntakeDigestMismatch => {
                StatusCode::UNPROCESSABLE_ENTITY
            }
            ArtifactFsError::InvalidContentAddress
            | ArtifactFsError::BlobMissing
            | ArtifactFsError::UnsafeTarget
            | ArtifactFsError::BlobDrift
            | ArtifactFsError::Storage(_)
            | ArtifactFsError::Task(_) => StatusCode::INTERNAL_SERVER_ERROR,
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
            "只有平台管理员可以管理 external-pool Adapter artifact source",
        ));
    }
    Ok(user.id)
}
