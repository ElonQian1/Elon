// server/src/node_agent_admin_status.rs

use axum::{extract::State, http::HeaderMap, Json};
use std::collections::HashSet;
use std::{sync::atomic::Ordering, sync::Arc};
use tracing::warn;

const ADMIN_STATUS_MAX_BYTES: usize = 64 * 1024;
const ADMIN_STATUS_PAYLOAD_BUDGET: usize = ADMIN_STATUS_MAX_BYTES - 1024;

pub(super) async fn admin_health() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "service": "elon-node-agent",
        "status": "ok"
    }))
}

fn visible_active_runtime(status: &str, task_id: &str, live_task_ids: &HashSet<String>) -> bool {
    matches!(status, "running" | "cancel_requested") && live_task_ids.contains(task_id)
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn health_response_is_small_and_self_identifying() {
        let response = super::admin_health().await.0;

        assert_eq!(response["service"], "elon-node-agent");
        assert_eq!(response["status"], "ok");
        assert!(response.to_string().len() < 128);
    }

    #[test]
    fn stale_running_journal_is_not_reported_after_handles_and_sidecars_end() {
        let mut live = std::collections::HashSet::new();
        assert!(!super::visible_active_runtime("running", "task-a", &live));
        live.insert("task-a".to_string());
        assert!(super::visible_active_runtime("running", "task-a", &live));
        assert!(!super::visible_active_runtime("done", "task-a", &live));
    }

    #[test]
    fn oversized_status_is_compacted_below_the_declared_limit() {
        let mut payload = serde_json::json!({
            "version": "test",
            "models": [{"description": "x".repeat(100_000)}],
            "cli_tools": [{"description": "x".repeat(100_000)}],
            "local_ai": {"models": [{"description": "x".repeat(100_000)}], "cli_tools": []},
            "update_recovery": {"install_gate": {"active_foreground_task_ids": ["x".repeat(100_000)]}},
            "desktop_supervision": {
                "protocol": "elon.desktop_pc_supervision.v1",
                "capabilities": ["delta_wait_v1"]
            },
            "desktop_review_broker": {
                "protocol": "elon.desktop_review_broker.v1",
                "available": true,
                "pipe_name": "elon-desktop-review-test"
            },
            "task_journal_supported": true,
            "task_journal_schema_version": 1,
            "local_admin_token": "local-secret",
            "owner_user_id": "owner-test",
            "user_token_configured": false,
        });
        super::enforce_status_response_limit(&mut payload);
        let bytes = serde_json::to_vec(&payload).unwrap();
        assert!(bytes.len() <= super::ADMIN_STATUS_MAX_BYTES);
        assert_eq!(payload["local_admin_token"], "local-secret");
        assert_eq!(payload["owner_user_id"], "owner-test");
        assert_eq!(payload["user_token_configured"], false);
        assert_eq!(
            payload["desktop_supervision"]["protocol"],
            "elon.desktop_pc_supervision.v1"
        );
        assert_eq!(
            payload["desktop_supervision"]["capabilities"][0],
            "delta_wait_v1"
        );
        assert_eq!(payload["desktop_review_broker"]["available"], true);
        assert_eq!(
            payload["desktop_review_broker"]["pipe_name"],
            "elon-desktop-review-test"
        );
        assert_eq!(payload["task_journal_supported"], true);
        assert_eq!(payload["task_journal_schema_version"], 1);
        assert_eq!(
            payload["response_limits"]["max_bytes"],
            super::ADMIN_STATUS_MAX_BYTES
        );
        assert_eq!(payload["response_limits"]["compacted"], true);
    }
}

pub(super) async fn admin_status(
    State(rt): State<Arc<super::NodeRuntime>>,
    headers: HeaderMap,
) -> Json<serde_json::Value> {
    rt.refresh_models_background();
    rt.ensure_cli_probe_background(false).await;
    let creds = rt.creds().await;
    let (connected, last_event, live, connection) = {
        let st = rt.status.read().await;
        (
            st.connected,
            st.last_event.clone(),
            st.models_cached.clone(),
            serde_json::json!({
                "schema": "elon.node_connection_timing.v1",
                "attempt": st.connection_attempt,
                "stage": st.connection_stage,
                "attempt_started_at_ms": st.connection_attempt_started_at_ms,
                "stage_started_at_ms": st.stage_started_at_ms,
                "stage_durations_ms": st.stage_durations_ms,
                "connected_at_ms": st.connected_at_ms,
                "last_disconnected_at_ms": st.last_disconnected_at_ms,
                "last_connect_duration_ms": st.last_connect_duration_ms,
                "next_backoff_ms": st.next_backoff_ms,
            }),
        )
    };
    let hardware = rt.hardware_profile().await;
    let storage_settings = rt.storage_settings.read().await.clone();
    let storage = super::pc_storage_repo::storage_profile(&storage_settings);
    let node_data_root = rt.node_data_root.read().await.status_payload();
    let compute_plugin_bootstrap: crate::node_agent_compute_plugin_host::ComputePluginBootstrapStatus =
        rt.compute_plugin_bootstrap.status();
    let full_access_grant_count =
        match super::node_agent_full_access::current_grant_identity(&rt).await {
            Ok(identity) => rt.full_access_grants.list(&identity).await.len(),
            Err(_) => 0,
        };
    let active_cli_prompts = rt.active_cli_prompts.views_without_approvals().await;
    let active_cli_prompt_count = active_cli_prompts.len();
    let active_cli_prompt_task_ids = active_cli_prompts
        .iter()
        .map(|prompt| prompt.req_id.clone())
        .collect::<Vec<_>>();
    let recent_task_records = rt.task_journal.latest_records(20).unwrap_or_else(|error| {
        warn!("PC 任务 journal 读取失败，CLI 会话桥接状态降级为空摘要: {error}");
        Vec::new()
    });
    let mut sidecar_sessions = rt.cli_sidecars.all_sessions().unwrap_or_else(|error| {
        warn!("PC CLI sidecar registry 读取失败，CLI 会话桥接 sidecar 状态降级为空摘要: {error}");
        Vec::new()
    });
    sidecar_sessions.retain(|session| session.is_live_at(super::node_agent_cli_sidecar::now_ms()));
    sidecar_sessions.sort_by(|left, right| right.last_seen_at_ms.cmp(&left.last_seen_at_ms));
    sidecar_sessions.truncate(20);
    let mut live_task_ids = active_cli_prompts
        .iter()
        .map(|prompt| prompt.req_id.clone())
        .collect::<HashSet<_>>();
    live_task_ids.extend(
        sidecar_sessions
            .iter()
            .map(|session| session.task_id.clone()),
    );
    let active_task_runtime = recent_task_records
        .iter()
        .filter(|record| visible_active_runtime(&record.status, &record.req_id, &live_task_ids))
        .map(|record| {
            serde_json::json!({
                "task_id": record.req_id,
                "runtime": super::node_agent_task_journal::runtime_status_payload(Some(record)),
            })
        })
        .collect::<Vec<_>>();
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
    let update_recovery = rt
        .update_recovery
        .status_summary_payload(5)
        .unwrap_or_else(|error| {
            warn!(%error, "节点更新恢复状态读取失败，管理状态降级为空摘要");
            serde_json::json!({"protocol":"elon.node_update_recovery.v1","error":"unavailable"})
        });
    let mut payload = serde_json::json!({
        "version": env!("CARGO_PKG_VERSION"),
        "release_identity": super::node_agent_release_identity::current(),
        "local_admin_token_header": super::node_agent_local_admin::LOCAL_ADMIN_TOKEN_HEADER,
        "lifecycle_report_schema_version": 1,
        "task_journal_supported": true,
        "task_journal_schema_version": 1,
        "active_cli_prompt_count": active_cli_prompt_count,
        "active_task_runtime": active_task_runtime,
        "restart_recovery": super::node_agent_restart_drain::status_payload(),
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
        "update_recovery": update_recovery,
        "hardware": hardware,
        "storage": storage,
        "node_data_root": node_data_root,
        "compute_plugin_bootstrap": compute_plugin_bootstrap,
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
    if let Some(object) = payload.as_object_mut() {
        object.insert(
            "active_cli_prompt_task_ids".to_string(),
            serde_json::json!(active_cli_prompt_task_ids),
        );
        object.insert("connection".to_string(), connection);
        object.insert(
            "desktop_supervision".to_string(),
            super::node_agent_supervision_protocol::status_payload(),
        );
        object.insert(
            "desktop_review_broker".to_string(),
            rt.desktop_review_broker.status_payload(),
        );
        object.insert(
            "build_git_sha".to_string(),
            serde_json::json!(super::node_agent_release_identity::git_sha()),
        );
    }
    if super::node_agent_local_admin::can_expose_local_admin_token(&headers, &rt.cloud_http_url()) {
        if let Some(obj) = payload.as_object_mut() {
            obj.insert(
                "local_admin_token".to_string(),
                serde_json::json!(rt.local_admin_token()),
            );
        }
    }
    enforce_status_response_limit(&mut payload);
    Json(payload)
}

fn enforce_status_response_limit(payload: &mut serde_json::Value) {
    let original_bytes = serde_json::to_vec(payload)
        .map(|bytes| bytes.len())
        .unwrap_or(0);
    if original_bytes > ADMIN_STATUS_PAYLOAD_BUDGET {
        if let Some(object) = payload.as_object_mut() {
            object.insert("models".to_string(), serde_json::json!([]));
            object.insert("cli_tools".to_string(), serde_json::json!([]));
            if let Some(local_ai) = object
                .get_mut("local_ai")
                .and_then(serde_json::Value::as_object_mut)
            {
                local_ai.insert("models".to_string(), serde_json::json!([]));
                local_ai.insert("cli_tools".to_string(), serde_json::json!([]));
            }
        }
    }
    if serde_json::to_vec(payload)
        .map(|bytes| bytes.len())
        .unwrap_or(0)
        > ADMIN_STATUS_PAYLOAD_BUDGET
    {
        let essential = serde_json::json!({
            "version": payload.get("version").cloned(),
            "release_identity": payload.get("release_identity").cloned(),
            "build_git_sha": payload.get("build_git_sha").cloned(),
            "connected": payload.get("connected").cloned(),
            "logged_in": payload.get("logged_in").cloned(),
            "agent_id": payload.get("agent_id").cloned(),
            "owner_user_id": payload.get("owner_user_id").cloned(),
            "user_token_configured": payload.get("user_token_configured").cloned(),
            "active_cli_prompt_count": payload.get("active_cli_prompt_count").cloned(),
            "active_cli_prompt_task_ids": payload.get("active_cli_prompt_task_ids").cloned(),
            "active_task_runtime": payload.get("active_task_runtime").cloned(),
            "restart_recovery": payload.get("restart_recovery").cloned(),
            "update_recovery": payload.get("update_recovery").cloned(),
            "lifecycle": payload.get("lifecycle").cloned(),
            "compute_plugin_bootstrap": payload.get("compute_plugin_bootstrap").cloned(),
            "desktop_supervision": payload.get("desktop_supervision").cloned(),
            "desktop_review_broker": payload.get("desktop_review_broker").cloned(),
            "task_journal_supported": payload.get("task_journal_supported").cloned(),
            "task_journal_schema_version": payload.get("task_journal_schema_version").cloned(),
            "lifecycle_report_schema_version": payload.get("lifecycle_report_schema_version").cloned(),
            "local_admin_token_header": payload.get("local_admin_token_header").cloned(),
            "local_admin_token": payload.get("local_admin_token").cloned(),
            "compacted": true,
        });
        *payload = essential;
    }
    if serde_json::to_vec(payload)
        .map(|bytes| bytes.len())
        .unwrap_or(0)
        > ADMIN_STATUS_PAYLOAD_BUDGET
    {
        let blocker_count = payload
            .pointer("/update_recovery/install_gate/active_foreground_task_ids")
            .and_then(serde_json::Value::as_array)
            .map(Vec::len);
        *payload = serde_json::json!({
            "version": payload.get("version").cloned(),
            "release_identity": payload.get("release_identity").cloned(),
            "build_git_sha": payload.get("build_git_sha").cloned(),
            "connected": payload.get("connected").cloned(),
            "logged_in": payload.get("logged_in").cloned(),
            "agent_id": payload.get("agent_id").cloned(),
            "owner_user_id": payload.get("owner_user_id").cloned(),
            "user_token_configured": payload.get("user_token_configured").cloned(),
            "active_cli_prompt_count": payload.get("active_cli_prompt_count").cloned(),
            "update_blocker_count": blocker_count,
            "compute_plugin_bootstrap": payload.get("compute_plugin_bootstrap").cloned(),
            "desktop_supervision": payload.get("desktop_supervision").cloned(),
            "desktop_review_broker": payload.get("desktop_review_broker").cloned(),
            "task_journal_supported": payload.get("task_journal_supported").cloned(),
            "task_journal_schema_version": payload.get("task_journal_schema_version").cloned(),
            "lifecycle_report_schema_version": payload.get("lifecycle_report_schema_version").cloned(),
            "local_admin_token_header": payload.get("local_admin_token_header").cloned(),
            "local_admin_token": payload.get("local_admin_token").cloned(),
            "compacted": true,
        });
    }
    let actual_bytes = serde_json::to_vec(payload)
        .map(|bytes| bytes.len())
        .unwrap_or(0);
    if let Some(object) = payload.as_object_mut() {
        object.insert(
            "response_limits".to_string(),
            serde_json::json!({
                "schema": "elon.node_status_limits.v1",
                "max_bytes": ADMIN_STATUS_MAX_BYTES,
                "actual_bytes_before_metadata": actual_bytes,
                "original_bytes": original_bytes,
                "compacted": original_bytes > ADMIN_STATUS_PAYLOAD_BUDGET,
            }),
        );
    }
    if serde_json::to_vec(payload)
        .map(|bytes| bytes.len())
        .unwrap_or(ADMIN_STATUS_MAX_BYTES + 1)
        > ADMIN_STATUS_MAX_BYTES
    {
        *payload = serde_json::json!({
            "service": "elon-node-agent",
            "status": "compacted",
            "compute_plugin_bootstrap": payload.get("compute_plugin_bootstrap").cloned(),
            "response_limits": {
                "schema": "elon.node_status_limits.v1",
                "max_bytes": ADMIN_STATUS_MAX_BYTES,
                "original_bytes": original_bytes,
                "compacted": true,
            }
        });
    }
}
