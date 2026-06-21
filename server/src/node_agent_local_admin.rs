// server/src/node_agent_local_admin.rs

use axum::{
    body::Body,
    extract::State,
    http::{header::ORIGIN, HeaderMap, HeaderValue, Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

pub(crate) const LOCAL_ADMIN_TOKEN_HEADER: &str = "x-elon-local-admin-token";

pub(crate) fn generate_local_admin_token() -> String {
    format!("la_{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple())
}

pub(crate) async fn require_local_admin(
    State(runtime): State<Arc<crate::NodeRuntime>>,
    headers: HeaderMap,
    request: Request<Body>,
    next: Next,
) -> Response {
    match verify_local_admin_request(
        &headers,
        runtime.local_admin_token(),
        &runtime.cloud_http_url(),
    ) {
        Ok(()) => next.run(request).await,
        Err(error) => (
            StatusCode::FORBIDDEN,
            Json(json!({
                "ok": false,
                "error": error,
            })),
        )
            .into_response(),
    }
}

pub(crate) fn verify_local_admin_request(
    headers: &HeaderMap,
    expected_token: &str,
    cloud_http_url: &str,
) -> Result<(), String> {
    if expected_token.trim().is_empty() {
        return Err("本机管理授权尚未初始化，请重启一龙 PC 节点。".to_string());
    }
    let Some(actual_token) = header_str(headers, LOCAL_ADMIN_TOKEN_HEADER) else {
        return Err("缺少本机管理授权，请刷新 PC 页面后重试。".to_string());
    };
    if actual_token != expected_token {
        return Err("本机管理授权已过期，请刷新 PC 页面后重试。".to_string());
    }

    verify_origin(headers, cloud_http_url)
}

pub(crate) fn trusted_origin_header_values(cloud_http_url: &str) -> Vec<HeaderValue> {
    trusted_origins(cloud_http_url)
        .into_iter()
        .filter_map(|origin| HeaderValue::from_str(&origin).ok())
        .collect()
}

fn verify_origin(headers: &HeaderMap, cloud_http_url: &str) -> Result<(), String> {
    let Some(origin) = header_str(headers, ORIGIN.as_str()) else {
        if header_str(headers, "sec-fetch-site").is_some_and(|site| site == "cross-site") {
            return Err("拒绝未知来源的本机管理请求。".to_string());
        }
        return Ok(());
    };
    if trusted_origins(cloud_http_url)
        .iter()
        .any(|trusted| trusted == origin)
    {
        return Ok(());
    }
    Err("拒绝非一龙 PC 工作台来源的本机管理请求。".to_string())
}

fn trusted_origins(cloud_http_url: &str) -> Vec<String> {
    let mut origins = vec![
        "http://43.139.149.158:8080".to_string(),
        "http://127.0.0.1:7799".to_string(),
        "http://localhost:7799".to_string(),
        "http://127.0.0.1:8080".to_string(),
        "http://localhost:8080".to_string(),
        "http://127.0.0.1:3000".to_string(),
        "http://localhost:3000".to_string(),
    ];
    if let Some(origin) = origin_from_url(cloud_http_url) {
        if !origins.iter().any(|item| item == &origin) {
            origins.push(origin);
        }
    }
    origins
}

fn origin_from_url(raw: &str) -> Option<String> {
    let url = reqwest::Url::parse(raw.trim()).ok()?;
    let origin = url.origin().ascii_serialization();
    (origin != "null").then_some(origin)
}

fn header_str<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

#[cfg(test)]
mod tests {
    use super::{verify_local_admin_request, LOCAL_ADMIN_TOKEN_HEADER};
    use axum::http::{HeaderMap, HeaderName, HeaderValue};

    fn headers(token: Option<&str>, origin: Option<&str>) -> HeaderMap {
        let mut headers = HeaderMap::new();
        if let Some(token) = token {
            headers.insert(
                HeaderName::from_static(LOCAL_ADMIN_TOKEN_HEADER),
                HeaderValue::from_str(token).unwrap(),
            );
        }
        if let Some(origin) = origin {
            headers.insert("origin", HeaderValue::from_str(origin).unwrap());
        }
        headers
    }

    #[test]
    fn accepts_token_from_trusted_cloud_origin() {
        let headers = headers(Some("secret"), Some("http://43.139.149.158:8080"));
        assert!(
            verify_local_admin_request(&headers, "secret", "http://43.139.149.158:8080").is_ok()
        );
    }

    #[test]
    fn accepts_token_without_browser_origin_for_native_tools() {
        let headers = headers(Some("secret"), None);
        assert!(verify_local_admin_request(&headers, "secret", "http://example.com").is_ok());
    }

    #[test]
    fn rejects_missing_or_stale_token() {
        let missing = headers(None, Some("http://43.139.149.158:8080"));
        assert!(
            verify_local_admin_request(&missing, "secret", "http://43.139.149.158:8080").is_err()
        );

        let stale = headers(Some("old"), Some("http://43.139.149.158:8080"));
        assert!(
            verify_local_admin_request(&stale, "secret", "http://43.139.149.158:8080").is_err()
        );
    }

    #[test]
    fn rejects_untrusted_browser_origin() {
        let headers = headers(Some("secret"), Some("http://evil.example"));
        assert!(
            verify_local_admin_request(&headers, "secret", "http://43.139.149.158:8080").is_err()
        );
    }

    #[test]
    fn rejects_cross_site_fetch_without_origin() {
        let mut headers = headers(Some("secret"), None);
        headers.insert("sec-fetch-site", HeaderValue::from_static("cross-site"));
        assert!(
            verify_local_admin_request(&headers, "secret", "http://43.139.149.158:8080").is_err()
        );
    }
}
