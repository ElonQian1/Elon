use chrono::{Duration, Utc};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::{
    open_commerce_consumer_model::ConsumerPreferences,
    open_commerce_consumer_preference_model::{
        UpsertConsumerPreferenceDisclosureRequest, UpsertConsumerPreferenceProfileRequest,
        PREFERENCE_FIELD_CATEGORIES, PREFERENCE_FIELD_CITY,
    },
    open_commerce_consumer_preference_service, open_commerce_directory_service,
    open_commerce_model::{
        CreateCapabilityRequest, CreateMerchantRequest, ACCESS_PUBLIC, HANDLER_STATIC_JSON,
    },
    open_commerce_portability_model::{
        CreateConsumerPortabilityExportRequest, CONSUMER_PORTABILITY_EXPORT_SCHEMA,
        CONSUMER_PORTABILITY_EXPORT_SCHEMA_V1, CONSUMER_PORTABILITY_PAYLOAD_SCHEMA,
        CONSUMER_PORTABILITY_PAYLOAD_SCHEMA_V1,
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
fn v2_exports_owner_preferences_and_keeps_idempotent_snapshots() {
    let fixture = fixture();
    let actor = fixture.actor();
    save_profile(&fixture, &actor, "吉安", vec!["安静"]);
    open_commerce_consumer_preference_service::upsert_disclosure(
        &fixture.store,
        &fixture.consumer_project_id,
        &fixture.relationship_id,
        &actor,
        UpsertConsumerPreferenceDisclosureRequest {
            shared_fields: vec![
                PREFERENCE_FIELD_CATEGORIES.to_string(),
                PREFERENCE_FIELD_CITY.to_string(),
            ],
        },
    )
    .unwrap();

    let first = export(&fixture, &actor, "portable-v2-first");
    assert_eq!(first.schema, CONSUMER_PORTABILITY_EXPORT_SCHEMA);
    assert_eq!(first.payload.schema, CONSUMER_PORTABILITY_PAYLOAD_SCHEMA);
    assert_eq!(
        first
            .payload
            .preference_profile
            .as_ref()
            .and_then(|profile| profile.preferences.city.as_deref()),
        Some("吉安")
    );
    assert_eq!(first.payload.preference_disclosures.len(), 1);
    assert_eq!(
        first.payload.preference_disclosures[0].shared_fields,
        vec![PREFERENCE_FIELD_CATEGORIES, PREFERENCE_FIELD_CITY]
    );
    let summary = first.summary();
    assert!(summary.preference_profile_included);
    assert_eq!(summary.preference_disclosure_count, 1);
    assert_private_owner_fields_absent(&serde_json::to_value(&first).unwrap(), &fixture.user_id);

    save_profile(&fixture, &actor, "南昌", vec!["新品"]);
    let retry = export(&fixture, &actor, "portable-v2-first");
    assert_eq!(retry.id, first.id);
    assert_eq!(retry.payload_sha256, first.payload_sha256);
    assert_eq!(
        retry
            .payload
            .preference_profile
            .as_ref()
            .and_then(|profile| profile.preferences.city.as_deref()),
        Some("吉安")
    );

    let second = export(&fixture, &actor, "portable-v2-second");
    assert_ne!(second.id, first.id);
    assert_eq!(
        second
            .payload
            .preference_profile
            .as_ref()
            .and_then(|profile| profile.preferences.city.as_deref()),
        Some("南昌")
    );
    assert_eq!(second.payload.preference_disclosures[0].profile_revision, 1);
}

#[test]
fn legacy_v1_payload_remains_byte_for_byte_verifiable() {
    let fixture = fixture();
    let actor = fixture.actor();
    let generated_at = "2026-08-01T00:00:00+00:00";
    let payload_json = format!(
        "{{\"schema\":\"{CONSUMER_PORTABILITY_PAYLOAD_SCHEMA_V1}\",\"source_project_id\":\"{}\",\"generated_at\":\"{generated_at}\",\"relationships\":[],\"relationship_renewals\":[],\"data_requests\":[]}}",
        fixture.consumer_project_id
    );
    let payload_sha256 = hex::encode(Sha256::digest(payload_json.as_bytes()));
    let (stored, created) = fixture
        .store
        .save_consumer_portability_export(
            &fixture.consumer_project_id,
            &fixture.user_id,
            "portable-legacy-v1",
            CONSUMER_PORTABILITY_EXPORT_SCHEMA_V1,
            &payload_json,
            &payload_sha256,
        )
        .unwrap();
    assert!(created);
    assert_eq!(stored.schema, CONSUMER_PORTABILITY_EXPORT_SCHEMA_V1);
    assert!(stored.payload.preference_profile.is_none());
    assert!(stored.payload.preference_disclosures.is_empty());

    let verified = open_commerce_portability_service::get_export(
        &fixture.store,
        &fixture.consumer_project_id,
        &stored.id,
        &actor,
    )
    .unwrap();
    assert_eq!(verified.payload_sha256, payload_sha256);
    let reserialized = serde_json::to_string(&verified.payload).unwrap();
    assert_eq!(reserialized, payload_json);

    let (mixed, _) = fixture
        .store
        .save_consumer_portability_export(
            &fixture.consumer_project_id,
            &fixture.user_id,
            "portable-mixed-version",
            CONSUMER_PORTABILITY_EXPORT_SCHEMA,
            &payload_json,
            &payload_sha256,
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
    user_id: String,
    consumer_project_id: String,
    relationship_id: String,
}

impl Fixture {
    fn actor(&self) -> OpenCommerceActor<'_> {
        OpenCommerceActor {
            user_id: &self.user_id,
            app_id: "pc-web",
            project_role: Some("owner"),
        }
    }
}

fn fixture() -> Fixture {
    let path = std::env::temp_dir().join(format!(
        "elon-open-commerce-portability-v2-{}.sqlite",
        Uuid::new_v4().simple()
    ));
    let store = Store::open(&path).unwrap();
    let merchant_owner = store
        .create_user("portability-v2-merchant@example.com", "secret1", None, None)
        .unwrap();
    let merchant_project = store
        .create_project(&merchant_owner.id, "Portability V2 merchant", None, None)
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
            display_name: "可携带偏好测试商户".to_string(),
            slug: Some(format!("portability-v2-{}", Uuid::new_v4().simple())),
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
            handler_config: Some(json!({"response":{}})),
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
        .create_user("portability-v2-consumer@example.com", "secret1", None, None)
        .unwrap();
    let consumer_project = store
        .create_project(&consumer.id, "Portability V2 wallet", None, None)
        .unwrap()
        .project;
    let actor = OpenCommerceActor {
        user_id: &consumer.id,
        app_id: "pc-web",
        project_role: Some("owner"),
    };
    let relationship = open_commerce_relationship_service::create_relationship(
        &store,
        &consumer_project.id,
        &actor,
        CreateConsumerRelationshipRequest {
            merchant_id: merchant.id.clone(),
            source_app_id: "pc-web".to_string(),
            scopes: vec![RELATIONSHIP_SCOPE_PREFERENCE_REMEMBER.to_string()],
            purpose: "导出本人低敏偏好".to_string(),
            expires_at: (Utc::now() + Duration::days(90)).to_rfc3339(),
        },
    )
    .unwrap();
    Fixture {
        store,
        user_id: consumer.id,
        consumer_project_id: consumer_project.id,
        relationship_id: relationship.id,
    }
}

fn save_profile(fixture: &Fixture, actor: &OpenCommerceActor<'_>, city: &str, tags: Vec<&str>) {
    open_commerce_consumer_preference_service::upsert_profile(
        &fixture.store,
        &fixture.consumer_project_id,
        actor,
        UpsertConsumerPreferenceProfileRequest {
            preferences: ConsumerPreferences {
                categories: vec!["咖啡".to_string()],
                tags: tags.into_iter().map(str::to_string).collect(),
                city: Some(city.to_string()),
                max_unit_price_micros: Some(50_000_000),
                prefer_public: true,
            },
        },
    )
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

fn assert_private_owner_fields_absent(value: &Value, user_id: &str) {
    let serialized = value.to_string();
    assert!(!serialized.contains(user_id));
    assert!(!serialized.contains("consumer_user_id"));
    assert!(!serialized.contains("consumer_project_id"));
}
