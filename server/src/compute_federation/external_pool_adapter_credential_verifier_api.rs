//! Authenticated platform-administrator API for credential-verifier identities.

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

use super::external_pool_adapter_credential_verifier_service::{
    self as service, ActivateCredentialVerifierBody, CredentialVerifierServiceError,
    RegisterCredentialVerifierBody, RevokeCredentialVerifierBody,
};

pub(crate) fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/admin/compute/external-pool-adapter-credential-verifiers", post(register))
        .route("/api/admin/compute/external-pool-adapter-credential-verifiers/:verifier_record_id/activate", post(activate))
        .route("/api/admin/compute/external-pool-adapter-credential-verifiers/:verifier_record_id/revoke", post(revoke))
        .route("/api/admin/compute/external-pool-adapter-credential-verifiers/:verifier_record_id/currentness", get(currentness))
}

async fn register(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    payload: Result<Json<RegisterCredentialVerifierBody>, JsonRejection>,
) -> Response {
    let actor = match platform_admin(&state, &headers) {
        Ok(x) => x,
        Err(x) => return x,
    };
    let body = match json_body(payload) {
        Ok(x) => x,
        Err(x) => return x,
    };
    registration_response(service::register_for_admin(&state.store, &actor, body))
}

async fn activate(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    payload: Result<Json<ActivateCredentialVerifierBody>, JsonRejection>,
) -> Response {
    let actor = match platform_admin(&state, &headers) {
        Ok(x) => x,
        Err(x) => return x,
    };
    let body = match json_body(payload) {
        Ok(x) => x,
        Err(x) => return x,
    };
    transition_response(service::activate_for_admin(&state.store, &actor, &id, body))
}

async fn revoke(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    payload: Result<Json<RevokeCredentialVerifierBody>, JsonRejection>,
) -> Response {
    let actor = match platform_admin(&state, &headers) {
        Ok(x) => x,
        Err(x) => return x,
    };
    let body = match json_body(payload) {
        Ok(x) => x,
        Err(x) => return x,
    };
    transition_response(service::revoke_for_admin(&state.store, &actor, &id, body))
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

fn registration_response(
    result: Result<
        crate::store::ExternalPoolAdapterCredentialVerifierRegistrationWriteReceipt,
        CredentialVerifierServiceError,
    >,
) -> Response {
    match result {
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

fn transition_response(
    result: Result<
        crate::store::ExternalPoolAdapterCredentialVerifierTransitionWriteReceipt,
        CredentialVerifierServiceError,
    >,
) -> Response {
    match result {
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

fn json_body<T>(payload: Result<Json<T>, JsonRejection>) -> Result<T, Response> {
    payload
        .map(|Json(x)| x)
        .map_err(|error| json_error(StatusCode::UNPROCESSABLE_ENTITY, error))
}

fn error_response(error: CredentialVerifierServiceError) -> Response {
    let status = match &error {
        CredentialVerifierServiceError::NotFound => StatusCode::NOT_FOUND,
        CredentialVerifierServiceError::Invalid(_) => StatusCode::BAD_REQUEST,
        CredentialVerifierServiceError::Conflict(_) => StatusCode::CONFLICT,
    };
    json_error(status, error)
}

fn platform_admin(state: &AppState, headers: &HeaderMap) -> Result<String, Response> {
    let user = auth_from_headers(state, headers)
        .map_err(|error| json_error(StatusCode::UNAUTHORIZED, error))?;
    if !matches!(user.role.as_str(), "admin" | "owner") {
        return Err(json_error(
            StatusCode::FORBIDDEN,
            "只有平台管理员可以管理 external-pool Adapter credential verifiers",
        ));
    }
    Ok(user.id)
}
