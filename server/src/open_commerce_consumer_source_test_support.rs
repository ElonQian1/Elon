use chrono::{DateTime, Duration, Utc};
use serde_json::json;
use uuid::Uuid;

use crate::{
    open_commerce_capability_source_model::LinkCapabilitySourceRequest,
    open_commerce_capability_source_service,
    open_commerce_consumer_model::{
        ConsumerDiscoveryRequest, ConsumerDiscoveryResponse, ConsumerPreferences,
    },
    open_commerce_directory_model::OpenCommerceDirectoryCapability,
    open_commerce_directory_service,
    open_commerce_integration_model::{CreateIntegrationRequest, RecordSyncReceiptRequest},
    open_commerce_model::{
        CreateCapabilityRequest, CreateMerchantRequest, UpdateCapabilityRequest, ACCESS_PUBLIC,
        HANDLER_STATIC_JSON,
    },
    open_commerce_service::{self, OpenCommerceActor},
    store::Store,
};

pub(super) const CATALOG_SEARCH: &str = "catalog.search";
pub(super) const CATALOG_QUOTE: &str = "catalog.quote";
pub(super) const STATIC_NAME: &str = "alpha_erp 名称静态商户";
pub(super) const RECENT_NAME: &str = "近期 Alpha 商户";
pub(super) const OLD_NAME: &str = "过期 Beta 商户";
pub(super) const FUTURE_NAME: &str = "未来时间商户";
pub(super) const STALE_LINK_NAME: &str = "版本失效商户";
pub(super) const DISABLED_LINK_NAME: &str = "接入停用商户";

#[derive(Debug, PartialEq, Eq)]
pub(super) struct ReadSnapshot {
    counts: Vec<(&'static str, i64)>,
}

pub(super) struct Fixture {
    pub(super) store: Store,
    pub(super) consumer_id: String,
    pub(super) static_merchant_id: String,
    pub(super) recent_merchant_id: String,
    pub(super) stale_link_merchant_id: String,
    pub(super) disabled_link_merchant_id: String,
}

pub(super) fn fixture() -> Fixture {
    let path = std::env::temp_dir().join(format!(
        "elon-consumer-source-filter-{}.sqlite",
        Uuid::new_v4().simple()
    ));
    let store = Store::open(&path).expect("consumer source filter store should open");
    let owner = store
        .create_user(
            &format!(
                "consumer-source-owner-{}@example.com",
                Uuid::new_v4().simple()
            ),
            "secret1",
            Some("Consumer Source Owner"),
            None,
        )
        .unwrap();
    let consumer = store
        .create_user(
            &format!(
                "consumer-source-user-{}@example.com",
                Uuid::new_v4().simple()
            ),
            "secret1",
            Some("Consumer Source User"),
            None,
        )
        .unwrap();
    let project = store
        .create_project(&owner.id, "Consumer Source Merchant Project", None, None)
        .unwrap()
        .project;
    let actor = actor(&owner.id);
    let now = Utc::now();

    let static_merchant_id =
        create_static_merchant(&store, &project.id, &actor, STATIC_NAME, "source-static");
    let recent = create_sourced_merchant(
        &store,
        &project.id,
        &actor,
        SourcedMerchantSpec {
            display_name: RECENT_NAME,
            slug: "source-recent",
            provider_key: "alpha_erp",
            data_domain: "catalog",
            completed_at: now - Duration::seconds(60),
            freshness_seconds: 3_600,
            capability_keys: &[CATALOG_SEARCH, CATALOG_QUOTE],
        },
    );
    create_capability(
        &store,
        &project.id,
        &actor,
        &recent.merchant_id,
        "aaa.static",
        3_600,
    );
    let mixed_stale = create_capability(
        &store,
        &project.id,
        &actor,
        &recent.merchant_id,
        "aab.stale",
        3_600,
    );
    open_commerce_capability_source_service::link_source(
        &store,
        &project.id,
        &mixed_stale.id,
        &actor,
        LinkCapabilitySourceRequest {
            integration_id: recent.integration_id.clone(),
            sync_receipt_id: recent.sync_receipt_id.clone(),
            data_domain: "catalog".to_string(),
        },
    )
    .unwrap();
    invalidate_capability_link(&store, &project.id, &actor, &mixed_stale.id);
    let _old = create_sourced_merchant(
        &store,
        &project.id,
        &actor,
        SourcedMerchantSpec {
            display_name: OLD_NAME,
            slug: "source-old",
            provider_key: "beta_erp",
            data_domain: "inventory",
            completed_at: now - Duration::days(2),
            freshness_seconds: 3_600,
            capability_keys: &[CATALOG_SEARCH],
        },
    );
    let _future = create_sourced_merchant(
        &store,
        &project.id,
        &actor,
        SourcedMerchantSpec {
            display_name: FUTURE_NAME,
            slug: "source-future",
            provider_key: "future_erp",
            data_domain: "catalog",
            completed_at: now + Duration::minutes(5),
            freshness_seconds: 3_600,
            capability_keys: &[CATALOG_SEARCH],
        },
    );
    let stale_link = create_sourced_merchant(
        &store,
        &project.id,
        &actor,
        SourcedMerchantSpec {
            display_name: STALE_LINK_NAME,
            slug: "source-stale-link",
            provider_key: "stale_erp",
            data_domain: "catalog",
            completed_at: now - Duration::seconds(30),
            freshness_seconds: 3_600,
            capability_keys: &[CATALOG_SEARCH],
        },
    );
    invalidate_capability_link(&store, &project.id, &actor, &stale_link.capability_ids[0]);
    let disabled_link = create_sourced_merchant(
        &store,
        &project.id,
        &actor,
        SourcedMerchantSpec {
            display_name: DISABLED_LINK_NAME,
            slug: "source-disabled-link",
            provider_key: "disabled_erp",
            data_domain: "catalog",
            completed_at: now - Duration::seconds(30),
            freshness_seconds: 3_600,
            capability_keys: &[CATALOG_SEARCH],
        },
    );
    open_commerce_service::set_integration_enabled(
        &store,
        &project.id,
        &disabled_link.integration_id,
        &actor,
        false,
    )
    .unwrap();

    Fixture {
        store,
        consumer_id: consumer.id,
        static_merchant_id,
        recent_merchant_id: recent.merchant_id,
        stale_link_merchant_id: stale_link.merchant_id,
        disabled_link_merchant_id: disabled_link.merchant_id,
    }
}

impl Fixture {
    pub(super) fn discover(
        &self,
        request: ConsumerDiscoveryRequest,
    ) -> anyhow::Result<ConsumerDiscoveryResponse> {
        super::discover(&self.store, &self.consumer_id, request)
    }

    pub(super) fn directory_capability(
        &self,
        merchant_id: &str,
        capability_key: &str,
    ) -> OpenCommerceDirectoryCapability {
        open_commerce_directory_service::discover_merchant(&self.store, merchant_id)
            .unwrap()
            .capabilities
            .into_iter()
            .find(|capability| capability.capability_key == capability_key)
            .unwrap()
    }

    pub(super) fn snapshot(&self) -> ReadSnapshot {
        let conn = self.store.conn().unwrap();
        let counts = [
            "open_commerce_merchants",
            "open_commerce_capabilities",
            "open_commerce_integrations",
            "open_commerce_sync_receipts",
            "open_commerce_capability_source_links",
            "open_commerce_authorization_requests",
            "open_commerce_grants",
            "open_commerce_invocations",
            "open_commerce_audit_events",
        ]
        .into_iter()
        .map(|table| {
            let count = conn
                .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                    row.get(0)
                })
                .unwrap();
            (table, count)
        })
        .collect();
        ReadSnapshot { counts }
    }
}

pub(super) fn request() -> ConsumerDiscoveryRequest {
    ConsumerDiscoveryRequest {
        query: None,
        capability_key: None,
        requester_app_id: String::new(),
        ranking_policy: None,
        include_ranking_receipt: false,
        require_current_declaration: false,
        require_internal_sync_receipt: false,
        source_provider_key: None,
        source_data_domain: None,
        max_source_age_seconds: None,
        price_currency: None,
        capability_kind: None,
        access_level: None,
        require_city_match: false,
        require_category_match: false,
        require_all_tags_match: false,
        preferences: ConsumerPreferences::default(),
        limit: 50,
    }
}

pub(super) fn merchant_names(response: &ConsumerDiscoveryResponse) -> Vec<String> {
    let mut names = response
        .matches
        .iter()
        .map(|item| item.merchant.display_name.clone())
        .collect::<Vec<_>>();
    names.sort();
    names
}

struct SourcedMerchantSpec<'a> {
    display_name: &'a str,
    slug: &'a str,
    provider_key: &'a str,
    data_domain: &'a str,
    completed_at: DateTime<Utc>,
    freshness_seconds: i64,
    capability_keys: &'a [&'a str],
}

struct SourcedMerchant {
    merchant_id: String,
    capability_ids: Vec<String>,
    integration_id: String,
    sync_receipt_id: String,
}

fn create_static_merchant(
    store: &Store,
    project_id: &str,
    actor: &OpenCommerceActor<'_>,
    display_name: &str,
    slug: &str,
) -> String {
    let merchant = create_merchant(store, project_id, actor, display_name, slug);
    create_capability(
        store,
        project_id,
        actor,
        &merchant.id,
        CATALOG_SEARCH,
        86_400,
    );
    publish_directory(store, project_id, actor, &merchant.id);
    merchant.id
}

fn create_sourced_merchant(
    store: &Store,
    project_id: &str,
    actor: &OpenCommerceActor<'_>,
    spec: SourcedMerchantSpec<'_>,
) -> SourcedMerchant {
    let merchant = create_merchant(store, project_id, actor, spec.display_name, spec.slug);
    let capabilities = spec
        .capability_keys
        .iter()
        .map(|key| {
            create_capability(
                store,
                project_id,
                actor,
                &merchant.id,
                key,
                spec.freshness_seconds,
            )
        })
        .collect::<Vec<_>>();
    let integration = open_commerce_service::create_integration(
        store,
        project_id,
        actor,
        CreateIntegrationRequest {
            merchant_id: merchant.id.clone(),
            integration_key: format!("{}.adapter", spec.slug),
            provider_key: spec.provider_key.to_string(),
            display_name: format!("{} 数据接入", spec.display_name),
            connection_mode: "local_adapter".to_string(),
            scopes: vec!["catalog.read".to_string()],
            data_domains: vec![spec.data_domain.to_string()],
        },
    )
    .unwrap();
    let started_at = spec.completed_at - Duration::seconds(30);
    let receipt = open_commerce_service::record_sync_receipt(
        store,
        project_id,
        actor,
        RecordSyncReceiptRequest {
            integration_id: integration.id.clone(),
            receipt_key: format!("{}.receipt", spec.slug),
            sync_kind: "full".to_string(),
            status: "succeeded".to_string(),
            records_seen: 10,
            records_changed: 4,
            cursor_digest: Some(format!("{}-private-cursor", spec.slug)),
            error_code: None,
            started_at: started_at.to_rfc3339(),
            completed_at: spec.completed_at.to_rfc3339(),
        },
    )
    .unwrap();
    for capability in &capabilities {
        open_commerce_capability_source_service::link_source(
            store,
            project_id,
            &capability.id,
            actor,
            LinkCapabilitySourceRequest {
                integration_id: integration.id.clone(),
                sync_receipt_id: receipt.id.clone(),
                data_domain: spec.data_domain.to_string(),
            },
        )
        .unwrap();
    }
    publish_directory(store, project_id, actor, &merchant.id);
    SourcedMerchant {
        merchant_id: merchant.id,
        capability_ids: capabilities
            .into_iter()
            .map(|capability| capability.id)
            .collect(),
        integration_id: integration.id,
        sync_receipt_id: receipt.id,
    }
}

fn invalidate_capability_link(
    store: &Store,
    project_id: &str,
    actor: &OpenCommerceActor<'_>,
    capability_id: &str,
) {
    open_commerce_service::update_capability(
        store,
        project_id,
        capability_id,
        actor,
        UpdateCapabilityRequest {
            display_name: None,
            description: Some("更新后使旧来源绑定失效".to_string()),
            access_level: None,
            input_schema: None,
            output_schema: None,
            handler_type: None,
            handler_config: None,
            unit_price_micros: None,
            currency: None,
            freshness_seconds: None,
            status: None,
        },
    )
    .unwrap();
}

fn create_merchant(
    store: &Store,
    project_id: &str,
    actor: &OpenCommerceActor<'_>,
    display_name: &str,
    slug: &str,
) -> crate::open_commerce_model::OpenCommerceMerchant {
    open_commerce_service::create_merchant(
        store,
        project_id,
        actor,
        CreateMerchantRequest {
            display_name: display_name.to_string(),
            slug: Some(slug.to_string()),
            description: "消费者可信来源筛选验证".to_string(),
            node_mode: "platform_hosted".to_string(),
            public_profile: json!({"category":"retail","city":"吉安"}),
        },
    )
    .unwrap()
}

fn create_capability(
    store: &Store,
    project_id: &str,
    actor: &OpenCommerceActor<'_>,
    merchant_id: &str,
    capability_key: &str,
    freshness_seconds: i64,
) -> crate::open_commerce_model::OpenCommerceCapability {
    open_commerce_service::publish_capability(
        store,
        project_id,
        merchant_id,
        actor,
        CreateCapabilityRequest {
            capability_key: capability_key.to_string(),
            display_name: format!("{capability_key} 能力"),
            description: String::new(),
            kind: "query".to_string(),
            access_level: ACCESS_PUBLIC.to_string(),
            input_schema: json!({"type":"object"}),
            output_schema: json!({"type":"object"}),
            handler_type: HANDLER_STATIC_JSON.to_string(),
            handler_config: Some(json!({"response":{"items":[]}})),
            unit_price_micros: 0,
            currency: "CNY".to_string(),
            freshness_seconds,
        },
    )
    .unwrap()
}

fn publish_directory(
    store: &Store,
    project_id: &str,
    actor: &OpenCommerceActor<'_>,
    merchant_id: &str,
) {
    open_commerce_directory_service::set_publication(store, project_id, merchant_id, actor, true)
        .unwrap();
}

fn actor(user_id: &str) -> OpenCommerceActor<'_> {
    OpenCommerceActor {
        user_id,
        app_id: "pc-web",
        project_role: Some("owner"),
    }
}
