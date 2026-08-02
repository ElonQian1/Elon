use chrono::{Duration, Utc};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::{
    open_commerce_developer_model::CreateDeveloperAppRequest,
    open_commerce_directory_service,
    open_commerce_model::{
        CreateCapabilityRequest, CreateMerchantRequest, ACCESS_PUBLIC, HANDLER_STATIC_JSON,
    },
    open_commerce_relationship_model::{
        CreateConsumerRelationshipRequest, RenewConsumerRelationshipRequest,
        RELATIONSHIP_SCOPE_PREFERENCE_REMEMBER,
    },
    open_commerce_relationship_service,
    open_commerce_service::{self, OpenCommerceActor},
    store::Store,
};

struct Fixture {
    store: Store,
    merchant_owner_id: String,
    merchant_project_id: String,
    merchant_id: String,
    consumer_user_id: String,
    consumer_project_id: String,
}

#[test]
fn renewal_rotates_alias_is_idempotent_and_stays_consumer_private() {
    let fixture = fixture();
    let consumer_actor = actor(&fixture.consumer_user_id, "pc-web", "owner");
    let merchant_actor = actor(&fixture.merchant_owner_id, "pc-web", "owner");
    let source = create_relationship(&fixture, &consumer_actor);
    let first_renewal_request = renewal_request("pc-web", 90);

    let renewed = open_commerce_relationship_service::renew_relationship(
        &fixture.store,
        &fixture.consumer_project_id,
        &source.id,
        &consumer_actor,
        first_renewal_request.clone(),
    )
    .unwrap();
    assert_ne!(renewed.id, source.id);
    assert_ne!(renewed.subject_alias, source.subject_alias);
    assert_eq!(renewed.scopes, source.scopes);
    assert_eq!(renewed.purpose, source.purpose);

    open_commerce_directory_service::set_publication(
        &fixture.store,
        &fixture.merchant_project_id,
        &fixture.merchant_id,
        &merchant_actor,
        false,
    )
    .unwrap();
    let retry = open_commerce_relationship_service::renew_relationship(
        &fixture.store,
        &fixture.consumer_project_id,
        &source.id,
        &consumer_actor,
        first_renewal_request,
    )
    .unwrap();
    assert_eq!(retry.id, renewed.id);
    assert_eq!(retry.subject_alias, renewed.subject_alias);

    let own = open_commerce_relationship_service::list_consumer_relationships(
        &fixture.store,
        &fixture.consumer_project_id,
        &consumer_actor,
        10,
    )
    .unwrap();
    assert_eq!(own[0].id, renewed.id);
    assert_eq!(own[1].status, "revoked");
    assert_private_chain_absent(&serde_json::to_value(&own).unwrap());
    let merchant_view = open_commerce_relationship_service::list_merchant_relationships(
        &fixture.store,
        &fixture.merchant_project_id,
        &fixture.merchant_id,
        &merchant_actor,
        10,
    )
    .unwrap();
    assert_private_chain_absent(&serde_json::to_value(&merchant_view).unwrap());

    assert!(fixture
        .store
        .renew_open_commerce_consumer_relationship(
            &fixture.consumer_project_id,
            &fixture.consumer_user_id,
            &renewed.id,
            "pc-web",
            &(Utc::now() + Duration::days(90)).to_rfc3339(),
        )
        .is_err());
    assert!(open_commerce_relationship_service::renew_relationship(
        &fixture.store,
        &fixture.consumer_project_id,
        &renewed.id,
        &consumer_actor,
        renewal_request("pc-web", 90),
    )
    .is_err());
}

#[test]
fn renewal_requires_owner_valid_source_app_and_bounded_expiry() {
    let fixture = fixture();
    let consumer_actor = actor(&fixture.consumer_user_id, "pc-web", "owner");
    let source = create_relationship(&fixture, &consumer_actor);
    let disabled_app = fixture
        .store
        .create_open_commerce_developer_app(
            &fixture.consumer_project_id,
            &fixture.consumer_user_id,
            CreateDeveloperAppRequest {
                app_id: "renewal-client".to_string(),
                display_name: "续期客户端".to_string(),
            },
        )
        .unwrap();
    fixture
        .store
        .disable_open_commerce_developer_app(&fixture.consumer_project_id, &disabled_app.app.id)
        .unwrap();
    assert!(fixture
        .store
        .renew_open_commerce_consumer_relationship(
            &fixture.consumer_project_id,
            &fixture.consumer_user_id,
            &source.id,
            "renewal-client",
            &(Utc::now() + Duration::days(90)).to_rfc3339(),
        )
        .is_err());
    let stranger = fixture
        .store
        .create_user(
            "relationship-renewal-stranger@example.com",
            "secret1",
            None,
            None,
        )
        .unwrap();
    let stranger_actor = actor(&stranger.id, "pc-web", "editor");
    assert!(open_commerce_relationship_service::renew_relationship(
        &fixture.store,
        &fixture.consumer_project_id,
        &source.id,
        &stranger_actor,
        renewal_request("pc-web", 90),
    )
    .is_err());

    assert!(open_commerce_relationship_service::renew_relationship(
        &fixture.store,
        &fixture.consumer_project_id,
        &source.id,
        &consumer_actor,
        renewal_request("pc-web", 400),
    )
    .unwrap_err()
    .to_string()
    .contains("366 天"));
    assert!(open_commerce_relationship_service::renew_relationship(
        &fixture.store,
        &fixture.consumer_project_id,
        &source.id,
        &consumer_actor,
        renewal_request("mcp-client", 90),
    )
    .unwrap_err()
    .to_string()
    .contains("不能冒充"));
}

#[tokio::test]
async fn mcp_renewal_uses_bound_app_and_can_continue_the_chain() {
    let fixture = fixture();
    let consumer_actor = actor(&fixture.consumer_user_id, "pc-web", "owner");
    let source = create_relationship(&fixture, &consumer_actor);
    let expires_at = (Utc::now() + Duration::days(30)).to_rfc3339();
    let renewed = crate::open_commerce_mcp::call_tool(
        &fixture.store,
        &fixture.consumer_project_id,
        &fixture.consumer_user_id,
        "owner",
        "mcp-client",
        json!({
            "name":"open_commerce_renew_consumer_relationship",
            "arguments":{"relationship_id":source.id,"expires_at":expires_at}
        }),
    )
    .await
    .unwrap();
    let content = &renewed["structuredContent"];
    assert_eq!(content["source_app_id"], "mcp-client");
    assert_ne!(content["subject_alias"], source.subject_alias);
    assert_private_chain_absent(content);

    let renewed_id = content["id"].as_str().unwrap();
    let next = open_commerce_relationship_service::renew_relationship(
        &fixture.store,
        &fixture.consumer_project_id,
        renewed_id,
        &consumer_actor,
        renewal_request("pc-web", 30),
    )
    .unwrap();
    assert_ne!(next.id, renewed_id);
}

fn fixture() -> Fixture {
    let path = std::env::temp_dir().join(format!(
        "elon-open-commerce-relationship-renewal-{}.sqlite",
        Uuid::new_v4().simple()
    ));
    let store = Store::open(&path).unwrap();
    let merchant_owner = store
        .create_user(
            "relationship-renewal-merchant@example.com",
            "secret1",
            None,
            None,
        )
        .unwrap();
    let merchant_project = store
        .create_project(&merchant_owner.id, "Renewal merchant", None, None)
        .unwrap()
        .project;
    let merchant_actor = actor(&merchant_owner.id, "pc-web", "owner");
    let merchant = open_commerce_service::create_merchant(
        &store,
        &merchant_project.id,
        &merchant_actor,
        CreateMerchantRequest {
            display_name: "续期测试商户".to_string(),
            slug: Some(format!("renewal-merchant-{}", Uuid::new_v4().simple())),
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
        .create_user(
            "relationship-renewal-consumer@example.com",
            "secret1",
            None,
            None,
        )
        .unwrap();
    let consumer_project = store
        .create_project(&consumer.id, "Renewal consumer", None, None)
        .unwrap()
        .project;
    Fixture {
        store,
        merchant_owner_id: merchant_owner.id,
        merchant_project_id: merchant_project.id,
        merchant_id: merchant.id,
        consumer_user_id: consumer.id,
        consumer_project_id: consumer_project.id,
    }
}

fn actor<'a>(user_id: &'a str, app_id: &'a str, role: &'a str) -> OpenCommerceActor<'a> {
    OpenCommerceActor {
        user_id,
        app_id,
        project_role: Some(role),
    }
}

fn create_relationship(
    fixture: &Fixture,
    actor: &OpenCommerceActor<'_>,
) -> crate::open_commerce_relationship_model::OpenCommerceConsumerRelationship {
    open_commerce_relationship_service::create_relationship(
        &fixture.store,
        &fixture.consumer_project_id,
        actor,
        CreateConsumerRelationshipRequest {
            merchant_id: fixture.merchant_id.clone(),
            source_app_id: "pc-web".to_string(),
            scopes: vec![RELATIONSHIP_SCOPE_PREFERENCE_REMEMBER.to_string()],
            purpose: "在授权期内记住我主动提供的偏好".to_string(),
            expires_at: (Utc::now() + Duration::days(30)).to_rfc3339(),
        },
    )
    .unwrap()
}

fn renewal_request(source_app_id: &str, days: i64) -> RenewConsumerRelationshipRequest {
    RenewConsumerRelationshipRequest {
        source_app_id: source_app_id.to_string(),
        expires_at: (Utc::now() + Duration::days(days)).to_rfc3339(),
    }
}

fn assert_private_chain_absent(value: &Value) {
    let text = serde_json::to_string(value).unwrap();
    assert!(!text.contains("renewed_from_relationship_id"));
    assert!(!text.contains("previous_subject_alias"));
    assert!(!text.contains("consumer_user_id"));
    assert!(!text.contains("consumer_project_id"));
}
