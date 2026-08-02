use chrono::{Duration, Utc};
use serde_json::json;
use uuid::Uuid;

use crate::{
    open_commerce_consumer_model::ConsumerPreferences,
    open_commerce_consumer_preference_mcp,
    open_commerce_consumer_preference_model::{
        UpsertConsumerPreferenceDisclosureRequest, UpsertConsumerPreferenceProfileRequest,
        PREFERENCE_FIELD_CITY, PREFERENCE_FIELD_TAGS,
    },
    open_commerce_consumer_preference_service, open_commerce_directory_service,
    open_commerce_model::{
        CreateCapabilityRequest, CreateMerchantRequest, ACCESS_PUBLIC, HANDLER_STATIC_JSON,
    },
    open_commerce_relationship_model::{
        CreateConsumerRelationshipRequest, RELATIONSHIP_SCOPE_MEMBERSHIP_LINK,
        RELATIONSHIP_SCOPE_PREFERENCE_REMEMBER,
    },
    open_commerce_relationship_service,
    open_commerce_service::{self, OpenCommerceActor},
    store::Store,
};

#[test]
fn preference_profile_and_disclosure_are_owner_controlled_and_fail_closed() {
    let fixture = fixture();
    let consumer_actor = fixture.consumer_actor();
    let profile = open_commerce_consumer_preference_service::upsert_profile(
        &fixture.store,
        &fixture.consumer_project_id,
        &consumer_actor,
        profile_request(),
    )
    .unwrap();
    assert_eq!(profile.revision, 1);
    assert_eq!(profile.preferences.tags, vec!["quiet", "coffee"]);
    assert_eq!(profile.preferences.city.as_deref(), Some("Ji'an"));

    let stranger = fixture
        .store
        .create_user("preference-stranger@example.com", "secret1", None, None)
        .unwrap();
    let stranger_actor = OpenCommerceActor {
        user_id: &stranger.id,
        app_id: "pc-web",
        project_role: Some("editor"),
    };
    assert!(open_commerce_consumer_preference_service::get_profile(
        &fixture.store,
        &fixture.consumer_project_id,
        &stranger_actor,
    )
    .unwrap()
    .is_none());

    let disclosure = open_commerce_consumer_preference_service::upsert_disclosure(
        &fixture.store,
        &fixture.consumer_project_id,
        &fixture.relationship_id,
        &consumer_actor,
        UpsertConsumerPreferenceDisclosureRequest {
            shared_fields: vec![
                PREFERENCE_FIELD_TAGS.to_string(),
                PREFERENCE_FIELD_CITY.to_string(),
            ],
        },
    )
    .unwrap();
    assert_eq!(disclosure.profile_revision, profile.revision);
    assert_eq!(disclosure.preferences.tags.unwrap().len(), 2);
    assert_eq!(disclosure.preferences.city.as_deref(), Some("Ji'an"));
    assert!(disclosure.preferences.categories.is_none());

    let merchant_view = open_commerce_consumer_preference_service::list_merchant_disclosures(
        &fixture.store,
        &fixture.merchant_project_id,
        &fixture.merchant_id,
        &fixture.merchant_actor(),
        10,
    )
    .unwrap();
    assert_eq!(merchant_view.len(), 1);
    let serialized = serde_json::to_string(&merchant_view).unwrap();
    assert!(!serialized.contains(&fixture.consumer_user_id));
    assert!(!serialized.contains(&fixture.consumer_project_id));
    assert!(!serialized.contains("consumer_user_id"));

    open_commerce_relationship_service::revoke_relationship(
        &fixture.store,
        &fixture.consumer_project_id,
        &fixture.relationship_id,
        &consumer_actor,
    )
    .unwrap();
    assert!(
        open_commerce_consumer_preference_service::list_merchant_disclosures(
            &fixture.store,
            &fixture.merchant_project_id,
            &fixture.merchant_id,
            &fixture.merchant_actor(),
            10,
        )
        .unwrap()
        .is_empty()
    );
    let owner_view = open_commerce_consumer_preference_service::get_disclosure(
        &fixture.store,
        &fixture.consumer_project_id,
        &fixture.relationship_id,
        &consumer_actor,
    )
    .unwrap()
    .unwrap();
    assert_eq!(owner_view.relationship_status, "revoked");

    let membership_only = create_relationship(
        &fixture,
        vec![RELATIONSHIP_SCOPE_MEMBERSHIP_LINK.to_string()],
    );
    assert!(
        open_commerce_consumer_preference_service::upsert_disclosure(
            &fixture.store,
            &fixture.consumer_project_id,
            &membership_only.id,
            &consumer_actor,
            UpsertConsumerPreferenceDisclosureRequest {
                shared_fields: vec![PREFERENCE_FIELD_TAGS.to_string()],
            },
        )
        .is_err()
    );

    let preference_relationship = create_relationship(
        &fixture,
        vec![RELATIONSHIP_SCOPE_PREFERENCE_REMEMBER.to_string()],
    );
    open_commerce_consumer_preference_service::upsert_disclosure(
        &fixture.store,
        &fixture.consumer_project_id,
        &preference_relationship.id,
        &consumer_actor,
        UpsertConsumerPreferenceDisclosureRequest {
            shared_fields: vec![PREFERENCE_FIELD_TAGS.to_string()],
        },
    )
    .unwrap();
    let deleted = open_commerce_consumer_preference_service::delete_profile(
        &fixture.store,
        &fixture.consumer_project_id,
        &consumer_actor,
    )
    .unwrap();
    assert!(deleted.deleted_profile);
    assert!(deleted.removed_disclosures >= 2);
    assert!(open_commerce_consumer_preference_service::get_profile(
        &fixture.store,
        &fixture.consumer_project_id,
        &consumer_actor,
    )
    .unwrap()
    .is_none());
}

#[test]
fn preference_mcp_exposes_bounded_tools_and_reuses_domain_service() {
    let fixture = fixture();
    let definitions = open_commerce_consumer_preference_mcp::definitions();
    assert_eq!(definitions.len(), 8);
    assert!(definitions
        .iter()
        .any(|tool| { tool["name"] == "open_commerce_upsert_consumer_preference_disclosure" }));

    let saved = open_commerce_consumer_preference_mcp::call_if_handled(
        &fixture.store,
        &fixture.consumer_project_id,
        &fixture.consumer_user_id,
        "owner",
        "mcp-client",
        "open_commerce_upsert_consumer_preference_profile",
        json!({
            "preferences":{
                "categories":["cafe"],
                "tags":["quiet","coffee"],
                "city":"Ji'an",
                "max_unit_price_micros":20000000,
                "prefer_public":true
            }
        }),
    )
    .unwrap()
    .unwrap();
    assert_eq!(saved["revision"], 1);
    let shared = open_commerce_consumer_preference_mcp::call_if_handled(
        &fixture.store,
        &fixture.consumer_project_id,
        &fixture.consumer_user_id,
        "owner",
        "mcp-client",
        "open_commerce_upsert_consumer_preference_disclosure",
        json!({
            "relationship_id":fixture.relationship_id,
            "shared_fields":["tags"]
        }),
    )
    .unwrap()
    .unwrap();
    assert_eq!(shared["preferences"]["tags"][0], "quiet");

    let merchant_view = open_commerce_consumer_preference_mcp::call_if_handled(
        &fixture.store,
        &fixture.merchant_project_id,
        &fixture.merchant_user_id,
        "owner",
        "mcp-client",
        "open_commerce_list_merchant_preference_disclosures",
        json!({"merchant_id":fixture.merchant_id}),
    )
    .unwrap()
    .unwrap();
    assert_eq!(merchant_view.as_array().unwrap().len(), 1);
    assert!(!merchant_view
        .to_string()
        .contains(&fixture.consumer_user_id));
}

struct Fixture {
    store: Store,
    merchant_user_id: String,
    merchant_project_id: String,
    merchant_id: String,
    consumer_user_id: String,
    consumer_project_id: String,
    relationship_id: String,
}

impl Fixture {
    fn consumer_actor(&self) -> OpenCommerceActor<'_> {
        OpenCommerceActor {
            user_id: &self.consumer_user_id,
            app_id: "pc-web",
            project_role: Some("owner"),
        }
    }

    fn merchant_actor(&self) -> OpenCommerceActor<'_> {
        OpenCommerceActor {
            user_id: &self.merchant_user_id,
            app_id: "pc-web",
            project_role: Some("owner"),
        }
    }
}

fn fixture() -> Fixture {
    let path = std::env::temp_dir().join(format!(
        "elon-open-commerce-preference-{}.sqlite",
        Uuid::new_v4().simple()
    ));
    let store = Store::open(&path).unwrap();
    let merchant_owner = store
        .create_user("preference-merchant@example.com", "secret1", None, None)
        .unwrap();
    let merchant_project = store
        .create_project(&merchant_owner.id, "Preference merchant", None, None)
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
            display_name: "偏好披露测试商户".to_string(),
            slug: Some(format!("preference-{}", Uuid::new_v4().simple())),
            description: String::new(),
            node_mode: "platform_hosted".to_string(),
            public_profile: json!({"city":"Ji'an","tags":["quiet","coffee"]}),
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
        .create_user("preference-consumer@example.com", "secret1", None, None)
        .unwrap();
    let consumer_project = store
        .create_project(&consumer.id, "Preference wallet", None, None)
        .unwrap()
        .project;
    let relationship = open_commerce_relationship_service::create_relationship(
        &store,
        &consumer_project.id,
        &OpenCommerceActor {
            user_id: &consumer.id,
            app_id: "pc-web",
            project_role: Some("owner"),
        },
        relationship_request(
            &merchant.id,
            vec![RELATIONSHIP_SCOPE_PREFERENCE_REMEMBER.to_string()],
        ),
    )
    .unwrap();
    Fixture {
        store,
        merchant_user_id: merchant_owner.id,
        merchant_project_id: merchant_project.id,
        merchant_id: merchant.id,
        consumer_user_id: consumer.id,
        consumer_project_id: consumer_project.id,
        relationship_id: relationship.id,
    }
}

fn create_relationship(
    fixture: &Fixture,
    scopes: Vec<String>,
) -> crate::open_commerce_relationship_model::OpenCommerceConsumerRelationship {
    open_commerce_relationship_service::create_relationship(
        &fixture.store,
        &fixture.consumer_project_id,
        &fixture.consumer_actor(),
        relationship_request(&fixture.merchant_id, scopes),
    )
    .unwrap()
}

fn relationship_request(
    merchant_id: &str,
    scopes: Vec<String>,
) -> CreateConsumerRelationshipRequest {
    CreateConsumerRelationshipRequest {
        merchant_id: merchant_id.to_string(),
        source_app_id: "pc-web".to_string(),
        scopes,
        purpose: "测试消费者偏好披露".to_string(),
        expires_at: (Utc::now() + Duration::days(90)).to_rfc3339(),
    }
}

fn profile_request() -> UpsertConsumerPreferenceProfileRequest {
    UpsertConsumerPreferenceProfileRequest {
        preferences: ConsumerPreferences {
            categories: vec!["cafe".to_string()],
            tags: vec![
                " quiet ".to_string(),
                "coffee".to_string(),
                "QUIET".to_string(),
            ],
            city: Some(" Ji'an ".to_string()),
            max_unit_price_micros: Some(20_000_000),
            prefer_public: true,
        },
    }
}
