use std::{collections::HashMap, path::Path, sync::Arc};

use axum::{
    body::{to_bytes, Body},
    http::{header, Request, StatusCode},
    Router,
};
use serde_json::{json, Value};
use tokio::sync::RwLock;
use tower::ServiceExt;

use crate::{
    erp_blueprint::model::{
        ErpBlueprintDefinition, ErpModuleDefinition, ErpReleaseCompatibility, ErpReleaseManifest,
        ErpRollbackPlan, VersionedErpModule, BLUEPRINT_SCHEMA, RELEASE_SCHEMA,
    },
    open_commerce_model::CreateMerchantRequest,
    open_commerce_service::{self, OpenCommerceActor},
    store::Store,
    types::{AgentsConfig, AiBackend, AiCliConfig, AppState},
};

use super::{
    api,
    model::CreateManagedRolloutPlanRequest,
    service::{create_plan, get_plan, list_plans},
};

const RELEASE_SHA256: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const RUNTIME_SHA256: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

struct Fixture {
    store: Store,
    owner_id: String,
    project_id: String,
    other_project_id: String,
    instance_id: String,
    configuration_revision: i64,
}

#[test]
fn rollout_plan_is_hash_bound_idempotent_and_immutable() {
    let fixture = fixture();
    let request = valid_request(fixture.configuration_revision);
    let first = create_plan(
        &fixture.store,
        &fixture.project_id,
        &fixture.instance_id,
        &fixture.owner_id,
        request,
    )
    .unwrap();
    let replay = create_plan(
        &fixture.store,
        &fixture.project_id,
        &fixture.instance_id,
        &fixture.owner_id,
        valid_request(fixture.configuration_revision),
    )
    .unwrap();

    assert_eq!(first.id, replay.id);
    assert_eq!(first.plan_sha256, replay.plan_sha256);
    assert_eq!(first.status, "planned");
    assert_eq!(first.payload.source.configuration_revision, 2);
    assert_eq!(first.payload.source.release_manifest_sha256, RELEASE_SHA256);
    assert_eq!(first.payload.edge_route.upstream_addr, "127.0.0.1:18081");
    assert_eq!(
        first.payload.runtime_candidate.endpoint_base_url,
        "http://127.0.0.1:18443/merchants/coffee-a"
    );
    let encoded = serde_json::to_string(&first).unwrap();
    assert!(!encoded.contains("database-password"));
    assert!(!encoded.contains("runtime-secret-value"));

    let mut changed = valid_request(fixture.configuration_revision);
    changed.target_node_id = "commerce-node-b".to_string();
    let second = create_plan(
        &fixture.store,
        &fixture.project_id,
        &fixture.instance_id,
        &fixture.owner_id,
        changed,
    )
    .unwrap();
    assert_ne!(first.id, second.id);
    assert_ne!(first.plan_sha256, second.plan_sha256);
    assert_eq!(
        list_plans(
            &fixture.store,
            &fixture.project_id,
            &fixture.instance_id,
            20
        )
        .unwrap()
        .len(),
        2
    );
    assert_eq!(
        get_plan(
            &fixture.store,
            &fixture.project_id,
            &fixture.instance_id,
            &first.id
        )
        .unwrap()
        .plan_sha256,
        first.plan_sha256
    );

    fixture
        .store
        .conn()
        .unwrap()
        .execute(
            "UPDATE erp_managed_rollout_plans SET payload_json='{}' WHERE id=?1",
            rusqlite::params![first.id],
        )
        .unwrap();
    assert!(get_plan(
        &fixture.store,
        &fixture.project_id,
        &fixture.instance_id,
        &first.id
    )
    .is_err());

    fixture
        .store
        .conn()
        .unwrap()
        .execute(
            "UPDATE erp_managed_rollout_plans
             SET source_configuration_revision=source_configuration_revision + 1
             WHERE id=?1",
            rusqlite::params![second.id],
        )
        .unwrap();
    assert!(get_plan(
        &fixture.store,
        &fixture.project_id,
        &fixture.instance_id,
        &second.id
    )
    .is_err());
}

#[test]
fn rollout_plan_rejects_stale_identity_and_unsafe_targets() {
    let fixture = fixture();

    let mut unconfirmed = valid_request(fixture.configuration_revision);
    unconfirmed.merchant_confirmed = false;
    assert!(create_plan(
        &fixture.store,
        &fixture.project_id,
        &fixture.instance_id,
        &fixture.owner_id,
        unconfirmed
    )
    .is_err());

    let stale = valid_request(fixture.configuration_revision - 1);
    assert!(create_plan(
        &fixture.store,
        &fixture.project_id,
        &fixture.instance_id,
        &fixture.owner_id,
        stale
    )
    .is_err());

    let mut wrong_path = valid_request(fixture.configuration_revision);
    wrong_path.public_base_path = "/merchants/other-store".to_string();
    assert!(create_plan(
        &fixture.store,
        &fixture.project_id,
        &fixture.instance_id,
        &fixture.owner_id,
        wrong_path
    )
    .is_err());

    let mut mismatched_endpoint = valid_request(fixture.configuration_revision);
    mismatched_endpoint.endpoint_base_url =
        "http://127.0.0.1:18443/merchants/different".to_string();
    assert!(create_plan(
        &fixture.store,
        &fixture.project_id,
        &fixture.instance_id,
        &fixture.owner_id,
        mismatched_endpoint
    )
    .is_err());

    let mut inline_secret = valid_request(fixture.configuration_revision);
    inline_secret.secrets_source = "database-password".to_string();
    assert!(create_plan(
        &fixture.store,
        &fixture.project_id,
        &fixture.instance_id,
        &fixture.owner_id,
        inline_secret
    )
    .is_err());

    assert!(list_plans(
        &fixture.store,
        &fixture.other_project_id,
        &fixture.instance_id,
        20
    )
    .is_err());
}

#[tokio::test]
async fn rollout_http_requires_project_access_and_returns_bound_plan() {
    let fixture = fixture();
    let root = std::env::temp_dir().join(format!(
        "elon-managed-rollout-api-{}",
        uuid::Uuid::new_v4().simple()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let (token, _) = fixture
        .store
        .create_session(&fixture.owner_id, Some("test"), None)
        .unwrap();
    let project_id = fixture.project_id.clone();
    let instance_id = fixture.instance_id.clone();
    let revision = fixture.configuration_revision;
    let router = api::routes().with_state(Arc::new(test_state(fixture.store, &root)));
    let uri = format!("/api/projects/{project_id}/erp/instances/{instance_id}/managed-rollouts");
    let body = serde_json::to_value(valid_request(revision)).unwrap();

    let (status, _) = send(&router, "POST", &uri, None, Some(body.clone())).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    let (status, created) = send(&router, "POST", &uri, Some(&token), Some(body)).await;
    assert_eq!(status, StatusCode::OK, "{created}");
    assert_eq!(
        created["payload"]["schema"],
        "yilong.erp.managed_rollout_plan.v1"
    );
    assert_eq!(created["payload"]["source"]["instance_id"], instance_id);
    assert_eq!(
        created["payload"]["boundaries"].as_array().unwrap().len(),
        5
    );

    let (status, plans) = send(&router, "GET", &uri, Some(&token), None).await;
    assert_eq!(status, StatusCode::OK, "{plans}");
    assert_eq!(plans.as_array().unwrap().len(), 1);
    let detail_uri = format!("{uri}/{}", created["id"].as_str().unwrap());
    let (status, detail) = send(&router, "GET", &detail_uri, Some(&token), None).await;
    assert_eq!(status, StatusCode::OK, "{detail}");
    assert_eq!(detail["plan_sha256"], created["plan_sha256"]);
}

fn fixture() -> Fixture {
    let path = std::env::temp_dir().join(format!(
        "elon_managed_rollout_{}.db",
        uuid::Uuid::new_v4().simple()
    ));
    let store = Store::open(&path).unwrap();
    let owner = store
        .create_user("rollout-owner@example.com", "secret1", Some("Owner"), None)
        .unwrap();
    let blueprint_project = store
        .create_project(&owner.id, "ERP Blueprint", None, None)
        .unwrap()
        .project;
    let project = store
        .create_project(&owner.id, "Coffee Merchant", None, None)
        .unwrap()
        .project;
    let other_project = store
        .create_project(&owner.id, "Other Merchant", None, None)
        .unwrap()
        .project;
    let blueprint = store
        .create_erp_blueprint(
            ErpBlueprintDefinition {
                schema: BLUEPRINT_SCHEMA.to_string(),
                blueprint_key: "retail-core".to_string(),
                name: "Retail Core".to_string(),
                description: String::new(),
                source_project_id: blueprint_project.id,
                modules: vec![ErpModuleDefinition {
                    module_key: "core".to_string(),
                    version: "1.0.0".to_string(),
                    kind: "core".to_string(),
                    required: true,
                    dependencies: Vec::new(),
                }],
                capabilities: Vec::new(),
                themes: vec!["default".to_string()],
                extension_points: Vec::new(),
                proposal_threshold: 2,
            },
            &owner.id,
        )
        .unwrap();
    let manifest = ErpReleaseManifest {
        schema: RELEASE_SCHEMA.to_string(),
        blueprint_key: "retail-core".to_string(),
        version: "1.0.0".to_string(),
        previous_version: None,
        source_git_commit: "0123456789abcdef0123456789abcdef01234567".to_string(),
        modules: vec![VersionedErpModule {
            module_key: "core".to_string(),
            version: "1.0.0".to_string(),
            required: true,
        }],
        capabilities: Vec::new(),
        extension_points: Vec::new(),
        migrations: Vec::new(),
        runtime: None,
        compatibility: ErpReleaseCompatibility {
            minimum_instance_version: "1.0.0".to_string(),
            required_plugins: Vec::new(),
        },
        rollback: ErpRollbackPlan {
            supported: true,
            instructions: "restore previous release".to_string(),
        },
    };
    let version = store
        .create_erp_blueprint_version(&blueprint.id, &manifest, RELEASE_SHA256, &owner.id)
        .unwrap();
    let instance = store
        .create_erp_instance(
            "coffee-a",
            &project.id,
            &blueprint.id,
            &version.id,
            "coffee",
            "default",
            &["core".to_string()],
            &[],
            &[],
            "existing_project",
            &owner.id,
        )
        .unwrap();
    let actor = OpenCommerceActor {
        user_id: &owner.id,
        app_id: "pc-web",
        project_role: Some("owner"),
    };
    let merchant = open_commerce_service::create_merchant(
        &store,
        &project.id,
        &actor,
        CreateMerchantRequest {
            display_name: "Coffee A".to_string(),
            slug: Some("coffee-a".to_string()),
            description: String::new(),
            node_mode: "self_hosted".to_string(),
            public_profile: json!({"category":"coffee"}),
        },
    )
    .unwrap();
    let instance = store
        .update_erp_instance_open_commerce_merchant(&instance.id, 1, Some(&merchant.id))
        .unwrap();
    Fixture {
        store,
        owner_id: owner.id,
        project_id: project.id,
        other_project_id: other_project.id,
        instance_id: instance.id,
        configuration_revision: instance.configuration_revision,
    }
}

fn valid_request(configuration_revision: i64) -> CreateManagedRolloutPlanRequest {
    CreateManagedRolloutPlanRequest {
        expected_configuration_revision: configuration_revision,
        merchant_confirmed: true,
        target_node_id: "commerce-node-a".to_string(),
        service_user: "ym-coffee-a".to_string(),
        store_id: "7a25d9ac-7796-4d78-a6d5-19fe75d35422".to_string(),
        profile_source: "/srv/yilong/profiles/coffee-a.json".to_string(),
        secrets_source: "/etc/yilong/merchants/coffee-a.env".to_string(),
        listen_port: 18081,
        runtime_key_id: "OPEN_COMMERCE_RUNTIME_SECRET_COFFEE_A".to_string(),
        public_base_path: "/merchants/coffee-a".to_string(),
        endpoint_base_url: "http://127.0.0.1:18443/merchants/coffee-a".to_string(),
        runtime_manifest_sha256: RUNTIME_SHA256.to_string(),
        timeout_ms: 5_000,
    }
}

async fn send(
    router: &Router,
    method: &str,
    uri: &str,
    token: Option<&str>,
    body: Option<Value>,
) -> (StatusCode, Value) {
    let mut request = Request::builder().method(method).uri(uri);
    if let Some(token) = token {
        request = request.header(header::AUTHORIZATION, format!("Bearer {token}"));
    }
    let request_body = match body {
        Some(body) => {
            request = request.header(header::CONTENT_TYPE, "application/json");
            Body::from(body.to_string())
        }
        None => Body::empty(),
    };
    let response = router
        .clone()
        .oneshot(request.body(request_body).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, body)
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
