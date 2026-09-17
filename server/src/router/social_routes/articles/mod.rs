use crate::{
    project_auth::{auth_from_headers, json_error},
    store::articles::{ArticleDocument, ArticleFault},
    types::AppState,
};
use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
mod assets;

pub(super) fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .merge(assets::routes())
        .merge(share::routes())
        .route("/api/me/articles", get(list).post(create))
        .route("/api/me/articles/media", post(upload))
        .route("/api/me/articles/:id/draft", get(draft).put(save))
        .route("/api/me/articles/:id/publish", post(publish))
        .route("/api/me/articles/:id/withdraw", post(withdraw))
        .route("/api/me/articles/:id/revisions/:revision", get(read))
}
fn result<T: Serialize>(value: anyhow::Result<T>) -> Response {
    match value {
        Ok(value) => ([("cache-control", "private, no-store")], Json(value)).into_response(),
        Err(error) => {
            if let Some(fault) = error.downcast_ref::<ArticleFault>() {
                json_error(
                    StatusCode::from_u16(fault.0).unwrap_or(StatusCode::BAD_REQUEST),
                    fault.1.clone(),
                )
            } else {
                tracing::error!(%error, "article operation failed");
                json_error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "文章操作暂时失败，请重试",
                )
            }
        }
    }
}
macro_rules! user {
    ($state:expr, $headers:expr) => {
        match auth_from_headers($state, $headers) {
            Ok(user) => user,
            Err(error) => return json_error(StatusCode::UNAUTHORIZED, error.to_string()),
        }
    };
}
// Declared after `user!` so the macro's textual scope covers the public share routes.
mod share;
#[derive(Deserialize)]
struct ListQuery {
    group_id: Option<String>,
    offset: Option<i64>,
}
#[derive(Deserialize)]
struct ReadQuery {
    compact: Option<bool>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Save {
    version: i64,
    document: ArticleDocument,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Publish {
    version: i64,
    group_ids: Vec<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Version {
    version: i64,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Upload {
    base64: String,
}
async fn list(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(q): Query<ListQuery>,
) -> Response {
    let u = user!(&state, &headers);
    result(
        state
            .store
            .list_articles(&u.id, q.group_id.as_deref(), q.offset.unwrap_or(0)),
    )
}
async fn create(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(doc): Json<ArticleDocument>,
) -> Response {
    let u = user!(&state, &headers);
    result(state.store.create_article(&u.id, doc))
}
async fn draft(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Response {
    let u = user!(&state, &headers);
    result(state.store.article_draft(&u.id, &id))
}
async fn save(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(body): Json<Save>,
) -> Response {
    let u = user!(&state, &headers);
    result(
        state
            .store
            .save_article(&u.id, &id, body.version, body.document),
    )
}
async fn read(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((id, revision)): Path<(String, i64)>,
    Query(q): Query<ReadQuery>,
) -> Response {
    let u = user!(&state, &headers);
    result(
        state
            .store
            .read_article(&u.id, &id, revision, q.compact.unwrap_or(false)),
    )
}
async fn publish(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(body): Json<Publish>,
) -> Response {
    let u = user!(&state, &headers);
    let published = state
        .store
        .publish_article(&u.id, &id, body.version, &body.group_ids);
    if let Ok(publication) = &published {
        for message in &publication.messages {
            match state
                .store
                .friend_group_member_ids(&u.id, &message.group_id)
            {
                Ok(recipients) => crate::friend_events::publish_group_message(message, recipients),
                Err(error) => {
                    tracing::warn!(%error, "article committed; chat polling will recover notification")
                }
            }
        }
    }
    result(published)
}
async fn withdraw(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(body): Json<Version>,
) -> Response {
    let u = user!(&state, &headers);
    result(
        state
            .store
            .withdraw_article(&u.id, &id, body.version)
            .map(|()| serde_json::json!({"ok":true})),
    )
}
async fn upload(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<Upload>,
) -> Response {
    let u = user!(&state, &headers);
    // Decode and resize off the async request executor.
    match tokio::task::spawn_blocking(move || state.store.upload_article_media(&u.id, &body.base64))
        .await
    {
        Ok(value) => result(value),
        Err(_) => json_error(StatusCode::INTERNAL_SERVER_ERROR, "图片处理失败，请重试"),
    }
}
