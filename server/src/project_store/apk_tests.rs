use std::{collections::HashMap, sync::Arc};

use axum::{
    body::to_bytes,
    extract::{Path, Query, State},
    http::{header, HeaderMap, StatusCode},
};
use tokio::sync::RwLock;

use super::download_project_android;
use crate::types::{AgentsConfig, AiBackend, AiCliConfig, AppState};

#[tokio::test]
async fn official_quant_without_admitted_release_is_public_not_found() {
    let state = empty_download_state();
    let mut bearer_headers = HeaderMap::new();
    bearer_headers.insert(header::AUTHORIZATION, "Bearer not-a-token".parse().unwrap());

    for (headers, query) in [
        (HeaderMap::new(), HashMap::new()),
        (
            HeaderMap::new(),
            HashMap::from([("token".to_string(), "not-a-token".to_string())]),
        ),
        (bearer_headers, HashMap::new()),
    ] {
        let response = download_project_android(
            State(Arc::clone(&state)),
            headers,
            Path("yilong-quant".to_string()),
            Query(query),
        )
        .await;

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        assert_eq!(
            response.headers().get(header::CACHE_CONTROL).unwrap(),
            "no-store"
        );
        let body = to_bytes(response.into_body(), 4096).await.unwrap();
        let value: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(
            value,
            serde_json::json!({ "error": "这个项目暂无可安装新版 APK" })
        );
        assert!(!String::from_utf8_lossy(&body).contains("token"));
    }
}

#[tokio::test]
async fn non_official_projects_keep_member_download_fallback() {
    let state = empty_download_state();
    for project_id in ["yilong-quant-preview", "merchant-private"] {
        let response = download_project_android(
            State(Arc::clone(&state)),
            HeaderMap::new(),
            Path(project_id.to_string()),
            Query(HashMap::new()),
        )
        .await;

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        let body = to_bytes(response.into_body(), 4096).await.unwrap();
        assert!(String::from_utf8_lossy(&body).contains("token"));
    }
}

fn empty_download_state() -> Arc<AppState> {
    let root = std::env::temp_dir().join(format!(
        "elon_quant_public_empty_{}",
        uuid::Uuid::new_v4().simple()
    ));
    std::fs::create_dir_all(&root).expect("test directory should be created");
    let store = crate::store::Store::open(&root.join("store.db")).expect("store should open");
    let owner = store
        .create_user(
            "quant-public-empty-owner@example.com",
            "synthetic-password",
            None,
            None,
        )
        .expect("owner should be created");
    let conn = store.conn().expect("connection should open");
    for (id, name, join_mode, is_public) in [
        ("yilong-quant", "一龙量化交易", "readonly", 1),
        ("yilong-quant-preview", "量化预览项目", "open", 1),
        ("merchant-private", "普通私有项目", "invite", 0),
    ] {
        conn.execute(
            "INSERT INTO projects (
               id, name, workspace_key, template, source_type, status, created_by,
               created_at, updated_at, is_public, join_mode
             ) VALUES (?1, ?2, ?1, 'blank', 'template', 'active', ?3,
                       '2026-09-05T00:00:00Z', '2026-09-05T00:00:00Z', ?5, ?4)",
            rusqlite::params![id, name, owner.id, join_mode, is_public],
        )
        .expect("project should be inserted");
    }
    drop(conn);

    Arc::new(AppState {
        store,
        data_dir: root.clone(),
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
        project_root: root.clone(),
        workspace_root: root.to_string_lossy().into_owned(),
        public_url: "http://127.0.0.1".to_string(),
        http_client: reqwest::Client::new(),
        admin_token: "synthetic-admin-token".to_string(),
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
    })
}
