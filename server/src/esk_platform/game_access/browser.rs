//! Scoped browser entry on the verified Rust TLS listener; shares original main users.
use super::{model::Error, policy};
use crate::{
    project_auth::{login_inner, LoginRequest},
    types::AppState,
};
use axum::{
    extract::{rejection::JsonRejection, DefaultBodyLimit, Request, State},
    http::{header, HeaderValue, Method, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use serde_json::json;
use std::{
    collections::HashMap,
    path::Path,
    sync::{Arc, LazyLock, Mutex},
    time::{Duration, Instant},
};
use tower_http::services::ServeDir;

pub(crate) fn routes(data_dir: &Path) -> Router<Arc<AppState>> {
    Router::new()
        .route("/pc/game-access", get(crate::web::pc_spa_index))
        .route("/pc/game-login", get(crate::web::pc_spa_index))
        .nest_service(
            "/pc/assets",
            ServeDir::new(data_dir.join("pc-next-dist/assets")),
        )
        .route("/favicon.ico", get(crate::web::favicon))
        .route("/api/game-access/v1/login", post(login))
        .layer(DefaultBodyLimit::max(4096))
        .layer(middleware::from_fn(browser_headers))
        .layer(middleware::from_fn(
            super::super::access::transport::require_secure_transport,
        ))
}

async fn browser_headers(request: Request, next: Next) -> Response {
    // Login is only a same-origin browser action. Missing Origin is never implicit consent.
    let mut response = if request.method() == Method::POST
        && (request.headers().get_all(header::ORIGIN).iter().count() != 1
            || request.uri().query().is_some())
    {
        failure(StatusCode::FORBIDDEN, "game_login_origin_required")
    } else {
        next.run(request).await
    };
    response.headers_mut().insert(
        header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_static(
            "frame-ancestors 'none'; base-uri 'self'; object-src 'none'; form-action 'self'",
        ),
    );
    response.headers_mut().insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    response
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PasswordLogin {
    account: String,
    password: String,
}

fn failure(status: StatusCode, code: &str) -> Response {
    (status, Json(json!({"error":code}))).into_response()
}

// Process-wide because the secure router is assembled separately for each TLS connection.
#[derive(Default)]
struct LoginLimits {
    accounts: HashMap<String, Vec<Instant>>,
    global: Vec<Instant>,
}
impl LoginLimits {
    fn take(&mut self, account: &str, now: Instant) -> bool {
        let recent = |at: &Instant| now.saturating_duration_since(*at) < Duration::from_secs(60);
        self.global.retain(recent);
        self.accounts.retain(|_, times| {
            times.retain(recent);
            !times.is_empty()
        });
        let key = policy::hash(&account.trim().to_lowercase());
        if self.global.len() >= 120 || self.accounts.get(&key).is_some_and(|v| v.len() >= 5) {
            return false;
        }
        self.global.push(now);
        self.accounts.entry(key).or_default().push(now);
        true
    }
}
static LIMITS: LazyLock<Mutex<LoginLimits>> = LazyLock::new(|| Mutex::new(LoginLimits::default()));
static WORKERS: LazyLock<Arc<tokio::sync::Semaphore>> =
    LazyLock::new(|| Arc::new(tokio::sync::Semaphore::new(4)));

async fn login(
    State(state): State<Arc<AppState>>,
    body: Result<Json<PasswordLogin>, JsonRejection>,
) -> Response {
    if let Err(error) = policy::load() {
        return failure(
            StatusCode::SERVICE_UNAVAILABLE,
            if matches!(error.downcast_ref(), Some(Error::Disabled)) {
                "game_access_disabled"
            } else {
                "game_access_unavailable"
            },
        );
    }
    let Ok(Json(body)) = body else {
        return failure(StatusCode::BAD_REQUEST, "game_login_invalid_input");
    };
    if body.account.trim().is_empty()
        || body.account.len() > 254
        || body.password.is_empty()
        || body.password.len() > 1024
    {
        return failure(StatusCode::BAD_REQUEST, "game_login_invalid_input");
    }
    if !LIMITS
        .lock()
        .is_ok_and(|mut limits| limits.take(&body.account, Instant::now()))
    {
        return failure(StatusCode::TOO_MANY_REQUESTS, "game_login_rate_limited");
    }
    let Ok(permit) = Arc::clone(&WORKERS).try_acquire_owned() else {
        return failure(StatusCode::TOO_MANY_REQUESTS, "game_login_busy");
    };
    let result = tokio::task::spawn_blocking(move || {
        let _permit = permit;
        login_inner(
            &state,
            LoginRequest {
                account: body.account,
                password: body.password,
                device_name: Some("ESK game authorization".into()),
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
        // Do not reveal whether an account exists, is disabled, or has the wrong password.
        _ => failure(StatusCode::UNAUTHORIZED, "game_login_failed"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn game_login_limits_share_accounts_and_expire() {
        let mut limits = LoginLimits::default();
        let now = Instant::now();
        for _ in 0..5 {
            assert!(limits.take(" USER@example.test ", now));
        }
        assert!(!limits.take("user@example.test", now));
        for i in 0..115 {
            assert!(limits.take(&format!("other{i}"), now));
        }
        assert!(!limits.take("new", now));
        assert!(limits.take("user@example.test", now + Duration::from_secs(60)));
        assert_eq!(limits.accounts.len(), 1);
    }
}
