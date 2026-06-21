//! Grounding metadata for normalized external app tool results.

use serde_json::{json, Value};

pub(crate) fn normalize_parsed_tool_result(
    tool_name: &str,
    reason: &str,
    request_id: &str,
    parsed: &Value,
) -> Value {
    let success = parsed.get("success").and_then(Value::as_bool) == Some(true);
    let source_ids = normalized_source_ids(parsed.get("source_ids"));
    let visibility = parsed
        .get("visibility")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned);
    let grounding = tool_result_grounding(tool_name, success, &source_ids, visibility.as_deref());

    json!({
        "tool_name": tool_name,
        "request_id": parsed.get("request_id").cloned().unwrap_or_else(|| json!(request_id)),
        "status": if success { "ready" } else { "failed" },
        "success": success,
        "data": parsed.get("data").cloned().unwrap_or(Value::Null),
        "error": parsed.get("error").cloned().unwrap_or(Value::Null),
        "generated_at": parsed.get("generated_at").cloned().unwrap_or(Value::Null),
        "source_ids": source_ids,
        "visibility": visibility.map(Value::String).unwrap_or(Value::Null),
        "metrics": parsed.get("metrics").cloned().unwrap_or_else(|| json!({})),
        "grounding": grounding,
        "reason": reason
    })
}

fn tool_result_grounding(
    tool_name: &str,
    success: bool,
    source_ids: &[String],
    visibility: Option<&str>,
) -> Value {
    let expected_visibility = expected_visibility(tool_name);
    let source_ids_required = source_ids_required(tool_name);
    let mut warnings = Vec::new();

    if success {
        match (visibility, expected_visibility) {
            (None, Some(_)) => warnings.push("missing_visibility"),
            (Some(actual), Some(expected)) if actual != expected => {
                warnings.push("visibility_mismatch")
            }
            _ => {}
        }
        if source_ids_required && source_ids.is_empty() {
            warnings.push("missing_source_ids");
        }
    }

    let status = if !success {
        "unavailable"
    } else if warnings.contains(&"visibility_mismatch") {
        "unsafe"
    } else if warnings.is_empty() {
        "grounded"
    } else {
        "weak"
    };

    json!({
        "schema": "external_app.tool_result_grounding.v1",
        "status": status,
        "source_id_count": source_ids.len(),
        "source_ids_required": source_ids_required,
        "expected_visibility": expected_visibility,
        "actual_visibility": visibility,
        "warnings": warnings,
        "facts_allowed": matches!(status, "grounded" | "weak"),
        "requires_caveat": status == "weak"
    })
}

fn normalized_source_ids(value: Option<&Value>) -> Vec<String> {
    match value {
        Some(Value::Array(values)) => values
            .iter()
            .filter_map(source_id_value)
            .filter(|value| !value.is_empty())
            .collect(),
        Some(value) => source_id_value(value)
            .filter(|value| !value.is_empty())
            .into_iter()
            .collect(),
        None => Vec::new(),
    }
}

fn source_id_value(value: &Value) -> Option<String> {
    match value {
        Value::String(value) => Some(value.trim().to_string()),
        Value::Number(value) => Some(value.to_string()),
        _ => None,
    }
}

fn expected_visibility(tool_name: &str) -> Option<&'static str> {
    match tool_name {
        "search_matches" | "get_match_detail" | "search_group_opinions" => Some("group_context"),
        "search_user_orders" | "get_order_detail" => Some("current_user_only"),
        "platform_orders" => Some("privileged_summary"),
        "get_context_audit" => Some("audit_metadata_only"),
        "context_audit_summary" => Some("audit_metrics_only"),
        _ => None,
    }
}

fn source_ids_required(tool_name: &str) -> bool {
    matches!(
        tool_name,
        "search_matches"
            | "get_match_detail"
            | "search_user_orders"
            | "get_order_detail"
            | "search_group_opinions"
            | "platform_orders"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grounded_when_visibility_and_source_ids_match() {
        let result = normalize_parsed_tool_result(
            "search_matches",
            "reason",
            "req-1",
            &json!({
                "success": true,
                "source_ids": ["match-1"],
                "visibility": "group_context"
            }),
        );

        assert_eq!(result["grounding"]["status"], "grounded");
        assert_eq!(result["grounding"]["source_id_count"], 1);
        assert_eq!(result["grounding"]["facts_allowed"], true);
    }

    #[test]
    fn weak_when_source_ids_are_missing() {
        let result = normalize_parsed_tool_result(
            "search_user_orders",
            "reason",
            "req-1",
            &json!({
                "success": true,
                "visibility": "current_user_only"
            }),
        );

        assert_eq!(result["grounding"]["status"], "weak");
        assert_eq!(result["grounding"]["requires_caveat"], true);
        assert!(result["grounding"]["warnings"]
            .as_array()
            .unwrap()
            .contains(&json!("missing_source_ids")));
    }

    #[test]
    fn unsafe_when_visibility_is_wrong() {
        let result = normalize_parsed_tool_result(
            "search_user_orders",
            "reason",
            "req-1",
            &json!({
                "success": true,
                "source_ids": ["order-1"],
                "visibility": "group_context"
            }),
        );

        assert_eq!(result["grounding"]["status"], "unsafe");
        assert_eq!(result["grounding"]["facts_allowed"], false);
    }

    #[test]
    fn platform_orders_require_privileged_visibility_and_source_ids() {
        let result = normalize_parsed_tool_result(
            "platform_orders",
            "reason",
            "req-1",
            &json!({
                "success": true,
                "source_ids": ["platform_order_summary:2026-06-21:all"],
                "visibility": "privileged_summary"
            }),
        );

        assert_eq!(result["grounding"]["status"], "grounded");
        assert_eq!(
            result["grounding"]["expected_visibility"].as_str(),
            Some("privileged_summary")
        );
    }
}
