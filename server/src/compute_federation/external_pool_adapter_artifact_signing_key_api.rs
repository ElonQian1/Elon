//! Authenticated platform-administrator HTTP API for Artifact signer trust keys.

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

use super::external_pool_adapter_artifact_signing_key_service::{
    self as service, ActivateExternalPoolAdapterArtifactSigningKeyBody,
    ExternalPoolAdapterArtifactSigningKeyServiceError,
    RegisterExternalPoolAdapterArtifactSigningKeyBody,
    RevokeExternalPoolAdapterArtifactSigningKeyBody,
};

pub(crate) fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/api/admin/compute/external-pool-adapter-artifact-signing-keys",
            post(register),
        )
        .route(
            "/api/admin/compute/external-pool-adapter-artifact-signing-keys/:key_record_id/activate",
            post(activate),
        )
        .route(
            "/api/admin/compute/external-pool-adapter-artifact-signing-keys/:key_record_id/revoke",
            post(revoke),
        )
        .route(
            "/api/admin/compute/external-pool-adapter-artifact-signing-keys/:key_record_id/currentness",
            get(currentness),
        )
}

async fn register(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    payload: Result<Json<RegisterExternalPoolAdapterArtifactSigningKeyBody>, JsonRejection>,
) -> Response {
    let admin_user_id = match platform_admin(&state, &headers) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let body = match json_body(payload) {
        Ok(value) => value,
        Err(response) => return response,
    };
    write_response(service::register_for_admin(
        &state.store,
        &admin_user_id,
        body,
    ))
}

async fn activate(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(key_record_id): Path<String>,
    payload: Result<Json<ActivateExternalPoolAdapterArtifactSigningKeyBody>, JsonRejection>,
) -> Response {
    let admin_user_id = match platform_admin(&state, &headers) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let body = match json_body(payload) {
        Ok(value) => value,
        Err(response) => return response,
    };
    write_response(service::activate_for_admin(
        &state.store,
        &admin_user_id,
        &key_record_id,
        body,
    ))
}

async fn revoke(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(key_record_id): Path<String>,
    payload: Result<Json<RevokeExternalPoolAdapterArtifactSigningKeyBody>, JsonRejection>,
) -> Response {
    let admin_user_id = match platform_admin(&state, &headers) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let body = match json_body(payload) {
        Ok(value) => value,
        Err(response) => return response,
    };
    write_response(service::revoke_for_admin(
        &state.store,
        &admin_user_id,
        &key_record_id,
        body,
    ))
}

async fn currentness(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(key_record_id): Path<String>,
) -> Response {
    if let Err(response) = platform_admin(&state, &headers) {
        return response;
    }
    match service::currentness_for_admin(&state.store, &key_record_id) {
        Ok(receipt) => Json(receipt).into_response(),
        Err(error) => error_response(error),
    }
}

trait ReplayStatus {
    fn replayed(&self) -> bool;
}

impl ReplayStatus for crate::store::ExternalPoolAdapterArtifactSigningKeyRegistrationWriteReceipt {
    fn replayed(&self) -> bool {
        self.replayed
    }
}

impl ReplayStatus for crate::store::ExternalPoolAdapterArtifactSigningKeyActivationWriteReceipt {
    fn replayed(&self) -> bool {
        self.replayed
    }
}

impl ReplayStatus for crate::store::ExternalPoolAdapterArtifactSigningKeyRevocationWriteReceipt {
    fn replayed(&self) -> bool {
        self.replayed
    }
}

fn write_response<T>(
    result: Result<T, ExternalPoolAdapterArtifactSigningKeyServiceError>,
) -> Response
where
    T: serde::Serialize + ReplayStatus,
{
    match result {
        Ok(receipt) => {
            let status = if receipt.replayed() {
                StatusCode::OK
            } else {
                StatusCode::CREATED
            };
            (status, Json(receipt)).into_response()
        }
        Err(error) => error_response(error),
    }
}

fn json_body<T>(payload: Result<Json<T>, JsonRejection>) -> Result<T, Response> {
    payload
        .map(|Json(value)| value)
        .map_err(|error| json_error(StatusCode::BAD_REQUEST, error))
}

fn error_response(error: ExternalPoolAdapterArtifactSigningKeyServiceError) -> Response {
    let status = match &error {
        ExternalPoolAdapterArtifactSigningKeyServiceError::NotFound => StatusCode::NOT_FOUND,
        ExternalPoolAdapterArtifactSigningKeyServiceError::Invalid(_) => StatusCode::BAD_REQUEST,
        ExternalPoolAdapterArtifactSigningKeyServiceError::Conflict(_) => StatusCode::CONFLICT,
    };
    json_error(status, error)
}

fn platform_admin(state: &AppState, headers: &HeaderMap) -> Result<String, Response> {
    let user = auth_from_headers(state, headers)
        .map_err(|error| json_error(StatusCode::UNAUTHORIZED, error))?;
    if !matches!(user.role.as_str(), "admin" | "owner") {
        return Err(json_error(
            StatusCode::FORBIDDEN,
            "只有平台管理员可以管理 external-pool Adapter Artifact signer keys",
        ));
    }
    Ok(user.id)
}
