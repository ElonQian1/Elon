//! Bounded node status responses, shared by the live endpoint and contract tests.

const ADMIN_STATUS_MAX_BYTES: usize = 64 * 1024;
const ADMIN_STATUS_PAYLOAD_BUDGET: usize = ADMIN_STATUS_MAX_BYTES - 1024;

pub(super) fn enforce_status_response_limit(payload: &mut serde_json::Value) {
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

#[cfg(test)]
mod tests {
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
