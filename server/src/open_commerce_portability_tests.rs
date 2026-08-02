use chrono::{Duration, Utc};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::{
    open_commerce_data_request_model::CreateConsumerDataErasureRequest,
    open_commerce_data_request_service, open_commerce_directory_service,
    open_commerce_model::{
        CreateCapabilityRequest, CreateMerchantRequest, ACCESS_PUBLIC, HANDLER_STATIC_JSON,
    },
    open_commerce_portability_model::CreateConsumerPortabilityExportRequest,
    open_commerce_portability_service,
    open_commerce_relationship_model::{
        CreateConsumerRelationshipRequest, RenewConsumerRelationshipRequest,
        RELATIONSHIP_SCOPE_PREFERENCE_REMEMBER,
    },
    open_commerce_relationship_service,
    open_commerce_service::{self, OpenCommerceActor},
    store::Store,
};

#[test]
fn portability_export_is_immutable_owner_scoped_and_detects_tampering() {
    let fixture = fixture();
    let actor = fixture.consumer_actor("owner");
    let renewed = open_commerce_relationship_service::renew_relationship(
        &fixture.store,
        &fixture.consumer_project_id,
        &fixture.relationship_id,
        &actor,
        RenewConsumerRelationshipRequest {
            source_app_id: "pc-web".to_string(),
            expires_at: (Utc::now() + Duration::days(180)).to_rfc3339(),
        },
    )
    .unwrap();
    open_commerce_data_request_service::create_erasure_request(
        &fixture.store,
        &fixture.consumer_project_id,
        &actor,
        CreateConsumerDataErasureRequest {
            relationship_id: renewed.id.clone(),
        },
    )
    .unwrap();

    let first = open_commerce_portability_service::create_export(
        &fixture.store,
        &fixture.consumer_project_id,
        &actor,
        export_request("portable-first-snapshot"),
    )
    .unwrap();
    assert_eq!(first.payload.relationships.len(), 2);
    assert_eq!(first.payload.relationship_renewals.len(), 1);
    assert_eq!(first.payload.data_requests.len(), 1);
    assert_eq!(
        first.payload.relationship_renewals[0].source_relationship_id,
        fixture.relationship_id
    );
    assert_eq!(
        first.payload.relationship_renewals[0].renewed_relationship_id,
        renewed.id
    );
    assert_eq!(first.payload_sha256.len(), 64);
    assert_private_consumer_fields_absent(
        &serde_json::to_value(&first).unwrap(),
        &fixture.consumer_user_id,
    );

    open_commerce_relationship_service::create_relationship(
        &fixture.store,
        &fixture.consumer_project_id,
        &actor,
        relationship_request(&fixture.merchant_id),
    )
    .unwrap();
    let retry = open_commerce_portability_service::create_export(
        &fixture.store,
        &fixture.consumer_project_id,
        &actor,
        export_request("portable-first-snapshot"),
    )
    .unwrap();
    assert_eq!(retry.id, first.id);
    assert_eq!(retry.payload_sha256, first.payload_sha256);
    assert_eq!(retry.payload.relationships.len(), 2);

    let updated = open_commerce_portability_service::create_export(
        &fixture.store,
        &fixture.consumer_project_id,
        &actor,
        export_request("portable-second-snapshot"),
    )
    .unwrap();
    assert_ne!(updated.id, first.id);
    assert_eq!(updated.payload.relationships.len(), 3);

    let stranger = fixture
        .store
        .create_user("portability-stranger@example.com", "secret1", None, None)
        .unwrap();
    let stranger_actor = OpenCommerceActor {
        user_id: &stranger.id,
        app_id: "pc-web",
        project_role: Some("editor"),
    };
    assert!(open_commerce_portability_service::list_exports(
        &fixture.store,
        &fixture.consumer_project_id,
        &stranger_actor,
        100,
    )
    .unwrap()
    .is_empty());
    assert!(open_commerce_portability_service::get_export(
        &fixture.store,
        &fixture.consumer_project_id,
        &first.id,
        &stranger_actor,
    )
    .is_err());

    fixture
        .store
        .conn()
        .unwrap()
        .execute(
            "UPDATE open_commerce_consumer_portability_exports
                SET payload_sha256=?1 WHERE id=?2",
            rusqlite::params!["0".repeat(64), first.id],
        )
        .unwrap();
    assert!(open_commerce_portability_service::get_export(
        &fixture.store,
        &fixture.consumer_project_id,
        &retry.id,
        &actor,
    )
    .unwrap_err()
    .to_string()
    .contains("完整性校验失败"));
}

#[tokio::test]
async fn portability_mcp_creates_lists_and_reads_the_same_verified_snapshot() {
    let fixture = fixture();
    let created = crate::open_commerce_mcp::call_tool(
        &fixture.store,
        &fixture.consumer_project_id,
        &fixture.consumer_user_id,
        "owner",
        "mcp-client",
        json!({
            "name":"open_commerce_create_consumer_portability_export",
            "arguments":{"idempotency_key":"mcp-portable-snapshot-001"}
        }),
    )
    .await
    .unwrap();
    let export_id = created["structuredContent"]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let listed = crate::open_commerce_mcp::call_tool(
        &fixture.store,
        &fixture.consumer_project_id,
        &fixture.consumer_user_id,
        "owner",
        "mcp-client",
        json!({
            "name":"open_commerce_list_consumer_portability_exports",
            "arguments":{}
        }),
    )
    .await
    .unwrap();
    assert_eq!(listed["structuredContent"][0]["id"], export_id);
    let read = crate::open_commerce_mcp::call_tool(
        &fixture.store,
        &fixture.consumer_project_id,
        &fixture.consumer_user_id,
        "owner",
        "mcp-client",
        json!({
            "name":"open_commerce_get_consumer_portability_export",
            "arguments":{"export_id":export_id}
        }),
    )
    .await
    .unwrap();
    assert_eq!(
        read["structuredContent"]["payload_sha256"],
        created["structuredContent"]["payload_sha256"]
    );
    assert_private_consumer_fields_absent(&read["structuredContent"], &fixture.consumer_user_id);
}

struct Fixture {
    store: Store,
    merchant_id: String,
    consumer_user_id: String,
    consumer_project_id: String,
    relationship_id: String,
}

impl Fixture {
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
        "elon-open-commerce-portability-{}.sqlite",
        Uuid::new_v4().simple()
    ));
    let store = Store::open(&path).unwrap();
    let merchant_owner = store
        .create_user("portability-merchant@example.com", "secret1", None, None)
        .unwrap();
    let merchant_project = store
        .create_project(&merchant_owner.id, "Portability merchant", None, None)
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
            display_name: "可携带数据测试商户".to_string(),
            slug: Some(format!("portability-{}", Uuid::new_v4().simple())),
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
        .create_user("portability-consumer@example.com", "secret1", None, None)
        .unwrap();
    let consumer_project = store
        .create_project(&consumer.id, "Portability wallet", None, None)
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
        relationship_request(&merchant.id),
    )
    .unwrap();
    Fixture {
        store,
        merchant_id: merchant.id,
        consumer_user_id: consumer.id,
        consumer_project_id: consumer_project.id,
        relationship_id: relationship.id,
    }
}

fn relationship_request(merchant_id: &str) -> CreateConsumerRelationshipRequest {
    CreateConsumerRelationshipRequest {
        merchant_id: merchant_id.to_string(),
        source_app_id: "pc-web".to_string(),
        scopes: vec![RELATIONSHIP_SCOPE_PREFERENCE_REMEMBER.to_string()],
        purpose: "测试消费者可携带数据".to_string(),
        expires_at: (Utc::now() + Duration::days(90)).to_rfc3339(),
    }
}

fn export_request(idempotency_key: &str) -> CreateConsumerPortabilityExportRequest {
    CreateConsumerPortabilityExportRequest {
        idempotency_key: idempotency_key.to_string(),
    }
}

fn assert_private_consumer_fields_absent(value: &Value, user_id: &str) {
    let serialized = value.to_string();
    assert!(!serialized.contains(user_id));
    assert!(!serialized.contains("consumer_user_id"));
    assert!(!serialized.contains("consumer_project_id"));
    assert!(!serialized.contains("renewed_from_relationship_id"));
}
