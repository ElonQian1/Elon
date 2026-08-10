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
    open_commerce_directory_service,
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
    outsider_token: String,
    merchant_project_id: String,
    merchant_id: String,
    merchant_token: String,
    relationship_id: String,
}

impl Fixture {
    async fn create_request(&self) -> Value {
        let (status, body) = send_json(
            &self.router,
            Method::POST,
            &format!(
                "/api/projects/{}/open-commerce/consumer-data-requests",
                self.consumer_project_id
            ),
            Some(&self.consumer_token),
            json!({"relationship_id":self.relationship_id}),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        body
    }

    fn age_request(&self, request_id: &str, age: Duration) {
        let timestamp = (Utc::now() - age).to_rfc3339();
        self.state
            .store
            .conn()
            .unwrap()
            .execute(
                "UPDATE open_commerce_consumer_data_requests
                    SET requested_at=?1, updated_at=?1
                  WHERE id=?2",
                rusqlite::params![timestamp, request_id],
            )
            .unwrap();
    }

    fn follow_up_path(&self, request_id: &str) -> String {
        format!(
            "/api/projects/{}/open-commerce/consumer-data-requests/{request_id}/follow-up",
            self.consumer_project_id
        )
    }
}

#[tokio::test]
async fn follow_up_route_requires_auth_scope_and_elapsed_cooldown() {
    let fixture = fixture();
    let request = fixture.create_request().await;
    let request_id = request["id"].as_str().unwrap();
    let path = fixture.follow_up_path(request_id);
    let payload = json!({
        "action":"reminder",
        "idempotency_key":"http-auth-reminder",
        "note":""
    });

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
        Some(&fixture.consumer_token),
        payload,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body["error"].as_str().unwrap().contains("下一次允许催办"));
}

#[tokio::test]
async fn follow_up_route_replays_once_and_exposes_only_anonymous_merchant_state() {
    let fixture = fixture();
    let request = fixture.create_request().await;
    let request_id = request["id"].as_str().unwrap();
    fixture.age_request(request_id, Duration::hours(25));
    let path = fixture.follow_up_path(request_id);
    let payload = json!({
        "action":"reminder",
        "idempotency_key":"http-idempotent-reminder",
        "note":"请查看处理进度"
    });

    for _ in 0..2 {
        let (status, body) = send_json(
            &fixture.router,
            Method::POST,
            &path,
            Some(&fixture.consumer_token),
            payload.clone(),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(body["reminder_count"], 1);
        assert!(body.get("can_send_reminder").is_none());
    }

    let count: i64 = fixture
        .state
        .store
        .conn()
        .unwrap()
        .query_row(
            "SELECT COUNT(*) FROM open_commerce_data_request_followups
              WHERE data_request_id=?1 AND action_kind='reminder'",
            [request_id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(count, 1);

    let (status, body) = send_json(
        &fixture.router,
        Method::GET,
        &format!(
            "/api/projects/{}/open-commerce/merchants/{}/consumer-data-requests",
            fixture.merchant_project_id, fixture.merchant_id
        ),
        Some(&fixture.merchant_token),
        Value::Null,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["requests"].as_array().unwrap().len(), 1);
    assert_eq!(body["requests"][0]["reminder_count"], 1);
    let serialized = body.to_string();
    assert!(!serialized.contains("consumer_user_id"));
    assert!(!serialized.contains("consumer_project_id"));
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
        "elon-open-commerce-data-request-api-{}",
        Uuid::new_v4().simple()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let store = Store::open(&root.join("state.sqlite")).unwrap();
    let merchant_owner = store
        .create_user(
            "data-request-api-merchant@example.com",
            "secret1",
            None,
            None,
        )
        .unwrap();
    let merchant_project = store
        .create_project(&merchant_owner.id, "Data request API merchant", None, None)
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
            display_name: "删除请求 API 商户".to_string(),
            slug: Some(format!("data-request-api-{}", Uuid::new_v4().simple())),
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
            "data-request-api-consumer@example.com",
            "secret1",
            None,
            None,
        )
        .unwrap();
    let consumer_project = store
        .create_project(&consumer.id, "Data request API consumer", None, None)
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
            purpose: "HTTP 跟进测试".to_string(),
            expires_at: (Utc::now() + Duration::days(90)).to_rfc3339(),
        },
    )
    .unwrap();
    let outsider = store
        .create_user(
            "data-request-api-outsider@example.com",
            "secret1",
            None,
            None,
        )
        .unwrap();
    let (consumer_token, _) = store
        .create_session(&consumer.id, Some("test"), None)
        .unwrap();
    let (merchant_token, _) = store
        .create_session(&merchant_owner.id, Some("test"), None)
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
        outsider_token,
        merchant_project_id: merchant_project.id,
        merchant_id: merchant.id,
        merchant_token,
        relationship_id: relationship.id,
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
