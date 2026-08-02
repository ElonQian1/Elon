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

struct Fixture {
    store: Store,
    user_id: String,
    project_id: String,
    merchant_id: String,
    integration_id: String,
    invocation_id: String,
    result_sha256: String,
}

fn temp_store() -> Store {
    let path = std::env::temp_dir().join(format!(
        "elon_open_commerce_business_handoff_{}.db",
        Uuid::new_v4().simple()
    ));
    Store::open(&path).expect("business handoff test store should open")
}

fn fixture() -> Fixture {
    let store = temp_store();
    let owner = store
        .create_user(
            "business-handoff@example.com",
            "secret1",
            Some("Business Handoff"),
            None,
        )
        .unwrap();
    let project = store
        .create_project(&owner.id, "Business Handoff", None, None)
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
            display_name: "衔接咖啡店".to_string(),
            slug: Some("handoff-cafe".to_string()),
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
    let claim = start_invocation(
        &store,
        &project.id,
        &merchant.id,
        &capability.id,
        &capability.capability_key,
        &owner.id,
        "handoff-order-1",
    );
    store
        .finish_open_commerce_invocation_success(
            &claim,
            &json!({
                "order":{"id":"merchant-order-1001"},
                "_yilong_business_receipt":valid_business_receipt()
            }),
        )
        .unwrap();
    let evidence = open_commerce_merchant_evidence_service::get_evidence(
        &store,
        &project.id,
        &merchant.id,
        &claim,
    )
    .unwrap();

    Fixture {
        store,
        user_id: owner.id,
        project_id: project.id,
        merchant_id: merchant.id,
        integration_id: integration.id,
        invocation_id: claim,
        result_sha256: evidence.evidence.result_sha256.unwrap(),
    }
}

#[tokio::test]
async fn handoff_receipt_is_idempotent_auditable_and_does_not_store_raw_reference() {
    let fixture = fixture();
    let actor = owner_actor(&fixture.user_id);
    let request = applied_request(&fixture, "handoff.erp.1001");
    let receipt = open_commerce_business_handoff_service::record_receipt(
        &fixture.store,
        &fixture.project_id,
        &actor,
        request.clone(),
    )
    .unwrap();
    assert_eq!(receipt.status, "applied");
    assert_eq!(
        receipt.target_reference_sha256.as_deref().unwrap().len(),
        64
    );
    assert_eq!(receipt.assertion_authority, "project_editor_asserted");
    assert!(receipt.confirmed_by_user);
    assert!(!receipt.funds_moved);
    assert!(!serde_json::to_string(&receipt)
        .unwrap()
        .contains("erp-order-private-1001"));

    let replay = open_commerce_business_handoff_service::record_receipt(
        &fixture.store,
        &fixture.project_id,
        &actor,
        request,
    )
    .unwrap();
    assert_eq!(replay.id, receipt.id);
    assert_eq!(
        fixture
            .store
            .list_project_open_commerce_audit(&fixture.project_id, 100)
            .unwrap()
            .iter()
            .filter(|event| event.action == "business_handoff.recorded")
            .count(),
        1
    );

    let list = open_commerce_business_handoff_service::list_receipts(
        &fixture.store,
        &fixture.project_id,
        &fixture.merchant_id,
        50,
    )
    .unwrap();
    assert_eq!(list.receipts.len(), 1);
    assert!(list.boundary.iter().any(|item| item.contains("不代表支付")));

    let mcp_record = crate::open_commerce_mcp::call_tool(
        &fixture.store,
        &fixture.project_id,
        &fixture.user_id,
        "owner",
        "pc-web",
        json!({
            "name":"open_commerce_record_business_handoff_receipt",
            "arguments":{
                "merchant_id":fixture.merchant_id,
                "invocation_id":fixture.invocation_id,
                "integration_id":fixture.integration_id,
                "receipt_key":"handoff.erp.mcp-ignored",
                "status":"ignored",
                "target_domain":"erp",
                "evidence_result_sha256":fixture.result_sha256,
                "error_code":"already_present",
                "confirmed_by_user":true,
                "completed_at":"2026-08-03T02:01:00Z"
            }
        }),
    )
    .await
    .unwrap();
    assert_eq!(mcp_record["structuredContent"]["status"], "ignored");

    let mcp = crate::open_commerce_mcp::call_tool(
        &fixture.store,
        &fixture.project_id,
        &fixture.user_id,
        "owner",
        "pc-web",
        json!({
            "name":"open_commerce_list_business_handoff_receipts",
            "arguments":{"merchant_id":fixture.merchant_id}
        }),
    )
    .await
    .unwrap();
    assert_eq!(
        mcp["structuredContent"]["receipts"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert!(crate::open_commerce_business_handoff_mcp::definitions()
        .iter()
        .any(|tool| tool["name"] == "open_commerce_record_business_handoff_receipt"));
}

#[tokio::test]
async fn handoff_queue_is_derived_from_latest_receipt_and_keeps_rejections_retryable() {
    let fixture = fixture();
    let actor = owner_actor(&fixture.user_id);

    let pending = open_commerce_business_handoff_service::list_queue(
        &fixture.store,
        &fixture.project_id,
        &fixture.merchant_id,
        None,
        50,
    )
    .unwrap();
    assert_eq!(pending.items.len(), 1);
    assert_eq!(pending.items[0].queue_state, "pending");
    assert!(pending.items[0].can_apply);
    assert!(pending.items[0].latest_receipt.is_none());
    assert_eq!(pending.returned_pending_count, 1);
    assert_eq!(pending.returned_retry_required_count, 0);
    assert!(!pending.has_more);

    let mcp_pending = crate::open_commerce_mcp::call_tool(
        &fixture.store,
        &fixture.project_id,
        &fixture.user_id,
        "owner",
        "pc-web",
        json!({
            "name":"open_commerce_list_business_handoff_queue",
            "arguments":{"merchant_id":fixture.merchant_id,"state":"pending"}
        }),
    )
    .await
    .unwrap();
    assert_eq!(
        mcp_pending["structuredContent"]["items"]
            .as_array()
            .unwrap()
            .len(),
        1
    );

    let mut rejected = applied_request(&fixture, "handoff.erp.retry-1");
    rejected.status = "rejected".to_string();
    rejected.target_reference = None;
    rejected.error_code = Some("adapter_failed".to_string());
    open_commerce_business_handoff_service::record_receipt(
        &fixture.store,
        &fixture.project_id,
        &actor,
        rejected,
    )
    .unwrap();

    let no_longer_pending = open_commerce_business_handoff_service::list_queue(
        &fixture.store,
        &fixture.project_id,
        &fixture.merchant_id,
        Some("pending"),
        50,
    )
    .unwrap();
    assert!(no_longer_pending.items.is_empty());
    let retry = open_commerce_business_handoff_service::list_queue(
        &fixture.store,
        &fixture.project_id,
        &fixture.merchant_id,
        Some("retry_required"),
        50,
    )
    .unwrap();
    assert_eq!(retry.items.len(), 1);
    assert_eq!(retry.items[0].queue_state, "retry_required");
    assert_eq!(
        retry.items[0]
            .latest_receipt
            .as_ref()
            .map(|receipt| receipt.status.as_str()),
        Some("rejected")
    );

    let mut applied = applied_request(&fixture, "handoff.erp.retry-2");
    applied.completed_at = "2026-08-03T02:02:00Z".to_string();
    open_commerce_business_handoff_service::record_receipt(
        &fixture.store,
        &fixture.project_id,
        &actor,
        applied,
    )
    .unwrap();
    let resolved = open_commerce_business_handoff_service::list_queue(
        &fixture.store,
        &fixture.project_id,
        &fixture.merchant_id,
        None,
        50,
    )
    .unwrap();
    assert!(resolved.items.is_empty());
    assert!(open_commerce_business_handoff_service::list_queue(
        &fixture.store,
        &fixture.project_id,
        &fixture.merchant_id,
        Some("resolved"),
        50,
    )
    .unwrap_err()
    .to_string()
    .contains("pending"));
    assert!(crate::open_commerce_business_handoff_mcp::definitions()
        .iter()
        .any(|tool| tool["name"] == "open_commerce_list_business_handoff_queue"));
}

#[test]
fn handoff_receipt_fails_closed_on_tampering_rewrite_and_missing_confirmation() {
    let fixture = fixture();
    let actor = owner_actor(&fixture.user_id);

    let mut tampered = applied_request(&fixture, "handoff.erp.tampered");
    tampered.evidence_result_sha256 = "0".repeat(64);
    assert!(open_commerce_business_handoff_service::record_receipt(
        &fixture.store,
        &fixture.project_id,
        &actor,
        tampered,
    )
    .unwrap_err()
    .to_string()
    .contains("不匹配"));

    let mut unconfirmed = applied_request(&fixture, "handoff.erp.unconfirmed");
    unconfirmed.confirmed_by_user = false;
    assert!(open_commerce_business_handoff_service::record_receipt(
        &fixture.store,
        &fixture.project_id,
        &actor,
        unconfirmed,
    )
    .unwrap_err()
    .to_string()
    .contains("明确确认"));

    let original = applied_request(&fixture, "handoff.erp.stable");
    open_commerce_business_handoff_service::record_receipt(
        &fixture.store,
        &fixture.project_id,
        &actor,
        original.clone(),
    )
    .unwrap();
    let mut changed = original;
    changed.target_reference = Some("different-erp-order".to_string());
    assert!(open_commerce_business_handoff_service::record_receipt(
        &fixture.store,
        &fixture.project_id,
        &actor,
        changed,
    )
    .unwrap_err()
    .to_string()
    .contains("不能用于不同结果"));

    let viewer = OpenCommerceActor {
        user_id: &fixture.user_id,
        app_id: "pc-web",
        project_role: Some("viewer"),
    };
    assert!(open_commerce_business_handoff_service::record_receipt(
        &fixture.store,
        &fixture.project_id,
        &viewer,
        applied_request(&fixture, "handoff.erp.viewer"),
    )
    .unwrap_err()
    .to_string()
    .contains("项目编辑者"));

    open_commerce_service::set_integration_enabled(
        &fixture.store,
        &fixture.project_id,
        &fixture.integration_id,
        &actor,
        false,
    )
    .unwrap();
    assert!(open_commerce_business_handoff_service::record_receipt(
        &fixture.store,
        &fixture.project_id,
        &actor,
        applied_request(&fixture, "handoff.erp.disabled"),
    )
    .unwrap_err()
    .to_string()
    .contains("已停用"));
}

#[test]
fn applied_handoff_requires_valid_business_receipt_and_same_merchant_integration() {
    let fixture = fixture();
    let actor = owner_actor(&fixture.user_id);
    let digest_only_invocation = create_digest_only_invocation(&fixture);
    let digest_only_evidence = open_commerce_merchant_evidence_service::get_evidence(
        &fixture.store,
        &fixture.project_id,
        &fixture.merchant_id,
        &digest_only_invocation,
    )
    .unwrap();
    let mut digest_only = applied_request(&fixture, "handoff.erp.digest-only");
    digest_only.invocation_id = digest_only_invocation;
    digest_only.evidence_result_sha256 = digest_only_evidence.evidence.result_sha256.unwrap();
    assert!(open_commerce_business_handoff_service::record_receipt(
        &fixture.store,
        &fixture.project_id,
        &actor,
        digest_only,
    )
    .unwrap_err()
    .to_string()
    .contains("有效标准业务回执"));

    let other_merchant = open_commerce_service::create_merchant(
        &fixture.store,
        &fixture.project_id,
        &actor,
        CreateMerchantRequest {
            display_name: "另一商户".to_string(),
            slug: Some("other-merchant".to_string()),
            description: String::new(),
            node_mode: "platform_hosted".to_string(),
            public_profile: json!({}),
        },
    )
    .unwrap();
    let other_integration = open_commerce_service::create_integration(
        &fixture.store,
        &fixture.project_id,
        &actor,
        CreateIntegrationRequest {
            merchant_id: other_merchant.id,
            integration_key: "other.erp".to_string(),
            provider_key: "other_erp".to_string(),
            display_name: "另一 ERP".to_string(),
            connection_mode: "local_adapter".to_string(),
            scopes: vec![],
            data_domains: vec!["orders".to_string()],
        },
    )
    .unwrap();
    let mut cross_merchant = applied_request(&fixture, "handoff.erp.cross-merchant");
    cross_merchant.integration_id = other_integration.id;
    assert!(open_commerce_business_handoff_service::record_receipt(
        &fixture.store,
        &fixture.project_id,
        &actor,
        cross_merchant,
    )
    .unwrap_err()
    .to_string()
    .contains("不属于同一商户"));
}

fn applied_request(fixture: &Fixture, receipt_key: &str) -> RecordBusinessHandoffReceiptRequest {
    RecordBusinessHandoffReceiptRequest {
        merchant_id: fixture.merchant_id.clone(),
        invocation_id: fixture.invocation_id.clone(),
        integration_id: fixture.integration_id.clone(),
        receipt_key: receipt_key.to_string(),
        status: "applied".to_string(),
        target_domain: "erp".to_string(),
        evidence_result_sha256: fixture.result_sha256.clone(),
        target_reference: Some("erp-order-private-1001".to_string()),
        error_code: None,
        confirmed_by_user: true,
        completed_at: "2026-08-03T02:00:00Z".to_string(),
    }
}

fn owner_actor(user_id: &str) -> OpenCommerceActor<'_> {
    OpenCommerceActor {
        user_id,
        app_id: "pc-web",
        project_role: Some("owner"),
    }
}

fn create_digest_only_invocation(fixture: &Fixture) -> String {
    let capability = fixture
        .store
        .list_open_commerce_capabilities(&fixture.merchant_id)
        .unwrap()
        .into_iter()
        .next()
        .unwrap();
    let invocation_id = start_invocation(
        &fixture.store,
        &fixture.project_id,
        &fixture.merchant_id,
        &capability.id,
        &capability.capability_key,
        &fixture.user_id,
        "handoff-order-digest-only",
    );
    fixture
        .store
        .finish_open_commerce_invocation_success(
            &invocation_id,
            &json!({"order":{"id":"digest-only"}}),
        )
        .unwrap();
    invocation_id
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
            request_hash: "request-sha256",
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

fn valid_business_receipt() -> Value {
    json!({
        "schema":BUSINESS_RECEIPT_SCHEMA,
        "entity_type":"order",
        "reference_id":"merchant-order-1001",
        "state":"confirmed",
        "occurred_at":"2026-08-03T01:00:00Z",
        "amount_minor":2600,
        "currency":"CNY"
    })
}
