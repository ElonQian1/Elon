use crate::{
    project_auth::{auth_from_headers, json_error},
    store::articles::{snapshots::SnapshotDocument, ArticleFault},
    types::AppState,
};
use axum::{
    extract::{rejection::JsonRejection, DefaultBodyLimit, Path, Request, State},
    http::{HeaderMap, HeaderValue, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub(super) fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/api/me/groups/:group_id/ai-snapshots",
            post(create).layer(DefaultBodyLimit::max(2 * 1024 * 1024 + 4096)),
        )
        .route(
            "/api/me/groups/:group_id/ai-snapshots/assets",
            post(upload).layer(DefaultBodyLimit::max(16 * 1024 * 1024 + 4096)),
        )
        .route(
            "/api/me/groups/:group_id/ai-snapshots/:snapshot_id",
            get(read).delete(revoke),
        )
        .route(
            "/api/me/groups/:group_id/ai-snapshots/:snapshot_id/assets/:asset_id",
            get(asset),
        )
        .route_layer(middleware::from_fn(private_response))
}

async fn private_response(request: Request, next: Next) -> Response {
    let mut response = next.run(request).await;
    response.headers_mut().insert(
        "cache-control",
        HeaderValue::from_static("private, no-store"),
    );
    response.headers_mut().insert(
        "x-content-type-options",
        HeaderValue::from_static("nosniff"),
    );
    response
        .headers_mut()
        .insert("referrer-policy", HeaderValue::from_static("no-referrer"));
    response
}

fn result<T: Serialize>(value: anyhow::Result<T>) -> Response {
    match value {
        Ok(v) => Json(v).into_response(),
        Err(e) => error(e),
    }
}
fn error(error: anyhow::Error) -> Response {
    if let Some(fault) = error.downcast_ref::<ArticleFault>() {
        return json_error(
            StatusCode::from_u16(fault.0).unwrap_or(StatusCode::BAD_REQUEST),
            &fault.1,
        );
    }
    // Database and decoder errors must not echo user content or transport bytes.
    tracing::warn!("AI snapshot operation failed");
    json_error(
        StatusCode::INTERNAL_SERVER_ERROR,
        "Snapshot operation failed",
    )
}
fn invalid_json(error: JsonRejection) -> Response {
    let status = if error.status() == StatusCode::PAYLOAD_TOO_LARGE {
        StatusCode::PAYLOAD_TOO_LARGE
    } else {
        StatusCode::BAD_REQUEST
    };
    json_error(
        status,
        "Invalid snapshot JSON, unsupported fields or request too large",
    )
}
macro_rules! user {
    ($state:expr, $headers:expr) => {
        match auth_from_headers($state, $headers) {
            Ok(user) => user,
            Err(_) => return json_error(StatusCode::UNAUTHORIZED, "Authentication required"),
        }
    };
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Create {
    idempotency_key: String,
    document: SnapshotDocument,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Upload {
    base64: String,
}

async fn create(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(group): Path<String>,
    body: Result<Json<Create>, JsonRejection>,
) -> Response {
    let user = user!(&state, &headers);
    let Json(body) = match body {
        Ok(v) => v,
        Err(e) => return invalid_json(e),
    };
    let created =
        state
            .store
            .create_ai_snapshot(&user.id, &group, &body.idempotency_key, body.document);
    if let Ok(value) = &created {
        if !value.replayed {
            if let Ok(members) = state.store.friend_group_member_ids(&user.id, &group) {
                crate::friend_events::publish_group_message(&value.message, members);
            }
        }
    }
    result(created)
}
async fn read(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((group, id)): Path<(String, String)>,
) -> Response {
    let user = user!(&state, &headers);
    result(state.store.read_ai_snapshot(&user.id, &group, &id))
}
async fn revoke(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((group, id)): Path<(String, String)>,
) -> Response {
    let user = user!(&state, &headers);
    result(
        state
            .store
            .revoke_ai_snapshot(&user.id, &group, &id)
            .map(|()| serde_json::json!({"ok":true})),
    )
}
async fn upload(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(group): Path<String>,
    body: Result<Json<Upload>, JsonRejection>,
) -> Response {
    let user = user!(&state, &headers);
    let Json(body) = match body {
        Ok(v) => v,
        Err(e) => return invalid_json(e),
    };
    match tokio::task::spawn_blocking(move || {
        state
            .store
            .upload_ai_snapshot_asset(&user.id, &group, &body.base64)
    })
    .await
    {
        Ok(value) => result(value),
        Err(_) => json_error(StatusCode::INTERNAL_SERVER_ERROR, "Image processing failed"),
    }
}
async fn asset(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((group, id, asset)): Path<(String, String, String)>,
) -> Response {
    let user = user!(&state, &headers);
    match state
        .store
        .read_ai_snapshot_asset(&user.id, &group, &id, &asset)
    {
        Ok((mime, bytes)) => (
            [
                ("content-type", mime.as_str()),
                ("content-security-policy", "default-src 'none'; sandbox"),
            ],
            bytes,
        )
            .into_response(),
        Err(e) => error(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_transport_fields_fail_deserialization() {
        assert!(serde_json::from_str::<Upload>(
            r#"{"base64":"AA==","url":"https://private.invalid"}"#
        )
        .is_err());
        assert!(serde_json::from_str::<Create>(
            r#"{"idempotency_key":"operation","document":{},"cookie":"secret"}"#
        )
        .is_err());
    }

    #[tokio::test]
    async fn private_headers_cover_errors_and_media() {
        use tower::ServiceExt;
        let app = Router::new()
            .route("/asset", get(|| async { StatusCode::NOT_FOUND }))
            .route_layer(middleware::from_fn(private_response));
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/asset")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        assert_eq!(response.headers()["cache-control"], "private, no-store");
        assert_eq!(response.headers()["x-content-type-options"], "nosniff");
        assert_eq!(response.headers()["referrer-policy"], "no-referrer");
    }
}
