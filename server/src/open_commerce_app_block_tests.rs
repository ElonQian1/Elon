use serde_json::json;
use uuid::Uuid;

use crate::{
    open_commerce_app_block_model::{
        BlockOpenCommerceAppRequest, OpenCommerceAppBlocked, APP_BLOCK_STATUS_ACTIVE,
        APP_BLOCK_STATUS_UNBLOCKED,
    },
    open_commerce_app_block_service, open_commerce_consumer,
    open_commerce_developer_model::{CreateAuthorizationRequest, CreateDeveloperAppRequest},
    open_commerce_directory_service,
    open_commerce_model::{
        CreateCapabilityRequest, CreateGrantRequest, CreateMerchantRequest,
        InvokeCapabilityRequest, ACCESS_AUTHORIZED, ACCESS_PUBLIC, HANDLER_STATIC_JSON,
    },
    open_commerce_service::{self, OpenCommerceActor},
    store::Store,
};

fn temp_store() -> Store {
    let path = std::env::temp_dir().join(format!(
        "elon_open_commerce_app_block_{}.db",
        Uuid::new_v4().simple()
    ));
    Store::open(&path).expect("app-block test store should open")
}

#[tokio::test]
async fn merchant_block_revokes_trust_and_unblock_does_not_restore_it() {
    let store = temp_store();
    let merchant_owner = store
        .create_user(
            "app-block-merchant@example.com",
            "secret1",
            Some("App Block Merchant"),
            None,
        )
        .unwrap();
    let merchant_project = store
        .create_project(&merchant_owner.id, "App Block Merchant", None, None)
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
            display_name: "封禁测试咖啡店".to_string(),
            slug: Some("app-block-cafe".to_string()),
            description: String::new(),
            node_mode: "platform_hosted".to_string(),
            public_profile: json!({"category":"cafe"}),
        },
    )
    .unwrap();
    for (key, access_level) in [
        ("menu.lookup", ACCESS_PUBLIC),
        ("order.create", ACCESS_AUTHORIZED),
    ] {
        open_commerce_service::publish_capability(
            &store,
            &merchant_project.id,
            &merchant.id,
            &merchant_actor,
            CreateCapabilityRequest {
                capability_key: key.to_string(),
                display_name: key.to_string(),
                description: String::new(),
                kind: "query".to_string(),
                access_level: access_level.to_string(),
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
    }
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
            "app-block-developer@example.com",
            "secret1",
            Some("App Block Developer"),
            None,
        )
        .unwrap();
    let developer_project = store
        .create_project(&developer.id, "App Block Developer", None, None)
        .unwrap()
        .project;
    let credential = store
        .create_open_commerce_developer_app(
            &developer_project.id,
            &developer.id,
            CreateDeveloperAppRequest {
                app_id: "consumer.block-test".to_string(),
                display_name: "封禁消费者 App".to_string(),
            },
        )
        .unwrap();
    let grant = open_commerce_service::create_grant(
        &store,
        &merchant_project.id,
        &merchant_actor,
        CreateGrantRequest {
            merchant_id: merchant.id.clone(),
            grantee_app_id: credential.app.app_id.clone(),
            scopes: vec!["order.create".to_string()],
            purpose: "自动下单测试".to_string(),
            expires_at: None,
            max_invocations: None,
            max_amount_micros: None,
            budget_currency: "CNY".to_string(),
        },
    )
    .unwrap();
    let pending = open_commerce_consumer::create_authorization_request(
        &store,
        &developer.id,
        CreateAuthorizationRequest {
            merchant_id: merchant.id.clone(),
            requester_app_id: credential.app.app_id.clone(),
            scopes: vec!["order.create".to_string()],
            purpose: "补充订单授权".to_string(),
        },
    )
    .unwrap();

    let outcome = open_commerce_app_block_service::block_app(
        &store,
        &merchant_project.id,
        &merchant_owner.id,
        "pc-web",
        "owner",
        BlockOpenCommerceAppRequest {
            merchant_id: merchant.id.clone(),
            requester_app_id: credential.app.app_id.clone(),
            reason_code: "abusive_traffic".to_string(),
            reason_note: "短时间内重复调用".to_string(),
        },
    )
    .unwrap();
    assert_eq!(outcome.block.status, APP_BLOCK_STATUS_ACTIVE);
    assert_eq!(outcome.revoked_grants, 1);
    assert_eq!(outcome.canceled_authorization_requests, 1);
    assert_eq!(outcome.grants_restored, 0);
    assert!(store
        .open_commerce_grant(&grant.id)
        .unwrap()
        .revoked_at
        .is_some());
    let canceled = store
        .open_commerce_authorization_request(&pending.id)
        .unwrap();
    assert_eq!(canceled.status, "canceled");
    assert_eq!(
        canceled.decision_reason.as_deref(),
        Some("merchant_app_blocked")
    );

    let developer_actor = OpenCommerceActor {
        user_id: &developer.id,
        app_id: &credential.app.app_id,
        project_role: None,
    };
    let invocation_error = open_commerce_service::invoke(
        &store,
        &developer_actor,
        InvokeCapabilityRequest {
            merchant_id: merchant.id.clone(),
            capability_key: "menu.lookup".to_string(),
            requester_app_id: credential.app.app_id.clone(),
            grant_id: None,
            idempotency_key: "blocked-public-call".to_string(),
            input: json!({}),
        },
    )
    .await
    .unwrap_err();
    assert!(invocation_error.is::<OpenCommerceAppBlocked>());
    let authorization_error = open_commerce_consumer::create_authorization_request(
        &store,
        &developer.id,
        CreateAuthorizationRequest {
            merchant_id: merchant.id.clone(),
            requester_app_id: credential.app.app_id.clone(),
            scopes: vec!["order.create".to_string()],
            purpose: "封禁期间再次申请".to_string(),
        },
    )
    .unwrap_err();
    assert!(authorization_error.is::<OpenCommerceAppBlocked>());
    let grant_error = open_commerce_service::create_grant(
        &store,
        &merchant_project.id,
        &merchant_actor,
        CreateGrantRequest {
            merchant_id: merchant.id.clone(),
            grantee_app_id: credential.app.app_id.clone(),
            scopes: vec!["order.create".to_string()],
            purpose: "封禁期间误授权".to_string(),
            expires_at: None,
            max_invocations: None,
            max_amount_micros: None,
            budget_currency: "CNY".to_string(),
        },
    )
    .unwrap_err();
    assert!(grant_error.is::<OpenCommerceAppBlocked>());

    let repeated = open_commerce_app_block_service::block_app(
        &store,
        &merchant_project.id,
        &merchant_owner.id,
        "pc-web",
        "owner",
        BlockOpenCommerceAppRequest {
            merchant_id: merchant.id.clone(),
            requester_app_id: credential.app.app_id.clone(),
            reason_code: "policy_violation".to_string(),
            reason_note: "重复封禁保持幂等".to_string(),
        },
    )
    .unwrap();
    assert_eq!(repeated.block.id, outcome.block.id);
    assert_eq!(repeated.block.blocked_at, outcome.block.blocked_at);
    assert_eq!(repeated.revoked_grants, 0);
    assert_eq!(repeated.canceled_authorization_requests, 0);
    let blocks =
        open_commerce_app_block_service::list_blocks(&store, &merchant_project.id).unwrap();
    assert_eq!(blocks.len(), 1);

    let released = open_commerce_app_block_service::unblock_app(
        &store,
        &merchant_project.id,
        &outcome.block.id,
        &merchant_owner.id,
        "pc-web",
        "owner",
    )
    .unwrap();
    assert_eq!(released.block.status, APP_BLOCK_STATUS_UNBLOCKED);
    assert_eq!(released.grants_restored, 0);
    assert!(store
        .open_commerce_grant(&grant.id)
        .unwrap()
        .revoked_at
        .is_some());

    let response = open_commerce_service::invoke(
        &store,
        &developer_actor,
        InvokeCapabilityRequest {
            merchant_id: merchant.id.clone(),
            capability_key: "menu.lookup".to_string(),
            requester_app_id: credential.app.app_id.clone(),
            grant_id: None,
            idempotency_key: "unblocked-public-call".to_string(),
            input: json!({}),
        },
    )
    .await
    .unwrap();
    assert_eq!(response["result"]["ok"], true);
    let new_pending = open_commerce_consumer::create_authorization_request(
        &store,
        &developer.id,
        CreateAuthorizationRequest {
            merchant_id: merchant.id.clone(),
            requester_app_id: credential.app.app_id.clone(),
            scopes: vec!["order.create".to_string()],
            purpose: "解除后重新申请".to_string(),
        },
    )
    .unwrap();
    assert_eq!(new_pending.status, "pending");
    let audits = store
        .list_project_open_commerce_audit(&merchant_project.id, 20)
        .unwrap();
    assert!(audits
        .iter()
        .any(|event| event.action == "app_block.activated"));
    assert!(audits
        .iter()
        .any(|event| event.action == "app_block.released"));
}

#[test]
fn app_block_management_rejects_viewers_system_ids_and_unknown_apps() {
    let store = temp_store();
    let owner = store
        .create_user(
            "app-block-boundary@example.com",
            "secret1",
            Some("App Block Boundary"),
            None,
        )
        .unwrap();
    let project = store
        .create_project(&owner.id, "App Block Boundary", None, None)
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
            display_name: "边界商户".to_string(),
            slug: Some("block-boundary".to_string()),
            description: String::new(),
            node_mode: "platform_hosted".to_string(),
            public_profile: json!({}),
        },
    )
    .unwrap();
    let request = |app_id: &str| BlockOpenCommerceAppRequest {
        merchant_id: merchant.id.clone(),
        requester_app_id: app_id.to_string(),
        reason_code: "merchant_request".to_string(),
        reason_note: String::new(),
    };
    let viewer_error = open_commerce_app_block_service::block_app(
        &store,
        &project.id,
        &owner.id,
        "pc-web",
        "viewer",
        request("unknown.viewer-app"),
    )
    .unwrap_err();
    assert!(viewer_error.to_string().contains("编辑权限"));
    for system_app in ["pc-web", "mcp-client"] {
        let error = open_commerce_app_block_service::block_app(
            &store,
            &project.id,
            &owner.id,
            "pc-web",
            "owner",
            request(system_app),
        )
        .unwrap_err();
        assert!(error.to_string().contains("共享系统入口"));
    }
    let unknown_error = open_commerce_app_block_service::block_app(
        &store,
        &project.id,
        &owner.id,
        "pc-web",
        "owner",
        request("unknown.app"),
    )
    .unwrap_err();
    assert!(unknown_error.to_string().contains("开发者应用不存在"));
}
