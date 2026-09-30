//! Independent HTTPS account entry. Reuses main authentication, never game policy.
use crate::{
    project_auth::{login_inner, LoginRequest},
    types::AppState,
};
use axum::{
    extract::{rejection::JsonRejection, DefaultBodyLimit, Request, State},
    http::{header, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    path::Path,
    sync::{Arc, LazyLock, Mutex},
    time::{Duration, Instant},
};

pub(crate) fn routes(_data_dir: &Path) -> Router<Arc<AppState>> {
    Router::new()
        .route("/pc/esk-compute", get(crate::web::pc_spa_index))
        .route("/api/esk-compute-center/login", post(login))
        .layer(DefaultBodyLimit::max(4096))
        .layer(middleware::from_fn(same_origin_login))
        .layer(middleware::from_fn(
            super::super::access::transport::require_secure_transport,
        ))
}

async fn same_origin_login(request: Request, next: Next) -> Response {
    let mut response = if request.method() == axum::http::Method::POST
        && (request.headers().get_all(header::ORIGIN).iter().count() != 1
            || request.uri().query().is_some())
    {
        failure(StatusCode::FORBIDDEN, "需要从安全账户页面登录")
    } else {
        next.run(request).await
    };
    response.headers_mut().insert(
        header::CONTENT_SECURITY_POLICY,
        axum::http::HeaderValue::from_static(
            "frame-ancestors 'none'; base-uri 'self'; object-src 'none'; form-action 'self'",
        ),
    );
    response.headers_mut().insert(
        header::X_CONTENT_TYPE_OPTIONS,
        axum::http::HeaderValue::from_static("nosniff"),
    );
    response
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PasswordLogin {
    account: String,
    password: String,
}

#[derive(Default)]
struct Limits {
    global: Vec<Instant>,
    accounts: HashMap<String, Vec<Instant>>,
}
impl Limits {
    fn take(&mut self, account: &str, now: Instant) -> bool {
        let recent = |at: &Instant| now.saturating_duration_since(*at) < Duration::from_secs(60);
        self.global.retain(recent);
        self.accounts.retain(|_, rows| {
            rows.retain(recent);
            !rows.is_empty()
        });
        let key = hex::encode(Sha256::digest(account.trim().to_lowercase().as_bytes()));
        if self.global.len() >= 60 || self.accounts.get(&key).is_some_and(|rows| rows.len() >= 5) {
            return false;
        }
        self.global.push(now);
        self.accounts.entry(key).or_default().push(now);
        true
    }
}
static LIMITS: LazyLock<Mutex<Limits>> = LazyLock::new(|| Mutex::new(Limits::default()));
static WORKERS: LazyLock<Arc<tokio::sync::Semaphore>> =
    LazyLock::new(|| Arc::new(tokio::sync::Semaphore::new(4)));

async fn login(
    State(state): State<Arc<AppState>>,
    body: Result<Json<PasswordLogin>, JsonRejection>,
) -> Response {
    let Ok(Json(body)) = body else {
        return failure(StatusCode::BAD_REQUEST, "无效的登录内容");
    };
    if body.account.trim().is_empty()
        || body.account.len() > 254
        || body.password.is_empty()
        || body.password.len() > 1024
    {
        return failure(StatusCode::BAD_REQUEST, "请输入有效的账号和密码");
    }
    if !LIMITS
        .lock()
        .is_ok_and(|mut limits| limits.take(&body.account, Instant::now()))
    {
        return failure(
            StatusCode::TOO_MANY_REQUESTS,
            "尝试次数较多，请一分钟后重试",
        );
    }
    let Ok(permit) = Arc::clone(&WORKERS).try_acquire_owned() else {
        return failure(StatusCode::TOO_MANY_REQUESTS, "登录服务繁忙，请稍后重试");
    };
    let result = tokio::task::spawn_blocking(move || {
        let _permit = permit;
        login_inner(
            &state,
            LoginRequest {
                account: body.account,
                password: body.password,
                device_name: Some("ESK 与算力安全账户".into()),
                apk_version: None,
                remember_device: false,
            },
        )
    })
    .await;
    match result {
        Ok(Ok((token, expires_at, user))) => {
            Json(json!({"token":token,"expires_at":expires_at,"user":user})).into_response()
        }
        _ => failure(StatusCode::UNAUTHORIZED, "登录未成功，请检查账号和密码"),
    }
}
fn failure(status: StatusCode, message: &str) -> Response {
    (status, Json(json!({"error":message}))).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn throttling_is_bounded_and_expires() {
        let mut limits = Limits::default();
        let at = Instant::now();
        for _ in 0..5 {
            assert!(limits.take("synthetic@example.test", at));
        }
        assert!(!limits.take("SYNTHETIC@example.test", at));
        assert!(limits.take("synthetic@example.test", at + Duration::from_secs(61)));
        assert_eq!(limits.global.len(), 1);
    }
}
