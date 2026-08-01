use chrono::{Duration, Utc};
use serde_json::json;
use uuid::Uuid;

use crate::{
    open_commerce_authorization_decision::grant_request_for_authorization,
    open_commerce_consumer,
    open_commerce_developer_model::{
        CreateAuthorizationRequest, CreateDeveloperAppRequest, DecideAuthorizationRequest,
    },
    open_commerce_directory_service,
    open_commerce_model::{
        CreateCapabilityRequest, CreateMerchantRequest, InvokeCapabilityRequest, ACCESS_AUTHORIZED,
        HANDLER_STATIC_JSON,
    },
    open_commerce_service::{self, OpenCommerceActor},
    store::Store,
};

#[tokio::test]
async fn approved_authorization_preserves_terms_and_expired_grant_fails_closed() {
    let path = std::env::temp_dir().join(format!(
        "elon-open-commerce-grant-expiry-{}.sqlite",
        Uuid::new_v4().simple()
    ));
    let store = Store::open(&path).unwrap();
    let merchant_owner = store
        .create_user("expiry-merchant@example.com", "secret1", None, None)
        .unwrap();
    let merchant_project = store
        .create_project(&merchant_owner.id, "Expiry merchant", None, None)
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
            display_name: "限时授权测试商户".to_string(),
            slug: Some("grant-expiry-merchant".to_string()),
            description: String::new(),
            node_mode: "platform_hosted".to_string(),
            public_profile: json!({}),
        },
    )
    .unwrap();
    open_commerce_service::publish_capability(
        &store,
        &merchant_project.id,
        &merchant.id,
        &merchant_actor,
        CreateCapabilityRequest {
            capability_key: "menu.expiring".to_string(),
            display_name: "限时菜单".to_string(),
            description: String::new(),
            kind: "query".to_string(),
            access_level: ACCESS_AUTHORIZED.to_string(),
            input_schema: json!({}),
            output_schema: json!({}),
            handler_type: HANDLER_STATIC_JSON.to_string(),
            handler_config: Some(json!({"response":{"ok":true}})),
            unit_price_micros: 10,
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
        .create_user("expiry-developer@example.com", "secret1", None, None)
        .unwrap();
    let developer_project = store
        .create_project(&developer.id, "Expiry developer", None, None)
        .unwrap()
        .project;
    let credential = store
        .create_open_commerce_developer_app(
            &developer_project.id,
            &developer.id,
            CreateDeveloperAppRequest {
                app_id: "consumer.expiring".to_string(),
                display_name: "限时授权 App".to_string(),
            },
        )
        .unwrap();
    let authorization = open_commerce_consumer::create_authorization_request(
        &store,
        &developer.id,
        CreateAuthorizationRequest {
            merchant_id: merchant.id.clone(),
            requester_app_id: credential.app.app_id.clone(),
            scopes: vec!["menu.expiring".to_string()],
            purpose: "验证限时授权".to_string(),
        },
    )
    .unwrap();
    let expires_at = (Utc::now() + Duration::days(30)).to_rfc3339();
    let decision: DecideAuthorizationRequest = serde_json::from_value(json!({
        "reason":"期限和预算清晰",
        "expires_at":expires_at.clone(),
        "max_invocations":25,
        "max_amount_micros":500_000,
        "budget_currency":"CNY"
    }))
    .unwrap();
    let grant_request = grant_request_for_authorization(&authorization, &decision);
    assert_eq!(
        grant_request.expires_at.as_deref(),
        Some(expires_at.as_str())
    );
    let grant = open_commerce_service::create_grant(
        &store,
        &merchant_project.id,
        &merchant_actor,
        grant_request,
    )
    .unwrap();
    let approved = store
        .decide_open_commerce_authorization_request(
            &merchant_project.id,
            &authorization.id,
            &merchant_owner.id,
            "approved",
            &decision.reason,
            Some(&grant.id),
        )
        .unwrap();
    assert_eq!(approved.grant_expires_at, grant.expires_at);
    assert_eq!(approved.grant_max_invocations, Some(25));
    assert_eq!(approved.grant_max_amount_micros, Some(500_000));
    assert_eq!(approved.grant_budget_currency.as_deref(), Some("CNY"));
    assert_eq!(
        store
            .list_requester_project_open_commerce_authorization_requests(&developer_project.id, 10,)
            .unwrap()[0]
            .grant_expires_at,
        grant.expires_at
    );
    assert_eq!(
        store
            .active_open_commerce_grant_for_app_capability(
                &merchant.id,
                &credential.app.app_id,
                "menu.expiring",
            )
            .unwrap()
            .as_deref(),
        Some(grant.id.as_str())
    );

    store
        .conn()
        .unwrap()
        .execute(
            "UPDATE open_commerce_grants SET expires_at='2000-01-01T00:00:00Z' WHERE id=?1",
            rusqlite::params![grant.id],
        )
        .unwrap();
    assert!(store
        .active_open_commerce_grant_for_app_capability(
            &merchant.id,
            &credential.app.app_id,
            "menu.expiring",
        )
        .unwrap()
        .is_none());
    let developer_actor = OpenCommerceActor {
        user_id: &developer.id,
        app_id: &credential.app.app_id,
        project_role: None,
    };
    let error = open_commerce_service::invoke(
        &store,
        &developer_actor,
        InvokeCapabilityRequest {
            merchant_id: merchant.id,
            capability_key: "menu.expiring".to_string(),
            requester_app_id: credential.app.app_id.clone(),
            grant_id: Some(grant.id),
            idempotency_key: "expired-grant-call".to_string(),
            input: json!({}),
        },
    )
    .await
    .unwrap_err();
    assert!(error.to_string().contains("已过期"));
}
