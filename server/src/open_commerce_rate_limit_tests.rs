use serde_json::json;
use uuid::Uuid;

use crate::{
    open_commerce_developer_model::CreateDeveloperAppRequest,
    open_commerce_directory_service,
    open_commerce_model::{
        CreateCapabilityRequest, CreateMerchantRequest, InvokeCapabilityRequest, ACCESS_PUBLIC,
        HANDLER_STATIC_JSON,
    },
    open_commerce_rate_limit_model::{
        OpenCommerceRateLimitExceeded, UpsertOpenCommerceRateLimitRequest,
    },
    open_commerce_rate_limit_service,
    open_commerce_service::{self, OpenCommerceActor},
    store::Store,
};

fn temp_store() -> Store {
    let path = std::env::temp_dir().join(format!(
        "elon_open_commerce_rate_limit_{}.db",
        Uuid::new_v4().simple()
    ));
    Store::open(&path).expect("rate-limit test store should open")
}

#[tokio::test]
async fn external_invocations_are_limited_audited_and_idempotent_replays_are_free() {
    let store = temp_store();
    let merchant_owner = store
        .create_user(
            "rate-limit-merchant@example.com",
            "secret1",
            Some("Rate Limit Merchant"),
            None,
        )
        .unwrap();
    let merchant_project = store
        .create_project(&merchant_owner.id, "Rate Limit Merchant", None, None)
        .unwrap()
        .project;
    let merchant_actor = OpenCommerceActor {
        user_id: &merchant_owner.id,
        app_id: "pc-web",
        project_role: Some("owner"),
    };
    let merchant = open_commerce_service::create_merchant(
        &store,
        &merchant_project.id,
        &merchant_actor,
        CreateMerchantRequest {
            display_name: "限流测试咖啡店".to_string(),
            slug: Some("rate-limit-cafe".to_string()),
            description: String::new(),
            node_mode: "platform_hosted".to_string(),
            public_profile: json!({"category":"cafe"}),
        },
    )
    .unwrap();
    open_commerce_service::publish_capability(
        &store,
        &merchant_project.id,
        &merchant.id,
        &merchant_actor,
        CreateCapabilityRequest {
            capability_key: "menu.lookup".to_string(),
            display_name: "菜单查询".to_string(),
            description: String::new(),
            kind: "query".to_string(),
            access_level: ACCESS_PUBLIC.to_string(),
            input_schema: json!({}),
            output_schema: json!({}),
            handler_type: HANDLER_STATIC_JSON.to_string(),
            handler_config: Some(json!({"response":{"items":["拿铁"]}})),
            unit_price_micros: 50_000,
            currency: "CNY".to_string(),
            freshness_seconds: 0,
        },
    )
    .unwrap();
    open_commerce_directory_service::set_publication(
        &store,
        &merchant_project.id,
        &merchant.id,
        &merchant_actor,
        true,
    )
    .unwrap();

    let developer = store
        .create_user(
            "rate-limit-developer@example.com",
            "secret1",
            Some("Rate Limit Developer"),
            None,
        )
        .unwrap();
    let developer_project = store
        .create_project(&developer.id, "Rate Limit Developer", None, None)
        .unwrap()
        .project;
    let credential = store
        .create_open_commerce_developer_app(
            &developer_project.id,
            &developer.id,
            CreateDeveloperAppRequest {
                app_id: "consumer.rate-test".to_string(),
                display_name: "限流消费者 App".to_string(),
            },
        )
        .unwrap();

    let wildcard_policy = open_commerce_rate_limit_service::upsert_policy(
        &store,
        &merchant_project.id,
        &merchant_owner.id,
        "pc-web",
        "owner",
        UpsertOpenCommerceRateLimitRequest {
            merchant_id: merchant.id.clone(),
            capability_key: "menu.lookup".to_string(),
            requester_app_id: None,
            window_seconds: 3_600,
            max_requests: 1,
            enabled: true,
        },
    )
    .unwrap();
    let policy = open_commerce_rate_limit_service::upsert_policy(
        &store,
        &merchant_project.id,
        &merchant_owner.id,
        "pc-web",
        "owner",
        UpsertOpenCommerceRateLimitRequest {
            merchant_id: merchant.id.clone(),
            capability_key: "menu.lookup".to_string(),
            requester_app_id: Some(credential.app.app_id.clone()),
            window_seconds: 3_600,
            max_requests: 2,
            enabled: true,
        },
    )
    .unwrap();
    assert_eq!(
        policy.requester_app_id.as_deref(),
        Some("consumer.rate-test")
    );
    let mcp_policy = crate::open_commerce_mcp::call_tool(
        &store,
        &merchant_project.id,
        &merchant_owner.id,
        "owner",
        "mcp-client",
        json!({
            "name":"open_commerce_upsert_rate_limit",
            "arguments":{
                "merchant_id":merchant.id.clone(),
                "capability_key":"menu.lookup",
                "requester_app_id":"consumer.rate-test",
                "window_seconds":3600,
                "max_requests":2,
                "enabled":true
            }
        }),
    )
    .await
    .unwrap();
    assert_eq!(mcp_policy["structuredContent"]["id"], policy.id);

    let external_actor = OpenCommerceActor {
        user_id: &developer.id,
        app_id: &credential.app.app_id,
        project_role: None,
    };
    let request = |idempotency_key: &str| InvokeCapabilityRequest {
        merchant_id: merchant.id.clone(),
        capability_key: "menu.lookup".to_string(),
        requester_app_id: credential.app.app_id.clone(),
        grant_id: None,
        idempotency_key: idempotency_key.to_string(),
        input: json!({"locale":"zh-CN"}),
    };

    let first = open_commerce_service::invoke(&store, &external_actor, request("rate-first"))
        .await
        .unwrap();
    assert_eq!(first["status"], "succeeded");
    assert_eq!(first["replayed"], false);

    let second = open_commerce_service::invoke(&store, &external_actor, request("rate-second"))
        .await
        .unwrap();
    assert_eq!(second["status"], "succeeded");

    let limited = open_commerce_service::invoke(&store, &external_actor, request("rate-third"))
        .await
        .unwrap_err();
    assert!(limited.is::<OpenCommerceRateLimitExceeded>());

    let replay = open_commerce_service::invoke(&store, &external_actor, request("rate-first"))
        .await
        .unwrap();
    assert_eq!(replay["replayed"], true);

    let overview = open_commerce_service::overview(&store, &merchant_project.id).unwrap();
    assert_eq!(overview.totals.rate_limit_policies, 2);
    assert_eq!(overview.totals.active_rate_limit_policies, 2);
    assert_eq!(overview.totals.recent_rate_limited_invocations, 1);
    assert_eq!(
        overview
            .rate_limit_usage
            .iter()
            .find(|usage| usage.policy_id == policy.id)
            .unwrap()
            .accepted_requests,
        2
    );
    assert_eq!(
        overview
            .rate_limit_usage
            .iter()
            .find(|usage| usage.policy_id == wildcard_policy.id)
            .unwrap()
            .accepted_requests,
        0
    );
    let rejected = overview
        .recent_invocations
        .iter()
        .find(|invocation| invocation.error_code.as_deref() == Some("rate_limited"))
        .unwrap();
    assert_eq!(rejected.units, 0);
    assert_eq!(rejected.amount_micros, 0);
    assert!(overview
        .recent_audit_events
        .iter()
        .any(|event| event.action == "invocation.rate_limited"));

    let disabled = crate::open_commerce_mcp::call_tool(
        &store,
        &merchant_project.id,
        &merchant_owner.id,
        "owner",
        "mcp-client",
        json!({
            "name":"open_commerce_set_rate_limit_enabled",
            "arguments":{"policy_id":policy.id,"enabled":false}
        }),
    )
    .await
    .unwrap();
    assert_eq!(disabled["structuredContent"]["status"], "disabled");
    let after_disable =
        open_commerce_service::invoke(&store, &external_actor, request("rate-fourth"))
            .await
            .unwrap();
    assert_eq!(after_disable["status"], "succeeded");
    let wildcard_limited =
        open_commerce_service::invoke(&store, &external_actor, request("rate-fifth"))
            .await
            .unwrap_err();
    assert!(wildcard_limited.is::<OpenCommerceRateLimitExceeded>());
}

#[tokio::test]
async fn project_editors_bypass_external_rate_limits_for_merchant_debugging() {
    let store = temp_store();
    let owner = store
        .create_user(
            "rate-limit-owner@example.com",
            "secret1",
            Some("Rate Limit Owner"),
            None,
        )
        .unwrap();
    let project = store
        .create_project(&owner.id, "Editor Bypass", None, None)
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
            display_name: "商户调试节点".to_string(),
            slug: Some("merchant-debug-node".to_string()),
            description: String::new(),
            node_mode: "platform_hosted".to_string(),
            public_profile: json!({}),
        },
    )
    .unwrap();
    open_commerce_service::publish_capability(
        &store,
        &project.id,
        &merchant.id,
        &actor,
        CreateCapabilityRequest {
            capability_key: "profile.read".to_string(),
            display_name: "资料读取".to_string(),
            description: String::new(),
            kind: "query".to_string(),
            access_level: ACCESS_PUBLIC.to_string(),
            input_schema: json!({}),
            output_schema: json!({}),
            handler_type: HANDLER_STATIC_JSON.to_string(),
            handler_config: Some(json!({"response":{"ok":true}})),
            unit_price_micros: 0,
            currency: "CNY".to_string(),
            freshness_seconds: 0,
        },
    )
    .unwrap();
    open_commerce_rate_limit_service::upsert_policy(
        &store,
        &project.id,
        &owner.id,
        "pc-web",
        "owner",
        UpsertOpenCommerceRateLimitRequest {
            merchant_id: merchant.id.clone(),
            capability_key: "profile.read".to_string(),
            requester_app_id: None,
            window_seconds: 3_600,
            max_requests: 1,
            enabled: true,
        },
    )
    .unwrap();

    for key in ["editor-call-1", "editor-call-2"] {
        open_commerce_service::invoke(
            &store,
            &actor,
            InvokeCapabilityRequest {
                merchant_id: merchant.id.clone(),
                capability_key: "profile.read".to_string(),
                requester_app_id: "pc-web".to_string(),
                grant_id: None,
                idempotency_key: key.to_string(),
                input: json!({}),
            },
        )
        .await
        .unwrap();
    }
    let overview = open_commerce_service::overview(&store, &project.id).unwrap();
    assert_eq!(overview.rate_limit_usage[0].accepted_requests, 0);
}
