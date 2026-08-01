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
        CreateConsumerRelationshipRequest, RELATIONSHIP_SCOPE_MEMBERSHIP_LINK,
        RELATIONSHIP_SCOPE_PREFERENCE_REMEMBER,
    },
    open_commerce_relationship_service,
    open_commerce_service::{self, OpenCommerceActor},
    store::Store,
};

#[test]
fn consumer_relationship_is_owner_controlled_redacted_and_fails_closed() {
    let path = std::env::temp_dir().join(format!(
        "elon-open-commerce-relationship-{}.sqlite",
        Uuid::new_v4().simple()
    ));
    let store = Store::open(&path).unwrap();
    let merchant_owner = store
        .create_user("relationship-merchant@example.com", "secret1", None, None)
        .unwrap();
    let merchant_project = store
        .create_project(&merchant_owner.id, "Relationship merchant", None, None)
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
            display_name: "关系凭证测试商户".to_string(),
            slug: Some("relationship-merchant".to_string()),
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
        .create_user("relationship-consumer@example.com", "secret1", None, None)
        .unwrap();
    let consumer_project = store
        .create_project(&consumer.id, "Relationship wallet", None, None)
        .unwrap()
        .project;
    let consumer_actor = OpenCommerceActor {
        user_id: &consumer.id,
        app_id: "pc-web",
        project_role: Some("owner"),
    };
    let relationship = open_commerce_relationship_service::create_relationship(
        &store,
        &consumer_project.id,
        &consumer_actor,
        request(&merchant.id, 90),
    )
    .unwrap();
    assert_eq!(relationship.status, "active");
    assert!(relationship.subject_alias.starts_with("subject_"));
    assert_eq!(relationship.scopes.len(), 2);

    let serialized = serde_json::to_value(&relationship).unwrap();
    assert_no_private_identity(&serialized, &consumer.id, &consumer_project.id);
    let merchant_view = open_commerce_relationship_service::list_merchant_relationships(
        &store,
        &merchant_project.id,
        &merchant.id,
        &merchant_actor,
        10,
    )
    .unwrap();
    assert_eq!(merchant_view[0].subject_alias, relationship.subject_alias);
    assert_no_private_identity(
        &serde_json::to_value(&merchant_view).unwrap(),
        &consumer.id,
        &consumer_project.id,
    );

    let replacement = open_commerce_relationship_service::create_relationship(
        &store,
        &consumer_project.id,
        &consumer_actor,
        request(&merchant.id, 30),
    )
    .unwrap();
    assert_ne!(replacement.subject_alias, relationship.subject_alias);
    let own = open_commerce_relationship_service::list_consumer_relationships(
        &store,
        &consumer_project.id,
        &consumer_actor,
        10,
    )
    .unwrap();
    assert_eq!(own[0].status, "active");
    assert_eq!(own[1].status, "revoked");

    store
        .conn()
        .unwrap()
        .execute(
            "UPDATE open_commerce_consumer_relationships
                SET expires_at='2000-01-01T00:00:00Z' WHERE id=?1",
            rusqlite::params![replacement.id],
        )
        .unwrap();
    let expired = open_commerce_relationship_service::list_consumer_relationships(
        &store,
        &consumer_project.id,
        &consumer_actor,
        10,
    )
    .unwrap();
    assert_eq!(expired[0].status, "expired");

    let revoked = open_commerce_relationship_service::revoke_relationship(
        &store,
        &consumer_project.id,
        &replacement.id,
        &consumer_actor,
    )
    .unwrap();
    assert_eq!(revoked.status, "revoked");
    let revoked_again = open_commerce_relationship_service::revoke_relationship(
        &store,
        &consumer_project.id,
        &replacement.id,
        &consumer_actor,
    )
    .unwrap();
    assert_eq!(revoked_again.revoked_at, revoked.revoked_at);

    let stranger = store
        .create_user("relationship-stranger@example.com", "secret1", None, None)
        .unwrap();
    let stranger_actor = OpenCommerceActor {
        user_id: &stranger.id,
        app_id: "pc-web",
        project_role: Some("editor"),
    };
    assert!(
        open_commerce_relationship_service::list_consumer_relationships(
            &store,
            &consumer_project.id,
            &stranger_actor,
            10,
        )
        .unwrap()
        .is_empty()
    );
    assert!(open_commerce_relationship_service::revoke_relationship(
        &store,
        &consumer_project.id,
        &relationship.id,
        &stranger_actor,
    )
    .is_err());
}

#[tokio::test]
async fn relationship_mcp_closes_loop_and_rejects_invalid_requests() {
    let path = std::env::temp_dir().join(format!(
        "elon-open-commerce-relationship-validation-{}.sqlite",
        Uuid::new_v4().simple()
    ));
    let store = Store::open(&path).unwrap();
    let owner = store
        .create_user("relationship-validation@example.com", "secret1", None, None)
        .unwrap();
    let project = store
        .create_project(&owner.id, "Relationship validation", None, None)
        .unwrap()
        .project;
    let actor = OpenCommerceActor {
        user_id: &owner.id,
        app_id: "pc-web",
        project_role: Some("owner"),
    };
    assert!(store
        .create_open_commerce_developer_app(
            &project.id,
            &owner.id,
            CreateDeveloperAppRequest {
                app_id: "mcp-client".to_string(),
                display_name: "伪造系统客户端".to_string(),
            },
        )
        .unwrap_err()
        .to_string()
        .contains("系统保留"));
    let merchant = open_commerce_service::create_merchant(
        &store,
        &project.id,
        &actor,
        CreateMerchantRequest {
            display_name: "未发布商户".to_string(),
            slug: Some("unpublished-relationship".to_string()),
            description: String::new(),
            node_mode: "platform_hosted".to_string(),
            public_profile: json!({}),
        },
    )
    .unwrap();
    assert!(open_commerce_relationship_service::create_relationship(
        &store,
        &project.id,
        &actor,
        request(&merchant.id, 90),
    )
    .is_err());
    open_commerce_service::publish_capability(
        &store,
        &project.id,
        &merchant.id,
        &actor,
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
        &project.id,
        &merchant.id,
        &actor,
        true,
    )
    .unwrap();
    let mcp_created = crate::open_commerce_mcp::call_tool(
        &store,
        &project.id,
        &owner.id,
        "owner",
        "mcp-client",
        json!({
            "name":"open_commerce_create_consumer_relationship",
            "arguments":{
                "merchant_id":merchant.id.clone(),
                "scopes":[RELATIONSHIP_SCOPE_PREFERENCE_REMEMBER],
                "purpose":"MCP 消费者主动建立关系",
                "expires_at":(Utc::now() + Duration::days(30)).to_rfc3339()
            }
        }),
    )
    .await
    .unwrap();
    let relationship_id = mcp_created["structuredContent"]["id"]
        .as_str()
        .unwrap()
        .to_string();
    assert_eq!(
        mcp_created["structuredContent"]["source_app_id"],
        "mcp-client"
    );
    let mcp_list = crate::open_commerce_mcp::call_tool(
        &store,
        &project.id,
        &owner.id,
        "owner",
        "mcp-client",
        json!({
            "name":"open_commerce_list_consumer_relationships",
            "arguments":{}
        }),
    )
    .await
    .unwrap();
    assert_eq!(mcp_list["structuredContent"][0]["id"], relationship_id);

    assert!(open_commerce_relationship_service::create_relationship(
        &store,
        &project.id,
        &actor,
        request(&merchant.id, 400),
    )
    .unwrap_err()
    .to_string()
    .contains("366 天"));
    let mut impersonated = request(&merchant.id, 90);
    impersonated.source_app_id = "mcp-client".to_string();
    assert!(open_commerce_relationship_service::create_relationship(
        &store,
        &project.id,
        &actor,
        impersonated,
    )
    .unwrap_err()
    .to_string()
    .contains("不能冒充"));
    let mcp_revoked = crate::open_commerce_mcp::call_tool(
        &store,
        &project.id,
        &owner.id,
        "owner",
        "mcp-client",
        json!({
            "name":"open_commerce_revoke_consumer_relationship",
            "arguments":{"relationship_id":relationship_id}
        }),
    )
    .await
    .unwrap();
    assert_eq!(mcp_revoked["structuredContent"]["status"], "revoked");
}

fn request(merchant_id: &str, days: i64) -> CreateConsumerRelationshipRequest {
    CreateConsumerRelationshipRequest {
        merchant_id: merchant_id.to_string(),
        source_app_id: "pc-web".to_string(),
        scopes: vec![
            RELATIONSHIP_SCOPE_PREFERENCE_REMEMBER.to_string(),
            RELATIONSHIP_SCOPE_MEMBERSHIP_LINK.to_string(),
        ],
        purpose: "允许商户关联我主动提供的偏好和会员标识".to_string(),
        expires_at: (Utc::now() + Duration::days(days)).to_rfc3339(),
    }
}

fn assert_no_private_identity(value: &Value, user_id: &str, project_id: &str) {
    let text = serde_json::to_string(value).unwrap();
    assert!(!text.contains(user_id));
    assert!(!text.contains(project_id));
    assert!(!text.contains("consumer_user_id"));
    assert!(!text.contains("consumer_project_id"));
}
