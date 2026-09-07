//! This private node upload route is mounted only with verified TLS authority.
use crate::{private_read_projection as contract, types::AppState};
use axum::{
    body::Bytes,
    extract::{DefaultBodyLimit, RawQuery, State},
    http::{HeaderMap, StatusCode},
    middleware,
    response::{IntoResponse, Response},
    routing::post,
    Json, Router,
};
use serde_json::json;
use std::sync::Arc;

pub(crate) fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/node/private-read-projections", post(upload))
        .layer(DefaultBodyLimit::max(contract::MAX_BYTES))
        .layer(middleware::from_fn(
            crate::esk_asset::platform::access::transport::require_secure_transport,
        ))
}
async fn upload(
    State(state): State<Arc<AppState>>,
    RawQuery(query): RawQuery,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    if query.is_some() {
        return failure(StatusCode::BAD_REQUEST, "projection_invalid_query");
    }
    let at = contract::now_ms();
    let one = |key: &str| headers.get_all(key).iter().count() == 1;
    if !one("authorization") || !one("x-elon-node-id") {
        return failure(StatusCode::UNAUTHORIZED, "projection_node_unauthorized");
    }
    let Some(token) = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .filter(|v| !v.is_empty() && v.len() <= 512)
    else {
        return failure(StatusCode::UNAUTHORIZED, "projection_node_unauthorized");
    };
    let Some(node) = headers
        .get("x-elon-node-id")
        .and_then(|v| v.to_str().ok())
        .filter(|v| contract::identifier(v))
    else {
        return failure(StatusCode::UNAUTHORIZED, "projection_node_unauthorized");
    };
    let value = match contract::parse(&body, at) {
        Ok(v) => v,
        Err(code) => return failure(StatusCode::BAD_REQUEST, code),
    };
    match state.store.put_private_read_projection(node,token,&value,at) {
        Ok(changed)=>(StatusCode::OK,Json(json!({"schema":"yilong.private_read_projection.ack.v1",
          "revision":value.revision,"connection_id":value.connection_id,"status":if changed{"accepted"}else{"unchanged"}}))).into_response(),
        Err(cause)=>match cause.to_string().as_str() {
          "projection_out_of_order"=>failure(StatusCode::CONFLICT,"projection_out_of_order"),
          "projection_freshness_conflict"=>failure(StatusCode::CONFLICT,"projection_freshness_conflict"),
          "projection_revision_conflict"=>failure(StatusCode::CONFLICT,"projection_revision_conflict"),
          "projection_source_limit"|"projection_total_limit"=>failure(StatusCode::CONFLICT,"projection_capacity"),
          "projection_node_unauthorized"=>failure(StatusCode::UNAUTHORIZED,"projection_node_unauthorized"),
          _=>failure(StatusCode::SERVICE_UNAVAILABLE,"projection_store_unavailable")
        }
    }
}
fn failure(status: StatusCode, code: &'static str) -> Response {
    (status, Json(json!({"code":code}))).into_response()
}
