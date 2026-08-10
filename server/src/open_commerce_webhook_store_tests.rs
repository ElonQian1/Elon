#[path = "open_commerce_webhook_api_tests.rs"]
mod webhook_api_tests;
#[path = "open_commerce_webhook_dead_letter_api_tests.rs"]
mod webhook_dead_letter_api_tests;

use serde_json::{json, Value};
use uuid::Uuid;

use crate::{
    open_commerce_developer_credential_model::AuthenticatedDeveloperCredential,
    open_commerce_developer_model::{
        CreateDeveloperAppRequest, OpenCommerceDeveloperAppCredential,
    },
    open_commerce_directory_service, open_commerce_invocation_service,
    open_commerce_model::{
        CreateCapabilityRequest, CreateMerchantRequest, InvokeCapabilityRequest, ACCESS_PUBLIC,
        HANDLER_STATIC_JSON,
    },
    open_commerce_service::{self, OpenCommerceActor},
    store::Store,
};

struct Fixture {
    store: Store,
    project_id: String,
    merchant_id: String,
    first: OpenCommerceDeveloperAppCredential,
    second: OpenCommerceDeveloperAppCredential,
}

fn fixture() -> Fixture {
    let path = std::env::temp_dir().join(format!(
        "elon_open_commerce_webhook_store_{}.db",
        Uuid::new_v4().simple()
    ));
    let store = Store::open(&path).expect("webhook store test database should open");
    let owner = store
        .create_user(
            "webhook-store@example.com",
            "secret1",
            Some("Webhook Store"),
            None,
        )
        .unwrap();
    let project = store
        .create_project(&owner.id, "Webhook Store", None, None)
        .unwrap()
        .project;
    let actor = OpenCommerceActor {
        user_id: &owner.id,
        app_id: "pc-web",
        project_role: Some("owner"),
    };
    let merchant = open_commerce_service::create_merchant(
        &store,
        &project.id,
        &actor,
        CreateMerchantRequest {
            display_name: "Webhook 咖啡店".to_string(),
            slug: Some("webhook-store-cafe".to_string()),
            description: "Webhook 本地持久化测试".to_string(),
            node_mode: "platform_hosted".to_string(),
            public_profile: json!({"category":"cafe"}),
        },
    )
    .unwrap();
    open_commerce_service::publish_capability(
        &store,
        &project.id,
        &merchant.id,
        &actor,
        CreateCapabilityRequest {
            capability_key: "menu.preview".to_string(),
            display_name: "菜单预览".to_string(),
            description: "返回稳定沙箱菜单".to_string(),
            kind: "query".to_string(),
            access_level: ACCESS_PUBLIC.to_string(),
            input_schema: json!({}),
            output_schema: json!({}),
            handler_type: HANDLER_STATIC_JSON.to_string(),
            handler_config: Some(json!({"response":{"items":["拿铁"]}})),
            unit_price_micros: 10_000,
            currency: "CNY".to_string(),
            freshness_seconds: 60,
        },
    )
    .unwrap();
    open_commerce_directory_service::set_publication(
        &store,
        &project.id,
        &merchant.id,
        &actor,
        true,
    )
    .unwrap();
    let first = developer_app(&store, &project.id, &owner.id, "consumer.webhook.one");
    let second = developer_app(&store, &project.id, &owner.id, "consumer.webhook.two");
    Fixture {
        store,
        project_id: project.id,
        merchant_id: merchant.id,
        first,
        second,
    }
}

fn developer_app(
    store: &Store,
    project_id: &str,
    owner_user_id: &str,
    app_id: &str,
) -> OpenCommerceDeveloperAppCredential {
    store
        .create_open_commerce_developer_app(
            project_id,
            owner_user_id,
            CreateDeveloperAppRequest {
                app_id: app_id.to_string(),
                display_name: app_id.to_string(),
            },
        )
        .unwrap()
}

fn create_subscription(
    fixture: &Fixture,
    deliver_on_succeeded: bool,
    deliver_on_failed: bool,
) -> crate::open_commerce_webhook_model::DeveloperWebhookSubscription {
    fixture
        .store
        .create_open_commerce_developer_webhook(
            &fixture.first.app,
            "https://webhook.example.test/open-commerce",
            "test-master-key",
            "sandbox",
            deliver_on_succeeded,
            deliver_on_failed,
        )
        .unwrap()
}

async fn invoke_sandbox(
    fixture: &Fixture,
    app: &OpenCommerceDeveloperAppCredential,
    idempotency_key: &str,
) -> Value {
    let actor = OpenCommerceActor {
        user_id: &app.app.owner_user_id,
        app_id: &app.app.app_id,
        project_role: None,
    };
    open_commerce_invocation_service::invoke_with_developer_credential(
        &fixture.store,
        &AuthenticatedDeveloperCredential::sandbox(app.app.clone()),
        &actor,
        InvokeCapabilityRequest {
            merchant_id: fixture.merchant_id.clone(),
            capability_key: "menu.preview".to_string(),
            requester_app_id: app.app.app_id.clone(),
            grant_id: None,
            idempotency_key: idempotency_key.to_string(),
            input: json!({}),
        },
        None,
    )
    .await
    .unwrap()
}

#[tokio::test]
async fn verified_subscription_enqueues_only_matching_app_and_event_filter() {
    let fixture = fixture();
    invoke_sandbox(&fixture, &fixture.first, "before-subscription").await;

    let succeeded = create_subscription(&fixture, true, false);
    let failed = create_subscription(&fixture, false, true);
    assert_eq!(succeeded.status, "disabled");
    assert_eq!(succeeded.verification_status, "pending");
    assert!(fixture
        .store
        .set_open_commerce_developer_webhook_enabled(
            &fixture.project_id,
            &fixture.first.app.id,
            &succeeded.id,
            true,
        )
        .unwrap_err()
        .to_string()
        .contains("尚未验证"));

    let succeeded = fixture
        .store
        .verify_open_commerce_developer_webhook(
            &fixture.project_id,
            &fixture.first.app.id,
            &succeeded.id,
        )
        .unwrap();
    fixture
        .store
        .verify_open_commerce_developer_webhook(
            &fixture.project_id,
            &fixture.first.app.id,
            &failed.id,
        )
        .unwrap();
    assert_eq!(succeeded.status, "active");
    assert_eq!(succeeded.verification_status, "verified");

    let invocation = invoke_sandbox(&fixture, &fixture.first, "matching-success").await;
    invoke_sandbox(&fixture, &fixture.second, "other-app-success").await;

    let succeeded_deliveries = fixture
        .store
        .list_open_commerce_developer_webhook_deliveries(
            &fixture.project_id,
            &fixture.first.app.id,
            &succeeded.id,
            50,
        )
        .unwrap();
    assert_eq!(succeeded_deliveries.len(), 1);
    assert_eq!(succeeded_deliveries[0].event_type, "invocation.succeeded");
    assert_eq!(succeeded_deliveries[0].enqueue_source, "live");
    assert_eq!(
        succeeded_deliveries[0].invocation_id,
        invocation["invocation_id"].as_str().unwrap()
    );
    assert!(fixture
        .store
        .list_open_commerce_developer_webhook_deliveries(
            &fixture.project_id,
            &fixture.first.app.id,
            &failed.id,
            50,
        )
        .unwrap()
        .is_empty());
    assert!(fixture
        .store
        .open_commerce_developer_webhook_for_app(
            &fixture.project_id,
            &fixture.second.app.id,
            &succeeded.id,
        )
        .is_err());
}

#[tokio::test]
async fn delivery_lease_dead_letter_acknowledgement_and_retry_are_fail_closed() {
    let fixture = fixture();
    let subscription = create_subscription(&fixture, true, false);
    let subscription = fixture
        .store
        .verify_open_commerce_developer_webhook(
            &fixture.project_id,
            &fixture.first.app.id,
            &subscription.id,
        )
        .unwrap();
    invoke_sandbox(&fixture, &fixture.first, "delivery-lifecycle").await;

    let claim = fixture
        .store
        .claim_open_commerce_developer_webhook_delivery("worker-a")
        .unwrap()
        .expect("delivery should be claimable");
    assert_eq!(claim.delivery.status, "delivering");
    assert_eq!(claim.delivery.attempt_count, 1);
    assert!(fixture
        .store
        .claim_open_commerce_developer_webhook_delivery("worker-b")
        .unwrap()
        .is_none());
    let mut forged_claim = claim.clone();
    forged_claim.lease_owner = "worker-b".to_string();
    assert!(fixture
        .store
        .complete_open_commerce_developer_webhook_delivery(&forged_claim, 204)
        .is_err());

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
    let health = fixture
        .store
        .open_commerce_developer_webhook_environment_health(
            &fixture.project_id,
            &fixture.first.app.id,
        )
        .unwrap();
    let sandbox = health
        .iter()
        .find(|item| item.environment == "sandbox")
        .unwrap();
    assert_eq!(sandbox.unresolved_dead_delivery_count, 1);
    assert_eq!(sandbox.acknowledged_dead_delivery_count, 0);

    let reason = "已人工确认上游临时故障";
    let acknowledged = fixture
        .store
        .acknowledge_open_commerce_developer_webhook_dead_letter(
            &fixture.project_id,
            &fixture.first.app.id,
            &subscription.id,
            &claim.delivery.id,
            &fixture.first.app.owner_user_id,
            reason,
        )
        .unwrap();
    assert_eq!(acknowledged.status, "dead");
    assert_eq!(
        acknowledged.dead_letter_acknowledgement_reason.as_deref(),
        Some(reason)
    );
    let idempotent = fixture
        .store
        .acknowledge_open_commerce_developer_webhook_dead_letter(
            &fixture.project_id,
            &fixture.first.app.id,
            &subscription.id,
            &claim.delivery.id,
            &fixture.first.app.owner_user_id,
            reason,
        )
        .unwrap();
    assert_eq!(
        idempotent.dead_letter_acknowledged_at,
        acknowledged.dead_letter_acknowledged_at
    );
    assert!(fixture
        .store
        .acknowledge_open_commerce_developer_webhook_dead_letter(
            &fixture.project_id,
            &fixture.first.app.id,
            &subscription.id,
            &claim.delivery.id,
            &fixture.first.app.owner_user_id,
            "尝试覆盖既有证据",
        )
        .is_err());

    let retried = fixture
        .store
        .retry_open_commerce_developer_webhook_delivery(
            &fixture.project_id,
            &fixture.first.app.id,
            &subscription.id,
            &claim.delivery.id,
        )
        .unwrap();
    assert_eq!(retried.status, "pending");
    assert_eq!(retried.manual_retry_count, 1);
    assert!(retried.dead_letter_acknowledged_at.is_none());

    let retry_claim = fixture
        .store
        .claim_open_commerce_developer_webhook_delivery("worker-b")
        .unwrap()
        .expect("manual retry should be claimable");
    fixture
        .store
        .complete_open_commerce_developer_webhook_delivery(&retry_claim, 204)
        .unwrap();
    assert!(fixture
        .store
        .complete_open_commerce_developer_webhook_delivery(&retry_claim, 204)
        .is_err());
    let deliveries = fixture
        .store
        .list_open_commerce_developer_webhook_deliveries(
            &fixture.project_id,
            &fixture.first.app.id,
            &subscription.id,
            50,
        )
        .unwrap();
    assert_eq!(deliveries.len(), 1);
    assert_eq!(deliveries[0].status, "delivered");
    assert_eq!(deliveries[0].response_status, Some(204));
    assert_eq!(deliveries[0].manual_retry_count, 1);
    assert_eq!(deliveries[0].attempt_count, 1);
}
