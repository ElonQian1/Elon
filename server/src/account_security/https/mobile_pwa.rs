//! Opt-in HTTPS delivery of the existing browser application, with its original authorization.
use anyhow::{bail, Result};
use axum::{
    extract::Request,
    http::HeaderValue,
    middleware::{self, Next},
    response::Response,
    Router,
};

pub(super) fn enabled(value: Option<&str>, account_tls_enabled: bool) -> Result<bool> {
    match value {
        None | Some("false") => Ok(false),
        Some("true") if account_tls_enabled => Ok(true),
        Some("true") => bail!("MOBILE_PWA_HTTPS_REQUIRES_ACCOUNT_TLS"),
        Some(_) => bail!("MOBILE_PWA_HTTPS_ENABLED_INVALID"),
    }
}

pub(super) fn attach(account_routes: Router, browser_routes: Router, enabled: bool) -> Router {
    if !enabled {
        return account_routes;
    }
    // Existing explicit account/public routes retain their own policies and precedence.
    // Reuse the HTTP router, including authentication, limits and all browser features.
    account_routes.fallback_service(browser_routes.layer(middleware::from_fn(mark_response)))
}

async fn mark_response(request: Request, next: Next) -> Response {
    let mut response = next.run(request).await;
    response
        .headers_mut()
        .insert("x-elon-pwa-transport", HeaderValue::from_static("https-v1"));
    response
}

#[cfg(test)]
#[path = "mobile_pwa_tests.rs"]
mod tests;
