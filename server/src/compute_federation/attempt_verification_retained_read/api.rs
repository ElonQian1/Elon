use std::sync::Arc;

use axum::{
    body::{to_bytes, Body},
    extract::{FromRequestParts, Path, Request, State},
    http::{request::Parts, HeaderMap, StatusCode, Uri},
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

use crate::{project_auth::auth_from_headers, types::AppState};

use super::service::{
    self, AttemptVerificationRetainedReadError, ADMIN_FORBIDDEN, INVALID_REQUEST_INPUT,
    UNAUTHENTICATED,
};

pub(crate) async fn get_for_participant(
    State(state): State<Arc<AppState>>,
    request: Request,
) -> Response {
    let (mut parts, body) = request.into_parts();
    let user_id = match authenticated_user(&state, &parts.headers) {
        Ok(user_id) => user_id,
        Err(response) => return response,
    };
    if let Err(response) = require_path_only_input(&parts.uri, body).await {
        return response;
    }
    let lease_id = match lease_id_from_parts(&mut parts, &state).await {
        Ok(lease_id) => lease_id,
        Err(response) => return response,
    };
    retained_response(service::read_for_participant(
        &state.store,
        &user_id,
        &lease_id,
        None,
    ))
}

pub(crate) async fn get_for_admin(
    State(state): State<Arc<AppState>>,
    request: Request,
) -> Response {
    let (mut parts, body) = request.into_parts();
    if let Err(response) = platform_admin(&state, &parts.headers) {
        return response;
    }
    if let Err(response) = require_path_only_input(&parts.uri, body).await {
        return response;
    }
    let lease_id = match lease_id_from_parts(&mut parts, &state).await {
        Ok(lease_id) => lease_id,
        Err(response) => return response,
    };
    retained_response(service::read_for_admin(&state.store, &lease_id))
}

fn authenticated_user(state: &AppState, headers: &HeaderMap) -> Result<String, Response> {
    auth_from_headers(state, headers)
        .map(|user| user.id)
        .map_err(|_| coded_error(StatusCode::UNAUTHORIZED, UNAUTHENTICATED))
}

fn platform_admin(state: &AppState, headers: &HeaderMap) -> Result<(), Response> {
    let user = auth_from_headers(state, headers)
        .map_err(|_| coded_error(StatusCode::UNAUTHORIZED, UNAUTHENTICATED))?;
    if !matches!(user.role.as_str(), "admin" | "owner") {
        return Err(coded_error(StatusCode::FORBIDDEN, ADMIN_FORBIDDEN));
    }
    Ok(())
}

async fn require_path_only_input(uri: &Uri, body: Body) -> Result<(), Response> {
    if uri.query().is_some() {
        return Err(coded_error(StatusCode::BAD_REQUEST, INVALID_REQUEST_INPUT));
    }
    match to_bytes(body, 1).await {
        Ok(bytes) if bytes.is_empty() => Ok(()),
        _ => Err(coded_error(StatusCode::BAD_REQUEST, INVALID_REQUEST_INPUT)),
    }
}

async fn lease_id_from_parts(parts: &mut Parts, state: &Arc<AppState>) -> Result<String, Response> {
    Path::<String>::from_request_parts(parts, state)
        .await
        .map(|Path(lease_id)| lease_id)
        .map_err(|_| coded_error(StatusCode::BAD_REQUEST, service::INVALID_LEASE_ID))
}

fn retained_response<T: serde::Serialize>(
    result: Result<T, AttemptVerificationRetainedReadError>,
) -> Response {
    match result {
        Ok(receipt) => Json(receipt).into_response(),
        Err(error) => {
            let status = match &error {
                AttemptVerificationRetainedReadError::InvalidLeaseId => StatusCode::BAD_REQUEST,
                AttemptVerificationRetainedReadError::NotVisible
                | AttemptVerificationRetainedReadError::NotFound => StatusCode::NOT_FOUND,
                AttemptVerificationRetainedReadError::ProjectForbidden => StatusCode::FORBIDDEN,
                AttemptVerificationRetainedReadError::IntegrityConflict => StatusCode::CONFLICT,
                AttemptVerificationRetainedReadError::Unavailable => {
                    StatusCode::INTERNAL_SERVER_ERROR
                }
            };
            coded_error(status, error.code())
        }
    }
}

fn coded_error(status: StatusCode, code: &'static str) -> Response {
    (status, Json(json!({"error":code}))).into_response()
}
