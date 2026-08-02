use serde_json::json;
use uuid::Uuid;

use crate::{
    open_commerce_model::{
        CreateCapabilityRequest, CreateGrantRequest, CreateMerchantRequest,
        InvokeCapabilityRequest, UpdateCapabilityRequest, ACCESS_AUTHORIZED, ACCESS_PUBLIC,
        HANDLER_STATIC_JSON,
    },
    open_commerce_service::{self, OpenCommerceActor},
    store::Store,
};

struct ContractFixture {
    store: Store,
    owner_id: String,
    project_id: String,
    merchant_id: String,
}

impl ContractFixture {
    fn actor(&self) -> OpenCommerceActor<'_> {
        OpenCommerceActor {
            user_id: &self.owner_id,
            app_id: "pc-web",
            project_role: Some("owner"),
        }
    }
}

#[test]
fn publication_rejects_contracts_the_runtime_cannot_enforce() {
    let fixture = fixture("schema-publication");
    let error = open_commerce_service::publish_capability(
        &fixture.store,
        &fixture.project_id,
        &fixture.merchant_id,
        &fixture.actor(),
        capability(
            "catalog.unsupported",
            ACCESS_PUBLIC,
            json!({"type":"object","$ref":"https://example.com/schema.json"}),
            json!({}),
            json!({"ok":true}),
        ),
    )
    .unwrap_err();

    assert!(error.to_string().contains("不支持的关键字 $ref"));
    assert!(fixture
        .store
        .list_open_commerce_capabilities(&fixture.merchant_id)
        .unwrap()
        .is_empty());
}

#[tokio::test]
async fn invalid_input_is_rejected_before_invocation_and_metering() {
    let fixture = fixture("schema-input");
    open_commerce_service::publish_capability(
        &fixture.store,
        &fixture.project_id,
        &fixture.merchant_id,
        &fixture.actor(),
        capability(
            "catalog.search",
            ACCESS_PUBLIC,
            json!({
                "type":"object",
                "required":["query"],
                "properties":{"query":{"type":"string","maxLength":120}},
                "additionalProperties":false
            }),
            json!({
                "type":"object",
                "required":["items"],
                "properties":{"items":{"type":"array"}},
                "additionalProperties":false
            }),
            json!({"items":["coffee"]}),
        ),
    )
    .unwrap();

    let error = open_commerce_service::invoke(
        &fixture.store,
        &fixture.actor(),
        InvokeCapabilityRequest {
            merchant_id: fixture.merchant_id.clone(),
            capability_key: "catalog.search".to_string(),
            requester_app_id: "pc-web".to_string(),
            grant_id: None,
            idempotency_key: "invalid-input-1".to_string(),
            input: json!({"query":"private-search-value","admin":true}),
        },
    )
    .await
    .unwrap_err();

    assert!(error.to_string().contains("$.admin"));
    assert!(error.to_string().contains("additionalProperties"));
    assert!(!error.to_string().contains("private-search-value"));
    assert!(fixture
        .store
        .list_project_open_commerce_invocations(&fixture.project_id, 20)
        .unwrap()
        .is_empty());

    let valid = open_commerce_service::invoke(
        &fixture.store,
        &fixture.actor(),
        InvokeCapabilityRequest {
            merchant_id: fixture.merchant_id.clone(),
            capability_key: "catalog.search".to_string(),
            requester_app_id: "pc-web".to_string(),
            grant_id: None,
            idempotency_key: "valid-input-1".to_string(),
            input: json!({"query":"coffee"}),
        },
    )
    .await
    .unwrap();
    assert_eq!(valid["result"]["items"], json!(["coffee"]));
}

#[tokio::test]
async fn invalid_output_fails_closed_releases_budget_and_keeps_values_out_of_audit() {
    let fixture = fixture("schema-output");
    let published = open_commerce_service::publish_capability(
        &fixture.store,
        &fixture.project_id,
        &fixture.merchant_id,
        &fixture.actor(),
        capability(
            "order.quote.create",
            ACCESS_AUTHORIZED,
            json!({
                "type":"object",
                "properties":{"note":{"type":"string","maxLength":500}},
                "additionalProperties":false
            }),
            json!({
                "type":"object",
                "required":["status"],
                "properties":{"status":{"const":"quoted"}},
                "additionalProperties":false
            }),
            json!({"status":"merchant-private-broken-value"}),
        ),
    )
    .unwrap();
    let grant = open_commerce_service::create_grant(
        &fixture.store,
        &fixture.project_id,
        &fixture.actor(),
        CreateGrantRequest {
            merchant_id: fixture.merchant_id.clone(),
            grantee_app_id: "pc-web".to_string(),
            scopes: vec!["order.quote.create".to_string()],
            purpose: "验证契约失败时释放预算".to_string(),
            expires_at: None,
            max_invocations: Some(1),
            max_amount_micros: Some(2_000),
            budget_currency: "CNY".to_string(),
        },
    )
    .unwrap();

    let invoke = |idempotency_key: &str| InvokeCapabilityRequest {
        merchant_id: fixture.merchant_id.clone(),
        capability_key: "order.quote.create".to_string(),
        requester_app_id: "pc-web".to_string(),
        grant_id: Some(grant.id.clone()),
        idempotency_key: idempotency_key.to_string(),
        input: json!({"note":"consumer-private-order-note"}),
    };
    let error =
        open_commerce_service::invoke(&fixture.store, &fixture.actor(), invoke("invalid-output-1"))
            .await
            .unwrap_err();
    assert!(error.to_string().contains("$.status"));
    assert!(error.to_string().contains("const"));
    assert!(!error.to_string().contains("merchant-private-broken-value"));

    let failed = fixture
        .store
        .list_project_open_commerce_invocations(&fixture.project_id, 20)
        .unwrap();
    assert_eq!(failed.len(), 1);
    assert_eq!(failed[0].status, "failed");
    assert_eq!(
        failed[0].error_code.as_deref(),
        Some("output_schema_violation")
    );
    assert_eq!(failed[0].amount_micros, 0);
    let released = fixture.store.open_commerce_grant(&grant.id).unwrap();
    assert_eq!(released.used_invocations, 0);
    assert_eq!(released.used_amount_micros, 0);

    let audit = fixture
        .store
        .list_project_open_commerce_audit(&fixture.project_id, 20)
        .unwrap();
    let event = audit
        .iter()
        .find(|event| event.action == "invocation.failed")
        .unwrap();
    assert_eq!(event.metadata["contract_path"], "$.status");
    assert_eq!(event.metadata["contract_code"], "const");
    let audit_json = serde_json::to_string(event).unwrap();
    assert!(!audit_json.contains("merchant-private-broken-value"));
    assert!(!audit_json.contains("consumer-private-order-note"));

    open_commerce_service::update_capability(
        &fixture.store,
        &fixture.project_id,
        &published.id,
        &fixture.actor(),
        UpdateCapabilityRequest {
            display_name: None,
            description: None,
            access_level: None,
            input_schema: None,
            output_schema: Some(json!({
                "type":"object",
                "required":["status"],
                "properties":{"status":{"const":"merchant-private-broken-value"}},
                "additionalProperties":false
            })),
            handler_type: None,
            handler_config: None,
            unit_price_micros: None,
            currency: None,
            freshness_seconds: None,
            status: None,
        },
    )
    .unwrap();

    let success =
        open_commerce_service::invoke(&fixture.store, &fixture.actor(), invoke("valid-output-2"))
            .await
            .unwrap();
    assert_eq!(success["metering"]["amount_micros"], 1_000);
    let committed = fixture.store.open_commerce_grant(&grant.id).unwrap();
    assert_eq!(committed.used_invocations, 1);
    assert_eq!(committed.used_amount_micros, 1_000);
}

#[tokio::test]
async fn replayed_success_is_revalidated_against_the_current_output_contract() {
    let fixture = fixture("schema-replay");
    let published = open_commerce_service::publish_capability(
        &fixture.store,
        &fixture.project_id,
        &fixture.merchant_id,
        &fixture.actor(),
        capability(
            "catalog.replay",
            ACCESS_PUBLIC,
            json!({"type":"object","additionalProperties":false}),
            json!({
                "type":"object",
                "required":["status"],
                "properties":{"status":{"const":"original-private-result"}},
                "additionalProperties":false
            }),
            json!({"status":"original-private-result"}),
        ),
    )
    .unwrap();
    let request = || InvokeCapabilityRequest {
        merchant_id: fixture.merchant_id.clone(),
        capability_key: "catalog.replay".to_string(),
        requester_app_id: "pc-web".to_string(),
        grant_id: None,
        idempotency_key: "stable-replay-1".to_string(),
        input: json!({}),
    };

    let initial = open_commerce_service::invoke(&fixture.store, &fixture.actor(), request())
        .await
        .unwrap();
    assert_eq!(initial["contract_validation"]["output_validated"], true);

    open_commerce_service::update_capability(
        &fixture.store,
        &fixture.project_id,
        &published.id,
        &fixture.actor(),
        UpdateCapabilityRequest {
            display_name: None,
            description: None,
            access_level: None,
            input_schema: None,
            output_schema: Some(json!({
                "type":"object",
                "required":["status"],
                "properties":{"status":{"const":"new-contract-value"}},
                "additionalProperties":false
            })),
            handler_type: None,
            handler_config: None,
            unit_price_micros: None,
            currency: None,
            freshness_seconds: None,
            status: None,
        },
    )
    .unwrap();

    let error = open_commerce_service::invoke(&fixture.store, &fixture.actor(), request())
        .await
        .unwrap_err();
    assert!(error.to_string().contains("$.status"));
    assert!(!error.to_string().contains("original-private-result"));
    assert_eq!(
        fixture
            .store
            .list_project_open_commerce_invocations(&fixture.project_id, 20)
            .unwrap()[0]
            .status,
        "succeeded"
    );
    let audit = fixture
        .store
        .list_project_open_commerce_audit(&fixture.project_id, 20)
        .unwrap();
    let event = audit
        .iter()
        .find(|event| event.action == "invocation.replay_contract_rejected")
        .unwrap();
    assert_eq!(event.metadata["contract_path"], "$.status");
    assert!(!serde_json::to_string(event)
        .unwrap()
        .contains("original-private-result"));
}

fn fixture(name: &str) -> ContractFixture {
    let path = std::env::temp_dir().join(format!(
        "elon_open_commerce_contract_{}_{}.db",
        name,
        Uuid::new_v4().simple()
    ));
    let store = Store::open(&path).unwrap();
    let owner = store
        .create_user(
            &format!("{name}-{}@example.com", Uuid::new_v4().simple()),
            "secret1",
            Some("Contract Owner"),
            None,
        )
        .unwrap();
    let project = store
        .create_project(&owner.id, "Capability Contract", None, None)
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
            display_name: "契约测试商户".to_string(),
            slug: Some(format!("contract-{}", Uuid::new_v4().simple())),
            description: String::new(),
            node_mode: "platform_hosted".to_string(),
            public_profile: json!({}),
        },
    )
    .unwrap();
    ContractFixture {
        store,
        owner_id: owner.id,
        project_id: project.id,
        merchant_id: merchant.id,
    }
}

fn capability(
    key: &str,
    access_level: &str,
    input_schema: serde_json::Value,
    output_schema: serde_json::Value,
    response: serde_json::Value,
) -> CreateCapabilityRequest {
    CreateCapabilityRequest {
        capability_key: key.to_string(),
        display_name: "契约测试能力".to_string(),
        description: String::new(),
        kind: "query".to_string(),
        access_level: access_level.to_string(),
        input_schema,
        output_schema,
        handler_type: HANDLER_STATIC_JSON.to_string(),
        handler_config: Some(json!({"response":response})),
        unit_price_micros: 1_000,
        currency: "CNY".to_string(),
        freshness_seconds: 30,
    }
}
