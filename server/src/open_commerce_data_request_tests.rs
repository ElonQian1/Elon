use chrono::{Duration, Utc};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::{
    open_commerce_data_request_model::{
        CreateConsumerDataErasureRequest, DecideConsumerDataRequest,
    },
    open_commerce_data_request_service, open_commerce_directory_service,
    open_commerce_model::{
        CreateCapabilityRequest, CreateMerchantRequest, ACCESS_PUBLIC, HANDLER_STATIC_JSON,
    },
    open_commerce_relationship_model::{
        CreateConsumerRelationshipRequest, RELATIONSHIP_SCOPE_PREFERENCE_REMEMBER,
    },
    open_commerce_relationship_service,
    open_commerce_service::{self, OpenCommerceActor},
    store::Store,
};

#[test]
fn erasure_request_revokes_relationship_and_enforces_merchant_state_machine() {
    let fixture = fixture();
    let consumer_actor = fixture.consumer_actor("owner");
    let request = open_commerce_data_request_service::create_erasure_request(
        &fixture.store,
        &fixture.consumer_project_id,
        &consumer_actor,
        CreateConsumerDataErasureRequest {
            relationship_id: fixture.relationship_id.clone(),
        },
    )
    .unwrap();
    assert_eq!(request.status, "requested");
    assert_eq!(request.request_type, "erase_linked_data");
    assert_no_private_identity(
        &serde_json::to_value(&request).unwrap(),
        &fixture.consumer_user_id,
        &fixture.consumer_project_id,
    );
    let relationships = open_commerce_relationship_service::list_consumer_relationships(
        &fixture.store,
        &fixture.consumer_project_id,
        &consumer_actor,
        10,
    )
    .unwrap();
    assert_eq!(relationships[0].status, "revoked");

    let duplicate = open_commerce_data_request_service::create_erasure_request(
        &fixture.store,
        &fixture.consumer_project_id,
        &consumer_actor,
        CreateConsumerDataErasureRequest {
            relationship_id: fixture.relationship_id.clone(),
        },
    )
    .unwrap();
    assert_eq!(duplicate.id, request.id);

    let merchant_actor = fixture.merchant_actor("owner");
    let merchant_view = open_commerce_data_request_service::list_merchant_requests(
        &fixture.store,
        &fixture.merchant_project_id,
        &fixture.merchant_id,
        &merchant_actor,
        10,
    )
    .unwrap();
    assert_eq!(merchant_view[0].subject_alias, request.subject_alias);
    assert_no_private_identity(
        &serde_json::to_value(&merchant_view).unwrap(),
        &fixture.consumer_user_id,
        &fixture.consumer_project_id,
    );

    assert!(open_commerce_data_request_service::decide_request(
        &fixture.store,
        &fixture.merchant_project_id,
        &fixture.merchant_id,
        &request.id,
        &fixture.merchant_actor("viewer"),
        decision("accept", ""),
    )
    .unwrap_err()
    .to_string()
    .contains("编辑者"));
    let accepted = open_commerce_data_request_service::decide_request(
        &fixture.store,
        &fixture.merchant_project_id,
        &fixture.merchant_id,
        &request.id,
        &merchant_actor,
        decision("accept", "开始核对关联数据"),
    )
    .unwrap();
    assert_eq!(accepted.status, "in_progress");
    let accepted_again = open_commerce_data_request_service::decide_request(
        &fixture.store,
        &fixture.merchant_project_id,
        &fixture.merchant_id,
        &request.id,
        &merchant_actor,
        decision("accept", "重复接单"),
    )
    .unwrap();
    assert_eq!(accepted_again.accepted_at, accepted.accepted_at);
    assert!(open_commerce_data_request_service::withdraw_request(
        &fixture.store,
        &fixture.consumer_project_id,
        &request.id,
        &consumer_actor,
    )
    .unwrap_err()
    .to_string()
    .contains("不能撤回"));
    assert!(open_commerce_data_request_service::decide_request(
        &fixture.store,
        &fixture.merchant_project_id,
        &fixture.merchant_id,
        &request.id,
        &merchant_actor,
        decision("complete", ""),
    )
    .unwrap_err()
    .to_string()
    .contains("必须填写说明"));
    let completed = open_commerce_data_request_service::decide_request(
        &fixture.store,
        &fixture.merchant_project_id,
        &fixture.merchant_id,
        &request.id,
        &merchant_actor,
        decision("complete", "商户声明已完成外部系统关联数据清理"),
    )
    .unwrap();
    assert_eq!(completed.status, "completed");
    assert_eq!(
        completed.resolution_kind.as_deref(),
        Some("merchant_attested_completed")
    );
    let completed_again = open_commerce_data_request_service::decide_request(
        &fixture.store,
        &fixture.merchant_project_id,
        &fixture.merchant_id,
        &request.id,
        &merchant_actor,
        decision("complete", "重复完成"),
    )
    .unwrap();
    assert_eq!(completed_again.resolved_at, completed.resolved_at);
    assert!(open_commerce_data_request_service::decide_request(
        &fixture.store,
        &fixture.merchant_project_id,
        &fixture.merchant_id,
        &request.id,
        &merchant_actor,
        decision("reject", "不能处理"),
    )
    .is_err());
}

#[tokio::test]
async fn erasure_request_mcp_withdrawal_is_owner_scoped_and_fail_closed() {
    let fixture = fixture();
    let created = crate::open_commerce_mcp::call_tool(
        &fixture.store,
        &fixture.consumer_project_id,
        &fixture.consumer_user_id,
        "owner",
        "mcp-client",
        json!({
            "name":"open_commerce_create_consumer_data_erasure_request",
            "arguments":{"relationship_id":fixture.relationship_id.clone()}
        }),
    )
    .await
    .unwrap();
    let request_id = created["structuredContent"]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let own = crate::open_commerce_mcp::call_tool(
        &fixture.store,
        &fixture.consumer_project_id,
        &fixture.consumer_user_id,
        "owner",
        "mcp-client",
        json!({"name":"open_commerce_list_consumer_data_requests","arguments":{}}),
    )
    .await
    .unwrap();
    assert_eq!(own["structuredContent"][0]["id"], request_id);

    let stranger = fixture
        .store
        .create_user("data-request-stranger@example.com", "secret1", None, None)
        .unwrap();
    let stranger_actor = OpenCommerceActor {
        user_id: &stranger.id,
        app_id: "pc-web",
        project_role: Some("editor"),
    };
    assert!(open_commerce_data_request_service::list_consumer_requests(
        &fixture.store,
        &fixture.consumer_project_id,
        &stranger_actor,
        10,
    )
    .unwrap()
    .is_empty());
    assert!(open_commerce_data_request_service::withdraw_request(
        &fixture.store,
        &fixture.consumer_project_id,
        &request_id,
        &stranger_actor,
    )
    .is_err());

    let withdrawn = crate::open_commerce_mcp::call_tool(
        &fixture.store,
        &fixture.consumer_project_id,
        &fixture.consumer_user_id,
        "owner",
        "mcp-client",
        json!({
            "name":"open_commerce_withdraw_consumer_data_request",
            "arguments":{"request_id":request_id.clone()}
        }),
    )
    .await
    .unwrap();
    assert_eq!(withdrawn["structuredContent"]["status"], "withdrawn");
    let withdrawn_again = open_commerce_data_request_service::withdraw_request(
        &fixture.store,
        &fixture.consumer_project_id,
        &request_id,
        &fixture.consumer_actor("owner"),
    )
    .unwrap();
    assert_eq!(
        withdrawn_again.withdrawn_at.as_deref(),
        withdrawn["structuredContent"]["withdrawn_at"].as_str()
    );
    assert!(open_commerce_data_request_service::decide_request(
        &fixture.store,
        &fixture.merchant_project_id,
        &fixture.merchant_id,
        &request_id,
        &fixture.merchant_actor("owner"),
        decision("accept", ""),
    )
    .is_err());
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
    fn merchant_actor<'a>(&'a self, role: &'a str) -> OpenCommerceActor<'a> {
        OpenCommerceActor {
            user_id: &self.merchant_user_id,
            app_id: "pc-web",
            project_role: Some(role),
        }
    }

    fn consumer_actor<'a>(&'a self, role: &'a str) -> OpenCommerceActor<'a> {
        OpenCommerceActor {
            user_id: &self.consumer_user_id,
            app_id: "pc-web",
            project_role: Some(role),
        }
    }
}

fn fixture() -> Fixture {
    let path = std::env::temp_dir().join(format!(
        "elon-open-commerce-data-request-{}.sqlite",
        Uuid::new_v4().simple()
    ));
    let store = Store::open(&path).unwrap();
    let merchant_owner = store
        .create_user("data-request-merchant@example.com", "secret1", None, None)
        .unwrap();
    let merchant_project = store
        .create_project(&merchant_owner.id, "Data request merchant", None, None)
        .unwrap()
        .project;
    let actor = OpenCommerceActor {
        user_id: &merchant_owner.id,
        app_id: "pc-web",
        project_role: Some("owner"),
    };
    let merchant = open_commerce_service::create_merchant(
        &store,
        &merchant_project.id,
        &actor,
        CreateMerchantRequest {
            display_name: "删除请求测试商户".to_string(),
            slug: Some("data-request-merchant".to_string()),
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
        &merchant_project.id,
        &merchant.id,
        &actor,
        true,
    )
    .unwrap();
    let consumer = store
        .create_user("data-request-consumer@example.com", "secret1", None, None)
        .unwrap();
    let consumer_project = store
        .create_project(&consumer.id, "Data request wallet", None, None)
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
        CreateConsumerRelationshipRequest {
            merchant_id: merchant.id.clone(),
            source_app_id: "pc-web".to_string(),
            scopes: vec![RELATIONSHIP_SCOPE_PREFERENCE_REMEMBER.to_string()],
            purpose: "测试消费者关系".to_string(),
            expires_at: (Utc::now() + Duration::days(90)).to_rfc3339(),
        },
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

fn decision(action: &str, note: &str) -> DecideConsumerDataRequest {
    DecideConsumerDataRequest {
        action: action.to_string(),
        note: note.to_string(),
    }
}

fn assert_no_private_identity(value: &Value, user_id: &str, project_id: &str) {
    let serialized = value.to_string();
    assert!(!serialized.contains("consumer_user_id"));
    assert!(!serialized.contains("consumer_project_id"));
    assert!(!serialized.contains(user_id));
    assert!(!serialized.contains(project_id));
}
