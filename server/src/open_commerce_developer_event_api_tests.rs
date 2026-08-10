use axum::{
    body::{to_bytes, Body},
    http::{header, Request, StatusCode},
    Router,
};
use serde_json::Value;
use std::sync::Arc;
use tower::ServiceExt;

use super::{fixture, invoke_platform, invoke_sandbox};
use crate::{
    open_commerce_developer_event_api::routes,
    open_commerce_developer_production_test_support::test_app_state,
};

#[tokio::test]
async fn event_feed_http_is_credential_scoped_cursor_safe_and_redacted() {
    let fixture = fixture();
    invoke_sandbox(&fixture, &fixture.first, "http-events-first").await;
    invoke_sandbox(&fixture, &fixture.second, "http-events-second-app").await;
    invoke_platform(&fixture, &fixture.first, "http-events-platform-hidden").await;
    invoke_sandbox(&fixture, &fixture.first, "http-events-last").await;

    let first_token = fixture.first.test_token.clone();
    let second_token = fixture.second.test_token.clone();
    let first_app_id = fixture.first.app.app_id.clone();
    let root = std::env::temp_dir();
    let state = Arc::new(test_app_state(fixture.store, &root));
    let router = routes().with_state(state);

    assert_eq!(
        get(&router, "/api/open-commerce/developer/events", None)
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        get(
            &router,
            "/api/open-commerce/developer/events",
            Some("oc_test_unknown"),
        )
        .await
        .0,
        StatusCode::UNAUTHORIZED
    );

    let (first_status, first_page) = get(
        &router,
        "/api/open-commerce/developer/events?limit=1",
        Some(&first_token),
    )
    .await;
    assert_eq!(first_status, StatusCode::OK, "{first_page}");
    assert_eq!(first_page["app_id"], first_app_id);
    assert_eq!(first_page["credential_environment"], "sandbox");
    assert_eq!(first_page["events"].as_array().unwrap().len(), 1);
    assert_eq!(
        first_page["events"][0]["idempotency_key"],
        "http-events-first"
    );
    assert_eq!(first_page["has_more"], true);
    assert_redacted(&first_page, &[&first_token, &second_token]);
    let cursor = first_page["next_cursor"].as_str().unwrap();

    let (next_status, next_page) = get(
        &router,
        &format!("/api/open-commerce/developer/events?cursor={cursor}"),
        Some(&first_token),
    )
    .await;
    assert_eq!(next_status, StatusCode::OK, "{next_page}");
    assert_eq!(next_page["events"].as_array().unwrap().len(), 1);
    assert_eq!(
        next_page["events"][0]["idempotency_key"],
        "http-events-last"
    );
    assert_eq!(next_page["has_more"], false);
    assert!(!next_page
        .to_string()
        .contains("http-events-platform-hidden"));

    assert_eq!(
        get(
            &router,
            &format!("/api/open-commerce/developer/events?cursor={cursor}"),
            Some(&second_token),
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        get(
            &router,
            "/api/open-commerce/developer/events?cursor=not-a-cursor",
            Some(&first_token),
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );

    let (second_status, second_page) = get(
        &router,
        "/api/open-commerce/developer/events",
        Some(&second_token),
    )
    .await;
    assert_eq!(second_status, StatusCode::OK, "{second_page}");
    assert_eq!(second_page["events"].as_array().unwrap().len(), 1);
    assert_eq!(
        second_page["events"][0]["idempotency_key"],
        "http-events-second-app"
    );
}

#[tokio::test]
async fn event_detail_http_hides_cross_app_and_platform_invocations() {
    let fixture = fixture();
    let first = invoke_sandbox(&fixture, &fixture.first, "http-detail-first").await;
    let second = invoke_sandbox(&fixture, &fixture.second, "http-detail-second").await;
    let platform = invoke_platform(&fixture, &fixture.first, "http-detail-platform").await;
    let first_token = fixture.first.test_token.clone();
    let second_token = fixture.second.test_token.clone();
    let first_id = first["invocation_id"].as_str().unwrap().to_string();
    let second_id = second["invocation_id"].as_str().unwrap().to_string();
    let platform_id = platform["invocation_id"].as_str().unwrap().to_string();
    let root = std::env::temp_dir();
    let state = Arc::new(test_app_state(fixture.store, &root));
    let router = routes().with_state(state);
    let first_path = format!("/api/open-commerce/developer/events/{first_id}");

    assert_eq!(
        get(&router, &first_path, None).await.0,
        StatusCode::UNAUTHORIZED
    );
    let (detail_status, detail) = get(&router, &first_path, Some(&first_token)).await;
    assert_eq!(detail_status, StatusCode::OK, "{detail}");
    assert_eq!(detail["event"]["invocation_id"], first_id);
    assert_eq!(detail["event"]["credential_environment"], "sandbox");
    assert_eq!(detail["result"]["items"][0], "拿铁");
    assert_redacted(&detail, &[&first_token, &second_token]);

    for (token, invocation_id) in [
        (&second_token, first_id.as_str()),
        (&first_token, second_id.as_str()),
        (&first_token, platform_id.as_str()),
        (&first_token, "invocation-does-not-exist"),
    ] {
        let path = format!("/api/open-commerce/developer/events/{invocation_id}");
        let (status, body) = get(&router, &path, Some(token)).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{body}");
        assert_redacted(&body, &[&first_token, &second_token]);
    }
}

fn assert_redacted(value: &Value, secrets: &[&str]) {
    let serialized = value.to_string();
    for field in [
        "request_hash",
        "request_shape",
        "grant_id",
        "requester_user_id",
        "project_id",
        "test_token",
        "live_token",
    ] {
        assert!(!serialized.contains(field), "leaked field: {field}");
    }
    for secret in secrets {
        assert!(!serialized.contains(secret), "leaked credential");
    }
}

async fn get(router: &Router, path: &str, bearer: Option<&str>) -> (StatusCode, Value) {
    let mut builder = Request::builder().uri(path);
    if let Some(token) = bearer {
        builder = builder.header(header::AUTHORIZATION, format!("Bearer {token}"));
    }
    let response = router
        .clone()
        .oneshot(builder.body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, body)
}
