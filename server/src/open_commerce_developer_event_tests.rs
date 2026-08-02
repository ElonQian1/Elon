use serde_json::{json, Value};
use uuid::Uuid;

use crate::{
    open_commerce_developer_event_model::DeveloperTerminalEventQuery,
    open_commerce_developer_event_service,
    open_commerce_developer_model::{
        CreateDeveloperAppRequest, OpenCommerceDeveloperAppCredential,
    },
    open_commerce_directory_service,
    open_commerce_model::{
        CreateCapabilityRequest, CreateMerchantRequest, InvokeCapabilityRequest, ACCESS_PUBLIC,
        HANDLER_STATIC_JSON,
    },
    open_commerce_service::{self, OpenCommerceActor},
    store::Store,
};

struct Fixture {
    store: Store,
    merchant_id: String,
    first: OpenCommerceDeveloperAppCredential,
    second: OpenCommerceDeveloperAppCredential,
}

fn temp_store() -> Store {
    let path = std::env::temp_dir().join(format!(
        "elon_open_commerce_developer_events_{}.db",
        Uuid::new_v4().simple()
    ));
    Store::open(&path).expect("developer event test store should open")
}

fn fixture() -> Fixture {
    let store = temp_store();
    let owner = store
        .create_user(
            "developer-events@example.com",
            "secret1",
            Some("Developer Events"),
            None,
        )
        .unwrap();
    let project = store
        .create_project(&owner.id, "Developer Events", None, None)
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
            display_name: "终态事件咖啡店".to_string(),
            slug: Some("terminal-event-cafe".to_string()),
            description: "开发者调用事件测试".to_string(),
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
            description: "返回稳定沙盒菜单".to_string(),
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
    let first = developer_app(&store, &project.id, &owner.id, "consumer.events.one");
    let second = developer_app(&store, &project.id, &owner.id, "consumer.events.two");
    Fixture {
        store,
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

async fn invoke(fixture: &Fixture, app: &OpenCommerceDeveloperAppCredential, key: &str) -> Value {
    let actor = OpenCommerceActor {
        user_id: &app.app.owner_user_id,
        app_id: &app.app.app_id,
        project_role: None,
    };
    open_commerce_service::invoke(
        &fixture.store,
        &actor,
        InvokeCapabilityRequest {
            merchant_id: fixture.merchant_id.clone(),
            capability_key: "menu.preview".to_string(),
            requester_app_id: app.app.app_id.clone(),
            grant_id: None,
            idempotency_key: key.to_string(),
            input: json!({}),
        },
    )
    .await
    .unwrap()
}

#[tokio::test]
async fn developer_event_feed_is_app_scoped_cursor_safe_and_resumable() {
    let fixture = fixture();
    let first_result = invoke(&fixture, &fixture.first, "events-one-first").await;
    invoke(&fixture, &fixture.second, "events-two-only").await;
    let last_result = invoke(&fixture, &fixture.first, "events-one-last").await;

    let first_page = open_commerce_developer_event_service::list_terminal_events(
        &fixture.store,
        &fixture.first.app,
        DeveloperTerminalEventQuery {
            cursor: None,
            limit: Some(1),
        },
    )
    .unwrap();
    assert_eq!(first_page.events.len(), 1);
    assert!(first_page.has_more);
    assert_eq!(first_page.events[0].idempotency_key, "events-one-first");
    let cursor = first_page.next_cursor.clone().unwrap();

    let second_page = open_commerce_developer_event_service::list_terminal_events(
        &fixture.store,
        &fixture.first.app,
        DeveloperTerminalEventQuery {
            cursor: Some(cursor.clone()),
            limit: Some(1000),
        },
    )
    .unwrap();
    assert_eq!(second_page.events.len(), 1);
    assert!(!second_page.has_more);
    assert_eq!(second_page.events[0].idempotency_key, "events-one-last");
    assert!(second_page.events[0].result_available);
    assert!(!second_page.events[0].funds_moved);

    let checkpoint = second_page.next_cursor.clone().unwrap();
    let empty_page = open_commerce_developer_event_service::list_terminal_events(
        &fixture.store,
        &fixture.first.app,
        DeveloperTerminalEventQuery {
            cursor: Some(checkpoint.clone()),
            limit: None,
        },
    )
    .unwrap();
    assert!(empty_page.events.is_empty());
    assert_eq!(empty_page.next_cursor.as_deref(), Some(checkpoint.as_str()));

    let second_app_page = open_commerce_developer_event_service::list_terminal_events(
        &fixture.store,
        &fixture.second.app,
        DeveloperTerminalEventQuery {
            cursor: None,
            limit: None,
        },
    )
    .unwrap();
    assert_eq!(second_app_page.events.len(), 1);
    assert_eq!(second_app_page.events[0].idempotency_key, "events-two-only");

    let cross_app_cursor = open_commerce_developer_event_service::list_terminal_events(
        &fixture.store,
        &fixture.second.app,
        DeveloperTerminalEventQuery {
            cursor: Some(cursor),
            limit: None,
        },
    )
    .unwrap_err();
    assert!(cross_app_cursor.to_string().contains("不属于当前应用"));

    let detail = open_commerce_developer_event_service::terminal_event_detail(
        &fixture.store,
        &fixture.first.app,
        last_result["invocation_id"].as_str().unwrap(),
    )
    .unwrap();
    assert_eq!(detail.result, Some(json!({"items":["拿铁"]})));
    assert!(
        open_commerce_developer_event_service::terminal_event_detail(
            &fixture.store,
            &fixture.second.app,
            first_result["invocation_id"].as_str().unwrap(),
        )
        .is_err()
    );
}

#[tokio::test]
async fn developer_event_list_omits_request_and_internal_authorization_data() {
    let fixture = fixture();
    invoke(&fixture, &fixture.first, "privacy-event").await;
    let page = open_commerce_developer_event_service::list_terminal_events(
        &fixture.store,
        &fixture.first.app,
        DeveloperTerminalEventQuery {
            cursor: None,
            limit: None,
        },
    )
    .unwrap();
    let serialized = serde_json::to_string(&page).unwrap();
    for forbidden in [
        "request_hash",
        "request_shape",
        "grant_id",
        "requester_user_id",
        "project_id",
    ] {
        assert!(!serialized.contains(forbidden), "leaked field: {forbidden}");
    }
    assert!(open_commerce_developer_event_service::list_terminal_events(
        &fixture.store,
        &fixture.first.app,
        DeveloperTerminalEventQuery {
            cursor: Some("not-a-cursor".to_string()),
            limit: None,
        },
    )
    .is_err());
}
