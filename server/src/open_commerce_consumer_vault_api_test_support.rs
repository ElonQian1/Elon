use std::{collections::HashMap, path::Path, sync::Arc};

use axum::{
    body::{to_bytes, Body},
    http::{header, Method, Request, StatusCode},
    Router,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use serde_json::{json, Value};
use tokio::sync::RwLock;
use tower::ServiceExt;
use uuid::Uuid;

use super::super::routes;
use crate::{
    store::Store,
    types::{AgentsConfig, AiBackend, AiCliConfig, AppState},
};

pub(super) struct Fixture {
    pub(super) state: Arc<AppState>,
    pub(super) router: Router,
    pub(super) project_id: String,
    pub(super) second_project_id: String,
    pub(super) owner_id: String,
    pub(super) owner_token: String,
    pub(super) member_token: String,
    pub(super) outsider_token: String,
}

pub(super) async fn create_item(
    fixture: &Fixture,
    project_id: &str,
    token: &str,
    id: &str,
    label: &str,
    ciphertext_byte: u8,
) -> (StatusCode, Value) {
    send_json(
        &fixture.router,
        Method::POST,
        &list_path(project_id),
        Some(token),
        json!({
            "id": id,
            "label": label,
            "item_kind": "private_note",
            "envelope": envelope(id, 1, ciphertext_byte),
        }),
    )
    .await
}

pub(super) async fn get_item(fixture: &Fixture, project_id: &str, token: &str, id: &str) -> Value {
    let (status, body) = send_json(
        &fixture.router,
        Method::GET,
        &item_path(project_id, id),
        Some(token),
        Value::Null,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    body
}

pub(super) fn envelope(id: &str, revision: i64, ciphertext_byte: u8) -> Value {
    json!({
        "schema": "open_commerce.consumer_data_vault_envelope.v1",
        "record_id": id,
        "revision": revision,
        "kdf": {
            "name": "PBKDF2",
            "hash": "SHA-256",
            "iterations": 310000,
            "salt_base64": BASE64.encode([7_u8; 16]),
        },
        "cipher": {
            "name": "AES-256-GCM",
            "nonce_base64": BASE64.encode([9_u8; 12]),
            "auth_tag_length_bits": 128,
        },
        "ciphertext_base64": BASE64.encode([ciphertext_byte; 17]),
        "created_at": "2026-08-10T10:30:00Z",
    })
}

pub(super) fn list_path(project_id: &str) -> String {
    format!("/api/projects/{project_id}/open-commerce/consumer-data-vault-items")
}

pub(super) fn item_path(project_id: &str, id: &str) -> String {
    format!("{}/{id}", list_path(project_id))
}

pub(super) async fn send_json(
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

pub(super) fn fixture() -> Fixture {
    let root = std::env::temp_dir().join(format!(
        "elon-open-commerce-consumer-vault-api-{}",
        Uuid::new_v4().simple()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let store = Store::open(&root.join("state.sqlite")).unwrap();
    let owner = store
        .create_user("vault-owner@example.com", "secret1", None, None)
        .unwrap();
    let member = store
        .create_user("vault-member@example.com", "secret1", None, None)
        .unwrap();
    let outsider = store
        .create_user("vault-outsider@example.com", "secret1", None, None)
        .unwrap();
    let project = store
        .create_project(&owner.id, "Vault API project", None, None)
        .unwrap()
        .project;
    let second_project = store
        .create_project(&owner.id, "Vault API second project", None, None)
        .unwrap()
        .project;
    store
        .add_project_member_by_account(&project.id, &member.id, "member")
        .unwrap();
    let (owner_token, _) = store.create_session(&owner.id, Some("test"), None).unwrap();
    let (member_token, _) = store
        .create_session(&member.id, Some("test"), None)
        .unwrap();
    let (outsider_token, _) = store
        .create_session(&outsider.id, Some("test"), None)
        .unwrap();
    let state = Arc::new(test_state(store, &root));
    let router = routes().with_state(Arc::clone(&state));
    Fixture {
        state,
        router,
        project_id: project.id,
        second_project_id: second_project.id,
        owner_id: owner.id,
        owner_token,
        member_token,
        outsider_token,
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
