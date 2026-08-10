use std::{collections::HashMap, path::Path, sync::Arc};

use axum::{
    body::{to_bytes, Body},
    http::{header, Method, Request, StatusCode},
    Router,
};
use serde_json::{json, Value};
use tokio::sync::RwLock;
use tower::ServiceExt;

use super::test_support::fixture;
use crate::{
    open_commerce_action_confirmation_api::routes,
    open_commerce_developer_model::CreateDeveloperAppRequest,
    types::{AgentsConfig, AiBackend, AiCliConfig, AppState},
};

struct RouteFixture {
    state: Arc<AppState>,
    router: Router,
    merchant_id: String,
    project_id: String,
    owner_token: String,
    outsider_token: String,
    developer_token: String,
    other_developer_token: String,
}

#[tokio::test]
async fn authenticated_cancel_route_is_actor_bound_idempotent_and_explicit() {
    let fixture = route_fixture();
    let (status, prepared) = send_json(
        &fixture.router,
        Method::POST,
        "/api/open-commerce/action-confirmations",
        Some(&fixture.owner_token),
        Some("pc-web"),
        action_request(&fixture, "http-consumer-cancel", "route-secret"),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{prepared}");
    assert!(!prepared.to_string().contains("route-secret"));
    let confirmation_id = prepared["id"].as_str().unwrap();
    let path = format!("/api/open-commerce/action-confirmations/{confirmation_id}/cancel");

    let (status, _) = send_json(
        &fixture.router,
        Method::POST,
        &path,
        None,
        None,
        cancellation_request("CANCEL_ACTION"),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    let (status, _) = send_json(
        &fixture.router,
        Method::POST,
        &path,
        Some(&fixture.outsider_token),
        Some("pc-web"),
        cancellation_request("CANCEL_ACTION"),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    let (status, _) = send_json(
        &fixture.router,
        Method::POST,
        &path,
        Some(&fixture.owner_token),
        Some("consumer.route.other"),
        cancellation_request("CANCEL_ACTION"),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    let (status, body) = send_json(
        &fixture.router,
        Method::POST,
        &path,
        Some(&fixture.owner_token),
        Some("pc-web"),
        cancellation_request("cancel"),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body["error"].as_str().unwrap().contains("短语无效"));

    for _ in 0..2 {
        let (status, canceled) = send_json(
            &fixture.router,
            Method::POST,
            &path,
            Some(&fixture.owner_token),
            Some("pc-web"),
            cancellation_request("CANCEL_ACTION"),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{canceled}");
        assert_cancellation_projection(&canceled, confirmation_id, "pc-web");
    }

    assert_persisted_cancellation(&fixture, confirmation_id);
}

#[tokio::test]
async fn developer_cancel_route_requires_the_bound_app_credential() {
    let fixture = route_fixture();
    let (status, prepared) = send_json(
        &fixture.router,
        Method::POST,
        "/api/open-commerce/developer/action-confirmations",
        Some(&fixture.developer_token),
        None,
        json!({
            "merchant_id":fixture.merchant_id,
            "capability_key":"order.commit",
            "idempotency_key":"http-developer-cancel",
            "input":{"private_note":"developer-route-secret"}
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{prepared}");
    assert!(!prepared.to_string().contains("developer-route-secret"));
    let confirmation_id = prepared["id"].as_str().unwrap();
    let path =
        format!("/api/open-commerce/developer/action-confirmations/{confirmation_id}/cancel");

    let (status, _) = send_json(
        &fixture.router,
        Method::POST,
        &path,
        None,
        None,
        cancellation_request("CANCEL_ACTION"),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    let (status, _) = send_json(
        &fixture.router,
        Method::POST,
        &path,
        Some(&fixture.other_developer_token),
        None,
        cancellation_request("CANCEL_ACTION"),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    let (status, canceled) = send_json(
        &fixture.router,
        Method::POST,
        &path,
        Some(&fixture.developer_token),
        None,
        cancellation_request("CANCEL_ACTION"),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{canceled}");
    assert_cancellation_projection(&canceled, confirmation_id, "consumer.route.app");

    assert_persisted_cancellation(&fixture, confirmation_id);
}

fn assert_cancellation_projection(body: &Value, confirmation_id: &str, app_id: &str) {
    assert_eq!(
        body["schema"],
        "open_commerce.consumer_action_confirmation_cancellation.v1"
    );
    assert_eq!(body["confirmation_id"], confirmation_id);
    assert_eq!(body["requester_app_id"], app_id);
    assert_eq!(body["status"], "canceled");
    assert!(body["canceled_at"].is_string());
    assert_eq!(body["invocation_created"], false);
    assert_eq!(body["next_step"], "stop");
    let serialized = body.to_string();
    for forbidden in [
        "stored_status",
        "request_hash",
        "request_shape",
        "requester_user_id",
        "project_id",
        "invocation_id",
        "private_note",
    ] {
        assert!(!serialized.contains(forbidden), "leaked {forbidden}");
    }
}

fn assert_persisted_cancellation(fixture: &RouteFixture, confirmation_id: &str) {
    let stored = fixture
        .state
        .store
        .open_commerce_action_confirmation(confirmation_id)
        .unwrap();
    assert_eq!(stored.status, "expired");
    assert!(stored.canceled_at.is_some());
    assert!(stored.invocation_id.is_none());
    assert!(fixture
        .state
        .store
        .list_project_open_commerce_invocations(&fixture.project_id, 20)
        .unwrap()
        .is_empty());
    let cancellation_audits = fixture
        .state
        .store
        .list_project_open_commerce_audit(&fixture.project_id, 200)
        .unwrap()
        .into_iter()
        .filter(|event| {
            event.action == "action_confirmation.canceled" && event.subject_id == confirmation_id
        })
        .count();
    assert_eq!(cancellation_audits, 1);
}

fn action_request(fixture: &RouteFixture, idempotency_key: &str, secret: &str) -> Value {
    json!({
        "merchant_id":fixture.merchant_id,
        "capability_key":"order.commit",
        "requester_app_id":"pc-web",
        "idempotency_key":idempotency_key,
        "input":{"private_note":secret}
    })
}

fn cancellation_request(phrase: &str) -> Value {
    json!({"confirmation_phrase":phrase})
}

async fn send_json(
    router: &Router,
    method: Method,
    path: &str,
    bearer: Option<&str>,
    app_id: Option<&str>,
    body: Value,
) -> (StatusCode, Value) {
    let mut builder = Request::builder()
        .method(method)
        .uri(path)
        .header(header::CONTENT_TYPE, "application/json");
    if let Some(token) = bearer {
        builder = builder.header(header::AUTHORIZATION, format!("Bearer {token}"));
    }
    if let Some(app_id) = app_id {
        builder = builder.header("x-elon-app-id", app_id);
    }
    let request = builder
        .body(Body::from(serde_json::to_vec(&body).unwrap()))
        .unwrap();
    let response = router.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, body)
}

fn route_fixture() -> RouteFixture {
    let fixture = fixture();
    let outsider = fixture
        .store
        .create_user("action-route-outsider@example.com", "secret1", None, None)
        .unwrap();
    let (owner_token, _) = fixture
        .store
        .create_session(&fixture.owner_id, Some("action-route-owner"), None)
        .unwrap();
    let (outsider_token, _) = fixture
        .store
        .create_session(&outsider.id, Some("action-route-outsider"), None)
        .unwrap();
    let developer = fixture
        .store
        .create_open_commerce_developer_app(
            &fixture.project_id,
            &fixture.owner_id,
            CreateDeveloperAppRequest {
                app_id: "consumer.route.app".to_string(),
                display_name: "Route consumer".to_string(),
            },
        )
        .unwrap();
    let other_developer = fixture
        .store
        .create_open_commerce_developer_app(
            &fixture.project_id,
            &fixture.owner_id,
            CreateDeveloperAppRequest {
                app_id: "consumer.route.other".to_string(),
                display_name: "Other route consumer".to_string(),
            },
        )
        .unwrap();
    let root = fixture
        .path
        .parent()
        .expect("test database should have a parent")
        .to_path_buf();
    let state = Arc::new(test_state(fixture.store, &root));
    let router = routes().with_state(Arc::clone(&state));
    RouteFixture {
        state,
        router,
        merchant_id: fixture.merchant_id,
        project_id: fixture.project_id,
        owner_token,
        outsider_token,
        developer_token: developer.test_token,
        other_developer_token: other_developer.test_token,
    }
}

fn test_state(store: crate::store::Store, root: &Path) -> AppState {
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
