use serde_json::json;
use uuid::Uuid;

use crate::{
    open_commerce_developer_model::CreateDeveloperAppRequest,
    open_commerce_directory_service,
    open_commerce_grant_budget_model::OpenCommerceGrantBudgetExceeded,
    open_commerce_model::{
        CreateCapabilityRequest, CreateGrantRequest, CreateMerchantRequest,
        InvokeCapabilityRequest, ACCESS_AUTHORIZED, HANDLER_STATIC_JSON,
    },
    open_commerce_service::{self, OpenCommerceActor},
    store::Store,
};

struct Fixture {
    store: Store,
    project_id: String,
    merchant_id: String,
    owner_id: String,
    app_owner_id: String,
    app_id: String,
}

fn temp_store() -> Store {
    let path = std::env::temp_dir().join(format!(
        "elon_open_commerce_grant_budget_{}.db",
        Uuid::new_v4().simple()
    ));
    Store::open(&path).expect("grant-budget test store should open")
}

fn fixture() -> Fixture {
    let store = temp_store();
    let owner = store
        .create_user(
            "budget-owner@example.com",
            "secret1",
            Some("Budget Owner"),
            None,
        )
        .unwrap();
    let project = store
        .create_project(&owner.id, "Grant Budget Merchant", None, None)
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
            display_name: "预算测试咖啡店".to_string(),
            slug: Some("grant-budget-cafe".to_string()),
            description: String::new(),
            node_mode: "platform_hosted".to_string(),
            public_profile: json!({"category":"cafe"}),
        },
    )
    .unwrap();
    for (key, config) in [
        ("menu.lookup", Some(json!({"response":{"items":["拿铁"]}}))),
        ("menu.broken", None),
    ] {
        open_commerce_service::publish_capability(
            &store,
            &project.id,
            &merchant.id,
            &owner_actor,
            CreateCapabilityRequest {
                capability_key: key.to_string(),
                display_name: key.to_string(),
                description: String::new(),
                kind: "query".to_string(),
                access_level: ACCESS_AUTHORIZED.to_string(),
                input_schema: json!({}),
                output_schema: json!({}),
                handler_type: HANDLER_STATIC_JSON.to_string(),
                handler_config: config,
                unit_price_micros: 40_000,
                currency: "CNY".to_string(),
                freshness_seconds: 0,
            },
        )
        .unwrap();
    }
    open_commerce_directory_service::set_publication(
        &store,
        &project.id,
        &merchant.id,
        &owner_actor,
        true,
    )
    .unwrap();

    let app_owner = store
        .create_user(
            "budget-app@example.com",
            "secret1",
            Some("Budget App"),
            None,
        )
        .unwrap();
    let app_project = store
        .create_project(&app_owner.id, "Grant Budget App", None, None)
        .unwrap()
        .project;
    let credential = store
        .create_open_commerce_developer_app(
            &app_project.id,
            &app_owner.id,
            CreateDeveloperAppRequest {
                app_id: "consumer.budget-test".to_string(),
                display_name: "预算消费者".to_string(),
            },
        )
        .unwrap();
    Fixture {
        store,
        project_id: project.id,
        merchant_id: merchant.id,
        owner_id: owner.id,
        app_owner_id: app_owner.id,
        app_id: credential.app.app_id,
    }
}

fn grant(fixture: &Fixture, max_calls: Option<i64>, max_amount: Option<i64>) -> String {
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
            scopes: vec!["menu.lookup".to_string(), "menu.broken".to_string()],
            purpose: "验证授权生命周期预算".to_string(),
            expires_at: None,
            max_invocations: max_calls,
            max_amount_micros: max_amount,
            budget_currency: "CNY".to_string(),
        },
    )
    .unwrap()
    .id
}

async fn invoke(
    fixture: &Fixture,
    grant_id: &str,
    capability_key: &str,
    idempotency_key: &str,
) -> anyhow::Result<serde_json::Value> {
    let actor = OpenCommerceActor {
        user_id: &fixture.app_owner_id,
        app_id: &fixture.app_id,
        project_role: None,
    };
    open_commerce_service::invoke(
        &fixture.store,
        &actor,
        InvokeCapabilityRequest {
            merchant_id: fixture.merchant_id.clone(),
            capability_key: capability_key.to_string(),
            requester_app_id: fixture.app_id.clone(),
            grant_id: Some(grant_id.to_string()),
            idempotency_key: idempotency_key.to_string(),
            input: json!({"private_note":"must-not-enter-audit"}),
        },
    )
    .await
}

#[tokio::test]
async fn grant_budget_reserves_commits_releases_and_rejects_atomically() {
    let fixture = fixture();
    let grant_id = grant(&fixture, Some(2), Some(80_000));

    let first = invoke(&fixture, &grant_id, "menu.lookup", "budget-first")
        .await
        .unwrap();
    assert_eq!(first["status"], "succeeded");
    let replay = invoke(&fixture, &grant_id, "menu.lookup", "budget-first")
        .await
        .unwrap();
    assert_eq!(replay["replayed"], true);

    let handler_error = invoke(&fixture, &grant_id, "menu.broken", "budget-broken")
        .await
        .unwrap_err();
    assert!(handler_error.to_string().contains("缺少配置"));
    let after_failure = fixture.store.open_commerce_grant(&grant_id).unwrap();
    assert_eq!(after_failure.used_invocations, 1);
    assert_eq!(after_failure.used_amount_micros, 40_000);

    invoke(&fixture, &grant_id, "menu.lookup", "budget-second")
        .await
        .unwrap();
    let exhausted = invoke(&fixture, &grant_id, "menu.lookup", "budget-third")
        .await
        .unwrap_err();
    assert!(exhausted.is::<OpenCommerceGrantBudgetExceeded>());

    let final_grant = fixture.store.open_commerce_grant(&grant_id).unwrap();
    assert_eq!(final_grant.used_invocations, 2);
    assert_eq!(final_grant.used_amount_micros, 80_000);
    let overview = open_commerce_service::overview(&fixture.store, &fixture.project_id).unwrap();
    let rejected = overview
        .recent_invocations
        .iter()
        .find(|item| item.error_code.as_deref() == Some("grant_budget_exceeded"))
        .unwrap();
    assert_eq!(rejected.units, 0);
    assert_eq!(rejected.amount_micros, 0);
    let failed = overview
        .recent_invocations
        .iter()
        .find(|item| item.error_code.as_deref() == Some("handler_failed"))
        .unwrap();
    let conn = fixture.store.conn().unwrap();
    let committed: String = conn
        .query_row(
            "SELECT status FROM open_commerce_grant_budget_reservations WHERE invocation_id = ?1",
            rusqlite::params![first["invocation_id"].as_str().unwrap()],
            |row| row.get(0),
        )
        .unwrap();
    let released: String = conn
        .query_row(
            "SELECT status FROM open_commerce_grant_budget_reservations WHERE invocation_id = ?1",
            rusqlite::params![failed.id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(committed, "committed");
    assert_eq!(released, "released");
    let audit = serde_json::to_string(&overview.recent_audit_events).unwrap();
    assert!(audit.contains("invocation.grant_budget_exceeded"));
    assert!(!audit.contains("must-not-enter-audit"));
}

#[tokio::test]
async fn amount_only_budget_and_unlimited_grants_keep_explicit_semantics() {
    let fixture = fixture();
    let amount_limited = grant(&fixture, None, Some(30_000));
    let rejected = invoke(&fixture, &amount_limited, "menu.lookup", "amount-too-small")
        .await
        .unwrap_err();
    let exceeded = rejected
        .downcast_ref::<OpenCommerceGrantBudgetExceeded>()
        .unwrap();
    assert_eq!(exceeded.limit_kind, "amount_micros");
    assert_eq!(
        fixture
            .store
            .open_commerce_grant(&amount_limited)
            .unwrap()
            .used_amount_micros,
        0
    );

    let unlimited = grant(&fixture, None, None);
    for index in 0..3 {
        invoke(
            &fixture,
            &unlimited,
            "menu.lookup",
            &format!("unlimited-{index}"),
        )
        .await
        .unwrap();
    }
    let unlimited = fixture.store.open_commerce_grant(&unlimited).unwrap();
    assert_eq!(unlimited.used_invocations, 0);
    assert_eq!(unlimited.used_amount_micros, 0);
}
