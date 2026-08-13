use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use std::sync::Arc;

use crate::{
    project_auth::{auth_from_headers, json_error},
    types::AppState,
};

use super::service;

pub(crate) fn routes() -> Router<Arc<AppState>> {
    Router::new().route(
        "/api/open-commerce/consumer-order-closures/:invocation_id",
        get(get_order_closure),
    )
}

async fn get_order_closure(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(invocation_id): Path<String>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(user) => user,
        Err(error) => return json_error(StatusCode::UNAUTHORIZED, error),
    };
    match service::get_order_closure(&state.store, &user.id, &invocation_id) {
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
