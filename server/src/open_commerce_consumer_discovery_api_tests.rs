use std::{collections::HashMap, path::Path, sync::Arc};

use axum::{
    body::{to_bytes, Body},
    http::{header, Request, StatusCode},
    Router,
};
use serde_json::{json, Value};
use tokio::sync::RwLock;
use tower::ServiceExt;

use super::routes;
use crate::{
    open_commerce_consumer::source_test_support,
    types::{AgentsConfig, AiBackend, AiCliConfig, AppState},
};

#[tokio::test]
async fn consumer_http_discovery_applies_the_complete_pc_filter_contract() {
    let fixture = source_test_support::fixture();
    let root = std::env::temp_dir().join(format!(
        "elon-consumer-discovery-api-{}",
        uuid::Uuid::new_v4().simple()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let (token, _) = fixture
        .store
        .create_session(&fixture.consumer_id, Some("test"), None)
        .unwrap();
    let state = Arc::new(test_state(fixture.store, &root));
    let router = routes().with_state(Arc::clone(&state));
    let request = json!({
        "query":"商户",
        "capability_key":"catalog.search",
        "requester_app_id":"pc-web",
        "ranking_policy":"lowest_unit_price.v1",
        "include_ranking_receipt":true,
        "require_current_declaration":true,
        "require_internal_sync_receipt":true,
        "source_provider_key":" ALPHA_ERP ",
        "source_data_domain":" CATALOG ",
        "max_source_age_seconds":120,
        "price_currency":"cny",
        "capability_kind":"query",
        "access_level":"public",
        "require_city_match":true,
        "require_category_match":true,
        "require_all_tags_match":true,
        "preferences":{
            "categories":["retail"],
            "tags":["open"],
            "city":"吉安",
            "max_unit_price_micros":0,
            "prefer_public":true
        },
        "limit":1
    });

    let (unauthorized, _) = discover(&router, None, request.clone()).await;
    assert_eq!(unauthorized, StatusCode::UNAUTHORIZED);

    let audit_before = audit_count(&state.store);
    let (status, body) = discover(&router, Some(&token), request).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["schema"], "open_commerce.consumer_discovery.v1");
    assert_eq!(body["requester_app_id"], "pc-web");
    assert_eq!(body["ranking_policy"], "lowest_unit_price.v1");
    assert_eq!(body["ranking_is_paid"], false);
    assert_eq!(body["ranking_is_user_selected"], true);
    assert_eq!(body["source_filter"]["provider_key"], "alpha_erp");
    assert_eq!(body["source_filter"]["data_domain"], "catalog");
    assert_eq!(body["source_filter"]["max_age_seconds"], 120);
    assert_eq!(body["price_filter"]["currency"], "CNY");
    assert_eq!(body["price_filter"]["max_unit_price_micros"], 0);
    assert_eq!(body["capability_filter"]["kind"], "query");
    assert_eq!(body["capability_filter"]["access_level"], "public");
    assert_eq!(body["preference_constraints"]["require_city_match"], true);
    assert_eq!(
        body["preference_constraints"]["require_category_match"],
        true
    );
    assert_eq!(
        body["preference_constraints"]["require_all_tags_match"],
        true
    );
    assert_eq!(body["candidate_scope"]["eligible_match_count"], 1);
    assert_eq!(body["candidate_scope"]["returned_match_count"], 1);
    assert_eq!(body["matches"].as_array().unwrap().len(), 1);
    assert_eq!(
        body["matches"][0]["merchant"]["display_name"],
        "近期 Alpha 商户"
    );
    assert_eq!(
        body["matches"][0]["capability"]["source"]["provider_key"],
        "alpha_erp"
    );
    assert_eq!(
        body["matches"][0]["capability"]["source"]["data_domain"],
        "catalog"
    );
    assert_eq!(
        body["matches"][0]["capability"]["freshness"]["status"],
        "current"
    );
    assert_eq!(body["ranking_receipt"]["signed_by_operator"], false);
    let receipt: Value = serde_json::from_str(
        body["ranking_receipt"]["canonical_payload_json"]
            .as_str()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(receipt["source_filter"]["provider_key"], "alpha_erp");
    assert_eq!(receipt["price_filter"]["currency"], "CNY");
    assert_eq!(audit_before, audit_count(&state.store));
}

async fn discover(router: &Router, token: Option<&str>, body: Value) -> (StatusCode, Value) {
    let mut request = Request::builder()
        .method("POST")
        .uri("/api/open-commerce/sandbox/discover")
        .header(header::CONTENT_TYPE, "application/json");
    if let Some(token) = token {
        request = request.header(header::AUTHORIZATION, format!("Bearer {token}"));
    }
    let response = router
        .clone()
        .oneshot(request.body(Body::from(body.to_string())).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    (status, serde_json::from_slice(&bytes).unwrap())
}

fn audit_count(store: &crate::store::Store) -> i64 {
    store
        .conn()
        .unwrap()
        .query_row(
            "SELECT COUNT(*) FROM open_commerce_audit_events",
            [],
            |row| row.get(0),
        )
        .unwrap()
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
