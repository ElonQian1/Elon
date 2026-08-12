//! Authenticated platform-administrator API for revocable Adapter adoption authority.

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

use super::external_pool_adapter_adoption_service::{
    self as service, AdapterAdoptionServiceError, AdoptExternalPoolAdapterBody,
    RevokeExternalPoolAdapterAdoptionBody,
};

pub(crate) fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/api/admin/compute/external-pool-adapter-adoptions",
            post(adopt),
        )
        .route(
            "/api/admin/compute/external-pool-adapter-adoptions/:receipt_id/revoke",
            post(revoke),
        )
        .route(
            "/api/admin/compute/external-pool-adapter-adoptions/:receipt_id/currentness",
            get(currentness),
        )
}

async fn adopt(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    payload: Result<Json<AdoptExternalPoolAdapterBody>, JsonRejection>,
) -> Response {
    let actor = match platform_admin(&state, &headers) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let body = match json_body(payload) {
        Ok(value) => value,
        Err(response) => return response,
    };
    write_response(service::adopt_for_admin(&state.store, &actor, body))
}

async fn revoke(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(receipt_id): Path<String>,
    payload: Result<Json<RevokeExternalPoolAdapterAdoptionBody>, JsonRejection>,
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
        &receipt_id,
        body,
    ))
}

async fn currentness(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(receipt_id): Path<String>,
) -> Response {
    if let Err(response) = platform_admin(&state, &headers) {
        return response;
    }
    match service::currentness_for_admin(&state.store, &receipt_id) {
        Ok(output) => Json(output).into_response(),
        Err(error) => error_response(error),
    }
}

fn json_body<T>(payload: Result<Json<T>, JsonRejection>) -> Result<T, Response> {
    payload
        .map(|Json(value)| value)
        .map_err(|error| json_error(StatusCode::UNPROCESSABLE_ENTITY, error))
}

fn write_response(
    result: Result<
        crate::store::ExternalPoolAdapterAdoptionWriteReceipt,
        AdapterAdoptionServiceError,
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

fn error_response(error: AdapterAdoptionServiceError) -> Response {
    let status = match error {
        AdapterAdoptionServiceError::NotFound => StatusCode::NOT_FOUND,
        AdapterAdoptionServiceError::Invalid(_) => StatusCode::BAD_REQUEST,
        AdapterAdoptionServiceError::Conflict(_) => StatusCode::CONFLICT,
    };
    json_error(status, error)
}

fn platform_admin(state: &AppState, headers: &HeaderMap) -> Result<String, Response> {
    let user = auth_from_headers(state, headers)
        .map_err(|error| json_error(StatusCode::UNAUTHORIZED, error))?;
    if !matches!(user.role.as_str(), "admin" | "owner") {
        return Err(json_error(
            StatusCode::FORBIDDEN,
            "only platform administrators can manage external-pool Adapter adoption",
        ));
    }
    Ok(user.id)
}
