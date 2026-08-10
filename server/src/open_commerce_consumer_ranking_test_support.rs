use rusqlite::params;
use serde_json::{json, Value};
use uuid::Uuid;

use crate::{
    open_commerce_consumer_model::{
        ConsumerDiscoveryRequest, ConsumerDiscoveryResponse, ConsumerPreferences,
    },
    open_commerce_directory_service,
    open_commerce_model::{CreateCapabilityRequest, CreateMerchantRequest, HANDLER_STATIC_JSON},
    open_commerce_service::{self, OpenCommerceActor},
    store::Store,
};

#[derive(Debug, PartialEq, Eq)]
pub(super) struct ReadSnapshot {
    counts: Vec<(&'static str, i64)>,
}

pub(super) struct Fixture {
    pub(super) store: Store,
    owner_id: String,
    project_id: String,
    consumer_id: String,
}

pub(super) struct CapabilitySpec<'a> {
    pub(super) key: &'a str,
    pub(super) access_level: &'a str,
    pub(super) unit_price_micros: i64,
    pub(super) freshness_seconds: i64,
}

pub(super) struct MerchantSpec<'a> {
    pub(super) display_name: &'a str,
    pub(super) slug: &'a str,
    pub(super) description: &'a str,
    pub(super) public_profile: Value,
    pub(super) capabilities: Vec<CapabilitySpec<'a>>,
}

pub(super) fn fixture() -> Fixture {
    let path = std::env::temp_dir().join(format!(
        "elon-consumer-ranking-{}.sqlite",
        Uuid::new_v4().simple()
    ));
    let store = Store::open(&path).expect("consumer ranking store should open");
    let owner = store
        .create_user(
            &format!(
                "consumer-ranking-owner-{}@example.com",
                Uuid::new_v4().simple()
            ),
            "secret1",
            Some("Consumer Ranking Owner"),
            None,
        )
        .unwrap();
    let consumer = store
        .create_user(
            &format!(
                "consumer-ranking-user-{}@example.com",
                Uuid::new_v4().simple()
            ),
            "secret1",
            Some("Consumer Ranking User"),
            None,
        )
        .unwrap();
    let project = store
        .create_project(&owner.id, "Consumer Ranking Project", None, None)
        .unwrap()
        .project;
    Fixture {
        store,
        owner_id: owner.id,
        project_id: project.id,
        consumer_id: consumer.id,
    }
}

impl Fixture {
    pub(super) fn publish(&self, spec: MerchantSpec<'_>) -> String {
        let actor = self.actor();
        let merchant = open_commerce_service::create_merchant(
            &self.store,
            &self.project_id,
            &actor,
            CreateMerchantRequest {
                display_name: spec.display_name.to_string(),
                slug: Some(spec.slug.to_string()),
                description: spec.description.to_string(),
                node_mode: "platform_hosted".to_string(),
                public_profile: spec.public_profile,
            },
        )
        .unwrap();
        for capability in spec.capabilities {
            self.publish_capability(&merchant.id, capability);
        }
        open_commerce_directory_service::set_publication(
            &self.store,
            &self.project_id,
            &merchant.id,
            &actor,
            true,
        )
        .unwrap();
        merchant.id
    }

    pub(super) fn publish_capability(&self, merchant_id: &str, spec: CapabilitySpec<'_>) {
        let actor = self.actor();
        open_commerce_service::publish_capability(
            &self.store,
            &self.project_id,
            merchant_id,
            &actor,
            CreateCapabilityRequest {
                capability_key: spec.key.to_string(),
                display_name: format!("{} capability", spec.key),
                description: String::new(),
                kind: "query".to_string(),
                access_level: spec.access_level.to_string(),
                input_schema: json!({"type":"object"}),
                output_schema: json!({"type":"object"}),
                handler_type: HANDLER_STATIC_JSON.to_string(),
                handler_config: Some(json!({"response":{"ok":true}})),
                unit_price_micros: spec.unit_price_micros,
                currency: "CNY".to_string(),
                freshness_seconds: spec.freshness_seconds,
            },
        )
        .unwrap();
    }

    pub(super) fn set_capability_time(
        &self,
        merchant_id: &str,
        capability_key: &str,
        timestamp: &str,
    ) {
        let changed = self
            .store
            .conn()
            .unwrap()
            .execute(
                "UPDATE open_commerce_capabilities SET updated_at = ?1
                   WHERE merchant_id = ?2 AND capability_key = ?3",
                params![timestamp, merchant_id, capability_key],
            )
            .unwrap();
        assert_eq!(changed, 1);
    }

    pub(super) fn set_directory_time(&self, merchant_id: &str, timestamp: &str) {
        let conn = self.store.conn().unwrap();
        assert_eq!(
            conn.execute(
                "UPDATE open_commerce_merchants SET updated_at = ?1 WHERE id = ?2",
                params![timestamp, merchant_id],
            )
            .unwrap(),
            1
        );
        assert_eq!(
            conn.execute(
                "UPDATE open_commerce_directory_publications
                    SET updated_at = ?1, published_at = ?1 WHERE merchant_id = ?2",
                params![timestamp, merchant_id],
            )
            .unwrap(),
            1
        );
    }

    pub(super) fn discover(
        &self,
        request: ConsumerDiscoveryRequest,
    ) -> anyhow::Result<ConsumerDiscoveryResponse> {
        super::discover(&self.store, &self.consumer_id, request)
    }

    pub(super) fn snapshot(&self) -> ReadSnapshot {
        let conn = self.store.conn().unwrap();
        let counts = [
            "open_commerce_merchants",
            "open_commerce_capabilities",
            "open_commerce_authorization_requests",
            "open_commerce_action_confirmations",
            "open_commerce_invocations",
            "open_commerce_grants",
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

    fn actor(&self) -> OpenCommerceActor<'_> {
        OpenCommerceActor {
            user_id: &self.owner_id,
            app_id: "pc-web",
            project_role: Some("owner"),
        }
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

pub(super) fn capability<'a>(
    key: &'a str,
    access_level: &'a str,
    unit_price_micros: i64,
    freshness_seconds: i64,
) -> CapabilitySpec<'a> {
    CapabilitySpec {
        key,
        access_level,
        unit_price_micros,
        freshness_seconds,
    }
}

pub(super) fn merchant<'a>(
    display_name: &'a str,
    slug: &'a str,
    public_profile: Value,
    capabilities: Vec<CapabilitySpec<'a>>,
) -> MerchantSpec<'a> {
    MerchantSpec {
        display_name,
        slug,
        description: "consumer ranking fixture",
        public_profile,
        capabilities,
    }
}

pub(super) fn names(response: &ConsumerDiscoveryResponse) -> Vec<String> {
    response
        .matches
        .iter()
        .map(|item| item.merchant.display_name.clone())
        .collect()
}
