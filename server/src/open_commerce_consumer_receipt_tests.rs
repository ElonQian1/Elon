use serde_json::json;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::{
    open_commerce_consumer_receipt_mcp, open_commerce_consumer_receipt_service,
    open_commerce_directory_service,
    open_commerce_model::{
        CreateCapabilityRequest, CreateMerchantRequest, InvokeCapabilityRequest, ACCESS_PUBLIC,
        HANDLER_STATIC_JSON,
    },
    open_commerce_service::{self, OpenCommerceActor},
    store::{OpenCommerceInvocationStart, Store},
};

#[tokio::test]
async fn receipts_are_terminal_private_and_digest_verifiable() {
    let fixture = fixture();
    let consumer_actor = OpenCommerceActor {
        user_id: &fixture.consumer_id,
        app_id: "pc-web",
        project_role: None,
    };
    let invoked = open_commerce_service::invoke(
        &fixture.store,
        &consumer_actor,
        InvokeCapabilityRequest {
            merchant_id: fixture.merchant_id.clone(),
            capability_key: "order.receipt.demo".to_string(),
            requester_app_id: "pc-web".to_string(),
            grant_id: None,
            idempotency_key: "consumer-receipt-1".to_string(),
            input: json!({}),
        },
    )
    .await
    .unwrap();
    let invocation_id = invoked["invocation_id"].as_str().unwrap().to_string();

    let started = fixture
        .store
        .start_open_commerce_invocation(OpenCommerceInvocationStart {
            project_id: &fixture.merchant_project_id,
            merchant_id: &fixture.merchant_id,
            capability_id: &fixture.capability_id,
            capability_key: "order.receipt.demo",
            requester_user_id: &fixture.consumer_id,
            requester_app_id: "pc-web",
            grant_id: None,
            idempotency_key: "consumer-receipt-started",
            request_hash: "started-request-hash",
            request_shape: &json!({"input_fields":[],"input_bytes":2,"contains_raw_values":false}),
            unit_price_micros: 700,
            currency: "CNY",
        })
        .unwrap();

    let list = open_commerce_consumer_receipt_service::list_receipts(
        &fixture.store,
        &fixture.consumer_id,
        100,
    )
    .unwrap();
    assert_eq!(list.scope, "authenticated_user_account");
    assert_eq!(list.receipts.len(), 1);
    assert_eq!(list.receipts[0].invocation_id, invocation_id);
    assert!(list.receipts[0].result_available);
    assert!(!serde_json::to_string(&list)
        .unwrap()
        .contains("consumer-private-order-result"));

    let receipt = open_commerce_consumer_receipt_service::get_receipt(
        &fixture.store,
        &fixture.consumer_id,
        &invocation_id,
    )
    .unwrap();
    assert_eq!(
        receipt.payload.result.as_ref().unwrap()["order_id"],
        "consumer-private-order-result"
    );
    assert!(!receipt.payload.funds_moved);
    assert_eq!(
        receipt.payload_sha256,
        hex::encode(Sha256::digest(receipt.payload_json.as_bytes()))
    );
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&receipt.payload_json).unwrap(),
        serde_json::to_value(&receipt.payload).unwrap()
    );
    assert_eq!(
        receipt.payload.request_shape.input_fields,
        Vec::<String>::new()
    );
    assert!(!receipt.payload.request_shape.contains_raw_values);
    assert!(open_commerce_consumer_receipt_service::get_receipt(
        &fixture.store,
        &fixture.consumer_id,
        &started.invocation.id,
    )
    .unwrap_err()
    .to_string()
    .contains("不存在"));
    let serialized = serde_json::to_string(&receipt).unwrap();
    for forbidden in [
        "request_hash",
        "idempotency_key",
        "requester_user_id",
        "grant_id",
        "capability_id",
        "project_id",
    ] {
        assert!(!serialized.contains(forbidden), "leaked {forbidden}");
    }

    let other = open_commerce_consumer_receipt_service::list_receipts(
        &fixture.store,
        &fixture.other_consumer_id,
        100,
    )
    .unwrap();
    assert!(other.receipts.is_empty());
    assert!(open_commerce_consumer_receipt_service::get_receipt(
        &fixture.store,
        &fixture.other_consumer_id,
        &invocation_id,
    )
    .unwrap_err()
    .to_string()
    .contains("不存在"));

    fixture
        .store
        .conn()
        .unwrap()
        .execute(
            "UPDATE open_commerce_invocations SET settlement_status = 'charged' WHERE id = ?1",
            [&invocation_id],
        )
        .unwrap();
    assert!(open_commerce_consumer_receipt_service::get_receipt(
        &fixture.store,
        &fixture.consumer_id,
        &invocation_id,
    )
    .unwrap_err()
    .to_string()
    .contains("不能生成未扣真实资金"));
}

#[tokio::test]
async fn mcp_reads_only_the_current_users_receipts() {
    let fixture = fixture();
    let actor = OpenCommerceActor {
        user_id: &fixture.consumer_id,
        app_id: "pc-web",
        project_role: None,
    };
    open_commerce_service::invoke(
        &fixture.store,
        &actor,
        InvokeCapabilityRequest {
            merchant_id: fixture.merchant_id.clone(),
            capability_key: "order.receipt.demo".to_string(),
            requester_app_id: "pc-web".to_string(),
            grant_id: None,
            idempotency_key: "consumer-receipt-mcp".to_string(),
            input: json!({}),
        },
    )
    .await
    .unwrap();

    let definitions = open_commerce_consumer_receipt_mcp::definitions();
    assert!(definitions
        .iter()
        .any(|tool| tool["name"] == "open_commerce_list_my_invocation_receipts"));
    let mine = open_commerce_consumer_receipt_mcp::call_if_handled(
        &fixture.store,
        &fixture.consumer_id,
        "open_commerce_list_my_invocation_receipts",
        json!({}),
    )
    .unwrap()
    .unwrap();
    assert_eq!(mine["receipts"].as_array().unwrap().len(), 1);
    let other = open_commerce_consumer_receipt_mcp::call_if_handled(
        &fixture.store,
        &fixture.other_consumer_id,
        "open_commerce_list_my_invocation_receipts",
        json!({}),
    )
    .unwrap()
    .unwrap();
    assert!(other["receipts"].as_array().unwrap().is_empty());
    assert!(open_commerce_consumer_receipt_mcp::call_if_handled(
        &fixture.store,
        &fixture.consumer_id,
        "open_commerce_list_my_invocation_receipts",
        json!({"user_id": fixture.other_consumer_id}),
    )
    .unwrap_err()
    .to_string()
    .contains("参数无效"));
}

struct ReceiptFixture {
    store: Store,
    merchant_project_id: String,
    merchant_id: String,
    capability_id: String,
    consumer_id: String,
    other_consumer_id: String,
}

fn fixture() -> ReceiptFixture {
    let path = std::env::temp_dir().join(format!(
        "elon_open_commerce_receipt_{}.db",
        Uuid::new_v4().simple()
    ));
    let store = Store::open(&path).unwrap();
    let owner = create_user(&store, "receipt-owner");
    let consumer = create_user(&store, "receipt-consumer");
    let other_consumer = create_user(&store, "receipt-other");
    let project = store
        .create_project(&owner.id, "Receipt Merchant", None, None)
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
            display_name: "调用凭证测试商户".to_string(),
            slug: Some(format!("receipt-{}", Uuid::new_v4().simple())),
            description: String::new(),
            node_mode: "platform_hosted".to_string(),
            public_profile: json!({}),
        },
    )
    .unwrap();
    let capability = open_commerce_service::publish_capability(
        &store,
        &project.id,
        &merchant.id,
        &owner_actor,
        CreateCapabilityRequest {
            capability_key: "order.receipt.demo".to_string(),
            display_name: "演示订单".to_string(),
            description: String::new(),
            kind: "query".to_string(),
            access_level: ACCESS_PUBLIC.to_string(),
            input_schema: json!({"type":"object","additionalProperties":false}),
            output_schema: json!({
                "type":"object",
                "required":["order_id","status"],
                "properties":{
                    "order_id":{"type":"string"},
                    "status":{"const":"created"}
                },
                "additionalProperties":false
            }),
            handler_type: HANDLER_STATIC_JSON.to_string(),
            handler_config: Some(json!({
                "response":{"order_id":"consumer-private-order-result","status":"created"}
            })),
            unit_price_micros: 700,
            currency: "CNY".to_string(),
            freshness_seconds: 0,
        },
    )
    .unwrap();
    open_commerce_directory_service::set_publication(
        &store,
        &project.id,
        &merchant.id,
        &owner_actor,
        true,
    )
    .unwrap();
    ReceiptFixture {
        store,
        merchant_project_id: project.id,
        merchant_id: merchant.id,
        capability_id: capability.id,
        consumer_id: consumer.id,
        other_consumer_id: other_consumer.id,
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
