use std::sync::Arc;

use axum::{
    body::{to_bytes, Body},
    http::{header, Method, Request, StatusCode},
    Router,
};
use serde_json::{json, Value};
use tower::ServiceExt;
use uuid::Uuid;

use super::routes;
use crate::{
    open_commerce_adapter_service,
    open_commerce_developer_production_test_support::test_app_state,
    open_commerce_integration_model::CreateIntegrationRequest,
    open_commerce_merchant_evidence_model::BUSINESS_RECEIPT_SCHEMA,
    open_commerce_model::{
        CreateCapabilityRequest, CreateMerchantRequest, ACCESS_PUBLIC, HANDLER_MERCHANT_RUNTIME,
    },
    open_commerce_service::{self, OpenCommerceActor},
    store::{OpenCommerceInvocationStart, Store},
    types::AppState,
};

pub(super) struct ApiFixture {
    pub(super) state: Arc<AppState>,
    router: Router,
    pub(super) project_id: String,
    owner_user_id: String,
    integration_id: String,
    pub(super) owner_token: String,
    outsider_token: String,
    write_only_token: String,
}

#[tokio::test]
async fn claim_http_requires_explicit_scope_and_issues_only_one_lease() {
    let fixture = fixture();
    let claim_path = "/api/open-commerce/adapter/business-handoff-claims";

    assert_eq!(
        call(&fixture.router, Method::POST, claim_path, None, json!({}))
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        call(
            &fixture.router,
            Method::POST,
            claim_path,
            Some(&fixture.write_only_token),
            json!({"lease_seconds":300}),
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );

    let claim_token = rotate_claim_token(&fixture);
    let (claim_status, claimed) = call(
        &fixture.router,
        Method::POST,
        claim_path,
        Some(&claim_token),
        json!({"lease_seconds":300}),
    )
    .await;
    assert_eq!(claim_status, StatusCode::OK, "{claimed}");
    assert_eq!(claimed["claimed"], true);
    assert_eq!(claimed["issue"]["claim"]["attempt_no"], 1);
    assert_eq!(claimed["issue"]["lease_token_visible_once"], true);
    let lease_token = claimed["issue"]["lease_token"].as_str().unwrap();
    assert!(lease_token.starts_with("oc_claim_"));
    assert_eq!(
        claimed["issue"]["task"]["result"]["order"]["id"],
        "merchant-order-http-1"
    );
    assert_no_credential_secrets(&claimed);
    assert!(!claimed["issue"]["claim"].to_string().contains(lease_token));

    let (empty_status, empty) = call(
        &fixture.router,
        Method::POST,
        claim_path,
        Some(&claim_token),
        json!({"lease_seconds":300}),
    )
    .await;
    assert_eq!(empty_status, StatusCode::OK, "{empty}");
    assert_eq!(empty["claimed"], false);
    assert!(empty["issue"].is_null());
}

#[tokio::test]
async fn claim_http_renews_releases_reclaims_and_completes_idempotently() {
    let fixture = fixture();
    let claim_token = rotate_claim_token(&fixture);
    let (_, first) = call(
        &fixture.router,
        Method::POST,
        "/api/open-commerce/adapter/business-handoff-claims",
        Some(&claim_token),
        json!({"lease_seconds":60}),
    )
    .await;
    let first_id = first["issue"]["claim"]["id"].as_str().unwrap();
    let first_lease = first["issue"]["lease_token"].as_str().unwrap();

    let renew_path = format!("/api/open-commerce/adapter/business-handoff-claims/{first_id}/renew");
    let (renew_status, renewed) = call(
        &fixture.router,
        Method::POST,
        &renew_path,
        Some(&claim_token),
        json!({"lease_token":first_lease,"extend_seconds":300}),
    )
    .await;
    assert_eq!(renew_status, StatusCode::OK, "{renewed}");
    assert_eq!(renewed["renewed"], true);

    let release_path =
        format!("/api/open-commerce/adapter/business-handoff-claims/{first_id}/release");
    let (release_status, released) = call(
        &fixture.router,
        Method::POST,
        &release_path,
        Some(&claim_token),
        json!({"lease_token":first_lease,"reason_code":"capacity_pressure"}),
    )
    .await;
    assert_eq!(release_status, StatusCode::OK, "{released}");
    assert_eq!(released["claim"]["status"], "released");
    assert_eq!(released["retryable"], true);

    let (_, second) = call(
        &fixture.router,
        Method::POST,
        "/api/open-commerce/adapter/business-handoff-claims",
        Some(&claim_token),
        json!({"lease_seconds":300}),
    )
    .await;
    let second_id = second["issue"]["claim"]["id"].as_str().unwrap();
    let second_lease = second["issue"]["lease_token"].as_str().unwrap();
    assert_eq!(second["issue"]["claim"]["attempt_no"], 2);
    assert_ne!(second_id, first_id);

    let complete_path =
        format!("/api/open-commerce/adapter/business-handoff-claims/{second_id}/complete");
    let completion = json!({
        "lease_token":second_lease,
        "receipt_key":"http-claim-applied",
        "status":"applied",
        "target_domain":"erp",
        "target_reference":"erp-order-http-1",
        "completed_at":"2026-08-11T00:00:00Z"
    });
    let (complete_status, completed) = call(
        &fixture.router,
        Method::POST,
        &complete_path,
        Some(&claim_token),
        completion.clone(),
    )
    .await;
    assert_eq!(complete_status, StatusCode::OK, "{completed}");
    assert_eq!(completed["status"], "applied");
    assert_eq!(completed["funds_moved"], false);
    let receipt_id = completed["id"].clone();
    let (repeat_status, repeated) = call(
        &fixture.router,
        Method::POST,
        &complete_path,
        Some(&claim_token),
        completion,
    )
    .await;
    assert_eq!(repeat_status, StatusCode::OK, "{repeated}");
    assert_eq!(repeated["id"], receipt_id);

    let list_path = format!(
        "/api/projects/{}/open-commerce/adapter-handoff-claims",
        fixture.project_id
    );
    assert_eq!(
        call(
            &fixture.router,
            Method::GET,
            &list_path,
            Some(&fixture.outsider_token),
            Value::Null,
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    let (list_status, list) = call(
        &fixture.router,
        Method::GET,
        &list_path,
        Some(&fixture.owner_token),
        Value::Null,
    )
    .await;
    assert_eq!(list_status, StatusCode::OK, "{list}");
    assert_eq!(list["claims"].as_array().unwrap().len(), 2);
    assert_no_credential_secrets(&list);
    assert!(!list.to_string().contains(second_lease));
}

pub(super) fn rotate_claim_token(fixture: &ApiFixture) -> String {
    open_commerce_adapter_service::rotate_credential(
        &fixture.state.store,
        &fixture.project_id,
        &fixture.integration_id,
        90,
        true,
        &owner_actor(&fixture.owner_user_id),
    )
    .unwrap()
    .adapter_token
}

fn assert_no_credential_secrets(value: &Value) {
    let serialized = value.to_string();
    assert!(!serialized.contains("adapter_token"));
    assert!(!serialized.contains("token_hash"));
}

async fn call(
    router: &Router,
    method: Method,
    path: &str,
    bearer: Option<&str>,
    body: Value,
) -> (StatusCode, Value) {
    let mut builder = Request::builder().method(method).uri(path);
    if let Some(token) = bearer {
        builder = builder.header(header::AUTHORIZATION, format!("Bearer {token}"));
    }
    let body = if body.is_null() {
        Body::empty()
    } else {
        builder = builder.header(header::CONTENT_TYPE, "application/json");
        Body::from(body.to_string())
    };
    let response = router
        .clone()
        .oneshot(builder.body(body).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, body)
}

pub(super) fn fixture() -> ApiFixture {
    let path = std::env::temp_dir().join(format!(
        "elon_open_commerce_adapter_claim_api_{}.db",
        Uuid::new_v4().simple()
    ));
    let store = Store::open(&path).unwrap();
    let owner = store
        .create_user("adapter-claim-api@example.com", "secret1", None, None)
        .unwrap();
    let outsider = store
        .create_user("adapter-claim-outsider@example.com", "secret1", None, None)
        .unwrap();
    let project = store
        .create_project(&owner.id, "Adapter Claim API", None, None)
        .unwrap()
        .project;
    let actor = owner_actor(&owner.id);
    let merchant = open_commerce_service::create_merchant(
        &store,
        &project.id,
        &actor,
        CreateMerchantRequest {
            display_name: "HTTP 租约咖啡店".to_string(),
            slug: Some("adapter-claim-http-cafe".to_string()),
            description: String::new(),
            node_mode: "self_hosted".to_string(),
            public_profile: json!({"category":"coffee"}),
        },
    )
    .unwrap();
    let integration = open_commerce_service::create_integration(
        &store,
        &project.id,
        &actor,
        CreateIntegrationRequest {
            merchant_id: merchant.id.clone(),
            integration_key: "merchant.erp.claim.http".to_string(),
            provider_key: "merchant_erp".to_string(),
            display_name: "HTTP ERP 租约适配器".to_string(),
            connection_mode: "local_adapter".to_string(),
            scopes: vec!["orders.write".to_string()],
            data_domains: vec!["orders".to_string()],
        },
    )
    .unwrap();
    let capability = store
        .create_open_commerce_capability(
            &project.id,
            &merchant.id,
            CreateCapabilityRequest {
                capability_key: "order.commit".to_string(),
                display_name: "提交订单".to_string(),
                description: String::new(),
                kind: "action".to_string(),
                access_level: ACCESS_PUBLIC.to_string(),
                input_schema: json!({"type":"object"}),
                output_schema: json!({"type":"object"}),
                handler_type: HANDLER_MERCHANT_RUNTIME.to_string(),
                handler_config: None,
                unit_price_micros: 1_000,
                currency: "CNY".to_string(),
                freshness_seconds: 0,
            },
        )
        .unwrap();
    let invocation_id = store
        .start_open_commerce_invocation(OpenCommerceInvocationStart {
            project_id: &project.id,
            merchant_id: &merchant.id,
            capability_id: &capability.id,
            capability_key: &capability.capability_key,
            requester_user_id: &owner.id,
            requester_app_id: "consumer.ai",
            grant_id: None,
            idempotency_key: "adapter-claim-http-order-1",
            request_hash: "adapter-claim-http-request-hash",
            request_shape: &json!({"keys":[]}),
            unit_price_micros: capability.unit_price_micros,
            currency: &capability.currency,
        })
        .unwrap()
        .invocation
        .id;
    store
        .finish_open_commerce_invocation_success(
            &invocation_id,
            &json!({
                "order":{"id":"merchant-order-http-1"},
                "_yilong_business_receipt":{
                    "schema":BUSINESS_RECEIPT_SCHEMA,
                    "entity_type":"order",
                    "reference_id":"merchant-order-http-1",
                    "state":"accepted",
                    "occurred_at":"2026-08-11T00:00:00Z",
                    "amount_minor":3600,
                    "currency":"CNY"
                }
            }),
        )
        .unwrap();
    let write_only_token = open_commerce_adapter_service::rotate_credential(
        &store,
        &project.id,
        &integration.id,
        90,
        false,
        &actor,
    )
    .unwrap()
    .adapter_token;
    let owner_token = session(&store, &owner.id);
    let outsider_token = session(&store, &outsider.id);
    let state = Arc::new(test_app_state(store, &std::env::temp_dir()));
    let router = routes().with_state(state.clone());

    ApiFixture {
        state,
        router,
        project_id: project.id,
        owner_user_id: owner.id,
        integration_id: integration.id,
        owner_token,
        outsider_token,
        write_only_token,
    }
}

fn session(store: &Store, user_id: &str) -> String {
    store
        .create_session(user_id, Some("adapter-claim-api-test"), None)
        .unwrap()
        .0
}

fn owner_actor(user_id: &str) -> OpenCommerceActor<'_> {
    OpenCommerceActor {
        user_id,
        app_id: "pc-web",
        project_role: Some("owner"),
    }
}
