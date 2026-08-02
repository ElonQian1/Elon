use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::{
    open_commerce_directory_service,
    open_commerce_model::{
        CreateCapabilityRequest, CreateMerchantRequest, InvokeCapabilityRequest, ACCESS_PUBLIC,
        HANDLER_STATIC_JSON,
    },
    open_commerce_portability_model::{
        CreateConsumerPortabilityExportRequest, CONSUMER_PORTABILITY_EXPORT_SCHEMA,
        CONSUMER_PORTABILITY_EXPORT_SCHEMA_V2, CONSUMER_PORTABILITY_PAYLOAD_SCHEMA,
        CONSUMER_PORTABILITY_PAYLOAD_SCHEMA_V2,
    },
    open_commerce_portability_service,
    open_commerce_service::{self, OpenCommerceActor},
    store::Store,
};

#[tokio::test]
async fn v3_exports_verifiable_account_receipts_without_raw_inputs() {
    let fixture = fixture();
    invoke(&fixture, "portable-v3-call-one", "private-sku-one").await;
    let actor = fixture.consumer_actor();

    let first = export(&fixture, &actor, "portable-v3-export-one");
    assert_eq!(first.schema, CONSUMER_PORTABILITY_EXPORT_SCHEMA);
    assert_eq!(first.payload.schema, CONSUMER_PORTABILITY_PAYLOAD_SCHEMA);
    assert_eq!(
        first.payload.invocation_receipt_scope.as_deref(),
        Some("authenticated_user_account")
    );
    assert_eq!(first.payload.invocation_receipts.len(), 1);
    assert_eq!(first.summary().invocation_receipt_count, 1);
    let receipt = &first.payload.invocation_receipts[0];
    let receipt_payload: crate::open_commerce_consumer_receipt_model::ConsumerInvocationReceiptPayload =
        serde_json::from_str(&receipt.payload_json).unwrap();
    assert_eq!(
        receipt_payload.result.as_ref().unwrap()["offer_id"],
        "portable-result"
    );
    assert!(!receipt_payload.funds_moved);
    assert!(!receipt_payload.request_shape.contains_raw_values);
    assert_eq!(
        receipt.payload_sha256,
        hex::encode(Sha256::digest(receipt.payload_json.as_bytes()))
    );
    assert_eq!(
        first.payload_sha256,
        hex::encode(Sha256::digest(first.payload_json.as_bytes()))
    );
    assert_private_fields_absent(
        &serde_json::to_value(&first).unwrap(),
        &fixture.consumer_user_id,
    );

    invoke(&fixture, "portable-v3-call-two", "private-sku-two").await;
    let retry = export(&fixture, &actor, "portable-v3-export-one");
    assert_eq!(retry.id, first.id);
    assert_eq!(retry.payload_sha256, first.payload_sha256);
    assert_eq!(retry.payload.invocation_receipts.len(), 1);

    let second = export(&fixture, &actor, "portable-v3-export-two");
    assert_eq!(second.payload.invocation_receipts.len(), 2);
    assert_ne!(second.payload_sha256, first.payload_sha256);
}

#[tokio::test]
async fn v3_rejects_tampered_receipts_and_keeps_v2_bytes_verifiable() {
    let fixture = fixture();
    invoke(&fixture, "portable-v3-tamper-call", "private-sku-tamper").await;
    let actor = fixture.consumer_actor();
    let export = export(&fixture, &actor, "portable-v3-tamper-source");

    let mut tampered = serde_json::to_value(&export.payload).unwrap();
    tampered["invocation_receipts"][0]["payload_sha256"] = Value::String("0".repeat(64));
    let tampered_json = serde_json::to_string(&tampered).unwrap();
    let tampered_digest = hex::encode(Sha256::digest(tampered_json.as_bytes()));
    let (stored, _) = fixture
        .store
        .save_consumer_portability_export(
            &fixture.consumer_project_id,
            &fixture.consumer_user_id,
            "portable-v3-tampered",
            CONSUMER_PORTABILITY_EXPORT_SCHEMA,
            &tampered_json,
            &tampered_digest,
        )
        .unwrap();
    let error = open_commerce_portability_service::get_export(
        &fixture.store,
        &fixture.consumer_project_id,
        &stored.id,
        &actor,
    )
    .unwrap_err();
    assert!(error.to_string().contains("调用凭证完整性校验失败"));

    let generated_at = "2026-08-02T00:00:00+00:00";
    let v2_json = format!(
        "{{\"schema\":\"{CONSUMER_PORTABILITY_PAYLOAD_SCHEMA_V2}\",\"source_project_id\":\"{}\",\"generated_at\":\"{generated_at}\",\"relationships\":[],\"relationship_renewals\":[],\"data_requests\":[]}}",
        fixture.consumer_project_id
    );
    let v2_digest = hex::encode(Sha256::digest(v2_json.as_bytes()));
    let (legacy, _) = fixture
        .store
        .save_consumer_portability_export(
            &fixture.consumer_project_id,
            &fixture.consumer_user_id,
            "portable-legacy-v2",
            CONSUMER_PORTABILITY_EXPORT_SCHEMA_V2,
            &v2_json,
            &v2_digest,
        )
        .unwrap();
    let verified = open_commerce_portability_service::get_export(
        &fixture.store,
        &fixture.consumer_project_id,
        &legacy.id,
        &actor,
    )
    .unwrap();
    assert_eq!(verified.payload_json, v2_json);
    assert_eq!(serde_json::to_string(&verified.payload).unwrap(), v2_json);
    assert!(verified.payload.invocation_receipt_scope.is_none());
    assert!(verified.payload.invocation_receipts.is_empty());

    let (mixed, _) = fixture
        .store
        .save_consumer_portability_export(
            &fixture.consumer_project_id,
            &fixture.consumer_user_id,
            "portable-v3-mixed-v2",
            CONSUMER_PORTABILITY_EXPORT_SCHEMA,
            &v2_json,
            &v2_digest,
        )
        .unwrap();
    let mixed_error = open_commerce_portability_service::get_export(
        &fixture.store,
        &fixture.consumer_project_id,
        &mixed.id,
        &actor,
    )
    .unwrap_err();
    assert!(mixed_error.to_string().contains("版本不受支持"));
}

struct Fixture {
    store: Store,
    merchant_id: String,
    consumer_user_id: String,
    consumer_project_id: String,
}

impl Fixture {
    fn consumer_actor(&self) -> OpenCommerceActor<'_> {
        OpenCommerceActor {
            user_id: &self.consumer_user_id,
            app_id: "pc-web",
            project_role: Some("owner"),
        }
    }
}

fn fixture() -> Fixture {
    let path = std::env::temp_dir().join(format!(
        "elon-open-commerce-portability-v3-{}.sqlite",
        Uuid::new_v4().simple()
    ));
    let store = Store::open(&path).unwrap();
    let merchant_owner = store
        .create_user(
            &format!(
                "portable-v3-merchant-{}@example.com",
                Uuid::new_v4().simple()
            ),
            "secret1",
            None,
            None,
        )
        .unwrap();
    let merchant_project = store
        .create_project(&merchant_owner.id, "Portable V3 merchant", None, None)
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
            display_name: "可携带调用凭证测试商户".to_string(),
            slug: Some(format!("portable-v3-{}", Uuid::new_v4().simple())),
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
            capability_key: "offer.portable".to_string(),
            display_name: "可携带报价".to_string(),
            description: String::new(),
            kind: "query".to_string(),
            access_level: ACCESS_PUBLIC.to_string(),
            input_schema: json!({
                "type":"object",
                "required":["sku"],
                "properties":{"sku":{"type":"string"}},
                "additionalProperties":false
            }),
            output_schema: json!({
                "type":"object",
                "required":["offer_id"],
                "properties":{"offer_id":{"const":"portable-result"}},
                "additionalProperties":false
            }),
            handler_type: HANDLER_STATIC_JSON.to_string(),
            handler_config: Some(json!({"response":{"offer_id":"portable-result"}})),
            unit_price_micros: 700,
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

    let consumer = store
        .create_user(
            &format!(
                "portable-v3-consumer-{}@example.com",
                Uuid::new_v4().simple()
            ),
            "secret1",
            None,
            None,
        )
        .unwrap();
    let consumer_project = store
        .create_project(&consumer.id, "Portable V3 wallet", None, None)
        .unwrap()
        .project;
    Fixture {
        store,
        merchant_id: merchant.id,
        consumer_user_id: consumer.id,
        consumer_project_id: consumer_project.id,
    }
}

async fn invoke(fixture: &Fixture, idempotency_key: &str, private_sku: &str) {
    open_commerce_service::invoke(
        &fixture.store,
        &OpenCommerceActor {
            user_id: &fixture.consumer_user_id,
            app_id: "pc-web",
            project_role: None,
        },
        InvokeCapabilityRequest {
            merchant_id: fixture.merchant_id.clone(),
            capability_key: "offer.portable".to_string(),
            requester_app_id: "pc-web".to_string(),
            grant_id: None,
            idempotency_key: idempotency_key.to_string(),
            input: json!({"sku": private_sku}),
        },
    )
    .await
    .unwrap();
}

fn export(
    fixture: &Fixture,
    actor: &OpenCommerceActor<'_>,
    idempotency_key: &str,
) -> crate::open_commerce_portability_model::ConsumerPortabilityExport {
    open_commerce_portability_service::create_export(
        &fixture.store,
        &fixture.consumer_project_id,
        actor,
        CreateConsumerPortabilityExportRequest {
            idempotency_key: idempotency_key.to_string(),
        },
    )
    .unwrap()
}

fn assert_private_fields_absent(value: &Value, user_id: &str) {
    let serialized = value.to_string();
    for forbidden in [
        user_id,
        "private-sku-one",
        "portable-v3-call-one",
        "request_hash",
        "requester_user_id",
        "grant_id",
        "capability_id",
    ] {
        assert!(!serialized.contains(forbidden), "leaked {forbidden}");
    }
}
