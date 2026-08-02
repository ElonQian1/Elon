use serde_json::json;
use uuid::Uuid;

use crate::{
    open_commerce_adapter_model::AdapterBusinessHandoffReceiptRequest,
    open_commerce_adapter_service, open_commerce_business_handoff_service,
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
    integration_id: String,
    invocation_id: String,
    result_sha256: String,
}

fn fixture() -> Fixture {
    let path = std::env::temp_dir().join(format!(
        "elon_open_commerce_adapter_{}.db",
        Uuid::new_v4().simple()
    ));
    let store = Store::open(&path).unwrap();
    let owner = store
        .create_user("adapter@example.com", "secret1", Some("Adapter"), None)
        .unwrap();
    let project = store
        .create_project(&owner.id, "Adapter Project", None, None)
        .unwrap()
        .project;
    let actor = owner_actor(&owner.id);
    let merchant = open_commerce_service::create_merchant(
        &store,
        &project.id,
        &actor,
        CreateMerchantRequest {
            display_name: "机器凭据咖啡店".to_string(),
            slug: Some("adapter-cafe".to_string()),
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
            integration_key: "merchant.erp.adapter".to_string(),
            provider_key: "merchant_erp".to_string(),
            display_name: "商户 ERP 适配器".to_string(),
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
            idempotency_key: "adapter-order-1",
            request_hash: "adapter-request-hash",
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
                "order":{"id":"merchant-order-adapter-1"},
                "_yilong_business_receipt":{
                    "schema":BUSINESS_RECEIPT_SCHEMA,
                    "entity_type":"order",
                    "reference_id":"merchant-order-adapter-1",
                    "state":"accepted",
                    "occurred_at":"2026-08-03T05:30:00Z",
                    "amount_minor":2600,
                    "currency":"CNY"
                }
            }),
        )
        .unwrap();
    let result_sha256 = open_commerce_merchant_evidence_service::get_evidence(
        &store,
        &project.id,
        &merchant.id,
        &invocation_id,
    )
    .unwrap()
    .evidence
    .result_sha256
    .unwrap();
    Fixture {
        store,
        user_id: owner.id,
        project_id: project.id,
        integration_id: integration.id,
        invocation_id,
        result_sha256,
    }
}

#[tokio::test]
async fn adapter_token_is_one_time_rotatable_revocable_and_records_machine_authority() {
    let fixture = fixture();
    let actor = owner_actor(&fixture.user_id);
    let issue = open_commerce_adapter_service::rotate_credential(
        &fixture.store,
        &fixture.project_id,
        &fixture.integration_id,
        &actor,
    )
    .unwrap();
    assert!(issue.token_visible_once);
    assert!(issue.adapter_token.starts_with("oc_adapter_"));
    assert_eq!(issue.credential.scopes, vec!["business_handoff.write"]);
    assert!(!serde_json::to_string(
        &open_commerce_adapter_service::list_credentials(&fixture.store, &fixture.project_id)
            .unwrap()
    )
    .unwrap()
    .contains(&issue.adapter_token));

    let authenticated = fixture
        .store
        .authenticate_open_commerce_adapter_credential(&issue.adapter_token)
        .unwrap();
    let receipt = open_commerce_business_handoff_service::record_adapter_receipt(
        &fixture.store,
        &authenticated,
        applied_request(&fixture, "adapter-handoff-1"),
    )
    .unwrap();
    assert_eq!(receipt.assertion_authority, "adapter_token_authenticated");
    assert_eq!(
        receipt.adapter_credential_id.as_deref(),
        Some(issue.credential.id.as_str())
    );
    assert_eq!(
        receipt.adapter_credential_version,
        Some(issue.credential.credential_version)
    );
    assert!(!receipt.confirmed_by_user);
    assert!(!receipt.funds_moved);

    let rotated = open_commerce_adapter_service::rotate_credential(
        &fixture.store,
        &fixture.project_id,
        &fixture.integration_id,
        &actor,
    )
    .unwrap();
    assert_eq!(rotated.credential.id, issue.credential.id);
    assert_eq!(rotated.credential.credential_version, 2);
    assert!(fixture
        .store
        .authenticate_open_commerce_adapter_credential(&issue.adapter_token)
        .is_err());
    assert!(fixture
        .store
        .authenticate_open_commerce_adapter_credential(&rotated.adapter_token)
        .is_ok());
    assert!(
        open_commerce_business_handoff_service::record_adapter_receipt(
            &fixture.store,
            &authenticated,
            applied_request(&fixture, "adapter-stale-version"),
        )
        .unwrap_err()
        .to_string()
        .contains("不匹配或已撤销")
    );

    open_commerce_adapter_service::revoke_credential(
        &fixture.store,
        &fixture.project_id,
        &rotated.credential.id,
        &actor,
    )
    .unwrap();
    assert!(fixture
        .store
        .authenticate_open_commerce_adapter_credential(&rotated.adapter_token)
        .is_err());

    let mcp = crate::open_commerce_mcp::call_tool(
        &fixture.store,
        &fixture.project_id,
        &fixture.user_id,
        "owner",
        "pc-web",
        json!({"name":"open_commerce_list_adapter_credentials","arguments":{}}),
    )
    .await
    .unwrap();
    assert_eq!(
        mcp["structuredContent"]["credentials"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert!(crate::open_commerce_adapter_mcp::definitions()
        .iter()
        .any(|tool| tool["name"] == "open_commerce_rotate_adapter_credential"));

    let unconfirmed = crate::open_commerce_mcp::call_tool(
        &fixture.store,
        &fixture.project_id,
        &fixture.user_id,
        "owner",
        "pc-web",
        json!({
            "name":"open_commerce_rotate_adapter_credential",
            "arguments":{
                "integration_id":fixture.integration_id,
                "confirmed_by_user":false
            }
        }),
    )
    .await
    .unwrap_err();
    assert!(unconfirmed.to_string().contains("明确确认"));
}

#[test]
fn adapter_credential_management_requires_editor_and_disabled_integration_fails_closed() {
    let fixture = fixture();
    let viewer = OpenCommerceActor {
        user_id: &fixture.user_id,
        app_id: "pc-web",
        project_role: Some("viewer"),
    };
    assert!(open_commerce_adapter_service::rotate_credential(
        &fixture.store,
        &fixture.project_id,
        &fixture.integration_id,
        &viewer,
    )
    .unwrap_err()
    .to_string()
    .contains("项目编辑者"));

    let actor = owner_actor(&fixture.user_id);
    let issue = open_commerce_adapter_service::rotate_credential(
        &fixture.store,
        &fixture.project_id,
        &fixture.integration_id,
        &actor,
    )
    .unwrap();
    open_commerce_service::set_integration_enabled(
        &fixture.store,
        &fixture.project_id,
        &fixture.integration_id,
        &actor,
        false,
    )
    .unwrap();
    assert!(fixture
        .store
        .authenticate_open_commerce_adapter_credential(&issue.adapter_token)
        .unwrap_err()
        .to_string()
        .contains("停用"));
}

fn applied_request(fixture: &Fixture, receipt_key: &str) -> AdapterBusinessHandoffReceiptRequest {
    AdapterBusinessHandoffReceiptRequest {
        invocation_id: fixture.invocation_id.clone(),
        receipt_key: receipt_key.to_string(),
        status: "applied".to_string(),
        target_domain: "erp".to_string(),
        evidence_result_sha256: fixture.result_sha256.clone(),
        target_reference: Some("erp-order-adapter-1".to_string()),
        error_code: None,
        completed_at: "2026-08-03T05:31:00Z".to_string(),
    }
}

fn owner_actor(user_id: &str) -> OpenCommerceActor<'_> {
    OpenCommerceActor {
        user_id,
        app_id: "pc-web",
        project_role: Some("owner"),
    }
}
