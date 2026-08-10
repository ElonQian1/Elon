use std::{collections::HashMap, path::Path, sync::Arc};

use axum::{
    body::{to_bytes, Body},
    http::{header, Method, Request, StatusCode},
    Router,
};
use serde_json::Value;
use tokio::sync::RwLock;
use tower::ServiceExt;
use uuid::Uuid;

use crate::{
    open_commerce_consumer_model::ConsumerPreferences,
    open_commerce_consumer_preference_model::{
        ConsumerPreferenceProfile, UpsertConsumerPreferenceProfileRequest,
    },
    open_commerce_consumer_preference_service,
    open_commerce_portability_import_model::CreateConsumerPortabilityImportRequest,
    open_commerce_portability_import_service,
    open_commerce_portability_model::CreateConsumerPortabilityExportRequest,
    open_commerce_portability_service,
    open_commerce_service::OpenCommerceActor,
    store::Store,
    types::{AgentsConfig, AiBackend, AiCliConfig, AppState},
};

pub(super) struct Fixture {
    pub(super) state: Arc<AppState>,
    pub(super) router: Router,
    pub(super) target_project_id: String,
    pub(super) second_project_id: String,
    pub(super) owner_id: String,
    pub(super) member_id: String,
    pub(super) owner_token: String,
    pub(super) member_token: String,
    pub(super) outsider_token: String,
    pub(super) import_id: String,
    pub(super) current_preferences: ConsumerPreferences,
    pub(super) imported_preferences: ConsumerPreferences,
}

impl Fixture {
    pub(super) fn owner_actor(&self) -> OpenCommerceActor<'_> {
        OpenCommerceActor {
            user_id: &self.owner_id,
            app_id: "pc-web",
            project_role: Some("owner"),
        }
    }

    pub(super) fn member_actor(&self) -> OpenCommerceActor<'_> {
        OpenCommerceActor {
            user_id: &self.member_id,
            app_id: "pc-web",
            project_role: Some("member"),
        }
    }

    pub(super) fn owner_profile(&self) -> Option<ConsumerPreferenceProfile> {
        self.state
            .store
            .open_commerce_consumer_preference_profile(&self.target_project_id, &self.owner_id)
            .unwrap()
    }
}

pub(super) fn fixture(with_current_profile: bool) -> Fixture {
    let root = std::env::temp_dir().join(format!(
        "elon-open-commerce-portability-adoption-{}",
        Uuid::new_v4().simple()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let store = Store::open(&root.join("state.sqlite")).unwrap();
    let owner = store
        .create_user("adoption-owner@example.com", "secret1", None, None)
        .unwrap();
    let member = store
        .create_user("adoption-member@example.com", "secret1", None, None)
        .unwrap();
    let outsider = store
        .create_user("adoption-outsider@example.com", "secret1", None, None)
        .unwrap();
    let source_project = store
        .create_project(&owner.id, "Adoption source", None, None)
        .unwrap()
        .project;
    let target_project = store
        .create_project(&owner.id, "Adoption target", None, None)
        .unwrap()
        .project;
    let second_project = store
        .create_project(&owner.id, "Adoption second target", None, None)
        .unwrap()
        .project;
    store
        .add_project_member_by_account(&target_project.id, &member.id, "member")
        .unwrap();

    let imported_preferences = imported_preferences();
    let current_preferences = current_preferences();
    let source_actor = OpenCommerceActor {
        user_id: &owner.id,
        app_id: "pc-web",
        project_role: Some("owner"),
    };
    open_commerce_consumer_preference_service::upsert_profile(
        &store,
        &source_project.id,
        &source_actor,
        UpsertConsumerPreferenceProfileRequest {
            preferences: imported_preferences.clone(),
        },
    )
    .unwrap();
    let package = open_commerce_portability_service::create_export(
        &store,
        &source_project.id,
        &source_actor,
        CreateConsumerPortabilityExportRequest {
            idempotency_key: "selective-adoption-source-001".to_string(),
        },
    )
    .unwrap();
    let target_actor = OpenCommerceActor {
        user_id: &owner.id,
        app_id: "pc-web",
        project_role: Some("owner"),
    };
    let import_record = open_commerce_portability_import_service::create_import(
        &store,
        &target_project.id,
        &target_actor,
        CreateConsumerPortabilityImportRequest {
            source_operator: "fixture-operator".to_string(),
            package,
            signature: None,
        },
    )
    .unwrap();
    if with_current_profile {
        open_commerce_consumer_preference_service::upsert_profile(
            &store,
            &target_project.id,
            &target_actor,
            UpsertConsumerPreferenceProfileRequest {
                preferences: current_preferences.clone(),
            },
        )
        .unwrap();
    }

    let (owner_token, _) = store.create_session(&owner.id, Some("test"), None).unwrap();
    let (member_token, _) = store
        .create_session(&member.id, Some("test"), None)
        .unwrap();
    let (outsider_token, _) = store
        .create_session(&outsider.id, Some("test"), None)
        .unwrap();
    let state = Arc::new(test_state(store, &root));
    let router =
        crate::open_commerce_portability_adoption_api::routes().with_state(Arc::clone(&state));
    Fixture {
        state,
        router,
        target_project_id: target_project.id,
        second_project_id: second_project.id,
        owner_id: owner.id,
        member_id: member.id,
        owner_token,
        member_token,
        outsider_token,
        import_id: import_record.id,
        current_preferences,
        imported_preferences,
    }
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

fn current_preferences() -> ConsumerPreferences {
    ConsumerPreferences {
        categories: vec!["tea".to_string()],
        tags: vec!["nearby".to_string()],
        city: Some("Beijing".to_string()),
        max_unit_price_micros: Some(30_000_000),
        prefer_public: false,
    }
}

fn imported_preferences() -> ConsumerPreferences {
    ConsumerPreferences {
        categories: vec!["coffee".to_string(), "dessert".to_string()],
        tags: vec!["quiet".to_string(), "wifi".to_string()],
        city: Some("Shanghai".to_string()),
        max_unit_price_micros: Some(80_000_000),
        prefer_public: true,
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
