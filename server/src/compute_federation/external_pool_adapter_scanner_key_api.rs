//! Authenticated platform-administrator API for independent scanner trust roots.

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

use super::external_pool_adapter_scanner_key_service::{
    self as service, ActivateScannerKeyBody, RegisterScannerKeyBody, RevokeScannerKeyBody,
    ScannerKeyServiceError,
};

pub(crate) fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/api/admin/compute/external-pool-adapter-scanner-keys",
            post(register),
        )
        .route(
            "/api/admin/compute/external-pool-adapter-scanner-keys/:key_record_id/activate",
            post(activate),
        )
        .route(
            "/api/admin/compute/external-pool-adapter-scanner-keys/:key_record_id/revoke",
            post(revoke),
        )
        .route(
            "/api/admin/compute/external-pool-adapter-scanner-keys/:key_record_id/currentness",
            get(currentness),
        )
}

async fn register(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    payload: Result<Json<RegisterScannerKeyBody>, JsonRejection>,
) -> Response {
    let actor = match platform_admin(&state, &headers) {
        Ok(x) => x,
        Err(x) => return x,
    };
    let body = match json_body(payload) {
        Ok(x) => x,
        Err(x) => return x,
    };
    write_response(service::register_for_admin(&state.store, &actor, body))
}

async fn activate(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    payload: Result<Json<ActivateScannerKeyBody>, JsonRejection>,
) -> Response {
    let actor = match platform_admin(&state, &headers) {
        Ok(x) => x,
        Err(x) => return x,
    };
    let body = match json_body(payload) {
        Ok(x) => x,
        Err(x) => return x,
    };
    write_response(service::activate_for_admin(&state.store, &actor, &id, body))
}

async fn revoke(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    payload: Result<Json<RevokeScannerKeyBody>, JsonRejection>,
) -> Response {
    let actor = match platform_admin(&state, &headers) {
        Ok(x) => x,
        Err(x) => return x,
    };
    let body = match json_body(payload) {
        Ok(x) => x,
        Err(x) => return x,
    };
    write_response(service::revoke_for_admin(&state.store, &actor, &id, body))
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

trait ReplayStatus {
    fn replayed(&self) -> bool;
}
impl ReplayStatus for crate::store::ExternalPoolAdapterScannerKeyRegistrationWriteReceipt {
    fn replayed(&self) -> bool {
        self.replayed
    }
}
impl ReplayStatus for crate::store::ExternalPoolAdapterScannerKeyActivationWriteReceipt {
    fn replayed(&self) -> bool {
        self.replayed
    }
}
impl ReplayStatus for crate::store::ExternalPoolAdapterScannerKeyRevocationWriteReceipt {
    fn replayed(&self) -> bool {
        self.replayed
    }
}

fn write_response<T: serde::Serialize + ReplayStatus>(
    result: Result<T, ScannerKeyServiceError>,
) -> Response {
    match result {
        Ok(value) => (
            if value.replayed() {
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
        .map_err(|error| json_error(StatusCode::BAD_REQUEST, error))
}

fn error_response(error: ScannerKeyServiceError) -> Response {
    let status = match &error {
        ScannerKeyServiceError::NotFound => StatusCode::NOT_FOUND,
        ScannerKeyServiceError::Invalid(_) => StatusCode::BAD_REQUEST,
        ScannerKeyServiceError::Conflict(_) => StatusCode::CONFLICT,
    };
    json_error(status, error)
}

fn platform_admin(state: &AppState, headers: &HeaderMap) -> Result<String, Response> {
    let user = auth_from_headers(state, headers)
        .map_err(|error| json_error(StatusCode::UNAUTHORIZED, error))?;
    if !matches!(user.role.as_str(), "admin" | "owner") {
        return Err(json_error(
            StatusCode::FORBIDDEN,
            "只有平台管理员可以管理 external-pool Adapter scanner keys",
        ));
    }
    Ok(user.id)
}
