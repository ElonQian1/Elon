use std::{collections::HashMap, path::Path, sync::Arc};

use axum::{
    body::{to_bytes, Body},
    http::{header, Method, Request, StatusCode},
    Router,
};
use chrono::{Duration, Utc};
use serde_json::{json, Value};
use tokio::sync::RwLock;
use tower::ServiceExt;
use uuid::Uuid;

use super::routes;
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
    types::{AgentsConfig, AiBackend, AiCliConfig, AppState},
};

struct Fixture {
    state: Arc<AppState>,
    router: Router,
    consumer_project_id: String,
    consumer_token: String,
    unrelated_consumer_project_id: String,
    unrelated_consumer_token: String,
    outsider_token: String,
    merchant_project_id: String,
    merchant_id: String,
    merchant_owner_id: String,
    merchant_token: String,
    merchant_reader_token: String,
    request_id: String,
}

impl Fixture {
    fn complete_request(&self) {
        let actor = OpenCommerceActor {
            user_id: &self.merchant_owner_id,
            app_id: "pc-web",
            project_role: Some("owner"),
        };
        open_commerce_data_request_service::decide_request(
            &self.state.store,
            &self.merchant_project_id,
            &self.merchant_id,
            &self.request_id,
            &actor,
            DecideConsumerDataRequest {
                action: "accept".to_string(),
                note: String::new(),
            },
        )
        .unwrap();
        open_commerce_data_request_service::decide_request(
            &self.state.store,
            &self.merchant_project_id,
            &self.merchant_id,
            &self.request_id,
            &actor,
            DecideConsumerDataRequest {
                action: "complete".to_string(),
                note: "商户已完成内部删除流程".to_string(),
            },
        )
        .unwrap();
    }

    fn create_path(&self) -> String {
        format!(
            "/api/projects/{}/open-commerce/merchants/{}/consumer-data-requests/{}/evidence",
            self.merchant_project_id, self.merchant_id, self.request_id
        )
    }
}

#[tokio::test]
async fn evidence_route_requires_auth_edit_role_confirmation_and_completion() {
    let fixture = fixture();
    let path = fixture.create_path();
    let payload = evidence_payload('a');

    let (status, _) = send_json(&fixture.router, Method::POST, &path, None, payload.clone()).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    let (status, _) = send_json(
        &fixture.router,
        Method::POST,
        &path,
        Some(&fixture.outsider_token),
        payload.clone(),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    let (status, body) = send_json(
        &fixture.router,
        Method::POST,
        &path,
        Some(&fixture.merchant_token),
        payload.clone(),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body["error"].as_str().unwrap().contains("已声明完成"));

    fixture.complete_request();

    let mut unconfirmed = payload.clone();
    unconfirmed["merchant_confirmed_unverified"] = Value::Bool(false);
    let (status, body) = send_json(
        &fixture.router,
        Method::POST,
        &path,
        Some(&fixture.merchant_token),
        unconfirmed,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body["error"].as_str().unwrap().contains("未经平台核验"));

    let (status, body) = send_json(
        &fixture.router,
        Method::POST,
        &path,
        Some(&fixture.merchant_reader_token),
        payload.clone(),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body["error"].as_str().unwrap().contains("编辑者"));

    let wrong_merchant_path = path.replace(&fixture.merchant_id, "merchant-missing");
    let (status, body) = send_json(
        &fixture.router,
        Method::POST,
        &wrong_merchant_path,
        Some(&fixture.merchant_token),
        payload,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body["error"].as_str().unwrap().contains("商户"));
}

#[tokio::test]
async fn evidence_route_is_idempotent_scoped_and_permanently_unverified() {
    let fixture = fixture();
    fixture.complete_request();
    let path = fixture.create_path();
    let payload = evidence_payload('a');

    let mut first_id = String::new();
    for _ in 0..2 {
        let (status, body) = send_json(
            &fixture.router,
            Method::POST,
            &path,
            Some(&fixture.merchant_token),
            payload.clone(),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(body["source_authority"], "merchant_supplied_unverified");
        assert_eq!(body["platform_verified"], false);
        assert!(!body.to_string().contains("submitted_by_user_id"));
        if first_id.is_empty() {
            first_id = body["id"].as_str().unwrap().to_string();
        } else {
            assert_eq!(body["id"], first_id);
        }
    }

    let (status, second) = send_json(
        &fixture.router,
        Method::POST,
        &path,
        Some(&fixture.merchant_token),
        evidence_payload('b'),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{second}");
    assert_ne!(second["id"], first_id);

    let (evidence_count, audit_count): (i64, i64) = {
        let conn = fixture.state.store.conn().unwrap();
        let evidence_count = conn
            .query_row(
                "SELECT COUNT(*) FROM open_commerce_data_erasure_evidence
                  WHERE data_request_id=?1",
                [&fixture.request_id],
                |row| row.get(0),
            )
            .unwrap();
        let audit_count = conn
            .query_row(
                "SELECT COUNT(*) FROM open_commerce_audit_events
                  WHERE project_id=?1 AND action='consumer_data_erasure.evidence_attached'",
                [&fixture.merchant_project_id],
                |row| row.get(0),
            )
            .unwrap();
        (evidence_count, audit_count)
    };
    assert_eq!(evidence_count, 2);
    assert_eq!(audit_count, 2);

    let (status, merchant_list) = send_json(
        &fixture.router,
        Method::GET,
        &format!(
            "/api/projects/{}/open-commerce/merchants/{}/consumer-data-erasure-evidence?limit=500",
            fixture.merchant_project_id, fixture.merchant_id
        ),
        Some(&fixture.merchant_reader_token),
        Value::Null,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{merchant_list}");
    assert_eq!(merchant_list["evidence"].as_array().unwrap().len(), 2);

    let (status, consumer_list) = send_json(
        &fixture.router,
        Method::GET,
        &format!(
            "/api/projects/{}/open-commerce/consumer-data-erasure-evidence?limit=500",
            fixture.consumer_project_id
        ),
        Some(&fixture.consumer_token),
        Value::Null,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{consumer_list}");
    assert_eq!(consumer_list["evidence"].as_array().unwrap().len(), 2);
    assert_eq!(consumer_list["boundary"].as_array().unwrap().len(), 3);
    let serialized = consumer_list.to_string();
    assert!(!serialized.contains("consumer_user_id"));
    assert!(!serialized.contains("consumer_project_id"));
    assert!(!serialized.contains("submitted_by_user_id"));

    let (status, unrelated_list) = send_json(
        &fixture.router,
        Method::GET,
        &format!(
            "/api/projects/{}/open-commerce/consumer-data-erasure-evidence?limit=500",
            fixture.unrelated_consumer_project_id
        ),
        Some(&fixture.unrelated_consumer_token),
        Value::Null,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{unrelated_list}");
    assert!(unrelated_list["evidence"].as_array().unwrap().is_empty());
}

fn evidence_payload(digest_character: char) -> Value {
    json!({
        "evidence_kind":"external_system_receipt",
        "external_system":"erp",
        "reference_id":format!("receipt-{digest_character}"),
        "receipt_sha256":digest_character.to_string().repeat(64),
        "summary":"商户持有的外部回执摘要",
        "merchant_confirmed_unverified":true
    })
}

async fn send_json(
    router: &Router,
    method: Method,
    path: &str,
    token: Option<&str>,
    body: Value,
) -> (StatusCode, Value) {
    let mut builder = Request::builder().method(method).uri(path);
    if let Some(token) = token {
        builder = builder.header(header::AUTHORIZATION, format!("Bearer {token}"));
    }
    let request = if body.is_null() {
        builder.body(Body::empty()).unwrap()
    } else {
        builder
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(serde_json::to_vec(&body).unwrap()))
            .unwrap()
    };
    let response = router.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap()
    };
    (status, body)
}

fn fixture() -> Fixture {
    let root = std::env::temp_dir().join(format!(
        "elon-open-commerce-erasure-evidence-api-{}",
        Uuid::new_v4().simple()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let store = Store::open(&root.join("state.sqlite")).unwrap();
    let merchant_owner = store
        .create_user("evidence-merchant@example.com", "secret1", None, None)
        .unwrap();
    let merchant_project = store
        .create_project(&merchant_owner.id, "Evidence merchant", None, None)
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
            display_name: "删除证明商户".to_string(),
            slug: Some(format!("erasure-evidence-{}", Uuid::new_v4().simple())),
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

    let merchant_reader = store
        .create_user("evidence-reader@example.com", "secret1", None, None)
        .unwrap();
    store
        .add_project_member_by_account(
            &merchant_project.id,
            "evidence-reader@example.com",
            "member",
        )
        .unwrap();
    let consumer = store
        .create_user("evidence-consumer@example.com", "secret1", None, None)
        .unwrap();
    let consumer_project = store
        .create_project(&consumer.id, "Evidence consumer", None, None)
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
        CreateConsumerRelationshipRequest {
            merchant_id: merchant.id.clone(),
            source_app_id: "pc-web".to_string(),
            scopes: vec![RELATIONSHIP_SCOPE_PREFERENCE_REMEMBER.to_string()],
            purpose: "HTTP 删除证明测试".to_string(),
            expires_at: (Utc::now() + Duration::days(90)).to_rfc3339(),
        },
    )
    .unwrap();
    let data_request = open_commerce_data_request_service::create_erasure_request(
        &store,
        &consumer_project.id,
        &consumer_actor,
        CreateConsumerDataErasureRequest {
            relationship_id: relationship.id,
        },
    )
    .unwrap();

    let unrelated_consumer = store
        .create_user("evidence-unrelated@example.com", "secret1", None, None)
        .unwrap();
    let unrelated_consumer_project = store
        .create_project(
            &unrelated_consumer.id,
            "Unrelated evidence consumer",
            None,
            None,
        )
        .unwrap()
        .project;
    let outsider = store
        .create_user("evidence-outsider@example.com", "secret1", None, None)
        .unwrap();
    let (consumer_token, _) = store
        .create_session(&consumer.id, Some("test"), None)
        .unwrap();
    let (unrelated_consumer_token, _) = store
        .create_session(&unrelated_consumer.id, Some("test"), None)
        .unwrap();
    let (merchant_token, _) = store
        .create_session(&merchant_owner.id, Some("test"), None)
        .unwrap();
    let (merchant_reader_token, _) = store
        .create_session(&merchant_reader.id, Some("test"), None)
        .unwrap();
    let (outsider_token, _) = store
        .create_session(&outsider.id, Some("test"), None)
        .unwrap();
    let state = Arc::new(test_state(store, &root));
    let router = routes().with_state(Arc::clone(&state));

    Fixture {
        state,
        router,
        consumer_project_id: consumer_project.id,
        consumer_token,
        unrelated_consumer_project_id: unrelated_consumer_project.id,
        unrelated_consumer_token,
        outsider_token,
        merchant_project_id: merchant_project.id,
        merchant_id: merchant.id,
        merchant_owner_id: merchant_owner.id,
        merchant_token,
        merchant_reader_token,
        request_id: data_request.id,
    }
}

fn test_state(store: Store, root: &Path) -> AppState {
    AppState {
        store,
        data_dir: root.to_path_buf(),
        default_backend: AiBackend::Api,
        ai_cli: AiCliConfig {
            enabled: false,
            options: Vec::new(),
            default_option: None,
            fallback_to_api: false,
            codex_cli_only: true,
            fallback_cli_option: None,
        },
        agents_config: RwLock::new(AgentsConfig {
            agents: HashMap::new(),
            default_agent: String::new(),
        }),
        project_root: root.to_path_buf(),
        workspace_root: root.to_string_lossy().into_owned(),
        public_url: "http://127.0.0.1".to_string(),
        http_client: reqwest::Client::new(),
        admin_token: "test".to_string(),
        require_login: true,
        min_apk_version_code: 0,
        config_path: root.join("agents.json"),
        image_model: None,
        peer_registry: Arc::new(RwLock::new(HashMap::new())),
        lan_peer_registry: Arc::new(RwLock::new(HashMap::new())),
        node_registry: Arc::new(crate::node_registry::NodeRegistry::new()),
        online_users: Arc::new(RwLock::new(HashMap::new())),
        agent_manager: Arc::new(crate::homecli_agent::AgentManager::new()),
        project_task_scheduler: Arc::new(crate::types::ProjectTaskScheduler::new()),
        codex_prewarm: Arc::new(crate::types::CodexPrewarmRegistry::new()),
        route_a_session_leases: Arc::new(crate::types::RouteASessionLeaseRegistry::new()),
        codex_network: Arc::new(crate::codex_health::CodexNetworkHealth::from_env()),
        server_traces: Arc::new(crate::server_trace::ServerTraceStore::new()),
        owner_token: None,
    }
}
