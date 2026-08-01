use serde_json::json;
use uuid::Uuid;

use crate::{
    open_commerce_developer_model::CreateDeveloperAppRequest,
    open_commerce_invocation_recovery::{
        reconcile_expired_invocations, recover_interrupted_invocations,
    },
    open_commerce_model::{
        CreateCapabilityRequest, CreateGrantRequest, CreateMerchantRequest, HANDLER_STATIC_JSON,
    },
    open_commerce_service::{self, OpenCommerceActor},
    store::{OpenCommerceInvocationStart, Store},
};

struct Fixture {
    store: Store,
    project_id: String,
    merchant_id: String,
    capability_id: String,
    owner_id: String,
    app_owner_id: String,
    app_id: String,
}

fn fixture() -> Fixture {
    let path = std::env::temp_dir().join(format!(
        "elon-open-commerce-recovery-{}.sqlite",
        Uuid::new_v4().simple()
    ));
    let store = Store::open(&path).unwrap();
    let owner = store
        .create_user("recovery-owner@example.com", "secret1", None, None)
        .unwrap();
    let project = store
        .create_project(&owner.id, "Recovery merchant", None, None)
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
            display_name: "恢复测试商户".to_string(),
            slug: Some("recovery-merchant".to_string()),
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
            access_level: "authorized".to_string(),
            input_schema: json!({}),
            output_schema: json!({}),
            handler_type: HANDLER_STATIC_JSON.to_string(),
            handler_config: Some(json!({"response":{"items":[]}})),
            unit_price_micros: 40_000,
            currency: "CNY".to_string(),
            freshness_seconds: 0,
        },
    )
    .unwrap();
    let app_owner = store
        .create_user("recovery-app@example.com", "secret1", None, None)
        .unwrap();
    let app_project = store
        .create_project(&app_owner.id, "Recovery app", None, None)
        .unwrap()
        .project;
    let app = store
        .create_open_commerce_developer_app(
            &app_project.id,
            &app_owner.id,
            CreateDeveloperAppRequest {
                app_id: "consumer.recovery".to_string(),
                display_name: "恢复测试 App".to_string(),
            },
        )
        .unwrap()
        .app;
    Fixture {
        store,
        project_id: project.id,
        merchant_id: merchant.id,
        capability_id: capability.id,
        owner_id: owner.id,
        app_owner_id: app_owner.id,
        app_id: app.app_id,
    }
}

fn grant(fixture: &Fixture) -> String {
    let actor = OpenCommerceActor {
        user_id: &fixture.owner_id,
        app_id: "pc-web",
        project_role: Some("owner"),
    };
    open_commerce_service::create_grant(
        &fixture.store,
        &fixture.project_id,
        &actor,
        CreateGrantRequest {
            merchant_id: fixture.merchant_id.clone(),
            grantee_app_id: fixture.app_id.clone(),
            scopes: vec!["menu.lookup".to_string()],
            purpose: "验证孤儿调用恢复".to_string(),
            expires_at: None,
            max_invocations: Some(3),
            max_amount_micros: Some(120_000),
            budget_currency: "CNY".to_string(),
        },
    )
    .unwrap()
    .id
}

fn start_invocation(
    fixture: &Fixture,
    grant_id: Option<&str>,
    idempotency_key: &str,
) -> crate::open_commerce_model::OpenCommerceInvocation {
    fixture
        .store
        .start_open_commerce_invocation(OpenCommerceInvocationStart {
            project_id: &fixture.project_id,
            merchant_id: &fixture.merchant_id,
            capability_id: &fixture.capability_id,
            capability_key: "menu.lookup",
            requester_user_id: &fixture.app_owner_id,
            requester_app_id: &fixture.app_id,
            grant_id,
            idempotency_key,
            request_hash: idempotency_key,
            request_shape: &json!({"private_note":"must-not-enter-recovery-audit"}),
            unit_price_micros: 40_000,
            currency: "CNY",
        })
        .unwrap()
        .invocation
}

fn reservation_status(store: &Store, invocation_id: &str) -> String {
    store
        .conn()
        .unwrap()
        .query_row(
            "SELECT status FROM open_commerce_grant_budget_reservations
             WHERE invocation_id = ?1",
            rusqlite::params![invocation_id],
            |row| row.get(0),
        )
        .unwrap()
}

#[test]
fn startup_recovery_fails_all_started_invocations_and_releases_budget_once() {
    let fixture = fixture();
    let grant_id = grant(&fixture);
    let reserved = start_invocation(&fixture, Some(&grant_id), "startup-reserved");
    fixture
        .store
        .reserve_open_commerce_grant_budget(&reserved)
        .unwrap()
        .unwrap();
    let unreserved = start_invocation(&fixture, None, "startup-unreserved");

    assert_eq!(recover_interrupted_invocations(&fixture.store), 2);
    assert_eq!(recover_interrupted_invocations(&fixture.store), 0);
    for invocation in [&reserved, &unreserved] {
        let recovered = fixture
            .store
            .open_commerce_invocation(&invocation.id)
            .unwrap();
        assert_eq!(recovered.status, "failed");
        assert_eq!(
            recovered.error_code.as_deref(),
            Some("server_restart_interrupted")
        );
    }
    let recovered_grant = fixture.store.open_commerce_grant(&grant_id).unwrap();
    assert_eq!(recovered_grant.used_invocations, 0);
    assert_eq!(recovered_grant.used_amount_micros, 0);
    assert_eq!(reservation_status(&fixture.store, &reserved.id), "released");
    assert!(fixture
        .store
        .finish_open_commerce_invocation_success(&reserved.id, &json!({"late":true}))
        .is_err());
    assert!(fixture
        .store
        .reserve_open_commerce_grant_budget(&reserved)
        .is_err());

    let audit = serde_json::to_string(
        &fixture
            .store
            .list_project_open_commerce_audit(&fixture.project_id, 100)
            .unwrap(),
    )
    .unwrap();
    assert!(audit.contains("invocation.recovered_failed"));
    assert!(audit.contains("server_restart_interrupted"));
    assert!(!audit.contains("must-not-enter-recovery-audit"));
}

#[test]
fn periodic_recovery_only_closes_expired_invocations_and_is_idempotent() {
    let fixture = fixture();
    let grant_id = grant(&fixture);
    let expired = start_invocation(&fixture, Some(&grant_id), "periodic-expired");
    let fresh = start_invocation(&fixture, Some(&grant_id), "periodic-fresh");
    fixture
        .store
        .reserve_open_commerce_grant_budget(&expired)
        .unwrap();
    fixture
        .store
        .reserve_open_commerce_grant_budget(&fresh)
        .unwrap();
    fixture
        .store
        .conn()
        .unwrap()
        .execute(
            "UPDATE open_commerce_invocations SET created_at='2000-01-01T00:00:00Z'
             WHERE id = ?1",
            rusqlite::params![expired.id],
        )
        .unwrap();

    assert_eq!(reconcile_expired_invocations(&fixture.store), 1);
    assert_eq!(reconcile_expired_invocations(&fixture.store), 0);
    let expired = fixture.store.open_commerce_invocation(&expired.id).unwrap();
    assert_eq!(expired.status, "failed");
    assert_eq!(
        expired.error_code.as_deref(),
        Some("invocation_lease_expired")
    );
    assert_eq!(reservation_status(&fixture.store, &expired.id), "released");
    assert_eq!(
        fixture
            .store
            .open_commerce_invocation(&fresh.id)
            .unwrap()
            .status,
        "started"
    );
    let one_reserved = fixture.store.open_commerce_grant(&grant_id).unwrap();
    assert_eq!(one_reserved.used_invocations, 1);
    assert_eq!(one_reserved.used_amount_micros, 40_000);

    assert_eq!(recover_interrupted_invocations(&fixture.store), 1);
    let fully_released = fixture.store.open_commerce_grant(&grant_id).unwrap();
    assert_eq!(fully_released.used_invocations, 0);
    assert_eq!(fully_released.used_amount_micros, 0);
}
