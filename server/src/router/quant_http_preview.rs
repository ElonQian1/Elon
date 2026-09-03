//! Credential-free HTTP Paper preview hosted by the main Yilong server.
//!
//! This route intentionally exposes only public runtime and deterministic research data. It never
//! forwards cookies, authorization headers, user grants, ESK projections, or operator endpoints.

use std::{path::Path, sync::OnceLock, time::Duration};

use axum::{
    body::{Body, Bytes},
    extract::{DefaultBodyLimit, RawQuery},
    http::{header, HeaderName, HeaderValue, Method, StatusCode},
    response::{IntoResponse, Redirect, Response},
    routing::{get, post},
    Json, Router,
};
use serde_json::json;
use tower::ServiceBuilder;
use tower_http::{
    services::{ServeDir, ServeFile},
    set_header::SetResponseHeaderLayer,
};

const QUANT_LOOPBACK_ORIGIN: &str = "http://127.0.0.1:8787";
const MAX_BACKTEST_REQUEST_BYTES: usize = 32 * 1024;
const MAX_PUBLIC_QUERY_BYTES: usize = 2 * 1024;
const MAX_UPSTREAM_RESPONSE_BYTES: u64 = 2 * 1024 * 1024;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PublicQuantEndpoint {
    Health,
    Runtime,
    MarketOverview,
    ResearchSnapshots,
    ResearchBacktests,
}

impl PublicQuantEndpoint {
    fn method(self) -> Method {
        match self {
            Self::ResearchBacktests => Method::POST,
            Self::Health | Self::Runtime | Self::MarketOverview | Self::ResearchSnapshots => {
                Method::GET
            }
        }
    }

    fn path(self) -> &'static str {
        match self {
            Self::Health => "/api/health",
            Self::Runtime => "/api/v1/runtime",
            Self::MarketOverview => "/api/v1/markets/spot/overview",
            Self::ResearchSnapshots => "/api/v1/research/snapshots",
            Self::ResearchBacktests => "/api/v1/research/backtests",
        }
    }
}

pub(super) fn routes<S>(data_dir: &Path) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    let dist = data_dir.join("quant-http-preview-dist");
    let immutable_assets = ServiceBuilder::new()
        .layer(SetResponseHeaderLayer::overriding(
            header::CACHE_CONTROL,
            HeaderValue::from_static("public, max-age=31536000, immutable"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            header::X_CONTENT_TYPE_OPTIONS,
            HeaderValue::from_static("nosniff"),
        ))
        .service(ServeDir::new(dist.join("assets")));
    let index = ServiceBuilder::new()
        .layer(SetResponseHeaderLayer::overriding(
            header::CACHE_CONTROL,
            HeaderValue::from_static("no-store, no-cache, must-revalidate"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            HeaderName::from_static("content-security-policy"),
            HeaderValue::from_static(
                "default-src 'self'; script-src 'self'; connect-src 'self'; \
                 style-src 'self' 'unsafe-inline'; img-src 'self' data:; \
                 frame-ancestors 'none'; object-src 'none'; base-uri 'none'; form-action 'none'",
            ),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            header::REFERRER_POLICY,
            HeaderValue::from_static("no-referrer"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            header::X_CONTENT_TYPE_OPTIONS,
            HeaderValue::from_static("nosniff"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            HeaderName::from_static("x-frame-options"),
            HeaderValue::from_static("DENY"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            HeaderName::from_static("permissions-policy"),
            HeaderValue::from_static(
                "camera=(), microphone=(), geolocation=(), payment=(), usb=()",
            ),
        ))
        .service(ServeFile::new(dist.join("index.html")));

    Router::<S>::new()
        .route("/quant", get(|| async { Redirect::permanent("/quant/") }))
        .route_service("/quant/", index)
        .nest_service("/quant/assets", immutable_assets)
        .route("/quant/api/health", get(proxy_health))
        .route("/quant/api/v1/runtime", get(proxy_runtime))
        .route(
            "/quant/api/v1/markets/spot/overview",
            get(proxy_market_overview),
        )
        .route(
            "/quant/api/v1/research/snapshots",
            get(proxy_research_snapshots),
        )
        .route(
            "/quant/api/v1/research/backtests",
            post(proxy_research_backtest).layer(DefaultBodyLimit::max(MAX_BACKTEST_REQUEST_BYTES)),
        )
}

async fn proxy_health() -> Response {
    proxy(PublicQuantEndpoint::Health, None, None).await
}

async fn proxy_runtime() -> Response {
    proxy(PublicQuantEndpoint::Runtime, None, None).await
}

async fn proxy_market_overview(RawQuery(query): RawQuery) -> Response {
    proxy(PublicQuantEndpoint::MarketOverview, None, query.as_deref()).await
}

async fn proxy_research_snapshots() -> Response {
    proxy(PublicQuantEndpoint::ResearchSnapshots, None, None).await
}

async fn proxy_research_backtest(body: Bytes) -> Response {
    proxy(PublicQuantEndpoint::ResearchBacktests, Some(body), None).await
}

async fn proxy(
    endpoint: PublicQuantEndpoint,
    body: Option<Bytes>,
    raw_query: Option<&str>,
) -> Response {
    let url = match upstream_url(endpoint, raw_query) {
        Ok(url) => url,
        Err(response) => return response,
    };
    let mut request = direct_loopback_client().request(endpoint.method(), url);
    if let Some(body) = body {
        request = request
            .header(header::CONTENT_TYPE, "application/json")
            .body(body);
    }
    let upstream = match request.send().await {
        Ok(response) => response,
        Err(error) => {
            tracing::warn!(endpoint = endpoint.path(), %error, "quant HTTP preview upstream unavailable");
            return unavailable();
        }
    };
    if upstream
        .content_length()
        .is_some_and(|length| length > MAX_UPSTREAM_RESPONSE_BYTES)
    {
        tracing::warn!(
            endpoint = endpoint.path(),
            "quant HTTP preview response too large"
        );
        return unavailable();
    }
    let status =
        StatusCode::from_u16(upstream.status().as_u16()).unwrap_or(StatusCode::BAD_GATEWAY);
    let bytes = match upstream.bytes().await {
        Ok(bytes) if bytes.len() as u64 <= MAX_UPSTREAM_RESPONSE_BYTES => bytes,
        Ok(_) => {
            tracing::warn!(
                endpoint = endpoint.path(),
                "quant HTTP preview response exceeded limit"
            );
            return unavailable();
        }
        Err(error) => {
            tracing::warn!(endpoint = endpoint.path(), %error, "quant HTTP preview response failed");
            return unavailable();
        }
    };
    let mut response = Response::new(Body::from(bytes));
    *response.status_mut() = status;
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/json; charset=utf-8"),
    );
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}

fn upstream_url(
    endpoint: PublicQuantEndpoint,
    raw_query: Option<&str>,
) -> Result<reqwest::Url, Response> {
    if raw_query.is_some_and(|query| query.len() > MAX_PUBLIC_QUERY_BYTES) {
        return Err((
            StatusCode::URI_TOO_LONG,
            Json(json!({
                "code": "quant_http_preview_query_too_long",
                "message": "公开行情查询参数过长。"
            })),
        )
            .into_response());
    }

    let mut url = reqwest::Url::parse(QUANT_LOOPBACK_ORIGIN)
        .expect("fixed quant loopback origin must be a valid URL");
    url.set_path(endpoint.path());
    url.set_query(raw_query.filter(|query| !query.is_empty()));
    Ok(url)
}

fn unavailable() -> Response {
    (
        StatusCode::SERVICE_UNAVAILABLE,
        Json(json!({
            "code": "quant_http_preview_unavailable",
            "message": "量化 Paper 测试服务暂时不可用，请稍后重试。"
        })),
    )
        .into_response()
}

fn direct_loopback_client() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .connect_timeout(Duration::from_secs(2))
            .timeout(Duration::from_secs(30))
            .build()
            .expect("fixed loopback HTTP client must build")
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::Request;
    use tower::ServiceExt;

    #[test]
    fn public_allowlist_contains_only_credential_free_research_endpoints() {
        let endpoints = [
            PublicQuantEndpoint::Health,
            PublicQuantEndpoint::Runtime,
            PublicQuantEndpoint::MarketOverview,
            PublicQuantEndpoint::ResearchSnapshots,
            PublicQuantEndpoint::ResearchBacktests,
        ];
        let paths = endpoints.map(PublicQuantEndpoint::path);
        assert_eq!(
            paths,
            [
                "/api/health",
                "/api/v1/runtime",
                "/api/v1/markets/spot/overview",
                "/api/v1/research/snapshots",
                "/api/v1/research/backtests",
            ]
        );
        assert!(paths.iter().all(|path| {
            !path.contains("/me/")
                && !path.contains("operator")
                && !path.contains("import")
                && !path.contains("esk")
                && !path.contains("orders")
        }));
    }

    #[test]
    fn market_overview_url_preserves_query_on_fixed_loopback_path() {
        let url = upstream_url(
            PublicQuantEndpoint::MarketOverview,
            Some("symbol=BTCUSDT&interval=1m&limit=240"),
        )
        .expect("bounded public query must be accepted");

        assert_eq!(url.origin().ascii_serialization(), QUANT_LOOPBACK_ORIGIN);
        assert_eq!(url.path(), "/api/v1/markets/spot/overview");
        assert_eq!(url.query(), Some("symbol=BTCUSDT&interval=1m&limit=240"));
    }

    #[tokio::test]
    async fn oversized_market_query_is_rejected_before_upstream_access() {
        let app: Router = routes(Path::new("missing-quant-preview-dist"));
        let query = "x".repeat(MAX_PUBLIC_QUERY_BYTES + 1);
        let response = app
            .oneshot(
                Request::get(format!("/quant/api/v1/markets/spot/overview?{query}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::URI_TOO_LONG);
    }

    #[tokio::test]
    async fn sensitive_quant_paths_are_not_routed() {
        let app: Router = routes(Path::new("missing-quant-preview-dist"));
        for path in [
            "/quant/api/v1/paper/me/position",
            "/quant/api/v1/paper/orders",
            "/quant/api/v1/paper/operator/operations-snapshot",
            "/quant/api/v1/paper/imports/paid-supporters",
            "/quant/api/v1/markets/spot/overview/private",
            "/quant/api/v1/markets/spot/orders",
        ] {
            let response = app
                .clone()
                .oneshot(Request::get(path).body(Body::empty()).unwrap())
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::NOT_FOUND, "{path}");
        }
    }
}
