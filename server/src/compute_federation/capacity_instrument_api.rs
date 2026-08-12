//! Authenticated platform-administrator HTTP API for standardized capacity instruments.

#[cfg(test)]
#[path = "capacity_instrument_api_tests.rs"]
mod tests;

use std::sync::Arc;

use axum::{
    extract::{rejection::JsonRejection, Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use serde_json::json;

use crate::{
    project_auth::{auth_from_headers, json_error},
    types::AppState,
};

use super::capacity_instrument_service::{
    self as service, ActivateComputeCapacityInstrumentBody,
    AdoptComputeCapacityInstrumentOfferBody, ComputeCapacityInstrumentServiceError,
    RegisterComputeCapacityInstrumentBody, RetireComputeCapacityInstrumentBody,
};

pub(crate) fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/api/admin/compute/capacity-instruments",
            get(list).post(register),
        )
        .route(
            "/api/admin/compute/capacity-instruments/:instrument_id",
            get(get_instrument),
        )
        .route(
            "/api/admin/compute/capacity-instruments/:instrument_id/activate",
            post(activate),
        )
        .route(
            "/api/admin/compute/capacity-instruments/:instrument_id/retire",
            post(retire),
        )
        .route(
            "/api/admin/compute/capacity-instruments/:instrument_id/currentness",
            get(currentness),
        )
        .route(
            "/api/admin/compute/offers/:offer_id/capacity-instrument-adoption",
            get(get_offer_adoption).post(adopt_offer),
        )
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ListQuery {
    #[serde(default = "default_limit")]
    limit: usize,
}

async fn register(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    payload: Result<Json<RegisterComputeCapacityInstrumentBody>, JsonRejection>,
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
    Path(instrument_id): Path<String>,
    payload: Result<Json<ActivateComputeCapacityInstrumentBody>, JsonRejection>,
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
        &instrument_id,
        body,
    ))
}

async fn retire(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(instrument_id): Path<String>,
    payload: Result<Json<RetireComputeCapacityInstrumentBody>, JsonRejection>,
) -> Response {
    let admin_user_id = match platform_admin(&state, &headers) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let body = match json_body(payload) {
        Ok(value) => value,
        Err(response) => return response,
    };
    write_response(service::retire_for_admin(
        &state.store,
        &admin_user_id,
        &instrument_id,
        body,
    ))
}

async fn adopt_offer(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(offer_id): Path<String>,
    payload: Result<Json<AdoptComputeCapacityInstrumentOfferBody>, JsonRejection>,
) -> Response {
    let admin_user_id = match platform_admin(&state, &headers) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let body = match json_body(payload) {
        Ok(value) => value,
        Err(response) => return response,
    };
    write_response(service::adopt_offer_for_admin(
        &state.store,
        &admin_user_id,
        &offer_id,
        body,
    ))
}

async fn get_instrument(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(instrument_id): Path<String>,
) -> Response {
    if let Err(response) = platform_admin(&state, &headers) {
        return response;
    }
    read_response(service::get_for_admin(&state.store, &instrument_id))
}

async fn list(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(query): Query<ListQuery>,
) -> Response {
    if let Err(response) = platform_admin(&state, &headers) {
        return response;
    }
    read_response(
        service::list_for_admin(&state.store, query.limit)
            .map(|items| json!({"capacity_instruments": items})),
    )
}

async fn currentness(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(instrument_id): Path<String>,
) -> Response {
    if let Err(response) = platform_admin(&state, &headers) {
        return response;
    }
    read_response(service::currentness_for_admin(&state.store, &instrument_id))
}

async fn get_offer_adoption(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(offer_id): Path<String>,
) -> Response {
    if let Err(response) = platform_admin(&state, &headers) {
        return response;
    }
    read_response(service::offer_adoption_for_admin(&state.store, &offer_id))
}

trait ReplayStatus {
    fn replayed(&self) -> bool;
}

impl ReplayStatus for crate::store::ComputeCapacityInstrumentRegistrationWriteReceipt {
    fn replayed(&self) -> bool {
        self.replayed
    }
}

impl ReplayStatus for crate::store::ComputeCapacityInstrumentActivationWriteReceipt {
    fn replayed(&self) -> bool {
        self.replayed
    }
}

impl ReplayStatus for crate::store::ComputeCapacityInstrumentRetirementWriteReceipt {
    fn replayed(&self) -> bool {
        self.replayed
    }
}

impl ReplayStatus for crate::store::ComputeCapacityInstrumentOfferAdoptionWriteReceipt {
    fn replayed(&self) -> bool {
        self.replayed
    }
}

fn write_response<T>(result: Result<T, ComputeCapacityInstrumentServiceError>) -> Response
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
        Err(error) => business_error(error),
    }
}

fn read_response<T: serde::Serialize>(
    result: Result<T, ComputeCapacityInstrumentServiceError>,
) -> Response {
    match result {
        Ok(value) => Json(value).into_response(),
        Err(error) => business_error(error),
    }
}

fn json_body<T>(payload: Result<Json<T>, JsonRejection>) -> Result<T, Response> {
    payload
        .map(|Json(value)| value)
        .map_err(|error| json_error(StatusCode::UNPROCESSABLE_ENTITY, error))
}

fn business_error(error: ComputeCapacityInstrumentServiceError) -> Response {
    let status = match &error {
        ComputeCapacityInstrumentServiceError::NotFound => StatusCode::NOT_FOUND,
        ComputeCapacityInstrumentServiceError::Invalid(_) => StatusCode::UNPROCESSABLE_ENTITY,
        ComputeCapacityInstrumentServiceError::Conflict(_) => StatusCode::CONFLICT,
    };
    json_error(status, error)
}

fn platform_admin(state: &AppState, headers: &HeaderMap) -> Result<String, Response> {
    let user = auth_from_headers(state, headers)
        .map_err(|error| json_error(StatusCode::UNAUTHORIZED, error))?;
    if !matches!(user.role.as_str(), "admin" | "owner") {
        return Err(json_error(
            StatusCode::FORBIDDEN,
            "只有平台管理员可以管理标准化容量工具",
        ));
    }
    Ok(user.id)
}

fn default_limit() -> usize {
    20
}
