//! Authenticated administrator API for V249 Provider-neutral registry bindings.

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

use super::external_pool_adapter_registry_service::{
    self as service, AdapterRegistryServiceError, RegisterExternalPoolAdapterRegistryBindingBody,
};

pub(crate) fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/api/admin/compute/external-pool-adapter-registry-bindings",
            post(register),
        )
        .route(
            "/api/admin/compute/external-pool-adapter-registry-bindings/:binding_id/currentness",
            get(currentness),
        )
}

async fn register(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    payload: Result<Json<RegisterExternalPoolAdapterRegistryBindingBody>, JsonRejection>,
) -> Response {
    let actor = match platform_admin(&state, &headers) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let body = match payload {
        Ok(Json(value)) => value,
        Err(error) => return json_error(StatusCode::UNPROCESSABLE_ENTITY, error),
    };
    match service::register_for_admin(&state, &actor, body).await {
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

async fn currentness(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(binding_id): Path<String>,
) -> Response {
    if let Err(response) = platform_admin(&state, &headers) {
        return response;
    }
    match service::currentness_for_admin(&state, &binding_id).await {
        Ok(output) => Json(output).into_response(),
        Err(error) => error_response(error),
    }
}

fn error_response(error: AdapterRegistryServiceError) -> Response {
    let status = match error {
        AdapterRegistryServiceError::NotFound => StatusCode::NOT_FOUND,
        AdapterRegistryServiceError::Invalid(_) => StatusCode::BAD_REQUEST,
        AdapterRegistryServiceError::Conflict(_) => StatusCode::CONFLICT,
        AdapterRegistryServiceError::Task(_) | AdapterRegistryServiceError::Storage(_) => {
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
            "only platform administrators can manage external-pool Adapter registry bindings",
        ));
    }
    Ok(user.id)
}
