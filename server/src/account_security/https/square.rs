//! Square private API is assembled only in the verified native HTTPS listener.
use crate::{
    project_auth::{auth_from_headers, json_error},
    store::articles::{
        square::{Enqueue, Selection},
        ArticleFault,
    },
    types::AppState,
};
use axum::{
    extract::{ConnectInfo, DefaultBodyLimit, Multipart, Path, Query, Request, State},
    http::{header, HeaderMap, Method, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::{net::SocketAddr, sync::Arc, time::Duration};
use tower_http::cors::{AllowOrigin, CorsLayer};
const ROOT: &str = "/api/me/article-channels/binance-square";
#[cfg(test)]
#[path = "square_tests.rs"]
mod tests;

pub(super) fn routes(state: Arc<AppState>) -> Router {
    let origin = reqwest::Url::parse(&state.public_url)
        .ok()
        .map(|u| u.origin().ascii_serialization());
    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
        .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE])
        .allow_origin(AllowOrigin::predicate(move |o, _| {
            o.to_str()
                .is_ok_and(|v| allowed_origin(v, origin.as_deref()))
        }));
    Router::new()
        .route(&format!("{ROOT}/articles"), get(articles))
        .route(&format!("{ROOT}/articles/:id"), get(article))
        .route(
            &format!("{ROOT}/account"),
            get(account).put(bind).delete(unbind),
        )
        .route(&format!("{ROOT}/preview"), post(preview))
        .route(&format!("{ROOT}/jobs"), get(jobs).post(enqueue))
        .route(&format!("{ROOT}/jobs/:id"), get(job))
        .route(&format!("{ROOT}/jobs/:id/cancel"), post(cancel))
        .route(&format!("{ROOT}/jobs/:id/retry"), post(retry))
        .route(&format!("{ROOT}/jobs/:id/resolve"), post(resolve))
        .route(&format!("{ROOT}/media"), post(upload_image))
        .merge(
            Router::new()
                .route(&format!("{ROOT}/videos"), post(upload_video))
                .layer(DefaultBodyLimit::max(33 * 1024 * 1024)),
        )
        .layer(DefaultBodyLimit::max(800 * 1024))
        .layer(middleware::from_fn(guard))
        .layer(cors)
        .with_state(state)
}
fn allowed_origin(value: &str, public: Option<&str>) -> bool {
    if public == Some(value) {
        return true;
    }
    // Native desktop workbench uses a bounded loopback listener. Tokens remain required.
    if value == "tauri://localhost" || value == "http://tauri.localhost" {
        return true;
    }
    reqwest::Url::parse(value).ok().is_some_and(|u| {
        u.origin().ascii_serialization() == value
            && ((u.scheme() == "https"
                && u.host_str() == Some("43.139.149.158")
                && u.port_or_known_default() == Some(8443))
                || (u.scheme() == "http"
                    && matches!(u.host_str(), Some("127.0.0.1" | "localhost"))
                    && u.port().is_some_and(|p| (7799..=7819).contains(&p))))
    })
}
async fn guard(request: Request, next: Next) -> Response {
    if !request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.starts_with("Bearer "))
    {
        return json_error(StatusCode::UNAUTHORIZED, "请先登录一龙账号");
    }
    if request.method() != Method::GET {
        let Some(ConnectInfo(peer)) = request.extensions().get::<ConnectInfo<SocketAddr>>() else {
            return StatusCode::SERVICE_UNAVAILABLE.into_response();
        };
        if crate::auth_request_guard::check_rate_limit(
            "square_https_writes",
            &peer.ip().to_string(),
            60,
            Duration::from_secs(60),
        )
        .is_err()
        {
            return json_error(StatusCode::TOO_MANY_REQUESTS, "操作太频繁，请稍后重试");
        }
    }
    let mut response = next.run(request).await;
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, "private, no-store".parse().unwrap());
    response
        .headers_mut()
        .insert(header::REFERRER_POLICY, "no-referrer".parse().unwrap());
    response
}
fn result<T: Serialize>(value: anyhow::Result<T>) -> Response {
    match value {
        Ok(value) => Json(value).into_response(),
        Err(e) => match e.downcast_ref::<ArticleFault>() {
            Some(f) => json_error(
                StatusCode::from_u16(f.0).unwrap_or(StatusCode::BAD_REQUEST),
                f.1.clone(),
            ),
            None => {
                tracing::warn!("SQUARE_OPERATION_FAILED");
                json_error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "币安发布操作暂时失败，请重试",
                )
            }
        },
    }
}
macro_rules! user {($s:expr,$h:expr)=>{match auth_from_headers($s,$h) {Ok(u)=>u,Err(_)=>return json_error(StatusCode::UNAUTHORIZED,"登录已过期，请重新登录")}};}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Bind {
    label: String,
    api_key: String,
}
#[derive(Deserialize)]
struct Page {
    #[serde(default)]
    offset: i64,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Resolve {
    #[serde(default)]
    post_id: Option<String>,
    #[serde(default)]
    not_published: bool,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Image {
    base64: String,
}
async fn account(State(s): State<Arc<AppState>>, h: HeaderMap) -> Response {
    let u = user!(&s, &h);
    result(s.store.square_account(&u.id))
}
async fn articles(State(s): State<Arc<AppState>>, h: HeaderMap, Query(p): Query<Page>) -> Response {
    let u = user!(&s, &h);
    result(s.store.list_articles(&u.id, None, p.offset))
}
async fn article(State(s): State<Arc<AppState>>, h: HeaderMap, Path(id): Path<String>) -> Response {
    let u = user!(&s, &h);
    result(s.store.article_draft(&u.id, &id))
}
async fn bind(State(s): State<Arc<AppState>>, h: HeaderMap, Json(b): Json<Bind>) -> Response {
    let u = user!(&s, &h);
    result(s.store.square_bind(&u.id, &b.label, &b.api_key))
}
async fn unbind(State(s): State<Arc<AppState>>, h: HeaderMap) -> Response {
    let u = user!(&s, &h);
    result(s.store.square_unbind(&u.id))
}
async fn preview(
    State(s): State<Arc<AppState>>,
    h: HeaderMap,
    Json(b): Json<Selection>,
) -> Response {
    let u = user!(&s, &h);
    result(s.store.square_preview(&u.id, b))
}
async fn jobs(State(s): State<Arc<AppState>>, h: HeaderMap, Query(q): Query<Page>) -> Response {
    let u = user!(&s, &h);
    result(s.store.square_jobs(&u.id, q.offset))
}
async fn enqueue(State(s): State<Arc<AppState>>, h: HeaderMap, Json(b): Json<Enqueue>) -> Response {
    let u = user!(&s, &h);
    result(s.store.square_enqueue(&u.id, b))
}
async fn job(State(s): State<Arc<AppState>>, h: HeaderMap, Path(id): Path<String>) -> Response {
    let u = user!(&s, &h);
    result(s.store.square_job(&u.id, &id))
}
async fn cancel(State(s): State<Arc<AppState>>, h: HeaderMap, Path(id): Path<String>) -> Response {
    let u = user!(&s, &h);
    result(s.store.square_cancel(&u.id, &id))
}
async fn retry(State(s): State<Arc<AppState>>, h: HeaderMap, Path(id): Path<String>) -> Response {
    let u = user!(&s, &h);
    result(s.store.square_retry(&u.id, &id))
}
async fn resolve(
    State(s): State<Arc<AppState>>,
    h: HeaderMap,
    Path(id): Path<String>,
    Json(b): Json<Resolve>,
) -> Response {
    let u = user!(&s, &h);
    result(
        s.store
            .square_resolve(&u.id, &id, b.post_id.as_deref(), b.not_published),
    )
}
async fn upload_image(
    State(s): State<Arc<AppState>>,
    h: HeaderMap,
    Json(b): Json<Image>,
) -> Response {
    let u = user!(&s, &h);
    match tokio::task::spawn_blocking(move || s.store.upload_article_media(&u.id, &b.base64)).await
    {
        Ok(r) => result(r),
        Err(_) => json_error(StatusCode::INTERNAL_SERVER_ERROR, "图片处理失败"),
    }
}
async fn upload_video(
    State(s): State<Arc<AppState>>,
    h: HeaderMap,
    mut multipart: Multipart,
) -> Response {
    let u = user!(&s, &h);
    let mut bytes = None;
    let mut cover = None;
    let mut duration = None;
    loop {
        match multipart.next_field().await {
            Ok(Some(field)) => {
                let name = field.name().unwrap_or("").to_string();
                match name.as_str() {
                    "video" if bytes.is_none() => match field.bytes().await {
                        Ok(b) => bytes = Some(b.to_vec()),
                        Err(_) => {
                            return json_error(StatusCode::BAD_REQUEST, "视频上传失败或超过32MB")
                        }
                    },
                    "cover_id" if cover.is_none() => {
                        cover = field.text().await.ok().filter(|s| s.len() <= 128);
                    }
                    "duration" if duration.is_none() => {
                        duration = field.text().await.ok().and_then(|v| v.parse::<f64>().ok());
                    }
                    _ => return json_error(StatusCode::BAD_REQUEST, "视频上传字段无效"),
                }
            }
            Ok(None) => break,
            Err(_) => return json_error(StatusCode::BAD_REQUEST, "视频上传不完整"),
        }
    }
    let (Some(bytes), Some(cover), Some(duration)) = (bytes, cover, duration) else {
        return json_error(StatusCode::BAD_REQUEST, "请选择视频及首帧封面");
    };
    match tokio::task::spawn_blocking(move || s.store.square_video(&u.id, bytes, duration, &cover))
        .await
    {
        Ok(r) => result(r),
        Err(_) => json_error(StatusCode::INTERNAL_SERVER_ERROR, "视频处理失败"),
    }
}
