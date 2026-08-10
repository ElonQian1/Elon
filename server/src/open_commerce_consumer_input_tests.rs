use serde_json::{json, Value};
use uuid::Uuid;

use crate::{
    open_commerce_consumer_model::{
        ConsumerDiscoveryRequest, ConsumerDiscoveryResponse, ConsumerPreferences,
    },
    open_commerce_model::{CreateCapabilityRequest, CreateMerchantRequest, HANDLER_STATIC_JSON},
    open_commerce_service::{self, OpenCommerceActor},
    store::Store,
};

const CAPABILITY: &str = "menu.public";

struct Fixture {
    store: Store,
    owner_id: String,
    project_id: String,
    consumer_id: String,
}

#[derive(Debug, PartialEq, Eq)]
struct ReadSnapshot {
    merchants: Vec<(String, String, String)>,
    audit_count: i64,
}

fn fixture() -> Fixture {
    let path = std::env::temp_dir().join(format!(
        "elon_consumer_input_{}.db",
        Uuid::new_v4().simple()
    ));
    let store = Store::open(&path).expect("consumer input store should open");
    let owner = store
        .create_user(
            "consumer-input-owner@example.com",
            "secret1",
            Some("Consumer Input Owner"),
            None,
        )
        .unwrap();
    let project = store
        .create_project(&owner.id, "Consumer Input Merchant Project", None, None)
        .unwrap()
        .project;
    let consumer = store
        .create_user(
            "consumer-input-user@example.com",
            "secret1",
            Some("Consumer Input User"),
            None,
        )
        .unwrap();
    Fixture {
        store,
        owner_id: owner.id,
        project_id: project.id,
        consumer_id: consumer.id,
    }
}

impl Fixture {
    fn publish(&self, display_name: &str, slug: &str) -> String {
        let actor = OpenCommerceActor {
            user_id: &self.owner_id,
            app_id: "pc-web",
            project_role: Some("owner"),
        };
        let merchant = open_commerce_service::create_merchant(
            &self.store,
            &self.project_id,
            &actor,
            CreateMerchantRequest {
                display_name: display_name.to_string(),
                slug: Some(slug.to_string()),
                description: "消费者发现输入测试".to_string(),
                node_mode: "platform_hosted".to_string(),
                public_profile: json!({"category":"test"}),
            },
        )
        .unwrap();
        open_commerce_service::publish_capability(
            &self.store,
            &self.project_id,
            &merchant.id,
            &actor,
            CreateCapabilityRequest {
                capability_key: CAPABILITY.to_string(),
                display_name: "公开菜单".to_string(),
                description: String::new(),
                kind: "query".to_string(),
                access_level: "public".to_string(),
                input_schema: json!({}),
                output_schema: json!({}),
                handler_type: HANDLER_STATIC_JSON.to_string(),
                handler_config: Some(json!({"response":{"ok":true}})),
                unit_price_micros: 0,
                currency: "CNY".to_string(),
                freshness_seconds: 60,
            },
        )
        .unwrap();
        crate::open_commerce_directory_service::set_publication(
            &self.store,
            &self.project_id,
            &merchant.id,
            &actor,
            true,
        )
        .unwrap();
        merchant.id
    }

    fn discover(
        &self,
        request: ConsumerDiscoveryRequest,
    ) -> anyhow::Result<ConsumerDiscoveryResponse> {
        super::discover(&self.store, &self.consumer_id, request)
    }

    fn snapshot(&self) -> ReadSnapshot {
        let conn = self.store.conn().unwrap();
        let mut statement = conn
            .prepare("SELECT id, status, updated_at FROM open_commerce_merchants ORDER BY id")
            .unwrap();
        let merchants = statement
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
            .unwrap()
            .collect::<rusqlite::Result<Vec<_>>>()
            .unwrap();
        let audit_count = conn
            .query_row(
                "SELECT COUNT(*) FROM open_commerce_audit_events",
                [],
                |row| row.get(0),
            )
            .unwrap();
        ReadSnapshot {
            merchants,
            audit_count,
        }
    }
}

fn request(
    query: Option<String>,
    capability_key: Option<String>,
    limit: usize,
) -> ConsumerDiscoveryRequest {
    ConsumerDiscoveryRequest {
        query,
        capability_key,
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
        limit,
    }
}

fn names(response: &ConsumerDiscoveryResponse) -> Vec<String> {
    response
        .matches
        .iter()
        .map(|item| item.merchant.display_name.clone())
        .collect()
}

#[test]
fn sql_like_metacharacters_are_literal_and_reads_have_no_side_effects() {
    let fixture = fixture();
    fixture.publish("普通咖啡店", "normal-cafe");
    fixture.publish("百分号%店", "percent-cafe");
    fixture.publish("下划线_店", "underscore-cafe");
    fixture.publish("反斜杠\\店", "backslash-cafe");
    let before = fixture.snapshot();

    for (query, expected) in [
        ("%", "百分号%店"),
        ("_", "下划线_店"),
        ("\\", "反斜杠\\店"),
        ("  普通咖啡店  ", "普通咖啡店"),
    ] {
        let response = fixture
            .discover(request(Some(query.to_string()), None, 50))
            .unwrap();
        assert_eq!(names(&response), vec![expected.to_string()]);
    }
    assert_eq!(before, fixture.snapshot());
}

#[test]
fn query_capability_and_app_identifiers_share_one_normalization_boundary() {
    let fixture = fixture();
    fixture.publish("边界咖啡店", "boundary-cafe");

    let exact_limit = fixture
        .discover(request(Some("界".repeat(200)), None, 10))
        .unwrap();
    assert_eq!(exact_limit.requester_app_id, "pc-web");
    assert!(exact_limit.matches.is_empty());

    let too_long = fixture
        .discover(request(Some("界".repeat(201)), None, 10))
        .unwrap_err();
    assert!(too_long.to_string().contains("不能超过 200"));
    let control = fixture
        .discover(request(Some("咖啡\n店".to_string()), None, 10))
        .unwrap_err();
    assert!(control.to_string().contains("控制字符"));

    let normalized_capability = fixture
        .discover(request(None, Some("  MENU.PUBLIC  ".to_string()), 10))
        .unwrap();
    assert_eq!(normalized_capability.matches.len(), 1);
    let blank_capability = fixture
        .discover(request(None, Some("   ".to_string()), 10))
        .unwrap();
    assert_eq!(blank_capability.matches.len(), 1);
    let invalid_capability = fixture
        .discover(request(None, Some("bad key".to_string()), 10))
        .unwrap_err();
    assert!(invalid_capability.to_string().contains("不支持的字符"));

    let mut invalid_app = request(None, None, 10);
    invalid_app.requester_app_id = "bad app".to_string();
    assert!(fixture
        .discover(invalid_app)
        .unwrap_err()
        .to_string()
        .contains("不支持的字符"));
    let mut missing_app = request(None, None, 10);
    missing_app.requester_app_id = "missing.app".to_string();
    assert!(fixture
        .discover(missing_app)
        .unwrap_err()
        .to_string()
        .contains("开发者应用不存在"));
}

#[test]
fn result_limit_candidate_cap_and_receipt_fingerprint_use_validated_values() {
    let fixture = fixture();
    for index in 0..105 {
        fixture.publish(
            &format!("候选商户 {index:03}"),
            &format!("candidate-{index:03}"),
        );
    }
    for invalid in [0, 51, usize::MAX] {
        let error = fixture.discover(request(None, None, invalid)).unwrap_err();
        assert!(error.to_string().contains("必须在 1 到 50 之间"));
    }

    let mut one_request = request(None, None, 1);
    one_request.include_ranking_receipt = true;
    let one = fixture.discover(one_request).unwrap();
    assert_eq!(one.matches.len(), 1);
    assert_eq!(one.candidate_scope.candidate_cap, 100);
    assert_eq!(one.candidate_scope.directory_candidate_count, 100);
    assert_eq!(one.candidate_scope.eligible_match_count, 100);
    assert!(one.candidate_scope.results_truncated);

    let mut fifty_request = request(None, None, 50);
    fifty_request.include_ranking_receipt = true;
    let fifty = fixture.discover(fifty_request).unwrap();
    assert_eq!(fifty.matches.len(), 50);
    assert_eq!(fifty.candidate_scope.directory_candidate_count, 100);
    assert!(fifty.candidate_scope.results_truncated);

    let one_payload: Value =
        serde_json::from_str(&one.ranking_receipt.as_ref().unwrap().canonical_payload_json)
            .unwrap();
    let fifty_payload: Value = serde_json::from_str(
        &fifty
            .ranking_receipt
            .as_ref()
            .unwrap()
            .canonical_payload_json,
    )
    .unwrap();
    assert_eq!(one_payload["returned_match_count"], 1);
    assert_eq!(fifty_payload["returned_match_count"], 50);
    assert_ne!(
        one_payload["request_fingerprint_sha256"],
        fifty_payload["request_fingerprint_sha256"]
    );
    assert_eq!(one_payload["ordered_results"].as_array().unwrap().len(), 1);
    assert_eq!(
        fifty_payload["ordered_results"].as_array().unwrap().len(),
        50
    );
}
