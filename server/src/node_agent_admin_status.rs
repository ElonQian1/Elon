// server/src/node_agent_admin_status.rs

use axum::{extract::State, http::HeaderMap, Json};
use std::{sync::atomic::Ordering, sync::Arc};
use tracing::warn;

pub(super) async fn admin_status(
    State(rt): State<Arc<super::NodeRuntime>>,
    headers: HeaderMap,
) -> Json<serde_json::Value> {
    rt.refresh_models_background();
    rt.ensure_cli_probe_background(false).await;
    let creds = rt.creds().await;
    let (connected, last_event, live) = {
        let st = rt.status.read().await;
        (
            st.connected,
            st.last_event.clone(),
            st.models_cached.clone(),
        )
    };
    let hardware = rt.hardware_profile().await;
    let storage_settings = rt.storage_settings.read().await.clone();
    let storage = super::pc_storage_repo::storage_profile(&storage_settings);
    let node_data_root = rt.node_data_root.read().await.status_payload();
    let full_access_grant_count =
        match super::node_agent_full_access::current_grant_identity(&rt).await {
            Ok(identity) => rt.full_access_grants.list(&identity).await.len(),
            Err(_) => 0,
        };
    let active_cli_prompts = rt.active_cli_prompts.views_without_approvals().await;
    let active_cli_prompt_count = active_cli_prompts.len();
    let recent_task_records = rt.task_journal.latest_records(20).unwrap_or_else(|error| {
        warn!("PC 任务 journal 读取失败，CLI 会话桥接状态降级为空摘要: {error}");
        Vec::new()
    });
    let sidecar_sessions = rt.cli_sidecars.latest_sessions(20).unwrap_or_else(|error| {
        warn!("PC CLI sidecar registry 读取失败，CLI 会话桥接 sidecar 状态降级为空摘要: {error}");
        Vec::new()
    });
    let cli_probe = rt.cached_cli_probe().await;
    let cli_refreshing = rt.cli_probe_refreshing.load(Ordering::Acquire);
    let available_clis = cli_probe.available_names();
    let codex_cli = cli_probe.codex_status();
    let codex_path = codex_cli.as_ref().and_then(|tool| tool.path.as_deref());
    let lifecycle = rt
        .lifecycle
        .status_payload(super::node_agent_lifecycle::LifecycleInputs {
            connected,
            logged_in: creds.is_some(),
            last_event: &last_event,
            active_task_count: active_cli_prompt_count,
            sidecar_session_count: sidecar_sessions.len(),
        });
    let mut payload = serde_json::json!({
        "version": env!("CARGO_PKG_VERSION"),
        "local_admin_token_header": super::node_agent_local_admin::LOCAL_ADMIN_TOKEN_HEADER,
        "lifecycle_report_schema_version": 1,
        "task_journal_supported": true,
        "task_journal_schema_version": 1,
        "active_cli_prompt_count": active_cli_prompt_count,
        "logged_in": creds.is_some(),
        "agent_id": creds.as_ref().map(|c| c.agent_id.clone()),
        "device_name": super::machine_label(),
        "owner_user_id": creds.as_ref().map(|c| c.owner_user_id.clone()),
        "user_token_configured": creds.as_ref().map(|c| c.user_token.is_some()).unwrap_or(false),
        "cloud_url": rt.cfg.cloud_url,
        "cloud_http_url": rt.cfg.cloud_http_url,
        "ollama_url": rt.cfg.ollama_url,
        "lm_studio_url": rt.cfg.lm_studio_url,
        "custom_url": rt.cfg.custom_url,
        "price_per_1k": rt.cfg.price_per_1k,
        "connected": connected,
        "last_event": last_event,
        "lifecycle": lifecycle,
        "hardware": hardware,
        "storage": storage,
        "node_data_root": node_data_root,
        "full_access_grant_count": full_access_grant_count,
        "runtime_policy": super::node_agent_full_access::runtime_policy_summary(),
        "cli_session_bridge": super::node_agent_cli_session_bridge::status_payload_for(
            &active_cli_prompts,
            &recent_task_records,
            &sidecar_sessions,
        ),
        "cli_probe": {
            "refreshing": cli_refreshing,
            "refreshed_at_ms": cli_probe.refreshed_at_ms,
            "stale": cli_probe.is_stale(),
        },
        "download_router": super::node_agent_download_router::status_payload(),
        "cloud_network": super::node_agent_cloud_net::status_payload(&rt.cfg.cloud_url, &rt.cfg.cloud_http_url),
        "codex_vault": super::node_agent_codex_vault::local_status_payload(),
        "codex_cli": codex_cli,
        "codex_toolbox": super::node_agent_cli_env::codex_toolbox_status(codex_path),
        "allowed_clis": available_clis,
        "cli_tools": cli_probe.tools.clone(),
        "local_ai": {
            "cli_tools": cli_probe.tools.clone(),
            "models": live.clone(),
        },
        "models": live,
    });
    if super::node_agent_local_admin::can_expose_local_admin_token(&headers, &rt.cloud_http_url()) {
        if let Some(obj) = payload.as_object_mut() {
            obj.insert(
                "local_admin_token".to_string(),
                serde_json::json!(rt.local_admin_token()),
            );
        }
    }
    Json(payload)
}
