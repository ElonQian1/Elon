//! Authenticated administrator API for inert external-pool Adapter installation.

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

use super::external_pool_adapter_installation_service::{
    self as service, AdapterInstallationServiceError, InstallExternalPoolAdapterBody,
    RevokeExternalPoolAdapterInstallationBody,
};

pub(crate) fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/api/admin/compute/external-pool-adapter-installations",
            post(install),
        )
        .route(
            "/api/admin/compute/external-pool-adapter-installations/:installation_id/revoke",
            post(revoke),
        )
        .route(
            "/api/admin/compute/external-pool-adapter-installations/:installation_receipt_id/currentness",
            get(currentness),
        )
}

async fn install(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    payload: Result<Json<InstallExternalPoolAdapterBody>, JsonRejection>,
) -> Response {
    let actor = match platform_admin(&state, &headers) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let body = match payload {
        Ok(Json(value)) => value,
        Err(error) => return json_error(StatusCode::UNPROCESSABLE_ENTITY, error),
    };
    installation_write_response(service::install_for_admin(&state, &actor, body).await)
}

async fn revoke(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(installation_id): Path<String>,
    payload: Result<Json<RevokeExternalPoolAdapterInstallationBody>, JsonRejection>,
) -> Response {
    let actor = match platform_admin(&state, &headers) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let body = match payload {
        Ok(Json(value)) => value,
        Err(error) => return json_error(StatusCode::UNPROCESSABLE_ENTITY, error),
    };
    terminal_write_response(service::revoke_for_admin(
        &state,
        &actor,
        &installation_id,
        body,
    ))
}

async fn currentness(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(installation_receipt_id): Path<String>,
) -> Response {
    if let Err(response) = platform_admin(&state, &headers) {
        return response;
    }
    match service::currentness_for_admin(&state, &installation_receipt_id).await {
        Ok(output) => Json(output).into_response(),
        Err(error) => error_response(error),
    }
}

fn installation_write_response(
    result: Result<
        crate::store::ExternalPoolAdapterInstallationWriteReceipt,
        AdapterInstallationServiceError,
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
        Err(error) => error_response(error),
    }
}

fn terminal_write_response(
    result: Result<
        crate::store::ExternalPoolAdapterInstallationTerminalWriteReceipt,
        AdapterInstallationServiceError,
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
        Err(error) => error_response(error),
    }
}

fn error_response(error: AdapterInstallationServiceError) -> Response {
    let status = match error {
        AdapterInstallationServiceError::NotFound => StatusCode::NOT_FOUND,
        AdapterInstallationServiceError::Invalid(_) => StatusCode::BAD_REQUEST,
        AdapterInstallationServiceError::Conflict(_) => StatusCode::CONFLICT,
        AdapterInstallationServiceError::Task(_) | AdapterInstallationServiceError::Storage(_) => {
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
            "only platform administrators can manage external-pool Adapter installations",
        ));
    }
    Ok(user.id)
}
