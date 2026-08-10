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

use crate::{
    open_commerce_developer_model::{
        CreateDeveloperAppRequest, OpenCommerceDeveloperAppCredential,
    },
    open_commerce_directory_service,
    open_commerce_model::{
        CreateCapabilityRequest, CreateMerchantRequest, ACCESS_AUTHORIZED, ACCESS_PUBLIC,
        HANDLER_STATIC_JSON,
    },
    open_commerce_portability_import_model::CreateConsumerPortabilityImportRequest,
    open_commerce_portability_import_service,
    open_commerce_portability_model::CreateConsumerPortabilityExportRequest,
    open_commerce_portability_reauthorization_model::{
        CreatePortabilityReauthorizationRequest, CreatePortabilityRelationshipMappingRequest,
    },
    open_commerce_portability_service,
    open_commerce_relationship_model::{
        CreateConsumerRelationshipRequest, RELATIONSHIP_SCOPE_MEMBERSHIP_LINK,
        RELATIONSHIP_SCOPE_PREFERENCE_REMEMBER,
    },
    open_commerce_relationship_service,
    open_commerce_service::{self, OpenCommerceActor},
    store::Store,
    types::{AgentsConfig, AiBackend, AiCliConfig, AppState},
};

pub(super) const SOURCE_SCOPES: [&str; 2] = [
    RELATIONSHIP_SCOPE_MEMBERSHIP_LINK,
    RELATIONSHIP_SCOPE_PREFERENCE_REMEMBER,
];

pub(super) struct Fixture {
    pub(super) state: Arc<AppState>,
    pub(super) router: Router,
    pub(super) destination_project_id: String,
    pub(super) second_project_id: String,
    pub(super) owner_id: String,
    pub(super) member_id: String,
    pub(super) owner_token: String,
    pub(super) member_token: String,
    pub(super) outsider_token: String,
    pub(super) import_id: String,
    pub(super) source_relationship_id: String,
    pub(super) target_merchant_id: String,
    pub(super) alternate_target_merchant_id: String,
    pub(super) target_merchant_project_id: String,
    pub(super) merchant_owner_id: String,
    pub(super) app_id: String,
    pub(super) outsider_app_id: String,
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

    pub(super) fn set_target_published(&self, published: bool) {
        open_commerce_directory_service::set_publication(
            &self.state.store,
            &self.target_merchant_project_id,
            &self.target_merchant_id,
            &OpenCommerceActor {
                user_id: &self.merchant_owner_id,
                app_id: "pc-web",
                project_role: Some("owner"),
            },
            published,
        )
        .unwrap();
    }

    pub(super) fn count(&self, table: &str) -> i64 {
        assert!(matches!(
            table,
            "open_commerce_authorization_requests"
                | "open_commerce_consumer_relationships"
                | "open_commerce_grants"
                | "open_commerce_portability_relationship_mappings"
        ));
        self.state
            .store
            .conn()
            .unwrap()
            .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                row.get(0)
            })
            .unwrap()
    }

    pub(super) fn audit_metadata(&self, action: &str) -> Vec<Value> {
        let conn = self.state.store.conn().unwrap();
        let mut statement = conn
            .prepare(
                "SELECT metadata_json FROM open_commerce_audit_events
                  WHERE project_id=?1 AND action=?2 ORDER BY created_at, rowid",
            )
            .unwrap();
        statement
            .query_map([&self.destination_project_id, action], |row| {
                let value: String = row.get(0)?;
                Ok(value)
            })
            .unwrap()
            .map(|row| serde_json::from_str(&row.unwrap()).unwrap())
            .collect()
    }
}

pub(super) fn fixture() -> Fixture {
    let root = std::env::temp_dir().join(format!(
        "elon-open-commerce-portability-reauthorization-{}",
        Uuid::new_v4().simple()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let store = Store::open(&root.join("state.sqlite")).unwrap();

    let merchant_owner = store
        .create_user(
            "reauthorization-merchant@example.com",
            "secret1",
            None,
            None,
        )
        .unwrap();
    let source_merchant = create_published_merchant(
        &store,
        &merchant_owner.id,
        "Reauthorization source",
        "source",
        false,
    );
    let target_merchant = create_published_merchant(
        &store,
        &merchant_owner.id,
        "Reauthorization target",
        "target",
        true,
    );
    let alternate_target = create_published_merchant(
        &store,
        &merchant_owner.id,
        "Reauthorization alternate",
        "alternate",
        true,
    );

    let owner = store
        .create_user("reauthorization-owner@example.com", "secret1", None, None)
        .unwrap();
    let member = store
        .create_user("reauthorization-member@example.com", "secret1", None, None)
        .unwrap();
    let outsider = store
        .create_user(
            "reauthorization-outsider@example.com",
            "secret1",
            None,
            None,
        )
        .unwrap();
    let source_project = store
        .create_project(&owner.id, "Reauthorization source wallet", None, None)
        .unwrap()
        .project;
    let destination_project = store
        .create_project(&owner.id, "Reauthorization destination", None, None)
        .unwrap()
        .project;
    let second_project = store
        .create_project(&owner.id, "Reauthorization second destination", None, None)
        .unwrap()
        .project;
    store
        .add_project_member_by_account(&destination_project.id, &member.id, "member")
        .unwrap();

    let source_actor = OpenCommerceActor {
        user_id: &owner.id,
        app_id: "pc-web",
        project_role: Some("owner"),
    };
    let relationship = open_commerce_relationship_service::create_relationship(
        &store,
        &source_project.id,
        &source_actor,
        CreateConsumerRelationshipRequest {
            merchant_id: source_merchant.merchant_id,
            source_app_id: "pc-web".to_string(),
            scopes: SOURCE_SCOPES
                .iter()
                .map(|scope| (*scope).to_string())
                .collect(),
            purpose: "迁移消费者关系后重新授权".to_string(),
            expires_at: (Utc::now() + Duration::days(90)).to_rfc3339(),
        },
    )
    .unwrap();
    let package = open_commerce_portability_service::create_export(
        &store,
        &source_project.id,
        &source_actor,
        CreateConsumerPortabilityExportRequest {
            idempotency_key: "reauthorization-source-export".to_string(),
        },
    )
    .unwrap();
    let import_record = open_commerce_portability_import_service::create_import(
        &store,
        &destination_project.id,
        &source_actor,
        CreateConsumerPortabilityImportRequest {
            source_operator: "fixture-operator".to_string(),
            package,
            signature: None,
        },
    )
    .unwrap();

    let app = create_app(
        &store,
        &destination_project.id,
        &owner.id,
        "consumer.reauthorize",
    );
    let outsider_project = store
        .create_project(&outsider.id, "Reauthorization outsider", None, None)
        .unwrap()
        .project;
    let outsider_app = create_app(
        &store,
        &outsider_project.id,
        &outsider.id,
        "consumer.reauthorize-outsider",
    );
    let (owner_token, _) = store.create_session(&owner.id, Some("test"), None).unwrap();
    let (member_token, _) = store
        .create_session(&member.id, Some("test"), None)
        .unwrap();
    let (outsider_token, _) = store
        .create_session(&outsider.id, Some("test"), None)
        .unwrap();

    let state = Arc::new(test_state(store, &root));
    let router = crate::open_commerce_portability_reauthorization_api::routes()
        .with_state(Arc::clone(&state));
    Fixture {
        state,
        router,
        destination_project_id: destination_project.id,
        second_project_id: second_project.id,
        owner_id: owner.id,
        member_id: member.id,
        owner_token,
        member_token,
        outsider_token,
        import_id: import_record.id,
        source_relationship_id: relationship.id,
        target_merchant_id: target_merchant.merchant_id,
        alternate_target_merchant_id: alternate_target.merchant_id,
        target_merchant_project_id: target_merchant.project_id,
        merchant_owner_id: merchant_owner.id,
        app_id: app.app.app_id,
        outsider_app_id: outsider_app.app.app_id,
    }
}

pub(super) fn mapping_request(fixture: &Fixture) -> CreatePortabilityRelationshipMappingRequest {
    CreatePortabilityRelationshipMappingRequest {
        import_id: fixture.import_id.clone(),
        source_relationship_id: fixture.source_relationship_id.clone(),
        target_merchant_id: fixture.target_merchant_id.clone(),
        confirmed_by_user: true,
    }
}

pub(super) fn reauthorization_request(
    fixture: &Fixture,
) -> CreatePortabilityReauthorizationRequest {
    CreatePortabilityReauthorizationRequest {
        requester_app_id: fixture.app_id.clone(),
        scopes: vec![RELATIONSHIP_SCOPE_PREFERENCE_REMEMBER.to_string()],
        purpose: "迁移后请求目标商户重新授权".to_string(),
        confirmed_by_user: true,
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

struct MerchantFixture {
    merchant_id: String,
    project_id: String,
}

fn create_published_merchant(
    store: &Store,
    owner_id: &str,
    project_name: &str,
    slug_label: &str,
    authorized_relationship_capabilities: bool,
) -> MerchantFixture {
    let project = store
        .create_project(owner_id, project_name, None, None)
        .unwrap()
        .project;
    let actor = OpenCommerceActor {
        user_id: owner_id,
        app_id: "pc-web",
        project_role: Some("owner"),
    };
    let merchant = open_commerce_service::create_merchant(
        store,
        &project.id,
        &actor,
        CreateMerchantRequest {
            display_name: project_name.to_string(),
            slug: Some(format!(
                "reauthorization-{slug_label}-{}",
                Uuid::new_v4().simple()
            )),
            description: String::new(),
            node_mode: "platform_hosted".to_string(),
            public_profile: json!({"fixture":"portability-reauthorization"}),
        },
    )
    .unwrap();
    publish_capability(
        store,
        &project.id,
        &merchant.id,
        &actor,
        "profile.public",
        ACCESS_PUBLIC,
    );
    if authorized_relationship_capabilities {
        for scope in SOURCE_SCOPES {
            publish_capability(
                store,
                &project.id,
                &merchant.id,
                &actor,
                scope,
                ACCESS_AUTHORIZED,
            );
        }
    }
    open_commerce_directory_service::set_publication(
        store,
        &project.id,
        &merchant.id,
        &actor,
        true,
    )
    .unwrap();
    MerchantFixture {
        merchant_id: merchant.id,
        project_id: project.id,
    }
}

fn publish_capability(
    store: &Store,
    project_id: &str,
    merchant_id: &str,
    actor: &OpenCommerceActor<'_>,
    key: &str,
    access_level: &str,
) {
    open_commerce_service::publish_capability(
        store,
        project_id,
        merchant_id,
        actor,
        CreateCapabilityRequest {
            capability_key: key.to_string(),
            display_name: key.to_string(),
            description: String::new(),
            kind: "query".to_string(),
            access_level: access_level.to_string(),
            input_schema: json!({}),
            output_schema: json!({}),
            handler_type: HANDLER_STATIC_JSON.to_string(),
            handler_config: Some(json!({"response":{"ok":true}})),
            unit_price_micros: 0,
            currency: "CNY".to_string(),
            freshness_seconds: 0,
        },
    )
    .unwrap();
}

fn create_app(
    store: &Store,
    project_id: &str,
    owner_id: &str,
    app_id: &str,
) -> OpenCommerceDeveloperAppCredential {
    store
        .create_open_commerce_developer_app(
            project_id,
            owner_id,
            CreateDeveloperAppRequest {
                app_id: app_id.to_string(),
                display_name: app_id.to_string(),
            },
        )
        .unwrap()
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
