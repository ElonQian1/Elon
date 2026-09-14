//! Uses the existing complete production Store/session/router fixture.
use super::Fixture;
use crate::router::quant_native_grid_access::test_config;
use axum::{
    body::{to_bytes, Body},
    http::{header, Request, StatusCode},
    Router,
};
use serde_json::{json, Value};
use tower::ServiceExt;

const PATH: &str = "/api/me/quant/native-grid/access-grants";
fn body() -> Value {
    json!({"schema":"yilong.quant.native_grid_issue.v1","environment":"native_paper",
    "scopes":["native_grid.read","native_grid.create"],"explicit_consent":true,"confirmation":"授权使用原生模拟网格"})
}
fn tls(f: &Fixture) -> Router {
    crate::node_endpoint_transport::asset_access::test_routes("https://main.example.test")
        .with_state(f.state.clone())
}
async fn send(
    router: &Router,
    method: &str,
    path: &str,
    tokens: &[&str],
    origin: Option<&str>,
    raw: String,
) -> (StatusCode, Value) {
    let mut request = Request::builder()
        .method(method)
        .uri(path)
        .header(header::CONTENT_TYPE, "application/json");
    for token in tokens {
        request = request.header(header::AUTHORIZATION, format!("Bearer {token}"));
    }
    if let Some(origin) = origin {
        request = request.header(header::ORIGIN, origin);
    }
    let response = router
        .clone()
        .oneshot(request.body(Body::from(raw)).unwrap())
        .await
        .unwrap();
    assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
    assert_eq!(response.headers()[header::REFERRER_POLICY], "no-referrer");
    assert!(!response
        .headers()
        .contains_key(header::ACCESS_CONTROL_ALLOW_ORIGIN));
    let status = response.status();
    let value =
        serde_json::from_slice(&to_bytes(response.into_body(), 8192).await.unwrap()).unwrap();
    (status, value)
}

#[tokio::test]
async fn quant_native_grid_http_issues_real_session_and_rejects_after_revocation() {
    let f = Fixture::new();
    let router = tls(&f);
    let _config = test_config::set(true);
    let (status, value) = send(
        &router,
        "POST",
        PATH,
        &[&f.user_token],
        None,
        body().to_string(),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(value["environment"], "native_paper");
    assert_eq!(value["expires_in"], 300);
    assert!(value["access_token"].as_str().unwrap().starts_with("yng1."));
    let (_, other) = send(
        &router,
        "POST",
        PATH,
        &[&f.other_token],
        None,
        body().to_string(),
    )
    .await;
    assert_ne!(value["subject_ref"], other["subject_ref"]);
    f.state
        .store
        .conn()
        .unwrap()
        .execute(
            "UPDATE sessions SET revoked_at='revoked' WHERE user_id=?1",
            [&f.user_id],
        )
        .unwrap();
    assert_eq!(
        send(
            &router,
            "POST",
            PATH,
            &[&f.user_token],
            None,
            body().to_string()
        )
        .await
        .0,
        StatusCode::UNAUTHORIZED
    );
    drop(router);
    f.cleanup();
}

#[tokio::test]
async fn quant_native_grid_http_static_duplicate_and_missing_auth_fail() {
    let f = Fixture::new();
    let router = tls(&f);
    let _config = test_config::set(true);
    for tokens in [
        vec![],
        vec!["unknown"],
        vec![f.state.admin_token.as_str()],
        vec![&f.user_token, &f.user_token],
    ] {
        assert_eq!(
            send(&router, "POST", PATH, &tokens, None, body().to_string())
                .await
                .0,
            StatusCode::UNAUTHORIZED
        );
    }
    if let Some(owner) = &f.state.owner_token {
        assert_eq!(
            send(&router, "POST", PATH, &[owner], None, body().to_string())
                .await
                .0,
            StatusCode::UNAUTHORIZED
        );
    }
    drop(router);
    f.cleanup();
}

#[tokio::test]
async fn quant_native_grid_http_configuration_and_strict_boundaries() {
    let f = Fixture::new();
    let router = tls(&f);
    let _config = test_config::set(false);
    let (status, readiness) =
        send(&router, "GET", PATH, &[&f.user_token], None, String::new()).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(readiness["enabled"], false);
    assert_eq!(
        send(
            &router,
            "POST",
            PATH,
            &[&f.user_token],
            None,
            body().to_string()
        )
        .await
        .0,
        StatusCode::SERVICE_UNAVAILABLE
    );
    assert_eq!(
        send(
            &router,
            "POST",
            PATH,
            &["unknown"],
            None,
            body().to_string()
        )
        .await
        .0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        send(
            &router,
            "POST",
            &format!("{PATH}?owner=other"),
            &[&f.user_token],
            None,
            body().to_string()
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        send(
            &router,
            "POST",
            PATH,
            &[&f.user_token],
            Some("https://other.example.test"),
            body().to_string()
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    for raw in [
        "{".into(),
        " ".repeat(4097),
        body()
            .to_string()
            .replacen('{', "{\"environment\":\"live\",", 1),
        body()
            .to_string()
            .replace("native_grid.create", "native_grid.simulate"),
    ] {
        assert_eq!(
            send(&router, "POST", PATH, &[&f.user_token], None, raw)
                .await
                .0,
            StatusCode::BAD_REQUEST
        );
    }
    let plain = crate::router::quant_native_grid_access::routes().with_state(f.state.clone());
    assert_eq!(
        send(
            &plain,
            "POST",
            PATH,
            &[&f.user_token],
            None,
            body().to_string()
        )
        .await
        .0,
        StatusCode::UPGRADE_REQUIRED
    );
    drop(plain);
    drop(router);
    f.cleanup();
}
