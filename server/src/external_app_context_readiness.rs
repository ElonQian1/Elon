//! server/src/external_app_context_readiness.rs
//! Readiness guidance and live preflight checks for external app context packs.

use chrono::{SecondsFormat, Utc};
use serde_json::{json, Value};
use std::time::Duration;

use crate::{
    external_app_context_config::{
        fb2_base_url, fb2_context_token, timeout_secs, FB2_APP_ID, FB2_CONTEXT_HEADER,
    },
    external_app_context_response::compact_error,
    external_app_http_client::fb2_direct_client,
};

const FB2_CONTEXT_READINESS_PATH: &str = "/api/main-project/context/readiness";

pub(crate) async fn live_context_readiness(app_id: &str) -> Value {
    match app_id {
        FB2_APP_ID => fetch_fb2_context_readiness().await,
        _ => json!({
            "app_id": app_id,
            "schema": "external_app.live_context_readiness.v1",
            "status": "not_configured",
            "source": Value::Null,
            "warnings": ["unknown_external_app_readiness"]
        }),
    }
}

pub(crate) fn public_context_readiness_guidance(app_id: &str) -> Option<Value> {
    match app_id {
        "fb2" => Some(json!({
            "app_id": "fb2",
            "schema": "fb2.context_readiness.v1",
            "purpose": "给 fb2 代理和主项目代理做自动接入自检，判断业务上下文是否足够支撑 AI 回答。",
            "required_response_fields": [
                "context_pack_version",
                "generated_at",
                "context_pack",
                "matches",
                "user_orders",
                "group_messages",
                "citation_sources",
                "tool_contract",
                "metrics"
            ],
            "recommended_response_fields": [
                "context_audit_id",
                "platform_order_summary",
                "usage_policy",
                "answer_policy"
            ],
            "main_project_prompt_metadata": [
                "usage_policy",
                "answer_policy",
                "answer_rules",
                "context_quality",
                "context_budget",
                "external_metrics",
                "context_audit_id",
                "citation_sources",
                "tool_contract",
                "executed_external_app_tools"
            ],
            "readiness_levels": [
                {
                    "status": "blocked",
                    "conditions": [
                        "context_pack missing or empty",
                        "generated_at missing and user asks time-sensitive question",
                        "metrics.budget_status=empty",
                        "requested user order analysis but no current_user_only order source"
                    ],
                    "ai_behavior": "必须说明数据不足，不能预测比赛或剖析订单。"
                },
                {
                    "status": "degraded",
                    "conditions": [
                        "tool_contract missing or partial",
                        "matches empty but question does not depend on concrete matches",
                        "context_pack too large and was trimmed",
                        "stale_source_count > 0"
                    ],
                    "ai_behavior": "可以回答，但必须提示缺口、新鲜度或裁剪风险。"
                },
                {
                    "status": "ready",
                    "conditions": [
                        "context_pack present",
                        "generated_at present",
                        "source ids present for relevant claims",
                        "tool_contract declares recommended tools or enough sources are already present"
                    ],
                    "ai_behavior": "可以基于 fb2 上下文回答，并引用 match_id/order_id/message_id。"
                }
            ],
            "automated_checks": [
                {
                    "name": "has_context_pack",
                    "field": "context_pack",
                    "pass_when": "non_empty_string",
                    "failure_warning": "missing_context_pack"
                },
                {
                    "name": "has_generated_at",
                    "field": "generated_at",
                    "pass_when": "non_empty_iso8601_string",
                    "failure_warning": "missing_generated_at"
                },
                {
                    "name": "has_source_ids",
                    "fields": ["matches[].id", "user_orders[].order_id", "group_messages[].message_id"],
                    "pass_when": "present_for_claimed_sources",
                    "failure_warning": "missing_source_ids"
                },
                {
                    "name": "tool_readiness",
                    "field": "tool_contract.tools",
                    "pass_when": "contains recommended tools or context_pack has enough evidence",
                    "failure_warning": "missing_or_partial_tool_contract"
                },
                {
                    "name": "answer_policy_available",
                    "field": "answer_policy",
                    "pass_when": "provided_by_fb2_or_defaulted_by_main_project",
                    "failure_warning": "none"
                },
                {
                    "name": "answer_rules_available",
                    "field": "answer_policy_contract.prompt_answer_rules",
                    "pass_when": "provided_by_main_project_context_contract",
                    "failure_warning": "none"
                }
            ]
        })),
        _ => None,
    }
}

async fn fetch_fb2_context_readiness() -> Value {
    let Some(base_url) = fb2_base_url() else {
        return readiness_status("not_configured", "missing_fb2_base_url");
    };
    let Some(token) = fb2_context_token() else {
        return readiness_status("not_configured", "missing_fb2_context_token");
    };

    let url = format!("{base_url}{FB2_CONTEXT_READINESS_PATH}");
    let response = fb2_direct_client()
        .get(&url)
        .header(FB2_CONTEXT_HEADER, token)
        .timeout(Duration::from_secs(timeout_secs()))
        .send()
        .await;

    let response = match response {
        Ok(response) => response,
        Err(error) => {
            return readiness_error("unavailable", "request_failed", &error.to_string());
        }
    };
    let status_code = response.status().as_u16();
    let text = match response.text().await {
        Ok(text) => text,
        Err(error) => {
            return readiness_error("unavailable", "read_failed", &error.to_string());
        }
    };
    if !(200..300).contains(&status_code) {
        return json!({
            "app_id": FB2_APP_ID,
            "schema": "external_app.live_context_readiness.v1",
            "status": "unavailable",
            "source": format!("fb2:{FB2_CONTEXT_READINESS_PATH}"),
            "status_code": status_code,
            "error_code": "http_error",
            "error": compact_error(&text),
            "secret_values_exposed": false
        });
    }

    let parsed = match serde_json::from_str::<Value>(&text) {
        Ok(value) => value,
        Err(error) => {
            return readiness_error("degraded", "json_parse_failed", &error.to_string());
        }
    };
    if parsed.get("success").and_then(Value::as_bool) != Some(true) {
        return readiness_error(
            "degraded",
            "fb2_readiness_failed",
            parsed
                .get("error")
                .and_then(Value::as_str)
                .unwrap_or("fb2 returned success=false"),
        );
    }

    let data = parsed.get("data").unwrap_or(&Value::Null);
    normalized_live_readiness(data)
}

fn normalized_live_readiness(data: &Value) -> Value {
    let status = readiness_value(data).unwrap_or("degraded");
    json!({
        "app_id": FB2_APP_ID,
        "schema": "external_app.live_context_readiness.v1",
        "status": status,
        "source": format!("fb2:{FB2_CONTEXT_READINESS_PATH}"),
        "fetched_at": Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true),
        "context_pack_version": data.get("context_pack_version").cloned().unwrap_or(Value::Null),
        "checks": data.get("checks").cloned().unwrap_or(Value::Null),
        "warnings": readiness_warnings(data, status),
        "secret_values_exposed": false
    })
}

fn readiness_value(data: &Value) -> Option<&str> {
    [
        data.get("status"),
        data.get("readiness_status"),
        data.get("context_status"),
        data.get("context_readiness")
            .and_then(|value| value.get("status")),
    ]
    .into_iter()
    .flatten()
    .filter_map(Value::as_str)
    .map(str::trim)
    .find(|value| matches!(*value, "ready" | "degraded" | "blocked" | "unavailable"))
}

fn readiness_warnings(data: &Value, status: &str) -> Vec<Value> {
    // fb2 readiness 是 AI 回答前的前置门禁；主项目只记录摘要，避免泄露服务令牌或业务明细。
    let mut warnings = data
        .get("warnings")
        .and_then(Value::as_array)
        .map(|values| values.iter().take(12).cloned().collect::<Vec<_>>())
        .unwrap_or_default();
    if status != "ready" && warnings.is_empty() {
        warnings.push(json!(format!("fb2_readiness_{status}")));
    }
    warnings
}

fn readiness_status(status: &str, warning: &str) -> Value {
    json!({
        "app_id": FB2_APP_ID,
        "schema": "external_app.live_context_readiness.v1",
        "status": status,
        "source": format!("fb2:{FB2_CONTEXT_READINESS_PATH}"),
        "warnings": [warning],
        "secret_values_exposed": false
    })
}

fn readiness_error(status: &str, error_code: &str, error: &str) -> Value {
    json!({
        "app_id": FB2_APP_ID,
        "schema": "external_app.live_context_readiness.v1",
        "status": status,
        "source": format!("fb2:{FB2_CONTEXT_READINESS_PATH}"),
        "error_code": error_code,
        "error": compact_error(error),
        "secret_values_exposed": false
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exposes_public_context_readiness_guidance() {
        let guidance = public_context_readiness_guidance("fb2").unwrap();
        assert_eq!(guidance["schema"], "fb2.context_readiness.v1");
        let metadata = guidance["main_project_prompt_metadata"].as_array().unwrap();
        assert!(metadata.contains(&json!("answer_policy")));
        assert!(metadata.contains(&json!("answer_rules")));
        assert!(metadata.contains(&json!("citation_sources")));
        assert!(metadata.contains(&json!("executed_external_app_tools")));
        assert!(guidance["required_response_fields"]
            .as_array()
            .unwrap()
            .contains(&json!("citation_sources")));
        assert!(guidance["automated_checks"]
            .as_array()
            .unwrap()
            .iter()
            .any(|check| check["name"] == "has_source_ids"));
        assert!(public_context_readiness_guidance("unknown").is_none());
    }

    #[test]
    fn normalizes_nested_fb2_readiness_status() {
        let readiness = normalized_live_readiness(&json!({
            "context_pack_version": "fb2-chat-pack-v1",
            "context_readiness": {"status": "blocked"},
            "checks": [{"name": "has_orders", "status": "failed"}]
        }));

        assert_eq!(
            readiness["schema"],
            "external_app.live_context_readiness.v1"
        );
        assert_eq!(readiness["status"], "blocked");
        assert_eq!(readiness["secret_values_exposed"], false);
        assert!(readiness["warnings"]
            .as_array()
            .unwrap()
            .contains(&json!("fb2_readiness_blocked")));
    }

    #[test]
    fn readiness_status_never_exposes_secrets() {
        let status = readiness_status("not_configured", "missing_token");

        assert_eq!(status["secret_values_exposed"], false);
        assert!(status.get("FB2_MAIN_PROJECT_SHARED_SECRET").is_none());
    }
}
