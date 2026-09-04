use axum::{
    body::Bytes,
    extract::{rejection::BytesRejection, OriginalUri, State},
    http::HeaderMap,
    response::{IntoResponse, Response},
    Json,
};
use std::sync::Arc;

use crate::types::AppState;

use super::{api, PlatformError};

pub(super) async fn get_snapshot(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    OriginalUri(uri): OriginalUri,
    body: Result<Bytes, BytesRejection>,
) -> Response {
    let (actor, token) = match api::administrator(&state, &headers) {
        Ok(value) => value,
        Err(response) => return response,
    };
    if uri.query().is_some() || !matches!(body, Ok(ref bytes) if bytes.is_empty()) {
        return api::domain_error(PlatformError::InvalidInput.into());
    }
    // Deliberately read the stored policy, not the write-enabling environment.
    // The Store reauthenticates the administrator inside the read transaction.
    match state
        .store
        .esk_platform_reconciliation_snapshot(&actor.id, token)
    {
        Ok(snapshot) => Json(snapshot).into_response(),
        Err(error) => api::domain_error(error),
    }
}
