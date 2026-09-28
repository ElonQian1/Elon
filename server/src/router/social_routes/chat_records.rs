use crate::{
    project_auth::{auth_from_headers, json_error},
    store::articles::{chat_records, ArticleFault},
    types::AppState,
};
use axum::{
    body::Bytes,
    extract::{DefaultBodyLimit, Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use std::sync::Arc;
#[path = "chat_record_cache.rs"]
mod cache;

pub(super) fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/api/me/groups/:group/chat-records",
            post(create).layer(DefaultBodyLimit::max(chat_records::MAX_DOCUMENT + 4096)),
        )
        .route(
            "/api/me/groups/:group/chat-records/assets",
            post(upload).layer(DefaultBodyLimit::max(chat_records::MAX_ASSET)),
        )
        .route(
            "/api/me/groups/:group/chat-records/:record",
            get(read).delete(revoke),
        )
        .route(
            "/api/me/groups/:group/chat-records/:record/assets/:asset",
            get(asset),
        )
        .route_layer(axum::middleware::from_fn(cache::private_response))
}

fn result<T: serde::Serialize>(result: anyhow::Result<T>) -> Response {
    match result {
        Ok(value) => Json(value).into_response(),
        Err(error) => failure(error),
    }
}
fn failure(error: anyhow::Error) -> Response {
    if let Some(ArticleFault(status, message)) = error.downcast_ref::<ArticleFault>() {
        return json_error(
            StatusCode::from_u16(*status).unwrap_or(StatusCode::BAD_REQUEST),
            message,
        );
    }
    tracing::warn!("Chat record operation failed");
    json_error(StatusCode::INTERNAL_SERVER_ERROR, "聊天记录处理失败")
}
macro_rules! user {
    ($state:expr, $headers:expr) => {
        match auth_from_headers($state, $headers) {
            Ok(user) => user,
            Err(_) => return json_error(StatusCode::UNAUTHORIZED, "请先登录"),
        }
    };
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Create {
    operation: String,
    document: chat_records::Document,
}

async fn create(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(group): Path<String>,
    body: Result<Json<Create>, axum::extract::rejection::JsonRejection>,
) -> Response {
    let user = user!(&state, &headers);
    let Json(body) = match body {
        Ok(value) => value,
        Err(_) => return json_error(StatusCode::BAD_REQUEST, "聊天记录格式无效或过大"),
    };
    let value = state
        .store
        .create_chat_record(&user.id, &group, &body.operation, body.document);
    if let Ok(created) = &value {
        if !created.replayed {
            if let Ok(members) = state.store.friend_group_member_ids(&user.id, &group) {
                crate::friend_events::publish_group_message(&created.message, members);
            }
        }
    }
    result(value)
}
async fn read(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((group, record)): Path<(String, String)>,
) -> Response {
    let user = user!(&state, &headers);
    let version = match state
        .store
        .chat_record_version(&user.id, &group, &record, None)
    {
        Ok(version) => version,
        Err(error) => return failure(error),
    };
    let response = if cache::matches(&headers, &version) {
        StatusCode::NOT_MODIFIED.into_response()
    } else {
        result(state.store.read_chat_record(&user.id, &group, &record))
    };
    cache::versioned(response, &version)
}
async fn revoke(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((group, record)): Path<(String, String)>,
) -> Response {
    let user = user!(&state, &headers);
    result(
        state
            .store
            .revoke_chat_record(&user.id, &group, &record)
            .map(|_| serde_json::json!({"ok":true})),
    )
}
async fn upload(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(group): Path<String>,
    bytes: Bytes,
) -> Response {
    let user = user!(&state, &headers);
    match tokio::task::spawn_blocking(move || {
        state
            .store
            .upload_chat_record_asset(&user.id, &group, &bytes)
    })
    .await
    {
        Ok(value) => result(value),
        Err(_) => json_error(StatusCode::INTERNAL_SERVER_ERROR, "附件处理失败"),
    }
}
async fn asset(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((group, record, asset)): Path<(String, String, String)>,
) -> Response {
    let user = user!(&state, &headers);
    let version = match state
        .store
        .chat_record_version(&user.id, &group, &record, Some(&asset))
    {
        Ok(version) => version,
        Err(error) => return failure(error),
    };
    if cache::matches(&headers, &version) {
        return cache::versioned(StatusCode::NOT_MODIFIED.into_response(), &version);
    }
    let response = match state
        .store
        .read_chat_record_asset(&user.id, &group, &record, &asset)
    {
        Ok((mime, bytes)) => {
            let disposition = if mime.starts_with("image/") || mime.starts_with("video/") {
                "inline"
            } else {
                "attachment"
            };
            (
                [
                    ("content-type", mime.as_str()),
                    ("content-disposition", disposition),
                    ("content-security-policy", "default-src 'none'; sandbox"),
                ],
                bytes,
            )
                .into_response()
        }
        Err(error) => failure(error),
    };
    cache::versioned(response, &version)
}
