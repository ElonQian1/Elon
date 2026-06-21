//! Live external app tool manifest fetches.

use chrono::{SecondsFormat, Utc};
use serde_json::{json, Value};
use std::{sync::Arc, time::Duration};

use crate::{
    external_app_context_config::{
        fb2_base_url, fb2_context_token, timeout_secs, FB2_APP_ID, FB2_CONTEXT_HEADER,
    },
    external_app_context_response::compact_error,
    types::AppState,
};

const FB2_TOOL_MANIFEST_PATH: &str = "/api/main-project/context/tool-manifest";
const MAX_TOOL_IDS: usize = 32;

pub(crate) async fn public_live_tool_manifest(state: &Arc<AppState>, app_id: &str) -> Value {
    match app_id {
        FB2_APP_ID => fetch_fb2_tool_manifest(state).await,
        _ => json!({
            "app_id": app_id,
            "schema": "external_app.live_tool_manifest.v1",
            "status": "not_configured",
            "source": Value::Null,
            "warnings": ["unknown_external_app_tool_manifest"]
        }),
    }
}

async fn fetch_fb2_tool_manifest(state: &Arc<AppState>) -> Value {
    let Some(base_url) = fb2_base_url() else {
        return manifest_status("not_configured", "missing_fb2_base_url");
    };
    let Some(token) = fb2_context_token() else {
        return manifest_status("not_configured", "missing_fb2_context_token");
    };

    let url = format!("{base_url}{FB2_TOOL_MANIFEST_PATH}");
    let response = state
        .http_client
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
}
