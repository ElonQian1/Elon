use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use serde::Deserialize;
use std::sync::Arc;

use crate::{
    open_commerce_consumer_receipt_service,
    project_auth::{auth_from_headers, json_error},
    types::AppState,
};

#[derive(Debug, Deserialize)]
struct ReceiptQuery {
    #[serde(default = "default_limit")]
    limit: usize,
}

pub(crate) fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/api/open-commerce/consumer-invocation-receipts",
            get(list_receipts),
        )
        .route(
            "/api/open-commerce/consumer-invocation-receipts/:invocation_id",
            get(get_receipt),
        )
}

async fn list_receipts(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(query): Query<ReceiptQuery>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(user) => user,
        Err(error) => return json_error(StatusCode::UNAUTHORIZED, error),
    };
    service_response(open_commerce_consumer_receipt_service::list_receipts(
        &state.store,
        &user.id,
        query.limit,
    ))
}

async fn get_receipt(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(invocation_id): Path<String>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(user) => user,
        Err(error) => return json_error(StatusCode::UNAUTHORIZED, error),
    };
    service_response(open_commerce_consumer_receipt_service::get_receipt(
        &state.store,
        &user.id,
        &invocation_id,
    ))
}

fn service_response<T: serde::Serialize>(result: anyhow::Result<T>) -> Response {
    match result {
        Ok(value) => Json(value).into_response(),
        Err(error) => {
            let message = error.to_string();
            let status = if message.contains("不存在") {
                StatusCode::NOT_FOUND
            } else {
                StatusCode::BAD_REQUEST
            };
            json_error(status, message)
        }
    }
}

fn default_limit() -> usize {
    50
}
