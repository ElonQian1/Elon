use std::{collections::HashMap, path::Path, sync::Arc};

use chrono::{Duration, Utc};
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::{
    open_commerce_developer_admission_model::{
        ReviewDeveloperAppAdmissionRequest, SubmitDeveloperAppAdmissionRequest,
    },
    open_commerce_developer_admission_service,
    open_commerce_developer_credential_model::DeveloperProductionCredentialSecret,
    open_commerce_developer_manifest_service,
    open_commerce_developer_model::{
        CreateDeveloperAppRequest, OpenCommerceDeveloperApp, ReviewDeveloperAppManifestRequest,
        UpdateDeveloperAppManifestRequest,
    },
    open_commerce_service::OpenCommerceActor,
    store::Store,
    types::{AgentsConfig, AiBackend, AiCliConfig, AppState},
};

pub(crate) struct ApprovedDeveloperFixture {
    pub(crate) store: Store,
    pub(crate) project_id: String,
    pub(crate) owner_user_id: String,
    pub(crate) app: OpenCommerceDeveloperApp,
    pub(crate) admission_id: String,
}

pub(crate) fn approved_developer_fixture() -> ApprovedDeveloperFixture {
    let path = std::env::temp_dir().join(format!(
        "elon_open_commerce_production_state_{}.db",
        Uuid::new_v4().simple()
    ));
    let store = Store::open(&path).expect("production state test database should open");
    let owner = store
        .create_user(
            "production-state@example.com",
            "secret1",
            Some("Production State"),
            None,
        )
        .unwrap();
    let project = store
        .create_project(&owner.id, "Production State", None, None)
        .unwrap()
        .project;
    let created = store
        .create_open_commerce_developer_app(
            &project.id,
            &owner.id,
            CreateDeveloperAppRequest {
                app_id: "consumer.production.state".to_string(),
                display_name: "Production State Consumer".to_string(),
            },
        )
        .unwrap();
    let actor = OpenCommerceActor {
        user_id: &owner.id,
        app_id: "pc-web",
        project_role: Some("owner"),
    };
    let app = open_commerce_developer_manifest_service::update_manifest(
        &store,
        &project.id,
        &created.app.id,
        UpdateDeveloperAppManifestRequest {
            expected_manifest_revision: created.app.manifest_revision,
            homepage_url: Some("https://shop.example.test/app".to_string()),
            privacy_policy_url: Some("https://shop.example.test/privacy".to_string()),
            terms_url: Some("https://shop.example.test/terms".to_string()),
            support_email: Some("support@example.test".to_string()),
            requested_scopes: vec!["menu.preview".to_string()],
        },
        &actor,
    )
    .unwrap();

    // Direct store transitions only prepare local state. They are not DNS/TLS proof.
    let app = store
        .issue_open_commerce_developer_app_domain_challenge(
            &project.id,
            &app.id,
            app.manifest_revision,
            "shop.example.test",
            "local-challenge-hash",
            &(Utc::now() + Duration::hours(1)).to_rfc3339(),
        )
        .unwrap();
    let app = store
        .verify_open_commerce_developer_app_domain(&project.id, &app.id, app.manifest_revision)
        .unwrap();
    let app = open_commerce_developer_manifest_service::submit_manifest(
        &store,
        &project.id,
        &app.id,
        app.manifest_revision,
        &actor,
    )
    .unwrap();
    let app = open_commerce_developer_manifest_service::review_manifest(
        &store,
        &app.id,
        ReviewDeveloperAppManifestRequest {
            expected_manifest_revision: app.manifest_revision,
            decision: "approved".to_string(),
            note: "local approval fixture".to_string(),
        },
        "reviewer-user",
    )
    .unwrap();
    open_commerce_developer_admission_service::submit_admission(
        &store,
        &project.id,
        &app.id,
        SubmitDeveloperAppAdmissionRequest {
            expected_manifest_revision: app.manifest_revision,
            organization_name: "Example Merchant Ltd".to_string(),
            jurisdiction: "Test Jurisdiction".to_string(),
            registration_id: "TEST-REG-001".to_string(),
            information_attested: true,
        },
        &actor,
    )
    .unwrap();
    let admission = open_commerce_developer_admission_service::review_admission(
        &store,
        &app.id,
        ReviewDeveloperAppAdmissionRequest {
            expected_manifest_revision: app.manifest_revision,
            decision: "approved".to_string(),
            risk_tier: "standard".to_string(),
            note: "local admission fixture".to_string(),
        },
        "reviewer-user",
    )
    .unwrap();

    ApprovedDeveloperFixture {
        store,
        project_id: project.id,
        owner_user_id: owner.id,
        app,
        admission_id: admission.id,
    }
}

pub(crate) fn issue_local_credential(
    fixture: &ApprovedDeveloperFixture,
) -> DeveloperProductionCredentialSecret {
    fixture
        .store
        .issue_open_commerce_developer_production_credential(
            &fixture.app,
            &fixture.admission_id,
            &["menu.preview".to_string()],
            "reviewer-user",
            &(Utc::now() + Duration::days(30)).to_rfc3339(),
        )
        .unwrap()
}

pub(crate) fn test_app_state(store: Store, root: &Path) -> AppState {
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
