//! Live external app tool manifest fetches.

use chrono::{SecondsFormat, Utc};
use serde_json::{json, Value};
use std::{sync::Arc, time::Duration};

use crate::{
    external_app_context_config::{
        fb2_base_url, fb2_context_token, timeout_secs, FB2_APP_ID, FB2_CONTEXT_HEADER,
    },
    external_app_context_response::compact_error,
    external_app_context_tool_execution::{BB64A_ALLOWED_TOOLS, FB2_ALLOWED_TOOLS},
    external_app_http_client::fb2_direct_client,
    types::AppState,
};

const FB2_TOOL_MANIFEST_PATH: &str = "/api/main-project/context/tool-manifest";
const MAX_TOOL_IDS: usize = 32;

pub(crate) async fn public_live_tool_manifest(_state: &Arc<AppState>, app_id: &str) -> Value {
    match app_id {
        FB2_APP_ID => fetch_fb2_tool_manifest().await,
        "bb64a" => bb64a_static_tool_manifest(),
        _ => json!({
            "app_id": app_id,
            "schema": "external_app.live_tool_manifest.v1",
            "status": "not_configured",
            "source": Value::Null,
            "warnings": ["unknown_external_app_tool_manifest"]
        }),
    }
}

fn bb64a_static_tool_manifest() -> Value {
    json!({
        "app_id": "bb64a",
        "schema": "external_app.live_tool_manifest.v1",
        "status": "ready",
        "source": "bb64a:local-windows-mcp",
        "tool_count": BB64A_ALLOWED_TOOLS.len(),
        "tool_ids": BB64A_ALLOWED_TOOLS,
        "truncated": false,
        "context_pack_version": "bb64a-windows-diagnostic-pack-v1",
        "has_usage_policy": true,
        "has_tool_selection_policy": true,
        "main_project_tool_execution_policy": {
            "schema": "external_app.live_tool_execution_policy.v1",
            "chat_auto_executable_tool_ids": BB64A_ALLOWED_TOOLS,
            "manifest_only_tool_ids": [],
            "main_project_allowed_missing_tool_ids": [],
            "coverage_status": "ready",
            "rule": "BB64A tools are executed by the user's local Windows client MCP or node agent. Dangerous runtime tools are intentionally discoverable in the first version."
        },
        "secret_values_exposed": false
    })
}

async fn fetch_fb2_tool_manifest() -> Value {
    let Some(base_url) = fb2_base_url() else {
        return manifest_status("not_configured", "missing_fb2_base_url");
    };
    let Some(token) = fb2_context_token() else {
        return manifest_status("not_configured", "missing_fb2_context_token");
    };

    let url = format!("{base_url}{FB2_TOOL_MANIFEST_PATH}");
    let response = fb2_direct_client()
        .get(&url)
        .header(FB2_CONTEXT_HEADER, token)
        .timeout(Duration::from_secs(timeout_secs()))
        .send()
        .await;

    let response = match response {
        Ok(response) => response,
        Err(error) => {
            return manifest_error("unavailable", "request_failed", &error.to_string());
        }
    };
    let status_code = response.status().as_u16();
    let text = match response.text().await {
        Ok(text) => text,
        Err(error) => {
            return manifest_error("unavailable", "read_failed", &error.to_string());
        }
    };
    if !(200..300).contains(&status_code) {
        return json!({
            "app_id": FB2_APP_ID,
            "schema": "external_app.live_tool_manifest.v1",
            "status": "unavailable",
            "source": format!("fb2:{FB2_TOOL_MANIFEST_PATH}"),
            "status_code": status_code,
            "error_code": "http_error",
            "error": compact_error(&text),
            "secret_values_exposed": false
        });
    }

    let parsed = match serde_json::from_str::<Value>(&text) {
        Ok(value) => value,
        Err(error) => {
            return manifest_error("degraded", "json_parse_failed", &error.to_string());
        }
    };
    if parsed.get("success").and_then(Value::as_bool) != Some(true) {
        return manifest_error(
            "degraded",
            "fb2_manifest_failed",
            parsed
                .get("error")
                .and_then(Value::as_str)
                .unwrap_or("fb2 returned success=false"),
        );
    }

    let data = parsed.get("data").unwrap_or(&Value::Null);
    let tool_contract = data.get("tool_contract").unwrap_or(&Value::Null);
    let endpoints = tool_contract
        .get("endpoints")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let tool_ids = endpoints
        .iter()
        .filter_map(|endpoint| endpoint.get("id").and_then(Value::as_str))
        .filter_map(normalize_fb2_tool_id)
        .take(MAX_TOOL_IDS)
        .collect::<Vec<_>>();
    let tool_execution_policy = fb2_tool_execution_policy(&tool_ids);

    json!({
        "app_id": FB2_APP_ID,
        "schema": "external_app.live_tool_manifest.v1",
        "status": if tool_ids.is_empty() { "degraded" } else { "ready" },
        "source": format!("fb2:{FB2_TOOL_MANIFEST_PATH}"),
        "fetched_at": Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true),
        "tool_count": endpoints.len(),
        "tool_ids": tool_ids,
        "truncated": endpoints.len() > MAX_TOOL_IDS,
        "context_pack_version": data.get("context_pack_version").cloned().unwrap_or(Value::Null),
        "has_usage_policy": data.get("usage_policy").is_some(),
        "has_tool_selection_policy": tool_contract.get("tool_selection_policy").is_some(),
        "main_project_tool_execution_policy": tool_execution_policy,
        "secret_values_exposed": false
    })
}

fn manifest_status(status: &str, warning: &str) -> Value {
    json!({
        "app_id": FB2_APP_ID,
        "schema": "external_app.live_tool_manifest.v1",
        "status": status,
        "source": format!("fb2:{FB2_TOOL_MANIFEST_PATH}"),
        "warnings": [warning],
        "secret_values_exposed": false
    })
}

fn manifest_error(status: &str, error_code: &str, error: &str) -> Value {
    json!({
        "app_id": FB2_APP_ID,
        "schema": "external_app.live_tool_manifest.v1",
        "status": status,
        "source": format!("fb2:{FB2_TOOL_MANIFEST_PATH}"),
        "error_code": error_code,
        "error": compact_error(error),
        "secret_values_exposed": false
    })
}

fn normalize_fb2_tool_id(value: &str) -> Option<String> {
    let value = value.trim();
    if value.is_empty() {
        return None;
    }
    Some(value.strip_prefix("fb2.").unwrap_or(value).to_string())
}

fn fb2_tool_execution_policy(live_tool_ids: &[String]) -> Value {
    let auto_executable_tool_ids = FB2_ALLOWED_TOOLS
        .iter()
        .filter(|tool| live_tool_ids.iter().any(|live| live == **tool))
        .map(|tool| (*tool).to_string())
        .collect::<Vec<_>>();
    let manifest_only_tool_ids = live_tool_ids
        .iter()
        .filter(|tool| {
            !FB2_ALLOWED_TOOLS
                .iter()
                .any(|allowed| allowed == &tool.as_str())
        })
        .cloned()
        .collect::<Vec<_>>();
    let allowed_missing_tool_ids = FB2_ALLOWED_TOOLS
        .iter()
        .filter(|tool| !live_tool_ids.iter().any(|live| live == **tool))
        .map(|tool| (*tool).to_string())
        .collect::<Vec<_>>();
    let coverage_status = if allowed_missing_tool_ids.is_empty() {
        "ready"
    } else {
        "degraded"
    };

    json!({
        "schema": "external_app.live_tool_execution_policy.v1",
        "chat_auto_executable_tool_ids": auto_executable_tool_ids,
        "manifest_only_tool_ids": manifest_only_tool_ids,
        "main_project_allowed_missing_tool_ids": allowed_missing_tool_ids,
        "coverage_status": coverage_status,
        "rule": "Only chat_auto_executable_tool_ids may be planned and executed by main-project chat AI. manifest_only_tool_ids are discovery/callback/direct-context endpoints until explicitly allowlisted and grounded."
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_prefixed_fb2_tool_ids() {
        assert_eq!(
            normalize_fb2_tool_id("fb2.search_matches").as_deref(),
            Some("search_matches")
        );
        assert_eq!(
            normalize_fb2_tool_id(" opinion_result_reviews ").as_deref(),
            Some("opinion_result_reviews")
        );
        assert!(normalize_fb2_tool_id("  ").is_none());
    }

    #[test]
    fn manifest_status_never_exposes_secrets() {
        let status = manifest_status("not_configured", "missing_token");

        assert_eq!(status["secret_values_exposed"], false);
        assert!(status.get("FB2_MAIN_PROJECT_SHARED_SECRET").is_none());
    }

    #[test]
    fn live_manifest_marks_manifest_only_tools_as_non_executable() {
        let policy = fb2_tool_execution_policy(&[
            "search_matches".to_string(),
            "record_context_feedback".to_string(),
            "context_quality_summary".to_string(),
        ]);

        assert_eq!(
            policy["schema"],
            "external_app.live_tool_execution_policy.v1"
        );
        assert_eq!(
            policy["chat_auto_executable_tool_ids"]
                .as_array()
                .unwrap()
                .first()
                .and_then(Value::as_str),
            Some("search_matches")
        );
        assert!(policy["manifest_only_tool_ids"]
            .as_array()
            .unwrap()
            .contains(&json!("record_context_feedback")));
        assert!(policy["manifest_only_tool_ids"]
            .as_array()
            .unwrap()
            .contains(&json!("context_quality_summary")));
        assert!(policy["main_project_allowed_missing_tool_ids"]
            .as_array()
            .unwrap()
            .contains(&json!("get_match_detail")));
        assert_eq!(policy["coverage_status"], "degraded");
    }

    #[test]
    fn bb64a_static_manifest_exposes_local_mcp_tools() {
        let manifest = bb64a_static_tool_manifest();
        assert_eq!(manifest["status"], "ready");
        assert_eq!(manifest["source"], "bb64a:local-windows-mcp");
        assert!(manifest["tool_ids"]
            .as_array()
            .unwrap()
            .contains(&json!("bb64a_doctor")));
        assert!(
            manifest["main_project_tool_execution_policy"]["chat_auto_executable_tool_ids"]
                .as_array()
                .unwrap()
                .contains(&json!("close_all_proxies"))
        );
    }
}
