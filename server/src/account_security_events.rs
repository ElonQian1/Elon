use axum::{
    extract::{Query, State},
    http::HeaderMap,
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use serde::Deserialize;
use std::sync::Arc;

use crate::{account_security::authenticated_account, types::AppState};

#[derive(Debug, Deserialize)]
struct EventQuery {
    limit: Option<u32>,
    before: Option<String>,
}

pub(crate) fn routes() -> Router<Arc<AppState>> {
    Router::new().route("/api/auth/security/events", get(list_events))
}

async fn list_events(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(query): Query<EventQuery>,
) -> Response {
    let (user_id, _) = match authenticated_account(&state, &headers) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let limit = query.limit.unwrap_or(50).clamp(1, 100);
    match state.store.list_account_security_events(
        &user_id,
        query
            .before
            .as_deref()
            .map(str::trim)
            .filter(|v| !v.is_empty()),
        limit,
    ) {
        Ok(events) => {
            let next_before = events.last().map(|event| event.created_at.clone());
            Json(serde_json::json!({
                "schema": "elon.account_security_events.v1",
                "events": events,
                "next_before": next_before,
                "privacy": {
                    "request_ids_exposed": false,
                    "tokens_exposed": false,
                    "provider_credentials_exposed": false
                }
            }))
            .into_response()
        }
        Err(error) => {
            tracing::warn!(error = %error, "读取账号安全事件失败");
            crate::account_security::coded_error(
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "account_security_events_unavailable",
                "账号安全事件暂时不可用",
            )
        }
    }
}
