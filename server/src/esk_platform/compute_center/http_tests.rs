use super::{request, Fixture};
use axum::{
    body::{to_bytes, Body},
    http::{header, Request, StatusCode},
    Router,
};
use serde_json::{json, Value};
use std::sync::Arc;
use tower::ServiceExt;

fn secure(f: &Fixture) -> Router {
    crate::node_endpoint_transport::asset_access::test_routes("https://main.example.test")
        .merge(
            crate::node_endpoint_transport::asset_access::browser_routes(
                "https://main.example.test",
                &f.state.data_dir,
            ),
        )
        .with_state(Arc::clone(&f.state))
}

#[tokio::test]
async fn compute_center_requires_verified_transport_and_real_sessions() {
    let f = Fixture::new();
    let router = secure(&f);
    let path = "/api/me/esk-compute-center";
    let (status, _) = request(&f.router, "GET", path, Some(&f.user_token), Value::Null).await;
    assert_eq!(status, StatusCode::UPGRADE_REQUIRED);
    for token in [
        None,
        Some("synthetic-static-owner-not-a-session"),
        Some(f.state.admin_token.as_str()),
    ] {
        assert_eq!(
            request(&router, "GET", path, token, Value::Null).await.0,
            StatusCode::UNAUTHORIZED
        );
    }
    let (status, value) = request(&router, "GET", path, Some(&f.user_token), Value::Null).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(value["schema"], "yilong.esk.compute_center.v1");
    assert_eq!(value["asset"]["total_base_units"], "0");
    assert_eq!(value["billing"]["currency"], "CNY");
    assert_eq!(value["capabilities"]["esk_service_spending"], false);
    assert_eq!(value["capabilities"]["purchase"], false);
    assert!(value["esk_service_spent_base_units"].is_null());
    for suffix in ["?page=0", "?page=1001", "?user_id=other", "?page=1&page=2"] {
        assert_eq!(
            request(
                &router,
                "GET",
                &format!("{path}{suffix}"),
                Some(&f.user_token),
                Value::Null
            )
            .await
            .0,
            StatusCode::BAD_REQUEST
        );
    }
    f.cleanup();
}

#[tokio::test]
async fn compute_center_secure_browser_login_is_independent_of_game_policy() {
    let f = Fixture::new();
    let router = secure(&f);
    let body = json!({"account":"holder@example.test","password":"secret1"}).to_string();
    for (origin, status) in [
        (None, StatusCode::FORBIDDEN),
        (Some("https://evil.example.test"), StatusCode::FORBIDDEN),
        (Some("https://main.example.test"), StatusCode::OK),
    ] {
        let mut builder = Request::builder()
            .method("POST")
            .uri("/api/esk-compute-center/login")
            .header(header::CONTENT_TYPE, "application/json");
        if let Some(origin) = origin {
            builder = builder.header(header::ORIGIN, origin);
        }
        let response = router
            .clone()
            .oneshot(builder.body(Body::from(body.clone())).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), status);
        assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
        let data: Value =
            serde_json::from_slice(&to_bytes(response.into_body(), 4096).await.unwrap()).unwrap();
        if status == StatusCode::OK {
            let token = data["token"].as_str().unwrap();
            assert_eq!(
                request(
                    &router,
                    "GET",
                    "/api/me/esk-compute-center",
                    Some(token),
                    Value::Null
                )
                .await
                .0,
                StatusCode::OK
            );
        } else {
            assert!(data.get("token").is_none());
        }
    }
    f.cleanup();
}
