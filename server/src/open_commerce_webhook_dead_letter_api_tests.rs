use std::sync::Arc;

use axum::{
    body::{to_bytes, Body},
    http::{header, Method, Request, StatusCode},
    Router,
};
use serde_json::{json, Value};
use tower::ServiceExt;

use super::{create_subscription, fixture, invoke_sandbox};
use crate::{
    open_commerce_developer_production_test_support::test_app_state, open_commerce_webhook_api,
    open_commerce_webhook_dead_letter_api, store::Store, types::AppState,
};

struct DeadLetterApiFixture {
    state: Arc<AppState>,
    router: Router,
    project_id: String,
    app_record_id: String,
    subscription_id: String,
    other_subscription_id: String,
    delivery_id: String,
    owner_user_id: String,
    owner_token: String,
    other_editor_token: String,
}

#[tokio::test]
async fn dead_letter_acknowledgement_is_owner_scoped_and_non_overwritable() {
    let fixture = dead_letter_fixture().await;
    let path = fixture.acknowledge_path(&fixture.subscription_id);
    let reason = "已人工确认上游服务临时故障";

    assert_eq!(
        call(&fixture.router, Method::POST, &path, None, Some(reason))
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        call(
            &fixture.router,
            Method::POST,
            &path,
            Some(&fixture.other_editor_token),
            Some(reason),
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    assert!(stored_delivery(&fixture)
        .dead_letter_acknowledged_at
        .is_none());

    assert_eq!(
        call(
            &fixture.router,
            Method::POST,
            &path,
            Some(&fixture.owner_token),
            Some("bad"),
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    let cross_subscription_path = fixture.acknowledge_path(&fixture.other_subscription_id);
    assert_eq!(
        call(
            &fixture.router,
            Method::POST,
            &cross_subscription_path,
            Some(&fixture.owner_token),
            Some(reason),
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    assert!(stored_delivery(&fixture)
        .dead_letter_acknowledged_at
        .is_none());

    let (ack_status, acknowledged) = call(
        &fixture.router,
        Method::POST,
        &path,
        Some(&fixture.owner_token),
        Some(reason),
    )
    .await;
    assert_eq!(ack_status, StatusCode::OK, "{acknowledged}");
    assert_eq!(acknowledged["status"], "dead");
    assert_eq!(acknowledged["dead_letter_acknowledgement_reason"], reason);
    assert_redacted(&acknowledged);
    let acknowledged_at = acknowledged["dead_letter_acknowledged_at"]
        .as_str()
        .unwrap()
        .to_string();

    let (repeat_status, repeated) = call(
        &fixture.router,
        Method::POST,
        &path,
        Some(&fixture.owner_token),
        Some(reason),
    )
    .await;
    assert_eq!(repeat_status, StatusCode::OK, "{repeated}");
    assert_eq!(repeated["dead_letter_acknowledged_at"], acknowledged_at);
    assert_eq!(repeated["dead_letter_acknowledgement_reason"], reason);

    assert_eq!(
        call(
            &fixture.router,
            Method::POST,
            &path,
            Some(&fixture.owner_token),
            Some("尝试覆盖已经固定的确认理由"),
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    let stored = stored_delivery(&fixture);
    assert_eq!(
        stored.dead_letter_acknowledgement_reason.as_deref(),
        Some(reason)
    );
    assert_eq!(
        stored.dead_letter_acknowledged_at.as_deref(),
        Some(acknowledged_at.as_str())
    );
}

#[tokio::test]
async fn acknowledged_dead_letter_http_retry_clears_acknowledgement_once() {
    let fixture = dead_letter_fixture().await;
    let reason = "已确认回调端点临时不可用";
    fixture
        .state
        .store
        .acknowledge_open_commerce_developer_webhook_dead_letter(
            &fixture.project_id,
            &fixture.app_record_id,
            &fixture.subscription_id,
            &fixture.delivery_id,
            &fixture.owner_user_id,
            reason,
        )
        .unwrap();
    let retry_path = fixture.retry_path();

    assert_eq!(
        call(
            &fixture.router,
            Method::POST,
            &retry_path,
            Some(&fixture.other_editor_token),
            None,
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    let (retry_status, retried) = call(
        &fixture.router,
        Method::POST,
        &retry_path,
        Some(&fixture.owner_token),
        None,
    )
    .await;
    assert_eq!(retry_status, StatusCode::OK, "{retried}");
    assert_eq!(retried["status"], "pending");
    assert_eq!(retried["manual_retry_count"], 1);
    assert!(retried["dead_letter_acknowledged_at"].is_null());
    assert!(retried["dead_letter_acknowledgement_reason"].is_null());
    assert_redacted(&retried);

    assert_eq!(
        call(
            &fixture.router,
            Method::POST,
            &retry_path,
            Some(&fixture.owner_token),
            None,
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    let stored = stored_delivery(&fixture);
    assert_eq!(stored.status, "pending");
    assert_eq!(stored.manual_retry_count, 1);
    assert!(stored.dead_letter_acknowledged_at.is_none());
}

impl DeadLetterApiFixture {
    fn acknowledge_path(&self, subscription_id: &str) -> String {
        format!(
            "/api/projects/{}/open-commerce/developer-apps/{}/webhooks/{subscription_id}/deliveries/{}/acknowledge",
            self.project_id, self.app_record_id, self.delivery_id
        )
    }

    fn retry_path(&self) -> String {
        format!(
            "/api/projects/{}/open-commerce/developer-apps/{}/webhooks/{}/deliveries/{}/retry",
            self.project_id, self.app_record_id, self.subscription_id, self.delivery_id
        )
    }
}

fn stored_delivery(
    fixture: &DeadLetterApiFixture,
) -> crate::open_commerce_webhook_model::DeveloperWebhookDelivery {
    fixture
        .state
        .store
        .list_open_commerce_developer_webhook_deliveries(
            &fixture.project_id,
            &fixture.app_record_id,
            &fixture.subscription_id,
            50,
        )
        .unwrap()
        .into_iter()
        .find(|delivery| delivery.id == fixture.delivery_id)
        .unwrap()
}

fn assert_redacted(value: &Value) {
    let serialized = value.to_string();
    for field in [
        "\"signing_secret\":",
        "\"test_token\":",
        "\"live_token\":",
        "\"token_hash\":",
    ] {
        assert!(!serialized.contains(field), "leaked field: {field}");
    }
    assert!(!serialized.contains("whsec_"), "leaked signing secret");
}

async fn call(
    router: &Router,
    method: Method,
    path: &str,
    bearer: Option<&str>,
    reason: Option<&str>,
) -> (StatusCode, Value) {
    let mut builder = Request::builder().method(method).uri(path);
    if let Some(token) = bearer {
        builder = builder.header(header::AUTHORIZATION, format!("Bearer {token}"));
    }
    let body = if let Some(reason) = reason {
        builder = builder.header(header::CONTENT_TYPE, "application/json");
        Body::from(json!({"reason": reason}).to_string())
    } else {
        Body::empty()
    };
    let response = router
        .clone()
        .oneshot(builder.body(body).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, body)
}

async fn dead_letter_fixture() -> DeadLetterApiFixture {
    let fixture = fixture();
    let other_editor = user(&fixture.store, "webhook-dead-letter-editor@example.com");
    fixture
        .store
        .add_project_member_by_account(
            &fixture.project_id,
            "webhook-dead-letter-editor@example.com",
            "editor",
        )
        .unwrap();
    let subscription = create_subscription(&fixture, true, false);
    let subscription = fixture
        .store
        .verify_open_commerce_developer_webhook(
            &fixture.project_id,
            &fixture.first.app.id,
            &subscription.id,
        )
        .unwrap();
    let other_subscription = create_subscription(&fixture, true, false);
    invoke_sandbox(&fixture, &fixture.first, "dead-letter-api").await;
    let claim = fixture
        .store
        .claim_open_commerce_developer_webhook_delivery("dead-letter-worker")
        .unwrap()
        .expect("delivery should be claimable");
    fixture
        .store
        .fail_open_commerce_developer_webhook_delivery(
            &claim,
            Some(503),
            "upstream_unavailable",
            None,
            false,
        )
        .unwrap();
    let owner_token = session(&fixture.store, &fixture.first.app.owner_user_id);
    let other_editor_token = session(&fixture.store, &other_editor.id);
    let root = std::env::temp_dir();
    let state = Arc::new(test_app_state(fixture.store, &root));
    let router = open_commerce_webhook_api::routes()
        .merge(open_commerce_webhook_dead_letter_api::routes())
        .with_state(state.clone());

    DeadLetterApiFixture {
        state,
        router,
        project_id: fixture.project_id,
        app_record_id: fixture.first.app.id,
        subscription_id: subscription.id,
        other_subscription_id: other_subscription.id,
        delivery_id: claim.delivery.id,
        owner_user_id: fixture.first.app.owner_user_id,
        owner_token,
        other_editor_token,
    }
}

fn user(store: &Store, email: &str) -> crate::store::PublicUser {
    store.create_user(email, "secret1", None, None).unwrap()
}

fn session(store: &Store, user_id: &str) -> String {
    store
        .create_session(user_id, Some("webhook-dead-letter-api-test"), None)
        .unwrap()
        .0
}
