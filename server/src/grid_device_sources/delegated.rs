//! The existing grant explicitly covers Win snapshots, not remote Android data.
use super::{failure, storage};
use crate::{
    esk_asset::platform::access::{self, AccessError},
    types::AppState,
};
use axum::{
    extract::{RawQuery, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use std::sync::Arc;

pub(super) async fn read(
    State(state): State<Arc<AppState>>,
    RawQuery(query): RawQuery,
    headers: HeaderMap,
) -> Response {
    if query.is_some() {
        return failure(StatusCode::BAD_REQUEST, "source_query_invalid");
    }
    let token = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .filter(|v| access::valid_secret(v, "aat_"));
    let client = headers
        .get(access::CLIENT_HEADER)
        .and_then(|v| v.to_str().ok());
    if headers.get_all("authorization").iter().count() != 1
        || headers.get_all(access::CLIENT_HEADER).iter().count() != 1
        || token.is_none()
        || client != Some("quant.android")
    {
        return failure(StatusCode::UNAUTHORIZED, "source_unauthorized");
    }
    let now = crate::private_read_projection::now_ms();
    let result = state.store.asset_access_private_read(token.unwrap(), "quant.android", "grid.snapshot.read", |tx, grant| {
        let sources = storage::read(tx, grant.user_id(), now)?.into_iter()
            .filter(|source| source.snapshot.platform == "windows").collect::<Vec<_>>();
        Ok(json!({"schema":"yilong.grid_device_sources.authorization.v1","audience":"yilong-quant",
            "subject":grant.subject(),"client_id":grant.client_id(),"grant_id":grant.grant_id(),
            "expires_at":grant.expires_at(),"sources":sources}))
    });
    match result {
        Ok(value) => Json(value).into_response(),
        Err(cause) => match cause.downcast_ref::<AccessError>() {
            Some(AccessError::InsufficientScope) => {
                failure(StatusCode::FORBIDDEN, "source_scope_required")
            }
            Some(AccessError::Unauthorized) => {
                failure(StatusCode::UNAUTHORIZED, "source_unauthorized")
            }
            _ => failure(StatusCode::SERVICE_UNAVAILABLE, "source_store_unavailable"),
        },
    }
}
