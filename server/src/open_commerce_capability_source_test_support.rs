use serde_json::json;
use uuid::Uuid;

use crate::{
    open_commerce_capability_source_model::{
        LinkCapabilitySourceRequest, OpenCommerceCapabilitySourceLink,
    },
    open_commerce_capability_source_service::link_source,
    open_commerce_directory_service,
    open_commerce_integration_model::{CreateIntegrationRequest, RecordSyncReceiptRequest},
    open_commerce_model::{
        CreateCapabilityRequest, CreateMerchantRequest, ACCESS_PUBLIC, HANDLER_STATIC_JSON,
    },
    open_commerce_service::{self, OpenCommerceActor},
    store::Store,
};

pub(super) struct Fixture {
    pub(super) store: Store,
    pub(super) owner_id: String,
    pub(super) viewer_id: String,
    pub(super) project_id: String,
    pub(super) other_project_id: String,
    pub(super) merchant_id: String,
    pub(super) capability_id: String,
    pub(super) integration_id: String,
    pub(super) other_merchant_integration_id: String,
    pub(super) other_project_integration_id: String,
    pub(super) succeeded_receipt_id: String,
    pub(super) partial_receipt_id: String,
    pub(super) health_receipt_id: String,
    pub(super) failed_receipt_id: String,
    pub(super) other_integration_receipt_id: String,
    pub(super) other_project_receipt_id: String,
}

impl Fixture {
    pub(super) fn owner(&self) -> OpenCommerceActor<'_> {
        actor(&self.owner_id, "owner")
    }

    pub(super) fn viewer(&self) -> OpenCommerceActor<'_> {
        actor(&self.viewer_id, "viewer")
    }

    pub(super) fn link(
        &self,
        receipt_id: &str,
        data_domain: &str,
    ) -> anyhow::Result<OpenCommerceCapabilitySourceLink> {
        link_source(
            &self.store,
            &self.project_id,
            &self.capability_id,
            &self.owner(),
            LinkCapabilitySourceRequest {
                integration_id: self.integration_id.clone(),
                sync_receipt_id: receipt_id.to_string(),
                data_domain: data_domain.to_string(),
            },
        )
    }
}

pub(super) fn fixture(name: &str) -> Fixture {
    let path = std::env::temp_dir().join(format!(
        "elon-open-commerce-capability-source-{name}-{}.sqlite",
        Uuid::new_v4().simple()
    ));
    let store = Store::open(&path).unwrap();
    let owner = store
        .create_user(
            &format!("source-{name}-{}@example.com", Uuid::new_v4().simple()),
            "secret1",
            Some("Source Owner"),
            None,
        )
        .unwrap();
    let viewer = store
        .create_user(
            &format!(
                "source-viewer-{name}-{}@example.com",
                Uuid::new_v4().simple()
            ),
            "secret1",
            Some("Source Viewer"),
            None,
        )
        .unwrap();
    let project = store
        .create_project(&owner.id, &format!("Source {name}"), None, None)
        .unwrap()
        .project;
    store
        .add_project_member_by_account(&project.id, &viewer.id, "viewer")
        .unwrap();
    let other_project = store
        .create_project(&owner.id, &format!("Source other {name}"), None, None)
        .unwrap()
        .project;
    let owner_actor = actor(&owner.id, "owner");
    let merchant = create_merchant(&store, &project.id, &owner_actor, &format!("{name}-main"));
    let other_merchant =
        create_merchant(&store, &project.id, &owner_actor, &format!("{name}-other"));
    let other_project_merchant = create_merchant(
        &store,
        &other_project.id,
        &owner_actor,
        &format!("{name}-cross"),
    );
    let capability = create_capability(
        &store,
        &project.id,
        &owner_actor,
        &merchant.id,
        "catalog.search",
    );
    let integration =
        create_integration(&store, &project.id, &owner_actor, &merchant.id, "main.erp");
    let other_merchant_integration = create_integration(
        &store,
        &project.id,
        &owner_actor,
        &other_merchant.id,
        "other.erp",
    );
    let other_project_integration = create_integration(
        &store,
        &other_project.id,
        &owner_actor,
        &other_project_merchant.id,
        "cross.erp",
    );
    let succeeded_receipt = receipt(
        &store,
        &project.id,
        &owner_actor,
        &integration.id,
        "main-full",
        "full",
        "succeeded",
        "2026-08-01T00:00:00Z",
    );
    let partial_receipt = receipt(
        &store,
        &project.id,
        &owner_actor,
        &integration.id,
        "main-partial",
        "incremental",
        "partial",
        "2026-08-02T00:00:00Z",
    );
    let health_receipt = receipt(
        &store,
        &project.id,
        &owner_actor,
        &integration.id,
        "main-health",
        "health_check",
        "succeeded",
        "2026-08-03T00:00:00Z",
    );
    let failed_receipt = receipt(
        &store,
        &project.id,
        &owner_actor,
        &integration.id,
        "main-failed",
        "full",
        "failed",
        "2026-08-04T00:00:00Z",
    );
    let other_integration_receipt = receipt(
        &store,
        &project.id,
        &owner_actor,
        &other_merchant_integration.id,
        "other-full",
        "full",
        "succeeded",
        "2026-08-05T00:00:00Z",
    );
    let other_project_receipt = receipt(
        &store,
        &other_project.id,
        &owner_actor,
        &other_project_integration.id,
        "cross-full",
        "full",
        "succeeded",
        "2026-08-06T00:00:00Z",
    );
    Fixture {
        store,
        owner_id: owner.id,
        viewer_id: viewer.id,
        project_id: project.id,
        other_project_id: other_project.id,
        merchant_id: merchant.id,
        capability_id: capability.id,
        integration_id: integration.id,
        other_merchant_integration_id: other_merchant_integration.id,
        other_project_integration_id: other_project_integration.id,
        succeeded_receipt_id: succeeded_receipt.id,
        partial_receipt_id: partial_receipt.id,
        health_receipt_id: health_receipt.id,
        failed_receipt_id: failed_receipt.id,
        other_integration_receipt_id: other_integration_receipt.id,
        other_project_receipt_id: other_project_receipt.id,
    }
}

fn actor<'a>(user_id: &'a str, role: &'a str) -> OpenCommerceActor<'a> {
    OpenCommerceActor {
        user_id,
        app_id: "pc-web",
        project_role: Some(role),
    }
}

fn create_merchant(
    store: &Store,
    project_id: &str,
    actor: &OpenCommerceActor<'_>,
    slug: &str,
) -> crate::open_commerce_model::OpenCommerceMerchant {
    open_commerce_service::create_merchant(
        store,
        project_id,
        actor,
        CreateMerchantRequest {
            display_name: format!("{slug} 商户"),
            slug: Some(slug.to_string()),
            description: "来源验证商户".to_string(),
            node_mode: "platform_hosted".to_string(),
            public_profile: json!({"category":"verification"}),
        },
    )
    .unwrap()
}

fn create_capability(
    store: &Store,
    project_id: &str,
    actor: &OpenCommerceActor<'_>,
    merchant_id: &str,
    key: &str,
) -> crate::open_commerce_model::OpenCommerceCapability {
    open_commerce_service::publish_capability(
        store,
        project_id,
        merchant_id,
        actor,
        CreateCapabilityRequest {
            capability_key: key.to_string(),
            display_name: format!("{key} 能力"),
            description: String::new(),
            kind: "query".to_string(),
            access_level: ACCESS_PUBLIC.to_string(),
            input_schema: json!({"type":"object"}),
            output_schema: json!({"type":"object"}),
            handler_type: HANDLER_STATIC_JSON.to_string(),
            handler_config: Some(json!({"response":{"items":[]}})),
            unit_price_micros: 0,
            currency: "CNY".to_string(),
            freshness_seconds: 86_400,
        },
    )
    .unwrap()
}

fn create_integration(
    store: &Store,
    project_id: &str,
    actor: &OpenCommerceActor<'_>,
    merchant_id: &str,
    key: &str,
) -> crate::open_commerce_integration_model::OpenCommerceIntegration {
    open_commerce_service::create_integration(
        store,
        project_id,
        actor,
        CreateIntegrationRequest {
            merchant_id: merchant_id.to_string(),
            integration_key: key.to_string(),
            provider_key: "merchant_erp".to_string(),
            display_name: format!("{key} 接入"),
            connection_mode: "local_adapter".to_string(),
            scopes: vec!["catalog.read".to_string(), "inventory.read".to_string()],
            data_domains: vec!["catalog".to_string(), "inventory".to_string()],
        },
    )
    .unwrap()
}

fn receipt(
    store: &Store,
    project_id: &str,
    actor: &OpenCommerceActor<'_>,
    integration_id: &str,
    key: &str,
    kind: &str,
    status: &str,
    completed_at: &str,
) -> crate::open_commerce_integration_model::OpenCommerceSyncReceipt {
    open_commerce_service::record_sync_receipt(
        store,
        project_id,
        actor,
        RecordSyncReceiptRequest {
            integration_id: integration_id.to_string(),
            receipt_key: key.to_string(),
            sync_kind: kind.to_string(),
            status: status.to_string(),
            records_seen: 20,
            records_changed: 10,
            cursor_digest: Some("cursor-secret-digest".to_string()),
            error_code: (status == "failed").then(|| "upstream_failed".to_string()),
            started_at: "2026-08-01T00:00:00Z".to_string(),
            completed_at: completed_at.to_string(),
        },
    )
    .unwrap()
}

pub(super) fn publish_directory(fixture: &Fixture) {
    open_commerce_directory_service::set_publication(
        &fixture.store,
        &fixture.project_id,
        &fixture.merchant_id,
        &fixture.owner(),
        true,
    )
    .unwrap();
}

pub(super) fn assert_source_kind(fixture: &Fixture, expected: &str) {
    let detail =
        open_commerce_directory_service::discover_merchant(&fixture.store, &fixture.merchant_id)
            .unwrap();
    let source = &detail
        .capabilities
        .iter()
        .find(|value| value.capability_key == "catalog.search")
        .unwrap()
        .source;
    assert_eq!(source.kind, expected);
}

pub(super) fn current_link(fixture: &Fixture) -> OpenCommerceCapabilitySourceLink {
    fixture
        .store
        .list_project_open_commerce_capability_source_links(&fixture.project_id)
        .unwrap()
        .into_iter()
        .find(|value| value.capability_id == fixture.capability_id)
        .unwrap()
}

pub(super) fn source_audit_count(fixture: &Fixture) -> usize {
    fixture
        .store
        .list_project_open_commerce_audit(&fixture.project_id, 200)
        .unwrap()
        .into_iter()
        .filter(|event| event.action.starts_with("capability.source_"))
        .count()
}
