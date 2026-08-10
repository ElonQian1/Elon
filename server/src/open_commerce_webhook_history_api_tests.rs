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
    store::Store, types::AppState,
};

struct HistoryApiFixture {
    state: Arc<AppState>,
    router: Router,
    project_id: String,
    app_record_id: String,
    second_app_record_id: String,
    subscription_id: String,
    owner_token: String,
    other_editor_token: String,
}

#[tokio::test]
async fn history_replay_rejects_unauthorized_invalid_and_cross_app_requests() {
    let fixture = history_fixture().await;
    let path = fixture.replay_path(&fixture.app_record_id, &fixture.subscription_id);

    assert_eq!(
        call(&fixture.router, &path, None, replay_request(0, 2))
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        call(
            &fixture.router,
            &path,
            Some(&fixture.other_editor_token),
            replay_request(0, 2),
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    for request in [
        replay_request(-1, 2),
        replay_request(0, 0),
        replay_request(0, 101),
    ] {
        assert_eq!(
            call(&fixture.router, &path, Some(&fixture.owner_token), request,)
                .await
                .0,
            StatusCode::BAD_REQUEST
        );
    }
    let cross_app_path =
        fixture.replay_path(&fixture.second_app_record_id, &fixture.subscription_id);
    assert_eq!(
        call(
            &fixture.router,
            &cross_app_path,
            Some(&fixture.owner_token),
            replay_request(0, 2),
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    assert!(stored_deliveries(&fixture).is_empty());
}

#[tokio::test]
async fn history_replay_pages_and_deduplicates_matching_terminal_events() {
    let fixture = history_fixture().await;
    let path = fixture.replay_path(&fixture.app_record_id, &fixture.subscription_id);

    let (first_status, first) = call(
        &fixture.router,
        &path,
        Some(&fixture.owner_token),
        replay_request(0, 2),
    )
    .await;
    assert_eq!(first_status, StatusCode::OK, "{first}");
    assert_eq!(
        first["schema"],
        "open_commerce.developer_webhook_history_replay.v1"
    );
    assert_eq!(first["eligible_count"], 2);
    assert_eq!(first["enqueued_count"], 2);
    assert_eq!(first["already_present_count"], 0);
    assert_eq!(first["has_more"], true);
    let cursor = first["processed_through_sequence"].as_i64().unwrap();
    assert!(cursor > 0);
    assert_redacted(&first);

    let (repeat_status, repeated) = call(
        &fixture.router,
        &path,
        Some(&fixture.owner_token),
        replay_request(0, 2),
    )
    .await;
    assert_eq!(repeat_status, StatusCode::OK, "{repeated}");
    assert_eq!(repeated["processed_through_sequence"], cursor);
    assert_eq!(repeated["eligible_count"], 2);
    assert_eq!(repeated["enqueued_count"], 0);
    assert_eq!(repeated["already_present_count"], 2);
    assert_eq!(stored_deliveries(&fixture).len(), 2);

    let (next_status, next) = call(
        &fixture.router,
        &path,
        Some(&fixture.owner_token),
        replay_request(cursor, 100),
    )
    .await;
    assert_eq!(next_status, StatusCode::OK, "{next}");
    assert_eq!(next["eligible_count"], 1);
    assert_eq!(next["enqueued_count"], 1);
    assert_eq!(next["already_present_count"], 0);
    assert_eq!(next["has_more"], false);
    assert!(next["processed_through_sequence"].as_i64().unwrap() > cursor);

    let deliveries = stored_deliveries(&fixture);
    assert_eq!(deliveries.len(), 3);
    assert!(deliveries.iter().all(|delivery| {
        delivery.enqueue_source == "history_replay"
            && delivery.history_replay_requested_at.is_some()
            && delivery.event_type == "invocation.succeeded"
    }));
}

impl HistoryApiFixture {
    fn replay_path(&self, app_record_id: &str, subscription_id: &str) -> String {
        format!(
            "/api/projects/{}/open-commerce/developer-apps/{app_record_id}/webhooks/{subscription_id}/replay-history",
            self.project_id
        )
    }
}

fn replay_request(after_sequence: i64, limit: usize) -> Value {
    json!({"after_sequence": after_sequence, "limit": limit})
}

fn stored_deliveries(
    fixture: &HistoryApiFixture,
) -> Vec<crate::open_commerce_webhook_model::DeveloperWebhookDelivery> {
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
    path: &str,
    bearer: Option<&str>,
    request: Value,
) -> (StatusCode, Value) {
    let mut builder = Request::builder()
        .method(Method::POST)
        .uri(path)
        .header(header::CONTENT_TYPE, "application/json");
    if let Some(token) = bearer {
        builder = builder.header(header::AUTHORIZATION, format!("Bearer {token}"));
    }
    let response = router
        .clone()
        .oneshot(builder.body(Body::from(request.to_string())).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, body)
}

async fn history_fixture() -> HistoryApiFixture {
    let fixture = fixture();
    let other_editor = fixture
        .store
        .create_user("webhook-history-editor@example.com", "secret1", None, None)
        .unwrap();
    fixture
        .store
        .add_project_member_by_account(
            &fixture.project_id,
            "webhook-history-editor@example.com",
            "editor",
        )
        .unwrap();

    for key in ["history-one", "history-two", "history-three"] {
        invoke_sandbox(&fixture, &fixture.first, key).await;
    }
    invoke_sandbox(&fixture, &fixture.second, "history-other-app").await;
    let subscription = create_subscription(&fixture, true, false);
    let subscription = fixture
        .store
        .verify_open_commerce_developer_webhook(
            &fixture.project_id,
            &fixture.first.app.id,
            &subscription.id,
        )
        .unwrap();

    let project_id = fixture.project_id;
    let app_record_id = fixture.first.app.id;
    let second_app_record_id = fixture.second.app.id;
    let owner_token = session(&fixture.store, &fixture.first.app.owner_user_id);
    let other_editor_token = session(&fixture.store, &other_editor.id);
    let state = Arc::new(test_app_state(fixture.store, &std::env::temp_dir()));
    let router = open_commerce_webhook_api::routes().with_state(state.clone());

    HistoryApiFixture {
        state,
        router,
        project_id,
        app_record_id,
        second_app_record_id,
        subscription_id: subscription.id,
        owner_token,
        other_editor_token,
    }
}

fn session(store: &Store, user_id: &str) -> String {
    store
        .create_session(user_id, Some("webhook-history-api-test"), None)
        .unwrap()
        .0
}
