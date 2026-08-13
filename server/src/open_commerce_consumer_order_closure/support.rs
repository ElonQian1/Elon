use serde_json::{json, Value};
use uuid::Uuid;

use crate::{
    open_commerce_business_handoff_model::RecordBusinessHandoffReceiptRequest,
    open_commerce_business_handoff_service,
    open_commerce_integration_model::CreateIntegrationRequest,
    open_commerce_merchant_evidence_model::BUSINESS_RECEIPT_SCHEMA,
    open_commerce_merchant_evidence_service,
    open_commerce_model::{
        CreateCapabilityRequest, CreateMerchantRequest, ACCESS_PUBLIC, HANDLER_MERCHANT_RUNTIME,
    },
    open_commerce_service::{self, OpenCommerceActor},
    store::{OpenCommerceInvocationStart, Store},
};

pub(super) struct Fixture {
    pub store: Store,
    pub owner_id: String,
    pub consumer_id: String,
    pub other_consumer_id: String,
    pub project_id: String,
    pub merchant_id: String,
    pub integration_id: String,
    pub capability_id: String,
    pub capability_key: String,
    pub invocation_id: String,
}

pub(super) fn fixture() -> Fixture {
    let path = std::env::temp_dir().join(format!(
        "elon_consumer_order_closure_{}.db",
        Uuid::new_v4().simple()
    ));
    let store = Store::open(&path).unwrap();
    let owner = create_user(&store, "order-closure-owner");
    let consumer = create_user(&store, "order-closure-consumer");
    let other_consumer = create_user(&store, "order-closure-other");
    let project = store
        .create_project(&owner.id, "Order Closure", None, None)
        .unwrap()
        .project;
    let actor = owner_actor(&owner.id);
    let merchant = open_commerce_service::create_merchant(
        &store,
        &project.id,
        &actor,
        CreateMerchantRequest {
            display_name: "闭环咖啡店".to_string(),
            slug: Some(format!("closure-{}", Uuid::new_v4().simple())),
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
            integration_key: "merchant.erp.primary".to_string(),
            provider_key: "merchant_erp".to_string(),
            display_name: "商户 ERP".to_string(),
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
    let invocation_id = start_invocation(
        &store,
        &project.id,
        &merchant.id,
        &capability.id,
        &capability.capability_key,
        &consumer.id,
        "consumer-order-1",
    );
    store
        .finish_open_commerce_invocation_success(
            &invocation_id,
            &json!({
                "order":{"id":"merchant-order-1001","items":[{"sku":"coffee-1","quantity":1}]},
                "_yilong_business_receipt":business_receipt("order")
            }),
        )
        .unwrap();

    Fixture {
        store,
        owner_id: owner.id,
        consumer_id: consumer.id,
        other_consumer_id: other_consumer.id,
        project_id: project.id,
        merchant_id: merchant.id,
        integration_id: integration.id,
        capability_id: capability.id,
        capability_key: capability.capability_key,
        invocation_id,
    }
}

pub(super) fn record_handoff(
    fixture: &Fixture,
    invocation_id: &str,
    receipt_key: &str,
    status: &str,
    completed_at: &str,
) {
    let evidence = open_commerce_merchant_evidence_service::get_evidence(
        &fixture.store,
        &fixture.project_id,
        &fixture.merchant_id,
        invocation_id,
    )
    .unwrap();
    let target_reference = (status == "applied").then(|| "erp-order-private-1001".to_string());
    let error_code = match status {
        "rejected" => Some("adapter_failed".to_string()),
        "ignored" => Some("already_present".to_string()),
        _ => None,
    };
    open_commerce_business_handoff_service::record_receipt(
        &fixture.store,
        &fixture.project_id,
        &owner_actor(&fixture.owner_id),
        RecordBusinessHandoffReceiptRequest {
            merchant_id: fixture.merchant_id.clone(),
            invocation_id: invocation_id.to_string(),
            integration_id: fixture.integration_id.clone(),
            receipt_key: receipt_key.to_string(),
            status: status.to_string(),
            target_domain: "erp".to_string(),
            evidence_result_sha256: evidence.evidence.result_sha256.unwrap(),
            target_reference,
            error_code,
            confirmed_by_user: true,
            completed_at: completed_at.to_string(),
        },
    )
    .unwrap();
}

pub(super) fn create_terminal_invocation(
    fixture: &Fixture,
    key: &str,
    result: Option<Value>,
    failed: bool,
) -> String {
    let id = start_invocation(
        &fixture.store,
        &fixture.project_id,
        &fixture.merchant_id,
        &fixture.capability_id,
        &fixture.capability_key,
        &fixture.consumer_id,
        key,
    );
    if failed {
        fixture
            .store
            .finish_open_commerce_invocation_failure(&id, "merchant_failed")
            .unwrap();
    } else {
        fixture
            .store
            .finish_open_commerce_invocation_success(&id, &result.unwrap())
            .unwrap();
    }
    id
}

pub(super) fn create_started_invocation(fixture: &Fixture, key: &str) -> String {
    start_invocation(
        &fixture.store,
        &fixture.project_id,
        &fixture.merchant_id,
        &fixture.capability_id,
        &fixture.capability_key,
        &fixture.consumer_id,
        key,
    )
}

pub(super) fn business_receipt(entity_type: &str) -> Value {
    json!({
        "schema":BUSINESS_RECEIPT_SCHEMA,
        "entity_type":entity_type,
        "reference_id":"merchant-order-1001",
        "state":"confirmed",
        "occurred_at":"2026-08-13T12:00:00Z",
        "amount_minor":2600,
        "currency":"CNY"
    })
}

fn owner_actor(user_id: &str) -> OpenCommerceActor<'_> {
    OpenCommerceActor {
        user_id,
        app_id: "pc-web",
        project_role: Some("owner"),
    }
}

fn create_user(store: &Store, prefix: &str) -> crate::store::PublicUser {
    store
        .create_user(
            &format!("{prefix}-{}@example.com", Uuid::new_v4().simple()),
            "secret1",
            Some(prefix),
            None,
        )
        .unwrap()
}

fn start_invocation(
    store: &Store,
    project_id: &str,
    merchant_id: &str,
    capability_id: &str,
    capability_key: &str,
    user_id: &str,
    idempotency_key: &str,
) -> String {
    store
        .start_open_commerce_invocation(OpenCommerceInvocationStart {
            project_id,
            merchant_id,
            capability_id,
            capability_key,
            requester_user_id: user_id,
            requester_app_id: "consumer.ai",
            grant_id: None,
            idempotency_key,
            request_hash: "consumer-order-request-sha256",
            request_shape: &json!({
                "input_fields":["quote_id"],
                "input_bytes":32,
                "contains_raw_values":false
            }),
            unit_price_micros: 1_000,
            currency: "CNY",
        })
        .unwrap()
        .invocation
        .id
}
