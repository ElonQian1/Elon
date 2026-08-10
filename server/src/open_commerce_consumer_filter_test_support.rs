use serde_json::{json, Value};
use uuid::Uuid;

use crate::{
    open_commerce_consumer_model::{
        ConsumerDiscoveryRequest, ConsumerDiscoveryResponse, ConsumerPreferences,
    },
    open_commerce_directory_service,
    open_commerce_model::{
        CreateCapabilityRequest, CreateMerchantRequest, ACCESS_AUTHORIZED, ACCESS_OWNER_ONLY,
        ACCESS_PUBLIC, HANDLER_STATIC_JSON,
    },
    open_commerce_service::{self, OpenCommerceActor},
    store::Store,
};

pub(super) const PRICE_CAPABILITY: &str = "price.lookup";
pub(super) const MATRIX_CAPABILITY: &str = "matrix.operation";
pub(super) const CONSTRAINT_CAPABILITY: &str = "constraint.lookup";

pub(super) const PRICE_CNY_BELOW: &str = "Price CNY Below";
pub(super) const PRICE_CNY_EQUAL: &str = "Price CNY Equal";
pub(super) const PRICE_CNY_ABOVE: &str = "Price CNY Above";
pub(super) const PRICE_USD: &str = "Price USD";
pub(super) const PRICE_EUR: &str = "Price EUR";

pub(super) const MATRIX_QUERY_PUBLIC: &str = "Capability Query Public";
pub(super) const MATRIX_QUERY_AUTHORIZED: &str = "Capability Query Authorized";
pub(super) const MATRIX_ACTION_PUBLIC: &str = "Capability Action Public";
pub(super) const MATRIX_ACTION_AUTHORIZED: &str = "Capability Action Authorized";

pub(super) const CONSTRAINT_EXACT: &str = "Constraint Exact";
pub(super) const CONSTRAINT_WRONG_CITY: &str = "Constraint Wrong City";
pub(super) const CONSTRAINT_WRONG_CATEGORY: &str = "Constraint Wrong Category";
pub(super) const CONSTRAINT_PARTIAL_TAGS: &str = "Constraint Partial Tags";
pub(super) const CONSTRAINT_MALFORMED: &str = "Constraint Malformed";

#[derive(Debug, PartialEq, Eq)]
pub(super) struct ReadSnapshot {
    counts: Vec<(&'static str, i64)>,
}

pub(super) struct Fixture {
    pub(super) store: Store,
    pub(super) consumer_id: String,
}

pub(super) fn fixture() -> Fixture {
    let path = std::env::temp_dir().join(format!(
        "elon-consumer-filter-{}.sqlite",
        Uuid::new_v4().simple()
    ));
    let store = Store::open(&path).expect("consumer filter store should open");
    let owner = store
        .create_user(
            &format!(
                "consumer-filter-owner-{}@example.com",
                Uuid::new_v4().simple()
            ),
            "secret1",
            Some("Consumer Filter Owner"),
            None,
        )
        .unwrap();
    let consumer = store
        .create_user(
            &format!(
                "consumer-filter-user-{}@example.com",
                Uuid::new_v4().simple()
            ),
            "secret1",
            Some("Consumer Filter User"),
            None,
        )
        .unwrap();
    let project = store
        .create_project(&owner.id, "Consumer Filter Merchant Project", None, None)
        .unwrap()
        .project;
    let actor = actor(&owner.id);

    for (name, slug, price, currency) in [
        (PRICE_CNY_BELOW, "price-cny-below", 499, "CNY"),
        (PRICE_CNY_EQUAL, "price-cny-equal", 500, "CNY"),
        (PRICE_CNY_ABOVE, "price-cny-above", 501, "CNY"),
        (PRICE_USD, "price-usd", 1, "USD"),
        (PRICE_EUR, "price-eur", 1, "EUR"),
    ] {
        create_published_merchant(
            &store,
            &project.id,
            &actor,
            MerchantSpec {
                display_name: name,
                slug,
                public_profile: json!({"category":"price"}),
                capability_key: PRICE_CAPABILITY,
                kind: "query",
                access_level: ACCESS_PUBLIC,
                unit_price_micros: price,
                currency,
            },
        );
    }

    let query_public_id = create_published_merchant(
        &store,
        &project.id,
        &actor,
        MerchantSpec {
            display_name: MATRIX_QUERY_PUBLIC,
            slug: "matrix-query-public",
            public_profile: json!({"category":"matrix"}),
            capability_key: MATRIX_CAPABILITY,
            kind: "query",
            access_level: ACCESS_PUBLIC,
            unit_price_micros: 100,
            currency: "CNY",
        },
    );
    for (name, slug, kind, access_level) in [
        (
            MATRIX_QUERY_AUTHORIZED,
            "matrix-query-authorized",
            "query",
            ACCESS_AUTHORIZED,
        ),
        (
            MATRIX_ACTION_PUBLIC,
            "matrix-action-public",
            "action",
            ACCESS_PUBLIC,
        ),
        (
            MATRIX_ACTION_AUTHORIZED,
            "matrix-action-authorized",
            "action",
            ACCESS_AUTHORIZED,
        ),
    ] {
        create_published_merchant(
            &store,
            &project.id,
            &actor,
            MerchantSpec {
                display_name: name,
                slug,
                public_profile: json!({"category":"matrix"}),
                capability_key: MATRIX_CAPABILITY,
                kind,
                access_level,
                unit_price_micros: 100,
                currency: "CNY",
            },
        );
    }
    create_capability(
        &store,
        &project.id,
        &query_public_id,
        &actor,
        "matrix.owner",
        "query",
        ACCESS_OWNER_ONLY,
        0,
        "CNY",
    );

    for (name, slug, profile) in [
        (
            CONSTRAINT_EXACT,
            "constraint-exact",
            json!({"city":"JiAn","category":"Cafe","tags":["Quiet","WiFi","Vegan"]}),
        ),
        (
            CONSTRAINT_WRONG_CITY,
            "constraint-wrong-city",
            json!({"city":"NanChang","category":"Cafe","tags":["Quiet","WiFi","Vegan"]}),
        ),
        (
            CONSTRAINT_WRONG_CATEGORY,
            "constraint-wrong-category",
            json!({"city":"JiAn","category":"Retail","tags":["Quiet","WiFi","Vegan"]}),
        ),
        (
            CONSTRAINT_PARTIAL_TAGS,
            "constraint-partial-tags",
            json!({"city":"JiAn","category":"Cafe","tags":["Quiet"]}),
        ),
        (
            CONSTRAINT_MALFORMED,
            "constraint-malformed",
            json!({"city":42,"category":["Cafe"],"tags":"Quiet"}),
        ),
    ] {
        create_published_merchant(
            &store,
            &project.id,
            &actor,
            MerchantSpec {
                display_name: name,
                slug,
                public_profile: profile,
                capability_key: CONSTRAINT_CAPABILITY,
                kind: "query",
                access_level: ACCESS_PUBLIC,
                unit_price_micros: 100,
                currency: "CNY",
            },
        );
    }

    Fixture {
        store,
        consumer_id: consumer.id,
    }
}

impl Fixture {
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
}

pub(super) fn request(query: &str) -> ConsumerDiscoveryRequest {
    ConsumerDiscoveryRequest {
        query: Some(query.to_string()),
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

pub(super) fn names(response: &ConsumerDiscoveryResponse) -> Vec<String> {
    let mut names = response
        .matches
        .iter()
        .map(|item| item.merchant.display_name.clone())
        .collect::<Vec<_>>();
    names.sort();
    names
}

struct MerchantSpec<'a> {
    display_name: &'a str,
    slug: &'a str,
    public_profile: Value,
    capability_key: &'a str,
    kind: &'a str,
    access_level: &'a str,
    unit_price_micros: i64,
    currency: &'a str,
}

fn create_published_merchant(
    store: &Store,
    project_id: &str,
    actor: &OpenCommerceActor<'_>,
    spec: MerchantSpec<'_>,
) -> String {
    let merchant = open_commerce_service::create_merchant(
        store,
        project_id,
        actor,
        CreateMerchantRequest {
            display_name: spec.display_name.to_string(),
            slug: Some(spec.slug.to_string()),
            description: "消费者筛选矩阵".to_string(),
            node_mode: "platform_hosted".to_string(),
            public_profile: spec.public_profile,
        },
    )
    .unwrap();
    create_capability(
        store,
        project_id,
        &merchant.id,
        actor,
        spec.capability_key,
        spec.kind,
        spec.access_level,
        spec.unit_price_micros,
        spec.currency,
    );
    open_commerce_directory_service::set_publication(store, project_id, &merchant.id, actor, true)
        .unwrap();
    merchant.id
}

#[allow(clippy::too_many_arguments)]
fn create_capability(
    store: &Store,
    project_id: &str,
    merchant_id: &str,
    actor: &OpenCommerceActor<'_>,
    capability_key: &str,
    kind: &str,
    access_level: &str,
    unit_price_micros: i64,
    currency: &str,
) {
    open_commerce_service::publish_capability(
        store,
        project_id,
        merchant_id,
        actor,
        CreateCapabilityRequest {
            capability_key: capability_key.to_string(),
            display_name: format!("{capability_key} 能力"),
            description: String::new(),
            kind: kind.to_string(),
            access_level: access_level.to_string(),
            input_schema: json!({"type":"object"}),
            output_schema: json!({"type":"object"}),
            handler_type: HANDLER_STATIC_JSON.to_string(),
            handler_config: Some(json!({"response":{"ok":true}})),
            unit_price_micros,
            currency: currency.to_string(),
            freshness_seconds: 86_400,
        },
    )
    .unwrap();
}

fn actor(user_id: &str) -> OpenCommerceActor<'_> {
    OpenCommerceActor {
        user_id,
        app_id: "pc-web",
        project_role: Some("owner"),
    }
}
