//! Explicit public-content attachment to the existing TLS listener.
//! Account middleware stays on account routes; the shared public router owns its API allowlist.
use std::path::Path;

use anyhow::{bail, Result};
use axum::{
    extract::{DefaultBodyLimit, Request},
    http::{header, HeaderValue, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    Router,
};

pub(super) fn enabled(value: Option<&str>, account_tls_enabled: bool) -> Result<bool> {
    match value {
        None | Some("false") => Ok(false),
        Some("true") if account_tls_enabled => Ok(true),
        Some("true") => bail!("QUANT_PUBLIC_HTTPS_REQUIRES_ACCOUNT_TLS"),
        Some(_) => bail!("QUANT_PUBLIC_HTTPS_ENABLED_INVALID"),
    }
}

pub(super) fn attach(account_routes: Router, data_dir: &Path, enabled: bool) -> Router {
    if !enabled {
        return account_routes;
    }
    let public_routes = crate::router::quant_http_preview::routes(data_dir)
        .layer(DefaultBodyLimit::max(32 * 1024))
        .layer(middleware::from_fn(public_request));
    account_routes.merge(public_routes)
}

async fn public_request(mut request: Request, next: Next) -> Response {
    let path = request.uri().path();
    if path.len() > 1024
        || path.contains('%')
        || path.contains('\\')
        || path.contains("..")
        || path.contains("//")
    {
        return StatusCode::NOT_FOUND.into_response();
    }
    if request
        .uri()
        .query()
        .is_some_and(|query| query.len() > 2048)
    {
        return StatusCode::URI_TOO_LONG.into_response();
    }
    // Public data must not gain credentials from a same-origin account client.
    request.headers_mut().remove(header::AUTHORIZATION);
    request.headers_mut().remove(header::COOKIE);
    let mut response = next.run(request).await;
    response.headers_mut().remove(header::SET_COOKIE);
    response.headers_mut().insert(
        "x-yilong-quant-transport",
        HeaderValue::from_static("https-public-v1"),
    );
    response.headers_mut().insert(
        header::STRICT_TRANSPORT_SECURITY,
        HeaderValue::from_static("max-age=15552000"),
    );
    response
}

#[cfg(test)]
#[path = "quant_public_tests.rs"]
mod tests;
