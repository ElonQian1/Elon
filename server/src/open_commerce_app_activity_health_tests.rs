use serde_json::json;
use uuid::Uuid;

use crate::{
    open_commerce_model::{CreateCapabilityRequest, CreateMerchantRequest, HANDLER_STATIC_JSON},
    open_commerce_service::{self, OpenCommerceActor},
    store::{OpenCommerceInvocationStart, Store},
};

struct Fixture {
    store: Store,
    project_id: String,
    merchant_id: String,
    capability_id: String,
    requester_user_id: String,
}

fn fixture() -> Fixture {
    let path = std::env::temp_dir().join(format!(
        "elon-open-commerce-app-health-{}.sqlite",
        Uuid::new_v4().simple()
    ));
    let store = Store::open(&path).unwrap();
    let owner = store
        .create_user("app-health-owner@example.com", "secret1", None, None)
        .unwrap();
    let project = store
        .create_project(&owner.id, "App health merchant", None, None)
        .unwrap()
        .project;
    let owner_actor = OpenCommerceActor {
        user_id: &owner.id,
        app_id: "pc-web",
        project_role: Some("owner"),
    };
    let merchant = open_commerce_service::create_merchant(
        &store,
        &project.id,
        &owner_actor,
        CreateMerchantRequest {
            display_name: "App 健康测试商户".to_string(),
            slug: Some("app-health-merchant".to_string()),
            description: String::new(),
            node_mode: "platform_hosted".to_string(),
            public_profile: json!({"category":"test"}),
        },
    )
    .unwrap();
    let capability = open_commerce_service::publish_capability(
        &store,
        &project.id,
        &merchant.id,
        &owner_actor,
        CreateCapabilityRequest {
            capability_key: "menu.lookup".to_string(),
            display_name: "查询菜单".to_string(),
            description: String::new(),
            kind: "query".to_string(),
            access_level: "public".to_string(),
            input_schema: json!({}),
            output_schema: json!({}),
            handler_type: HANDLER_STATIC_JSON.to_string(),
            handler_config: Some(json!({"response":{"items":[]}})),
            unit_price_micros: 0,
            currency: "CNY".to_string(),
            freshness_seconds: 0,
        },
    )
    .unwrap();
    let requester = store
        .create_user("app-health-requester@example.com", "secret1", None, None)
        .unwrap();
    Fixture {
        store,
        project_id: project.id,
        merchant_id: merchant.id,
        capability_id: capability.id,
        requester_user_id: requester.id,
    }
}

fn start_invocation(fixture: &Fixture, app_id: &str, key: &str) -> String {
    fixture
        .store
        .start_open_commerce_invocation(OpenCommerceInvocationStart {
            project_id: &fixture.project_id,
            merchant_id: &fixture.merchant_id,
            capability_id: &fixture.capability_id,
            capability_key: "menu.lookup",
            requester_user_id: &fixture.requester_user_id,
            requester_app_id: app_id,
            grant_id: None,
            idempotency_key: key,
            request_hash: key,
            request_shape: &json!({}),
            unit_price_micros: 0,
            currency: "CNY",
        })
        .unwrap()
        .invocation
        .id
}

fn finish_failure(fixture: &Fixture, app_id: &str, key: &str, error_code: &str) -> String {
    let invocation_id = start_invocation(fixture, app_id, key);
    fixture
        .store
        .finish_open_commerce_invocation_failure(&invocation_id, error_code)
        .unwrap();
    invocation_id
}

fn finish_success(fixture: &Fixture, app_id: &str, key: &str) {
    let invocation_id = start_invocation(fixture, app_id, key);
    fixture
        .store
        .finish_open_commerce_invocation_success(&invocation_id, &json!({"ok":true}))
        .unwrap();
}

#[test]
fn app_activity_health_aggregates_explainable_evidence_without_automatic_blocking() {
    let fixture = fixture();
    for index in 0..3 {
        finish_failure(
            &fixture,
            "consumer.attention",
            &format!("rate-{index}"),
            "rate_limited",
        );
    }
    finish_failure(
        &fixture,
        "consumer.attention",
        "budget",
        "grant_budget_exceeded",
    );
    let recovered = start_invocation(&fixture, "consumer.attention", "recovered");
    fixture
        .store
        .conn()
        .unwrap()
        .execute(
            "UPDATE open_commerce_invocations
                SET created_at=strftime('%Y-%m-%dT%H:%M:%SZ','now','-5 minutes')
             WHERE id=?1",
            rusqlite::params![recovered],
        )
        .unwrap();
    assert_eq!(
        fixture
            .store
            .reconcile_expired_open_commerce_invocations()
            .unwrap(),
        1
    );
    finish_success(&fixture, "consumer.normal", "normal-1");
    finish_success(&fixture, "consumer.normal", "normal-2");
    finish_failure(&fixture, "pc-web", "system-failure", "handler_failed");
    let old = finish_failure(&fixture, "consumer.old", "old", "handler_failed");
    fixture
        .store
        .conn()
        .unwrap()
        .execute(
            "UPDATE open_commerce_invocations
                SET created_at='2000-01-01T00:00:00Z', completed_at='2000-01-01T00:00:01Z'
              WHERE id=?1",
            rusqlite::params![old],
        )
        .unwrap();

    let health = fixture
        .store
        .open_commerce_app_activity_health(&fixture.project_id)
        .unwrap();
    assert_eq!(health.len(), 2);
    let attention = health
        .iter()
        .find(|item| item.requester_app_id == "consumer.attention")
        .unwrap();
    assert_eq!(attention.status, "attention");
    assert_eq!(attention.total_invocations_24h, 5);
    assert_eq!(attention.failed_invocations_24h, 5);
    assert_eq!(attention.rate_limited_invocations_24h, 3);
    assert_eq!(attention.grant_budget_rejections_24h, 1);
    assert_eq!(attention.recovered_invocations_24h, 1);
    assert_eq!(
        attention.attention_codes,
        vec![
            "recovered_invocation",
            "repeated_failures",
            "rate_limit_pressure",
            "grant_budget_pressure"
        ]
    );
    let normal = health
        .iter()
        .find(|item| item.requester_app_id == "consumer.normal")
        .unwrap();
    assert_eq!(normal.status, "normal");
    assert_eq!(normal.succeeded_invocations_24h, 2);
    assert!(normal.attention_codes.is_empty());
    assert!(fixture
        .store
        .list_project_open_commerce_app_blocks(&fixture.project_id)
        .unwrap()
        .is_empty());

    let overview = open_commerce_service::overview(&fixture.store, &fixture.project_id).unwrap();
    assert_eq!(overview.app_activity_health.len(), 2);
}
