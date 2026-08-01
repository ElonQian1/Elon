use serde_json::json;
use uuid::Uuid;

use crate::{
    open_commerce_consumer,
    open_commerce_consumer_model::{ConsumerDiscoveryRequest, ConsumerPreferences},
    open_commerce_developer_model::{
        CreateAuthorizationRequest, CreateDeveloperAppRequest, DeveloperInvokeRequest,
    },
    open_commerce_directory_service,
    open_commerce_model::{
        CreateCapabilityRequest, CreateGrantRequest, CreateMerchantRequest,
        InvokeCapabilityRequest, ACCESS_AUTHORIZED, HANDLER_STATIC_JSON,
    },
    open_commerce_service::{self, OpenCommerceActor},
    store::Store,
};

fn temp_store() -> Store {
    let path = std::env::temp_dir().join(format!(
        "elon_open_commerce_client_{}.db",
        Uuid::new_v4().simple()
    ));
    Store::open(&path).expect("client test store should open")
}

#[tokio::test]
async fn consumer_discovery_request_approval_and_test_token_invocation_form_a_loop() {
    let store = temp_store();
    let merchant_owner = store
        .create_user(
            "merchant-owner@example.com",
            "secret1",
            Some("Merchant Owner"),
            None,
        )
        .unwrap();
    let merchant_project = store
        .create_project(&merchant_owner.id, "Merchant Project", None, None)
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
            display_name: "吉安测试咖啡店".to_string(),
            slug: Some("jian-test-cafe".to_string()),
            description: "消费者沙盒测试".to_string(),
            node_mode: "platform_hosted".to_string(),
            public_profile: json!({
                "category":"cafe",
                "city":"Ji'an",
                "tags":["quiet","coffee"]
            }),
        },
    )
    .unwrap();
    open_commerce_service::publish_capability(
        &store,
        &merchant_project.id,
        &merchant.id,
        &merchant_actor,
        CreateCapabilityRequest {
            capability_key: "menu.preview".to_string(),
            display_name: "菜单预览".to_string(),
            description: "授权后读取菜单".to_string(),
            kind: "query".to_string(),
            access_level: ACCESS_AUTHORIZED.to_string(),
            input_schema: json!({}),
            output_schema: json!({}),
            handler_type: HANDLER_STATIC_JSON.to_string(),
            handler_config: Some(json!({"response":{"items":["拿铁","美式"]}})),
            unit_price_micros: 20_000,
            currency: "CNY".to_string(),
            freshness_seconds: 60,
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
        .create_user("developer@example.com", "secret1", Some("Developer"), None)
        .unwrap();
    let developer_project = store
        .create_project(&developer.id, "Developer Project", None, None)
        .unwrap()
        .project;
    let credential = store
        .create_open_commerce_developer_app(
            &developer_project.id,
            &developer.id,
            CreateDeveloperAppRequest {
                app_id: "consumer.demo".to_string(),
                display_name: "消费者测试 App".to_string(),
            },
        )
        .unwrap();
    let pc_web_request = open_commerce_consumer::create_authorization_request(
        &store,
        &developer.id,
        CreateAuthorizationRequest {
            merchant_id: merchant.id.clone(),
            requester_app_id: "pc-web".to_string(),
            scopes: vec!["menu.preview".to_string()],
            purpose: "不应共享公共网页身份".to_string(),
        },
    )
    .unwrap_err();
    assert!(pc_web_request.to_string().contains("注册独立开发者应用"));

    let before = open_commerce_consumer::discover(
        &store,
        &developer.id,
        ConsumerDiscoveryRequest {
            query: Some("咖啡".to_string()),
            capability_key: Some("menu.preview".to_string()),
            requester_app_id: credential.app.app_id.clone(),
            preferences: ConsumerPreferences {
                categories: vec!["cafe".to_string()],
                tags: vec!["quiet".to_string()],
                city: Some("Ji'an".to_string()),
                max_unit_price_micros: Some(50_000),
                prefer_public: true,
            },
            limit: 10,
        },
    )
    .unwrap();
    assert_eq!(before.matches.len(), 1);
    assert_eq!(before.matches[0].authorization.status, "request_required");
    assert!(before.matches[0].score >= 80);
    assert!(!before.ranking_is_paid);
    let discovery_json = serde_json::to_string(&before).unwrap();
    assert!(!discovery_json.contains(&merchant_project.id));
    assert!(!discovery_json.contains(&merchant_owner.id));
    assert!(!discovery_json.contains("handler_type"));

    let request = open_commerce_consumer::create_authorization_request(
        &store,
        &developer.id,
        CreateAuthorizationRequest {
            merchant_id: merchant.id.clone(),
            requester_app_id: credential.app.app_id.clone(),
            scopes: vec!["menu.preview".to_string()],
            purpose: "为消费者展示实时菜单".to_string(),
        },
    )
    .unwrap();
    assert_eq!(request.status, "pending");

    let grant = open_commerce_service::create_grant(
        &store,
        &merchant_project.id,
        &merchant_actor,
        CreateGrantRequest {
            merchant_id: merchant.id.clone(),
            grantee_app_id: credential.app.app_id.clone(),
            scopes: request.scopes.clone(),
            purpose: request.purpose.clone(),
            expires_at: None,
        },
    )
    .unwrap();
    let approved = store
        .decide_open_commerce_authorization_request(
            &merchant_project.id,
            &request.id,
            &merchant_owner.id,
            "approved",
            "用途清晰",
            Some(&grant.id),
        )
        .unwrap();
    assert_eq!(approved.grant_id.as_deref(), Some(grant.id.as_str()));

    let after = open_commerce_consumer::discover(
        &store,
        &developer.id,
        ConsumerDiscoveryRequest {
            query: Some("咖啡".to_string()),
            capability_key: Some("menu.preview".to_string()),
            requester_app_id: credential.app.app_id.clone(),
            preferences: ConsumerPreferences::default(),
            limit: 10,
        },
    )
    .unwrap();
    assert_eq!(after.matches[0].authorization.status, "granted");

    let authenticated = store
        .authenticate_open_commerce_developer_app(&credential.test_token)
        .unwrap();
    let developer_actor = OpenCommerceActor {
        user_id: &authenticated.owner_user_id,
        app_id: &authenticated.app_id,
        project_role: None,
    };
    let request = DeveloperInvokeRequest {
        merchant_id: merchant.id,
        capability_key: "menu.preview".to_string(),
        grant_id: Some(grant.id),
        idempotency_key: "developer-debug-1".to_string(),
        input: json!({}),
    };
    let result = open_commerce_service::invoke(
        &store,
        &developer_actor,
        InvokeCapabilityRequest {
            merchant_id: request.merchant_id,
            capability_key: request.capability_key,
            requester_app_id: authenticated.app_id.clone(),
            grant_id: request.grant_id,
            idempotency_key: request.idempotency_key,
            input: request.input,
        },
    )
    .await
    .unwrap();
    assert_eq!(result["result"]["items"], json!(["拿铁", "美式"]));

    let impersonation = open_commerce_service::invoke(
        &store,
        &OpenCommerceActor {
            user_id: &merchant_owner.id,
            app_id: &authenticated.app_id,
            project_role: None,
        },
        InvokeCapabilityRequest {
            merchant_id: approved.merchant_id.clone(),
            capability_key: "menu.preview".to_string(),
            requester_app_id: authenticated.app_id.clone(),
            grant_id: approved.grant_id.clone(),
            idempotency_key: "developer-impersonation".to_string(),
            input: json!({}),
        },
    )
    .await
    .unwrap_err();
    assert!(impersonation.to_string().contains("不能代表该开发者应用"));

    open_commerce_directory_service::set_publication(
        &store,
        &merchant_project.id,
        &approved.merchant_id,
        &merchant_actor,
        false,
    )
    .unwrap();
    let hidden_invoke = open_commerce_service::invoke(
        &store,
        &developer_actor,
        InvokeCapabilityRequest {
            merchant_id: approved.merchant_id.clone(),
            capability_key: "menu.preview".to_string(),
            requester_app_id: authenticated.app_id.clone(),
            grant_id: approved.grant_id.clone(),
            idempotency_key: "developer-debug-hidden".to_string(),
            input: json!({}),
        },
    )
    .await
    .unwrap_err();
    assert!(hidden_invoke.to_string().contains("未发布到开放目录"));
    let hidden_authorization = open_commerce_consumer::create_authorization_request(
        &store,
        &developer.id,
        CreateAuthorizationRequest {
            merchant_id: approved.merchant_id.clone(),
            requester_app_id: authenticated.app_id.clone(),
            scopes: vec!["menu.preview".to_string()],
            purpose: "撤回目录后不应继续收到授权申请".to_string(),
        },
    )
    .unwrap_err();
    assert!(hidden_authorization
        .to_string()
        .contains("不能接收外部授权申请"));
    let hidden = open_commerce_consumer::discover(
        &store,
        &developer.id,
        ConsumerDiscoveryRequest {
            query: Some("咖啡".to_string()),
            capability_key: Some("menu.preview".to_string()),
            requester_app_id: credential.app.app_id.clone(),
            preferences: ConsumerPreferences::default(),
            limit: 10,
        },
    )
    .unwrap();
    assert!(hidden.matches.is_empty());

    let apps = store
        .list_project_open_commerce_developer_apps(&developer_project.id)
        .unwrap();
    let serialized = serde_json::to_string(&apps).unwrap();
    assert!(!serialized.contains(&credential.test_token));
}
