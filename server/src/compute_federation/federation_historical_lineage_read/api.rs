use std::sync::Arc;

use axum::{
    body::{to_bytes, Body},
    extract::{FromRequestParts, Path, Request, State},
    http::{request::Parts, HeaderMap, StatusCode, Uri},
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use serde_json::json;

use crate::{project_auth::auth_from_headers, types::AppState};

use super::service::{
    self, FederationHistoricalLineageReadError, ADMIN_FORBIDDEN, INVALID_REQUEST_INPUT,
    UNAUTHENTICATED,
};

pub(crate) fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/api/me/compute/attempt-leases/:lease_id/execution-source-lineage",
            get(get_my_execution_source_lineage),
        )
        .route(
            "/api/me/compute/attempt-leases/:lease_id/execution-verification-source-lineage",
            get(get_my_execution_verification_source_lineage),
        )
        .route(
            "/api/me/compute/attempt-leases/:lease_id/settlement-source-lineage",
            get(get_my_settlement_source_lineage),
        )
        .route(
            "/api/me/compute/attempt-leases/:lease_id/settlement-release-source-lineage",
            get(get_my_settlement_release_source_lineage),
        )
        .route(
            "/api/admin/compute/attempt-leases/:lease_id/execution-source-lineage",
            get(get_admin_execution_source_lineage),
        )
        .route(
            "/api/admin/compute/attempt-leases/:lease_id/execution-verification-source-lineage",
            get(get_admin_execution_verification_source_lineage),
        )
        .route(
            "/api/admin/compute/attempt-leases/:lease_id/settlement-source-lineage",
            get(get_admin_settlement_source_lineage),
        )
        .route(
            "/api/admin/compute/attempt-leases/:lease_id/settlement-release-source-lineage",
            get(get_admin_settlement_release_source_lineage),
        )
}

async fn get_my_execution_source_lineage(
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
    lineage_response(service::read_execution_for_participant(
        &state.store,
        &user_id,
        &lease_id,
        None,
    ))
}

async fn get_my_execution_verification_source_lineage(
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
    lineage_response(service::read_execution_verification_for_participant(
        &state.store,
        &user_id,
        &lease_id,
        None,
    ))
}

async fn get_my_settlement_source_lineage(
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
    lineage_response(service::read_settlement_for_participant(
        &state.store,
        &user_id,
        &lease_id,
        None,
    ))
}

async fn get_my_settlement_release_source_lineage(
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
    lineage_response(service::read_settlement_release_for_participant(
        &state.store,
        &user_id,
        &lease_id,
        None,
    ))
}

async fn get_admin_execution_source_lineage(
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
    lineage_response(service::read_execution_for_admin(&state.store, &lease_id))
}

async fn get_admin_execution_verification_source_lineage(
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
    lineage_response(service::read_execution_verification_for_admin(
        &state.store,
        &lease_id,
    ))
}

async fn get_admin_settlement_source_lineage(
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
    lineage_response(service::read_settlement_for_admin(&state.store, &lease_id))
}

async fn get_admin_settlement_release_source_lineage(
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
    lineage_response(service::read_settlement_release_for_admin(
        &state.store,
        &lease_id,
    ))
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

fn lineage_response<T: serde::Serialize>(
    result: Result<T, FederationHistoricalLineageReadError>,
) -> Response {
    match result {
        Ok(document) => Json(document).into_response(),
        Err(error) => {
            let status = match &error {
                FederationHistoricalLineageReadError::InvalidLeaseId => StatusCode::BAD_REQUEST,
                FederationHistoricalLineageReadError::NotVisible
                | FederationHistoricalLineageReadError::NotFound => StatusCode::NOT_FOUND,
                FederationHistoricalLineageReadError::ProjectForbidden => StatusCode::FORBIDDEN,
                FederationHistoricalLineageReadError::IntegrityConflict => StatusCode::CONFLICT,
            };
            coded_error(status, error.code())
        }
    }
}

fn coded_error(status: StatusCode, code: &'static str) -> Response {
    (status, Json(json!({"error":code}))).into_response()
}
