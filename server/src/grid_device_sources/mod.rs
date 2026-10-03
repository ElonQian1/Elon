//! Private cross-device read transport. Producer authority is always the authenticated owner.
mod delegated;
mod model;
mod storage;
#[cfg(test)]
mod tests;

use crate::{project_auth::auth_from_headers, types::AppState};
use axum::{
    body::Bytes,
    extract::{DefaultBodyLimit, RawQuery, State},
    http::{HeaderMap, StatusCode},
    middleware,
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use serde_json::json;
use std::sync::Arc;

pub(super) fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/me/grid-device-sources", get(read).post(publish))
        .route(
            "/api/asset-access/grid-device-sources",
            get(delegated::read),
        )
        .layer(DefaultBodyLimit::max(model::MAX_BYTES))
        .layer(middleware::from_fn(
            crate::esk_asset::platform::access::transport::require_secure_transport,
        ))
}
fn failure(status: StatusCode, code: &str) -> Response {
    (status, Json(json!({"code":code}))).into_response()
}
fn owner(state: &AppState, headers: &HeaderMap) -> Option<String> {
    if headers.get_all("authorization").iter().count() != 1 {
        return None;
    }
    auth_from_headers(state, headers)
        .ok()
        .filter(|u| u.id != "local-owner" && u.status == "active")
        .map(|u| u.id)
}
async fn read(
    State(state): State<Arc<AppState>>,
    RawQuery(query): RawQuery,
    headers: HeaderMap,
) -> Response {
    let Some(owner) = owner(&state, &headers) else {
        return failure(StatusCode::UNAUTHORIZED, "source_unauthorized");
    };
    if query.is_some() {
        return failure(StatusCode::BAD_REQUEST, "source_query_invalid");
    }
    let now = crate::private_read_projection::now_ms();
    match state
        .store
        .conn()
        .and_then(|conn| storage::read(&conn, &owner, now))
    {
        Ok(sources) => Json(json!({"schema":"yilong.grid_device_sources.v1","sources":sources}))
            .into_response(),
        Err(_) => failure(StatusCode::SERVICE_UNAVAILABLE, "source_store_unavailable"),
    }
}
async fn publish(
    State(state): State<Arc<AppState>>,
    RawQuery(query): RawQuery,
    headers: HeaderMap,
    bytes: Bytes,
) -> Response {
    let Some(owner) = owner(&state, &headers) else {
        return failure(StatusCode::UNAUTHORIZED, "source_unauthorized");
    };
    if query.is_some() {
        return failure(StatusCode::BAD_REQUEST, "source_query_invalid");
    }
    let now = crate::private_read_projection::now_ms();
    let value = match model::parse(&bytes, now) {
        Ok(value) => value,
        Err(code) => return failure(StatusCode::BAD_REQUEST, code),
    };
    match state.store.conn().and_then(|mut conn| storage::put(&mut conn, &owner, &value, now)) {
        Ok(status) => Json(json!({"schema":"yilong.grid_device_sources.ack.v1","sequence":value.sequence,"status":status})).into_response(),
        Err(error) => match error.to_string().as_str() {
            "source_out_of_order" | "source_revision_conflict" | "source_capacity" =>
                failure(StatusCode::CONFLICT, "source_update_conflict"),
            _ => failure(StatusCode::SERVICE_UNAVAILABLE, "source_store_unavailable"),
        }
    }
}
