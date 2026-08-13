use std::sync::Arc;

use axum::{
    body::{to_bytes, Body},
    http::{header, Method, Request, StatusCode},
    Router,
};
use serde_json::{json, Value};
use tower::ServiceExt;
use uuid::Uuid;

use super::routes;
use crate::{
    open_commerce_capability_source_service::test_support,
    open_commerce_developer_production_test_support::test_app_state, store::Store,
};

struct RouteFixture {
    state: Arc<crate::types::AppState>,
    router: Router,
    project_id: String,
    capability_id: String,
    integration_id: String,
    receipt_id: String,
    owner_token: String,
    viewer_token: String,
    outsider_token: String,
}

impl RouteFixture {
    fn path(&self) -> String {
        format!(
            "/api/projects/{}/open-commerce/capabilities/{}/source-link",
            self.project_id, self.capability_id
        )
    }

    fn payload(&self) -> Value {
        json!({
            "integration_id":self.integration_id,
            "sync_receipt_id":self.receipt_id,
            "data_domain":"catalog"
        })
    }
}

#[tokio::test]
async fn source_link_http_rejects_unauthenticated_outsider_and_viewer_without_writes() {
    let fixture = route_fixture();
    let before = source_state(&fixture.state.store, &fixture.project_id);

    for (token, expected) in [
        (None, StatusCode::UNAUTHORIZED),
        (Some(fixture.outsider_token.as_str()), StatusCode::FORBIDDEN),
        (Some(fixture.viewer_token.as_str()), StatusCode::FORBIDDEN),
    ] {
        let (status, _) = send(
            &fixture.router,
            Method::PUT,
            &fixture.path(),
            token,
            fixture.payload(),
        )
        .await;
        assert_eq!(status, expected);
    }

    assert_eq!(
        before,
        source_state(&fixture.state.store, &fixture.project_id)
    );
}

#[tokio::test]
async fn source_link_http_put_and_delete_are_audited_and_delete_is_idempotent() {
    let fixture = route_fixture();
    let path = fixture.path();
    let (status, linked) = send(
        &fixture.router,
        Method::PUT,
        &path,
        Some(&fixture.owner_token),
        fixture.payload(),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{linked}");
    assert_eq!(linked["project_id"], fixture.project_id);
    assert_eq!(linked["capability_id"], fixture.capability_id);
    assert_eq!(linked["integration_id"], fixture.integration_id);
    assert_eq!(linked["sync_receipt_id"], fixture.receipt_id);
    assert_eq!(linked["data_domain"], "catalog");
    assert_eq!(linked["provider_key"], "merchant_erp");
    assert_eq!(linked["publishable"], true);
    assert!(!linked.to_string().contains("cursor-secret"));
    assert_eq!(
        source_state(&fixture.state.store, &fixture.project_id),
        (1, 1)
    );

    let (status, removed) = send(
        &fixture.router,
        Method::DELETE,
        &path,
        Some(&fixture.owner_token),
        Value::Null,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{removed}");
    assert_eq!(removed["removed"], true);
    assert_eq!(
        source_state(&fixture.state.store, &fixture.project_id),
        (0, 2)
    );

    let (status, replayed) = send(
        &fixture.router,
        Method::DELETE,
        &path,
        Some(&fixture.owner_token),
        Value::Null,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{replayed}");
    assert_eq!(replayed["removed"], false);
    assert_eq!(
        source_state(&fixture.state.store, &fixture.project_id),
        (0, 2)
    );
}

async fn send(
    router: &Router,
    method: Method,
    path: &str,
    token: Option<&str>,
    body: Value,
) -> (StatusCode, Value) {
    let mut builder = Request::builder().method(method).uri(path);
    if let Some(token) = token {
        builder = builder.header(header::AUTHORIZATION, format!("Bearer {token}"));
    }
    let request = if body.is_null() {
        builder.body(Body::empty()).unwrap()
    } else {
        builder
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(body.to_string()))
            .unwrap()
    };
    let response = router.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, body)
}

fn route_fixture() -> RouteFixture {
    let fixture = test_support::fixture("http");
    let outsider = fixture
        .store
        .create_user(
            &format!(
                "source-http-outsider-{}@example.com",
                Uuid::new_v4().simple()
            ),
            "secret1",
            None,
            None,
        )
        .unwrap();
    let owner_token = session(&fixture.store, &fixture.owner_id);
    let viewer_token = session(&fixture.store, &fixture.viewer_id);
    let outsider_token = session(&fixture.store, &outsider.id);
    let root = std::env::temp_dir().join(format!(
        "elon-open-commerce-source-api-{}",
        Uuid::new_v4().simple()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let state = Arc::new(test_app_state(fixture.store, &root));
    let router = routes().with_state(Arc::clone(&state));
    RouteFixture {
        state,
        router,
        project_id: fixture.project_id,
        capability_id: fixture.capability_id,
        integration_id: fixture.integration_id,
        receipt_id: fixture.succeeded_receipt_id,
        owner_token,
        viewer_token,
        outsider_token,
    }
}

fn session(store: &Store, user_id: &str) -> String {
    store.create_session(user_id, Some("test"), None).unwrap().0
}

fn source_state(store: &Store, project_id: &str) -> (usize, usize) {
    let links = store
        .list_project_open_commerce_capability_source_links(project_id)
        .unwrap()
        .len();
    let audits = store
        .list_project_open_commerce_audit(project_id, 200)
        .unwrap()
        .into_iter()
        .filter(|event| event.action.starts_with("capability.source_"))
        .count();
    (links, audits)
}
