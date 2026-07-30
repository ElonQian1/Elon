use serde_json::json;
use uuid::Uuid;

use crate::{
    open_commerce_integration_model::{CreateIntegrationRequest, RecordSyncReceiptRequest},
    open_commerce_model::{
        CreateCapabilityRequest, CreateGrantRequest, CreateMerchantRequest,
        InvokeCapabilityRequest, ACCESS_AUTHORIZED, ACCESS_PUBLIC, HANDLER_MERCHANT_PROFILE,
        HANDLER_STATIC_JSON,
    },
    open_commerce_service::{self, OpenCommerceActor},
    store::Store,
};

fn temp_store() -> Store {
    let path = std::env::temp_dir().join(format!(
        "elon_open_commerce_e2e_{}.db",
        Uuid::new_v4().simple()
    ));
    Store::open(&path).expect("open-commerce test store should open")
}

#[test]
fn merchant_integration_receipts_feed_a_bounded_ai_development_context() {
    let store = temp_store();
    let owner = store
        .create_user(
            "integration-owner@example.com",
            "secret1",
            Some("Integration Owner"),
            None,
        )
        .expect("owner should be created");
    let project = store
        .create_project(&owner.id, "Integration Context", None, None)
        .expect("project should be created")
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
            display_name: "测试便利店".to_string(),
            slug: Some("context-store".to_string()),
            description: String::new(),
            node_mode: "platform_hosted".to_string(),
            public_profile: json!({}),
        },
    )
    .unwrap();
    let integration = open_commerce_service::create_integration(
        &store,
        &project.id,
        &actor,
        CreateIntegrationRequest {
            merchant_id: merchant.id,
            integration_key: "pos.main".to_string(),
            provider_key: "local_pos".to_string(),
            display_name: "线下收银系统".to_string(),
            connection_mode: "local_adapter".to_string(),
            scopes: vec!["read.orders".to_string(), "read.inventory".to_string()],
            data_domains: vec!["orders".to_string(), "inventory".to_string()],
        },
    )
    .unwrap();
    let started_at = "2026-07-30T12:00:00Z".to_string();
    let completed_at = "2026-07-30T12:00:01Z".to_string();
    let receipt = open_commerce_service::record_sync_receipt(
        &store,
        &project.id,
        &actor,
        RecordSyncReceiptRequest {
            integration_id: integration.id.clone(),
            receipt_key: "adapter-run-1".to_string(),
            sync_kind: "incremental".to_string(),
            status: "succeeded".to_string(),
            records_seen: 12,
            records_changed: 3,
            cursor_digest: Some("sha256:cursor1".to_string()),
            error_code: None,
            started_at,
            completed_at,
        },
    )
    .unwrap();
    assert_eq!(receipt.records_changed, 3);

    let replayed = open_commerce_service::record_sync_receipt(
        &store,
        &project.id,
        &actor,
        RecordSyncReceiptRequest {
            integration_id: integration.id,
            receipt_key: "adapter-run-1".to_string(),
            sync_kind: "incremental".to_string(),
            status: "succeeded".to_string(),
            records_seen: 12,
            records_changed: 3,
            cursor_digest: Some("sha256:cursor1".to_string()),
            error_code: None,
            started_at: "2026-07-30T12:00:00Z".to_string(),
            completed_at: "2026-07-30T12:00:01Z".to_string(),
        },
    )
    .unwrap();
    assert_eq!(replayed.id, receipt.id);

    let context = open_commerce_service::development_context(&store, &project.id).unwrap();
    assert_eq!(
        context["summary"]["connected_integrations"],
        serde_json::Value::from(1)
    );
    assert_eq!(
        context["merchants"][0]["integrations"][0]["data_domains"],
        json!(["orders", "inventory"])
    );
    let serialized = context.to_string();
    assert!(!serialized.contains("secret1"));
    assert!(!serialized.contains("access_token"));
    assert!(!serialized.contains("adapter-run-1"));
    assert!(!serialized.contains("sha256:cursor1"));

    let mcp_context = crate::open_commerce_mcp::call_tool(
        &store,
        &project.id,
        &owner.id,
        "owner",
        "pc-web",
        json!({
            "name":"open_commerce_get_development_context",
            "arguments":{}
        }),
    )
    .unwrap();
    assert_eq!(
        mcp_context["structuredContent"]["schema"],
        "open_commerce.development_context.v1"
    );
}

#[test]
fn merchant_to_authorized_invocation_is_audited_and_idempotent() {
    let store = temp_store();
    let owner = store
        .create_user(
            "open-commerce-owner@example.com",
            "secret1",
            Some("Open Commerce Owner"),
            None,
        )
        .expect("owner should be created");
    let project = store
        .create_project(&owner.id, "Open Commerce E2E", None, None)
        .expect("project should be created")
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
            display_name: "测试咖啡店".to_string(),
            slug: Some("test-cafe".to_string()),
            description: "用于验证完整开放商业调用链".to_string(),
            node_mode: "platform_hosted".to_string(),
            public_profile: json!({"category":"cafe","city":"Ji'an"}),
        },
    )
    .expect("merchant should be created");

    open_commerce_service::publish_capability(
        &store,
        &project.id,
        &merchant.id,
        &actor,
        CreateCapabilityRequest {
            capability_key: "merchant.profile".to_string(),
            display_name: "商户资料".to_string(),
            description: "公开资料查询".to_string(),
            kind: "query".to_string(),
            access_level: ACCESS_PUBLIC.to_string(),
            input_schema: json!({}),
            output_schema: json!({}),
            handler_type: HANDLER_MERCHANT_PROFILE.to_string(),
            handler_config: None,
            unit_price_micros: 0,
            currency: "CNY".to_string(),
            freshness_seconds: 60,
        },
    )
    .expect("public capability should be published");

    open_commerce_service::publish_capability(
        &store,
        &project.id,
        &merchant.id,
        &actor,
        CreateCapabilityRequest {
            capability_key: "menu.preview".to_string(),
            display_name: "菜单预览".to_string(),
            description: "需要授权的静态菜单".to_string(),
            kind: "query".to_string(),
            access_level: ACCESS_AUTHORIZED.to_string(),
            input_schema: json!({}),
            output_schema: json!({}),
            handler_type: HANDLER_STATIC_JSON.to_string(),
            handler_config: Some(json!({"response":{"items":["拿铁","美式"]}})),
            unit_price_micros: 25_000,
            currency: "CNY".to_string(),
            freshness_seconds: 300,
        },
    )
    .expect("authorized capability should be published");

    let discovery =
        open_commerce_service::discover_merchant(&store, &merchant.id).expect("merchant discover");
    assert_eq!(discovery.capabilities.len(), 2);
    assert!(discovery
        .capabilities
        .iter()
        .all(|capability| capability.handler_config.is_none()));

    let first_public = open_commerce_service::invoke(
        &store,
        &actor,
        InvokeCapabilityRequest {
            merchant_id: merchant.id.clone(),
            capability_key: "merchant.profile".to_string(),
            requester_app_id: "pc-web".to_string(),
            grant_id: None,
            idempotency_key: "profile-request-1".to_string(),
            input: json!({"locale":"zh-CN"}),
        },
    )
    .expect("public capability should invoke");
    assert_eq!(first_public["replayed"], false);
    assert_eq!(first_public["result"]["display_name"], "测试咖啡店");

    let replayed_public = open_commerce_service::invoke(
        &store,
        &actor,
        InvokeCapabilityRequest {
            merchant_id: merchant.id.clone(),
            capability_key: "merchant.profile".to_string(),
            requester_app_id: "pc-web".to_string(),
            grant_id: None,
            idempotency_key: "profile-request-1".to_string(),
            input: json!({"locale":"zh-CN"}),
        },
    )
    .expect("same invocation should replay");
    assert_eq!(replayed_public["replayed"], true);
    assert_eq!(
        replayed_public["invocation_id"],
        first_public["invocation_id"]
    );

    let grant = open_commerce_service::create_grant(
        &store,
        &project.id,
        &actor,
        CreateGrantRequest {
            merchant_id: merchant.id.clone(),
            grantee_app_id: "pc-web".to_string(),
            scopes: vec!["menu.preview".to_string()],
            purpose: "验证获得授权后的菜单查询".to_string(),
            expires_at: None,
        },
    )
    .expect("grant should be created");

    let authorized = open_commerce_service::invoke(
        &store,
        &actor,
        InvokeCapabilityRequest {
            merchant_id: merchant.id.clone(),
            capability_key: "menu.preview".to_string(),
            requester_app_id: "pc-web".to_string(),
            grant_id: Some(grant.id.clone()),
            idempotency_key: "menu-request-1".to_string(),
            input: json!({}),
        },
    )
    .expect("authorized capability should invoke");
    assert_eq!(authorized["result"]["items"], json!(["拿铁", "美式"]));
    assert_eq!(authorized["metering"]["amount_micros"], 25_000);
    assert_eq!(
        authorized["metering"]["settlement_status"],
        "recorded_not_charged"
    );

    let mcp_invocation = crate::open_commerce_mcp::call_tool(
        &store,
        &project.id,
        &owner.id,
        "owner",
        "pc-web",
        json!({
            "name":"open_commerce_invoke",
            "arguments":{
                "merchant_id":merchant.id,
                "capability_key":"menu.preview",
                "grant_id":grant.id,
                "idempotency_key":"menu-request-via-mcp",
                "input":{}
            }
        }),
    )
    .expect("MCP should call the same authorized service");
    assert_eq!(
        mcp_invocation["structuredContent"]["result"]["items"],
        json!(["拿铁", "美式"])
    );
    assert_eq!(
        mcp_invocation["structuredContent"]["metering"]["amount_micros"],
        25_000
    );

    let overview =
        open_commerce_service::overview(&store, &project.id).expect("overview should load");
    assert_eq!(overview.totals.active_merchants, 1);
    assert_eq!(overview.totals.active_capabilities, 2);
    assert_eq!(overview.totals.active_grants, 1);
    assert_eq!(overview.totals.invocations, 3);
    assert_eq!(overview.totals.metered_amount_micros, 50_000);
    assert!(overview
        .recent_audit_events
        .iter()
        .any(|event| event.action == "invocation.succeeded"));
    assert!(overview
        .recent_invocations
        .iter()
        .all(|invocation| invocation.request_shape["contains_raw_values"] == false));
}
