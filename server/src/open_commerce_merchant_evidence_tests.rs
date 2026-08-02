use serde_json::{json, Value};
use uuid::Uuid;

use crate::{
    open_commerce_merchant_evidence_model::{
        validate_optional_business_receipt, BUSINESS_RECEIPT_SCHEMA,
    },
    open_commerce_merchant_evidence_service,
    open_commerce_model::{
        CreateCapabilityRequest, CreateMerchantRequest, ACCESS_PUBLIC, HANDLER_MERCHANT_RUNTIME,
    },
    open_commerce_service::{self, OpenCommerceActor},
    store::{OpenCommerceInvocationStart, Store},
};

fn temp_store() -> Store {
    let path = std::env::temp_dir().join(format!(
        "elon_open_commerce_merchant_evidence_{}.db",
        Uuid::new_v4().simple()
    ));
    Store::open(&path).expect("merchant evidence test store should open")
}

fn valid_receipt() -> Value {
    json!({
        "schema":BUSINESS_RECEIPT_SCHEMA,
        "entity_type":"order",
        "reference_id":"order-coffee-1001",
        "state":"confirmed",
        "occurred_at":"2026-08-03T01:00:00Z",
        "amount_minor":2600,
        "currency":"CNY"
    })
}

#[test]
fn optional_business_receipt_is_strict_and_money_is_atomic() {
    let result = json!({"_yilong_business_receipt":valid_receipt()});
    let receipt = validate_optional_business_receipt(&result)
        .unwrap()
        .unwrap();
    assert_eq!(receipt.reference_id, "order-coffee-1001");

    let missing_currency = json!({
        "_yilong_business_receipt":{
            "schema":BUSINESS_RECEIPT_SCHEMA,
            "entity_type":"order",
            "reference_id":"order-2",
            "state":"confirmed",
            "occurred_at":"2026-08-03T01:00:00Z",
            "amount_minor":2600
        }
    });
    assert!(validate_optional_business_receipt(&missing_currency)
        .unwrap_err()
        .to_string()
        .contains("同时提供"));
    let unknown_field = json!({
        "_yilong_business_receipt":{
            "schema":BUSINESS_RECEIPT_SCHEMA,
            "entity_type":"order",
            "reference_id":"order-3",
            "state":"confirmed",
            "occurred_at":"2026-08-03T01:00:00Z",
            "merchant_note":"must not leak into the protocol"
        }
    });
    assert!(validate_optional_business_receipt(&unknown_field).is_err());
    assert!(validate_optional_business_receipt(&json!({"items":[]}))
        .unwrap()
        .is_none());
}

#[tokio::test]
async fn merchant_evidence_projects_terminal_result_without_claiming_payment() {
    let store = temp_store();
    let owner = store
        .create_user(
            "merchant-evidence@example.com",
            "secret1",
            Some("Merchant Evidence"),
            None,
        )
        .unwrap();
    let project = store
        .create_project(&owner.id, "Merchant Evidence", None, None)
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
            display_name: "证据咖啡店".to_string(),
            slug: Some("evidence-cafe".to_string()),
            description: String::new(),
            node_mode: "self_hosted".to_string(),
            public_profile: json!({"category":"coffee"}),
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
    let request_shape = json!({
        "input_fields":["quote_id"],
        "input_bytes":32,
        "contains_raw_values":false
    });
    let claim = store
        .start_open_commerce_invocation(OpenCommerceInvocationStart {
            project_id: &project.id,
            merchant_id: &merchant.id,
            capability_id: &capability.id,
            capability_key: &capability.capability_key,
            requester_user_id: &owner.id,
            requester_app_id: "consumer.ai",
            grant_id: None,
            idempotency_key: "merchant-evidence-order-1",
            request_hash: "request-sha256",
            request_shape: &request_shape,
            unit_price_micros: capability.unit_price_micros,
            currency: &capability.currency,
        })
        .unwrap();
    store
        .finish_open_commerce_invocation_success(
            &claim.invocation.id,
            &json!({
                "order":{"id":"order-coffee-1001"},
                "_yilong_business_receipt":valid_receipt()
            }),
        )
        .unwrap();

    let list = open_commerce_merchant_evidence_service::list_evidence(
        &store,
        &project.id,
        &merchant.id,
        50,
    )
    .unwrap();
    assert_eq!(list.evidence.len(), 1);
    assert!(list.erp_binding.is_none());
    assert_eq!(list.evidence[0].receipt_state, "valid");
    assert_eq!(
        list.evidence[0]
            .business_receipt
            .as_ref()
            .unwrap()
            .reference_id,
        "order-coffee-1001"
    );
    assert_eq!(list.evidence[0].result_sha256.as_ref().unwrap().len(), 64);
    assert!(!list.evidence[0].funds_moved);

    let detail = open_commerce_merchant_evidence_service::get_evidence(
        &store,
        &project.id,
        &merchant.id,
        &claim.invocation.id,
    )
    .unwrap();
    assert_eq!(detail.result.unwrap()["order"]["id"], "order-coffee-1001");
    let mcp = crate::open_commerce_mcp::call_tool(
        &store,
        &project.id,
        &owner.id,
        "owner",
        "pc-web",
        json!({
            "name":"open_commerce_list_merchant_business_evidence",
            "arguments":{"merchant_id":merchant.id}
        }),
    )
    .await
    .unwrap();
    assert_eq!(
        mcp["structuredContent"]["evidence"][0]["receipt_state"],
        "valid"
    );
    let invalid_claim = store
        .start_open_commerce_invocation(OpenCommerceInvocationStart {
            project_id: &project.id,
            merchant_id: &merchant.id,
            capability_id: &capability.id,
            capability_key: &capability.capability_key,
            requester_user_id: &owner.id,
            requester_app_id: "consumer.ai",
            grant_id: None,
            idempotency_key: "merchant-evidence-order-legacy",
            request_hash: "legacy-request-sha256",
            request_shape: &request_shape,
            unit_price_micros: capability.unit_price_micros,
            currency: &capability.currency,
        })
        .unwrap();
    store
        .finish_open_commerce_invocation_success(
            &invalid_claim.invocation.id,
            &json!({
                    "order":{"id":"legacy-order"},
            "_yilong_business_receipt":{
                "schema":BUSINESS_RECEIPT_SCHEMA,
                "entity_type":"Order",
                "reference_id":"legacy-order",
                "state":"confirmed",
                "occurred_at":"not-a-time"
            }
                }),
        )
        .unwrap();

    let historical = open_commerce_merchant_evidence_service::list_evidence(
        &store,
        &project.id,
        &merchant.id,
        50,
    )
    .unwrap();
    let invalid_evidence = historical
        .evidence
        .iter()
        .find(|item| item.invocation_id == invalid_claim.invocation.id)
        .unwrap();
    assert_eq!(invalid_evidence.receipt_state, "invalid_legacy");
    assert!(invalid_evidence.business_receipt.is_none());
}
