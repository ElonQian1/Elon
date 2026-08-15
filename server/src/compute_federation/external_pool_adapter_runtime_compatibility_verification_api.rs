//! Platform-administrator HTTP surface for V268 signed runtime compatibility evidence.

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

use super::external_pool_adapter_runtime_compatibility_signing_handoff_service::{
    self as signing_handoff_service, RuntimeCompatibilitySigningHandoffBody,
    RuntimeCompatibilitySigningHandoffResponse, RuntimeCompatibilitySigningHandoffServiceError,
};
use super::external_pool_adapter_runtime_compatibility_verification_service::{
    self as service, CreateRuntimeCompatibilityChallengeBody,
    RecordRuntimeCompatibilityVerificationBody, RevokeRuntimeCompatibilityVerificationBody,
    RuntimeCompatibilityVerificationServiceError,
};

const PROFILE_V2_PATH: &str =
    "/api/admin/compute/external-pool-adapter-runtime-compatibility-profile-v2";
const CHALLENGE_PATH: &str = "/api/admin/compute/external-pool-adapter-registry-releases/:registry_release_id/runtime-compatibility-verifications/challenge";
const VERIFICATIONS_PATH: &str = "/api/admin/compute/external-pool-adapter-registry-releases/:registry_release_id/runtime-compatibility-verifications";
const CURRENTNESS_PATH: &str = "/api/admin/compute/external-pool-adapter-registry-releases/:registry_release_id/runtime-compatibility-verifications/currentness";
const REVOCATION_PATH: &str = "/api/admin/compute/external-pool-adapter-registry-releases/:registry_release_id/runtime-compatibility-verifications/:verification_receipt_id/revoke";
const SIGNING_HANDOFF_PATH: &str = "/api/admin/compute/external-pool-adapter-registry-releases/:registry_release_id/runtime-compatibility-verifications/:challenge_id/signing-handoff";

pub(crate) fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route(PROFILE_V2_PATH, get(profile_v2))
        .route(CHALLENGE_PATH, post(challenge))
        .route(VERIFICATIONS_PATH, post(record))
        .route(CURRENTNESS_PATH, get(currentness))
        .route(REVOCATION_PATH, post(revoke))
        .route(SIGNING_HANDOFF_PATH, post(signing_handoff))
}

async fn profile_v2(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    if let Err(response) = platform_admin(&state, &headers) {
        return response;
    }
    read_response(service::profile_v2_for_admin())
}

async fn challenge(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(registry_release_id): Path<String>,
    payload: Result<Json<CreateRuntimeCompatibilityChallengeBody>, JsonRejection>,
) -> Response {
    let actor = match platform_admin(&state, &headers) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let body = match json_body(payload) {
        Ok(value) => value,
        Err(response) => return response,
    };
    write_response(service::challenge_for_admin(
        &state.store,
        &actor,
        &registry_release_id,
        body,
    ))
}

async fn record(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(registry_release_id): Path<String>,
    payload: Result<Json<RecordRuntimeCompatibilityVerificationBody>, JsonRejection>,
) -> Response {
    let actor = match platform_admin(&state, &headers) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let body = match json_body(payload) {
        Ok(value) => value,
        Err(response) => return response,
    };
    write_response(service::record_for_admin(
        &state.store,
        &actor,
        &registry_release_id,
        body,
    ))
}

async fn currentness(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(registry_release_id): Path<String>,
) -> Response {
    if let Err(response) = platform_admin(&state, &headers) {
        return response;
    }
    read_response(service::currentness_for_admin(
        &state.store,
        &registry_release_id,
    ))
}

async fn revoke(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((registry_release_id, verification_receipt_id)): Path<(String, String)>,
    payload: Result<Json<RevokeRuntimeCompatibilityVerificationBody>, JsonRejection>,
) -> Response {
    let actor = match platform_admin(&state, &headers) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let body = match json_body(payload) {
        Ok(value) => value,
        Err(response) => return response,
    };
    write_response(service::revoke_for_admin(
        &state.store,
        &actor,
        &registry_release_id,
        &verification_receipt_id,
        body,
    ))
}

async fn signing_handoff(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((registry_release_id, challenge_id)): Path<(String, String)>,
    payload: Result<Json<RuntimeCompatibilitySigningHandoffBody>, JsonRejection>,
) -> Response {
    let actor = match platform_admin(&state, &headers) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let body = match json_body(payload) {
        Ok(value) => value,
        Err(response) => return response,
    };
    signing_handoff_response(
        signing_handoff_service::signing_handoff_for_admin(
            state,
            &actor,
            &registry_release_id,
            &challenge_id,
            body,
        )
        .await,
    )
}

fn write_response(
    result: Result<serde_json::Value, RuntimeCompatibilityVerificationServiceError>,
) -> Response {
    match result {
        Ok(output) => {
            let status = if output
                .get("replayed")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false)
            {
                StatusCode::OK
            } else {
                StatusCode::CREATED
            };
            (status, Json(output)).into_response()
        }
        Err(error) => error_response(error),
    }
}

fn read_response(
    result: Result<serde_json::Value, RuntimeCompatibilityVerificationServiceError>,
) -> Response {
    match result {
        Ok(output) => Json(output).into_response(),
        Err(error) => error_response(error),
    }
}

fn signing_handoff_response(
    result: Result<
        RuntimeCompatibilitySigningHandoffResponse,
        RuntimeCompatibilitySigningHandoffServiceError,
    >,
) -> Response {
    match result {
        Ok(output) => {
            let status = if output.replayed {
                StatusCode::OK
            } else {
                StatusCode::CREATED
            };
            (status, Json(output)).into_response()
        }
        Err(error) => signing_handoff_error_response(error),
    }
}

fn json_body<T>(payload: Result<Json<T>, JsonRejection>) -> Result<T, Response> {
    payload
        .map(|Json(value)| value)
        .map_err(|error| json_error(StatusCode::UNPROCESSABLE_ENTITY, error))
}

fn error_response(error: RuntimeCompatibilityVerificationServiceError) -> Response {
    let status = match error {
        RuntimeCompatibilityVerificationServiceError::NotFound => StatusCode::NOT_FOUND,
        RuntimeCompatibilityVerificationServiceError::Invalid(_) => StatusCode::BAD_REQUEST,
        RuntimeCompatibilityVerificationServiceError::Conflict(_) => StatusCode::CONFLICT,
        RuntimeCompatibilityVerificationServiceError::Internal(_) => {
            StatusCode::INTERNAL_SERVER_ERROR
        }
    };
    json_error(status, error)
}

fn signing_handoff_error_response(
    error: RuntimeCompatibilitySigningHandoffServiceError,
) -> Response {
    let status = match error {
        RuntimeCompatibilitySigningHandoffServiceError::NotFound => StatusCode::NOT_FOUND,
        RuntimeCompatibilitySigningHandoffServiceError::Invalid(_) => StatusCode::BAD_REQUEST,
        RuntimeCompatibilitySigningHandoffServiceError::Conflict(_) => StatusCode::CONFLICT,
        RuntimeCompatibilitySigningHandoffServiceError::Unavailable(_) => {
            StatusCode::SERVICE_UNAVAILABLE
        }
        RuntimeCompatibilitySigningHandoffServiceError::Internal(_) => {
            StatusCode::INTERNAL_SERVER_ERROR
        }
    };
    json_error(status, error)
}

fn platform_admin(state: &AppState, headers: &HeaderMap) -> Result<String, Response> {
    let user = auth_from_headers(state, headers)
        .map_err(|error| json_error(StatusCode::UNAUTHORIZED, error))?;
    if !matches!(user.role.as_str(), "admin" | "owner") {
        return Err(json_error(
            StatusCode::FORBIDDEN,
            "only platform administrators can manage Adapter runtime compatibility verifications",
        ));
    }
    Ok(user.id)
}
