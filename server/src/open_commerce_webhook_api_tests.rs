use std::sync::Arc;

use axum::{
    body::{to_bytes, Body},
    http::{header, Method, Request, StatusCode},
    Router,
};
use serde_json::{json, Value};
use tower::ServiceExt;
use uuid::Uuid;

use crate::{
    open_commerce_developer_model::CreateDeveloperAppRequest,
    open_commerce_developer_production_test_support::test_app_state,
    open_commerce_webhook_api::routes, store::Store, types::AppState,
};

struct WebhookApiFixture {
    state: Arc<AppState>,
    router: Router,
    project_id: String,
    other_project_id: String,
    app_record_id: String,
    active_subscription_id: String,
    pending_subscription_id: String,
    production_subscription_id: String,
    owner_token: String,
    app_owner_token: String,
    other_editor_token: String,
    member_token: String,
    outsider_token: String,
    platform_admin_token: String,
}

#[tokio::test]
async fn webhook_reads_and_disable_require_exact_app_owner() {
    let fixture = fixture();
    let path = fixture.webhooks_path(&fixture.project_id);

    assert_eq!(
        call(&fixture.router, Method::GET, &path, None, None)
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
    for token in [
        &fixture.owner_token,
        &fixture.other_editor_token,
        &fixture.member_token,
        &fixture.outsider_token,
        &fixture.platform_admin_token,
    ] {
        let (status, body) = call(&fixture.router, Method::GET, &path, Some(token), None).await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{body}");
        assert_redacted(&body);
    }
    assert_eq!(
        call(
            &fixture.router,
            Method::GET,
            &fixture.webhooks_path(&fixture.other_project_id),
            Some(&fixture.app_owner_token),
            None,
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );

    let (list_status, listed) = call(
        &fixture.router,
        Method::GET,
        &path,
        Some(&fixture.app_owner_token),
        None,
    )
    .await;
    assert_eq!(list_status, StatusCode::OK, "{listed}");
    assert_eq!(listed["webhooks"].as_array().unwrap().len(), 3);
    assert_redacted(&listed);

    let health_path = format!(
        "/api/projects/{}/open-commerce/developer-apps/{}/webhook-health",
        fixture.project_id, fixture.app_record_id
    );
    let (health_status, health) = call(
        &fixture.router,
        Method::GET,
        &health_path,
        Some(&fixture.app_owner_token),
        None,
    )
    .await;
    assert_eq!(health_status, StatusCode::OK, "{health}");
    assert_redacted(&health);

    let deliveries_path = format!("{}/{}/deliveries", path, fixture.active_subscription_id);
    let (deliveries_status, deliveries) = call(
        &fixture.router,
        Method::GET,
        &deliveries_path,
        Some(&fixture.app_owner_token),
        None,
    )
    .await;
    assert_eq!(deliveries_status, StatusCode::OK, "{deliveries}");
    assert!(deliveries["deliveries"].as_array().unwrap().is_empty());

    let disable_path = format!("{}/{}/disable", path, fixture.active_subscription_id);
    assert_eq!(
        call(
            &fixture.router,
            Method::POST,
            &disable_path,
            Some(&fixture.other_editor_token),
            None,
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    let (disable_status, disabled) = call(
        &fixture.router,
        Method::POST,
        &disable_path,
        Some(&fixture.app_owner_token),
        None,
    )
    .await;
    assert_eq!(disable_status, StatusCode::OK, "{disabled}");
    assert_eq!(disabled["status"], "disabled");
    assert_redacted(&disabled);
    let (repeat_status, repeated) = call(
        &fixture.router,
        Method::POST,
        &disable_path,
        Some(&fixture.app_owner_token),
        None,
    )
    .await;
    assert_eq!(repeat_status, StatusCode::OK, "{repeated}");
    assert_eq!(repeated["status"], "disabled");
}

#[tokio::test]
async fn webhook_mutations_fail_closed_before_network_or_secret_exposure() {
    let fixture = fixture();
    let path = fixture.webhooks_path(&fixture.project_id);
    let create = json!({
        "callback_url": "http://public.example.test/open-commerce",
        "environment": "sandbox",
        "deliver_on_succeeded": true,
        "deliver_on_failed": true
    });

    assert_eq!(
        call(&fixture.router, Method::POST, &path, None, Some(&create))
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        call(
            &fixture.router,
            Method::POST,
            &path,
            Some(&fixture.owner_token),
            Some(&create),
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    let before = subscription_count(&fixture);
    let (create_status, create_error) = call(
        &fixture.router,
        Method::POST,
        &path,
        Some(&fixture.app_owner_token),
        Some(&create),
    )
    .await;
    assert_eq!(create_status, StatusCode::BAD_REQUEST, "{create_error}");
    assert_eq!(subscription_count(&fixture), before);
    assert_redacted(&create_error);

    let enable_path = format!("{}/{}/enable", path, fixture.pending_subscription_id);
    let (enable_status, enable_error) = call(
        &fixture.router,
        Method::POST,
        &enable_path,
        Some(&fixture.app_owner_token),
        None,
    )
    .await;
    assert_eq!(enable_status, StatusCode::BAD_REQUEST, "{enable_error}");
    assert_eq!(
        subscription(&fixture, &fixture.pending_subscription_id).status,
        "disabled"
    );

    let verify_path = format!("{}/{}/verify", path, fixture.pending_subscription_id);
    assert_eq!(
        call(
            &fixture.router,
            Method::POST,
            &verify_path,
            Some(&fixture.other_editor_token),
            None,
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );

    let rotate_path = format!(
        "{}/{}/rotate-secret",
        path, fixture.production_subscription_id
    );
    let before_rotation = subscription(&fixture, &fixture.production_subscription_id);
    let (rotate_status, rotate_error) = call(
        &fixture.router,
        Method::POST,
        &rotate_path,
        Some(&fixture.app_owner_token),
        None,
    )
    .await;
    assert_eq!(rotate_status, StatusCode::BAD_REQUEST, "{rotate_error}");
    let after_rotation = subscription(&fixture, &fixture.production_subscription_id);
    assert_eq!(
        after_rotation.signing_secret_version,
        before_rotation.signing_secret_version
    );
    assert_eq!(
        after_rotation.signing_key_id,
        before_rotation.signing_key_id
    );
    assert_redacted(&rotate_error);
}

impl WebhookApiFixture {
    fn webhooks_path(&self, project_id: &str) -> String {
        format!(
            "/api/projects/{project_id}/open-commerce/developer-apps/{}/webhooks",
            self.app_record_id
        )
    }
}

fn subscription(
    fixture: &WebhookApiFixture,
    subscription_id: &str,
) -> crate::open_commerce_webhook_model::DeveloperWebhookSubscription {
    fixture
        .state
        .store
        .open_commerce_developer_webhook_for_app(
            &fixture.project_id,
            &fixture.app_record_id,
            subscription_id,
        )
        .unwrap()
}

fn subscription_count(fixture: &WebhookApiFixture) -> usize {
    fixture
        .state
        .store
        .list_open_commerce_developer_webhooks(&fixture.project_id, &fixture.app_record_id)
        .unwrap()
        .len()
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
    body: Option<&Value>,
) -> (StatusCode, Value) {
    let mut builder = Request::builder().method(method).uri(path);
    if let Some(token) = bearer {
        builder = builder.header(header::AUTHORIZATION, format!("Bearer {token}"));
    }
    let body = if let Some(value) = body {
        builder = builder.header(header::CONTENT_TYPE, "application/json");
        Body::from(value.to_string())
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

fn fixture() -> WebhookApiFixture {
    let path = std::env::temp_dir().join(format!(
        "elon_open_commerce_webhook_api_{}.db",
        Uuid::new_v4().simple()
    ));
    let store = Store::open(&path).expect("webhook API test store should open");
    let owner = user(&store, "webhook-project-owner@example.com", None);
    let app_owner = user(&store, "webhook-app-owner@example.com", None);
    let other_editor = user(&store, "webhook-other-editor@example.com", None);
    let member = user(&store, "webhook-member@example.com", None);
    let outsider = user(&store, "webhook-outsider@example.com", None);
    let platform_admin = user(&store, "webhook-platform-admin@example.com", Some("admin"));
    let project = store
        .create_project(&owner.id, "Webhook API", None, None)
        .unwrap()
        .project;
    let other_project = store
        .create_project(&app_owner.id, "Other Webhook API", None, None)
        .unwrap()
        .project;
    for (account, role) in [
        ("webhook-app-owner@example.com", "editor"),
        ("webhook-other-editor@example.com", "editor"),
        ("webhook-member@example.com", "member"),
    ] {
        store
            .add_project_member_by_account(&project.id, account, role)
            .unwrap();
    }
    let app = store
        .create_open_commerce_developer_app(
            &project.id,
            &app_owner.id,
            CreateDeveloperAppRequest {
                app_id: "consumer.webhook.api".to_string(),
                display_name: "Webhook API Consumer".to_string(),
            },
        )
        .unwrap()
        .app;
    let active = store
        .create_open_commerce_developer_webhook(
            &app,
            "https://active.example.test/open-commerce",
            "test-master-key",
            "sandbox",
            true,
            true,
        )
        .unwrap();
    let active = store
        .verify_open_commerce_developer_webhook(&project.id, &app.id, &active.id)
        .unwrap();
    let pending = store
        .create_open_commerce_developer_webhook(
            &app,
            "https://pending.example.test/open-commerce",
            "test-master-key",
            "sandbox",
            true,
            false,
        )
        .unwrap();
    let production = store
        .create_open_commerce_developer_webhook(
            &app,
            "https://production.example.test/open-commerce",
            "test-master-key",
            "production",
            true,
            true,
        )
        .unwrap();
    let owner_token = session(&store, &owner.id);
    let app_owner_token = session(&store, &app_owner.id);
    let other_editor_token = session(&store, &other_editor.id);
    let member_token = session(&store, &member.id);
    let outsider_token = session(&store, &outsider.id);
    let platform_admin_token = session(&store, &platform_admin.id);
    let root = path.parent().unwrap().to_path_buf();
    let state = Arc::new(test_app_state(store, &root));
    let router = routes().with_state(state.clone());

    WebhookApiFixture {
        state,
        router,
        project_id: project.id,
        other_project_id: other_project.id,
        app_record_id: app.id,
        active_subscription_id: active.id,
        pending_subscription_id: pending.id,
        production_subscription_id: production.id,
        owner_token,
        app_owner_token,
        other_editor_token,
        member_token,
        outsider_token,
        platform_admin_token,
    }
}

fn user(store: &Store, email: &str, role: Option<&str>) -> crate::store::PublicUser {
    store.create_user(email, "secret1", None, role).unwrap()
}

fn session(store: &Store, user_id: &str) -> String {
    store
        .create_session(user_id, Some("webhook-api-test"), None)
        .unwrap()
        .0
}
