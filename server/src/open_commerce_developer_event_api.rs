//! Current test- or production-credential terminal event feed for developer Apps.

use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use std::sync::Arc;

use crate::{
    open_commerce_client_api::bearer_token,
    open_commerce_developer_event_model::DeveloperTerminalEventQuery,
    open_commerce_developer_event_service, project_auth::json_error, types::AppState,
};

pub(crate) fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/api/open-commerce/developer/events",
            get(list_terminal_events),
        )
        .route(
            "/api/open-commerce/developer/events/:invocation_id",
            get(terminal_event_detail),
        )
}

async fn list_terminal_events(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(query): Query<DeveloperTerminalEventQuery>,
) -> Response {
    let credential = match authenticated_credential(&state, &headers) {
        Ok(credential) => credential,
        Err(response) => return response,
    };
    service_response(open_commerce_developer_event_service::list_terminal_events(
        &state.store,
        &credential.app,
        query,
    ))
}

async fn terminal_event_detail(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(invocation_id): Path<String>,
) -> Response {
    let credential = match authenticated_credential(&state, &headers) {
        Ok(credential) => credential,
        Err(response) => return response,
    };
    service_response(
        open_commerce_developer_event_service::terminal_event_detail(
            &state.store,
            &credential.app,
            &invocation_id,
        ),
    )
}

fn authenticated_credential(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<
    crate::open_commerce_developer_credential_model::AuthenticatedDeveloperCredential,
    Response,
> {
    let token = bearer_token(headers)?;
    state
        .store
        .authenticate_open_commerce_developer_credential(&token)
        .map_err(|error| json_error(StatusCode::UNAUTHORIZED, error))
}

fn service_response<T: serde::Serialize>(result: anyhow::Result<T>) -> Response {
    match result {
        Ok(value) => Json(value).into_response(),
        Err(error) => {
            let message = format!("{error:#}");
            let status = if message.contains("不存在") {
                StatusCode::NOT_FOUND
            } else {
                StatusCode::BAD_REQUEST
            };
            json_error(status, message)
        }
    }
}
