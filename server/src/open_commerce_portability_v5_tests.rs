use chrono::{Duration, Utc};
use serde_json::json;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::{
    open_commerce_data_erasure_evidence_model::CreateDataErasureEvidenceRequest,
    open_commerce_data_erasure_evidence_service,
    open_commerce_data_request_model::{
        CreateConsumerDataErasureRequest, DecideConsumerDataRequest,
    },
    open_commerce_data_request_service, open_commerce_directory_service,
    open_commerce_model::{
        CreateCapabilityRequest, CreateMerchantRequest, ACCESS_PUBLIC, HANDLER_STATIC_JSON,
    },
    open_commerce_portability_model::{
        ConsumerPortabilityExport, CreateConsumerPortabilityExportRequest,
        CONSUMER_PORTABILITY_EXPORT_SCHEMA, CONSUMER_PORTABILITY_EXPORT_SCHEMA_V4,
        CONSUMER_PORTABILITY_PAYLOAD_SCHEMA, CONSUMER_PORTABILITY_PAYLOAD_SCHEMA_V4,
    },
    open_commerce_portability_service,
    open_commerce_relationship_model::{
        CreateConsumerRelationshipRequest, RELATIONSHIP_SCOPE_PREFERENCE_REMEMBER,
    },
    open_commerce_relationship_service,
    open_commerce_service::{self, OpenCommerceActor},
    store::Store,
};

#[test]
fn v5_export_captures_unverified_erasure_evidence_idempotently() {
    let fixture = fixture(true);
    let actor = fixture.consumer_actor();
    let first = open_commerce_portability_service::create_export(
        &fixture.store,
        &fixture.consumer_project_id,
        &actor,
        export_request("v5-evidence-export"),
    )
    .unwrap();
    let replay = open_commerce_portability_service::create_export(
        &fixture.store,
        &fixture.consumer_project_id,
        &actor,
        export_request("v5-evidence-export"),
    )
    .unwrap();

    assert_eq!(first.id, replay.id);
    assert_eq!(first.schema, CONSUMER_PORTABILITY_EXPORT_SCHEMA);
    assert_eq!(first.payload.schema, CONSUMER_PORTABILITY_PAYLOAD_SCHEMA);
    assert_eq!(first.payload.data_requests.len(), 1);
    assert_eq!(first.payload.data_requests[0].status, "completed");
    assert_eq!(first.payload.data_erasure_evidence.len(), 1);
    let evidence = &first.payload.data_erasure_evidence[0];
    assert_eq!(evidence.data_request_id, fixture.request_id);
    assert_eq!(evidence.merchant_id, fixture.merchant_id);
    assert_eq!(evidence.source_authority, "merchant_supplied_unverified");
    assert!(!evidence.platform_verified);
    assert!(first.payload_json.contains("data_erasure_evidence"));
    assert!(!first.payload_json.contains("submitted_by_user_id"));
    assert_eq!(
        first.payload_sha256,
        hex::encode(Sha256::digest(first.payload_json.as_bytes()))
    );

    let summaries = open_commerce_portability_service::list_exports(
        &fixture.store,
        &fixture.consumer_project_id,
        &actor,
        20,
    )
    .unwrap();
    assert_eq!(summaries.len(), 1);
    assert_eq!(summaries[0].data_erasure_evidence_count, 1);
}

#[test]
fn v5_omits_empty_evidence_without_changing_integrity() {
    let fixture = fixture(false);
    let export = open_commerce_portability_service::create_export(
        &fixture.store,
        &fixture.consumer_project_id,
        &fixture.consumer_actor(),
        export_request("v5-empty-evidence"),
    )
    .unwrap();

    assert!(export.payload.data_erasure_evidence.is_empty());
    assert!(!export.payload_json.contains("data_erasure_evidence"));
    open_commerce_portability_service::verify_external_export(export).unwrap();
}

#[test]
fn v5_semantic_tampering_and_legacy_field_smuggling_fail_closed() {
    let fixture = fixture(true);
    let export = open_commerce_portability_service::create_export(
        &fixture.store,
        &fixture.consumer_project_id,
        &fixture.consumer_actor(),
        export_request("v5-tamper-cases"),
    )
    .unwrap();

    let mut platform_verified = export.clone();
    platform_verified.payload.data_erasure_evidence[0].platform_verified = true;
    reseal(&mut platform_verified);
    assert!(open_commerce_portability_service::verify_external_export(platform_verified).is_err());

    let mut incomplete_request = export.clone();
    incomplete_request.payload.data_requests[0].status = "accepted".to_string();
    reseal(&mut incomplete_request);
    assert!(open_commerce_portability_service::verify_external_export(incomplete_request).is_err());

    let mut duplicate_evidence = export.clone();
    duplicate_evidence
        .payload
        .data_erasure_evidence
        .push(duplicate_evidence.payload.data_erasure_evidence[0].clone());
    reseal(&mut duplicate_evidence);
    assert!(open_commerce_portability_service::verify_external_export(duplicate_evidence).is_err());

    let mut too_many = export.clone();
    let evidence = too_many.payload.data_erasure_evidence[0].clone();
    too_many.payload.data_erasure_evidence = (0..=5_000)
        .map(|index| {
            let mut item = evidence.clone();
            item.id = format!("evidence-{index}");
            item
        })
        .collect();
    reseal(&mut too_many);
    assert!(open_commerce_portability_service::verify_external_export(too_many).is_err());

    let mut legacy = export;
    legacy.schema = CONSUMER_PORTABILITY_EXPORT_SCHEMA_V4.to_string();
    legacy.payload.schema = CONSUMER_PORTABILITY_PAYLOAD_SCHEMA_V4.to_string();
    reseal(&mut legacy);
    assert!(open_commerce_portability_service::verify_external_export(legacy).is_err());
}

struct Fixture {
    store: Store,
    consumer_user_id: String,
    consumer_project_id: String,
    merchant_id: String,
    request_id: String,
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

fn fixture(with_evidence: bool) -> Fixture {
    let suffix = Uuid::new_v4().simple();
    let path = std::env::temp_dir().join(format!("elon-portability-v5-{suffix}.sqlite"));
    let store = Store::open(&path).unwrap();
    let merchant_owner = store
        .create_user(
            &format!("portability-v5-merchant-{suffix}@example.com"),
            "secret1",
            None,
            None,
        )
        .unwrap();
    let merchant_project = store
        .create_project(&merchant_owner.id, "V5 merchant", None, None)
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
            display_name: "V5 删除证明商户".to_string(),
            slug: Some(format!("portability-v5-{suffix}")),
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
            capability_key: "profile.public".to_string(),
            display_name: "公开资料".to_string(),
            description: String::new(),
            kind: "query".to_string(),
            access_level: ACCESS_PUBLIC.to_string(),
            input_schema: json!({}),
            output_schema: json!({}),
            handler_type: HANDLER_STATIC_JSON.to_string(),
            handler_config: Some(json!({"response": {}})),
            unit_price_micros: 0,
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
            &format!("portability-v5-consumer-{suffix}@example.com"),
            "secret1",
            None,
            None,
        )
        .unwrap();
    let consumer_project = store
        .create_project(&consumer.id, "V5 consumer", None, None)
        .unwrap()
        .project;
    let consumer_actor = OpenCommerceActor {
        user_id: &consumer.id,
        app_id: "pc-web",
        project_role: Some("owner"),
    };
    let relationship = open_commerce_relationship_service::create_relationship(
        &store,
        &consumer_project.id,
        &consumer_actor,
        CreateConsumerRelationshipRequest {
            merchant_id: merchant.id.clone(),
            source_app_id: "pc-web".to_string(),
            scopes: vec![RELATIONSHIP_SCOPE_PREFERENCE_REMEMBER.to_string()],
            purpose: "V5 可携带删除证明测试".to_string(),
            expires_at: (Utc::now() + Duration::days(90)).to_rfc3339(),
        },
    )
    .unwrap();
    let request = open_commerce_data_request_service::create_erasure_request(
        &store,
        &consumer_project.id,
        &consumer_actor,
        CreateConsumerDataErasureRequest {
            relationship_id: relationship.id,
        },
    )
    .unwrap();
    for (action, note) in [("accept", ""), ("complete", "商户已完成内部删除流程")] {
        open_commerce_data_request_service::decide_request(
            &store,
            &merchant_project.id,
            &merchant.id,
            &request.id,
            &merchant_actor,
            DecideConsumerDataRequest {
                action: action.to_string(),
                note: note.to_string(),
            },
        )
        .unwrap();
    }
    if with_evidence {
        open_commerce_data_erasure_evidence_service::create_merchant_evidence(
            &store,
            &merchant_project.id,
            &merchant.id,
            &request.id,
            &merchant_actor,
            CreateDataErasureEvidenceRequest {
                evidence_kind: "external_system_receipt".to_string(),
                external_system: "merchant-erp".to_string(),
                reference_id: "receipt-v5-001".to_string(),
                receipt_sha256: "a".repeat(64),
                summary: "商户持有的外部删除回执摘要".to_string(),
                merchant_confirmed_unverified: true,
            },
        )
        .unwrap();
    }

    Fixture {
        store,
        consumer_user_id: consumer.id,
        consumer_project_id: consumer_project.id,
        merchant_id: merchant.id,
        request_id: request.id,
    }
}

fn export_request(idempotency_key: &str) -> CreateConsumerPortabilityExportRequest {
    CreateConsumerPortabilityExportRequest {
        idempotency_key: idempotency_key.to_string(),
    }
}

fn reseal(export: &mut ConsumerPortabilityExport) {
    export.payload_json = serde_json::to_string(&export.payload).unwrap();
    export.payload_sha256 = hex::encode(Sha256::digest(export.payload_json.as_bytes()));
}

#[path = "open_commerce_portability_v5_import_tests.rs"]
mod import_tests;
