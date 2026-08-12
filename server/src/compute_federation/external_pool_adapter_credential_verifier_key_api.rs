//! Authenticated platform-administrator API for V242 credential-verifier signing keys.

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

use super::external_pool_adapter_credential_verifier_key_service::{
    self as service, CredentialVerifierKeyServiceError, RegisterCredentialVerifierKeyBody,
    RevokeCredentialVerifierKeyBody,
};

pub(crate) fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/api/admin/compute/external-pool-adapter-credential-verifier-keys",
            post(register),
        )
        .route(
            "/api/admin/compute/external-pool-adapter-credential-verifier-keys/:key_record_id/revoke",
            post(revoke),
        )
        .route(
            "/api/admin/compute/external-pool-adapter-credential-verifier-keys/:key_record_id/currentness",
            get(currentness),
        )
}

async fn register(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    payload: Result<Json<RegisterCredentialVerifierKeyBody>, JsonRejection>,
) -> Response {
    let actor = match platform_admin(&state, &headers) {
        Ok(actor) => actor,
        Err(response) => return response,
    };
    let body = match json_body(payload) {
        Ok(body) => body,
        Err(response) => return response,
    };
    match service::register_for_admin(&state.store, &actor, body) {
        Ok(value) => (
            if value.replayed {
                StatusCode::OK
            } else {
                StatusCode::CREATED
            },
            Json(value),
        )
            .into_response(),
        Err(error) => error_response(error),
    }
}

async fn revoke(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    payload: Result<Json<RevokeCredentialVerifierKeyBody>, JsonRejection>,
) -> Response {
    let actor = match platform_admin(&state, &headers) {
        Ok(actor) => actor,
        Err(response) => return response,
    };
    let body = match json_body(payload) {
        Ok(body) => body,
        Err(response) => return response,
    };
    match service::revoke_for_admin(&state.store, &actor, &id, body) {
        Ok(value) => (
            if value.replayed {
                StatusCode::OK
            } else {
                StatusCode::CREATED
            },
            Json(value),
        )
            .into_response(),
        Err(error) => error_response(error),
    }
}

async fn currentness(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Response {
    if let Err(response) = platform_admin(&state, &headers) {
        return response;
    }
    match service::currentness_for_admin(&state.store, &id) {
        Ok(value) => Json(value).into_response(),
        Err(error) => error_response(error),
    }
}

fn json_body<T>(payload: Result<Json<T>, JsonRejection>) -> Result<T, Response> {
    payload
        .map(|Json(value)| value)
        .map_err(|error| json_error(StatusCode::UNPROCESSABLE_ENTITY, error))
}

fn error_response(error: CredentialVerifierKeyServiceError) -> Response {
    let status = match &error {
        CredentialVerifierKeyServiceError::NotFound => StatusCode::NOT_FOUND,
        CredentialVerifierKeyServiceError::Invalid(_) => StatusCode::BAD_REQUEST,
        CredentialVerifierKeyServiceError::Conflict(_) => StatusCode::CONFLICT,
    };
    json_error(status, error)
}

fn platform_admin(state: &AppState, headers: &HeaderMap) -> Result<String, Response> {
    let user = auth_from_headers(state, headers)
        .map_err(|error| json_error(StatusCode::UNAUTHORIZED, error))?;
    if !matches!(user.role.as_str(), "admin" | "owner") {
        return Err(json_error(
            StatusCode::FORBIDDEN,
            "只有平台管理员可以管理 external-pool Adapter credential verifier keys",
        ));
    }
    Ok(user.id)
}
