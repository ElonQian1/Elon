//! Audit summary for external app tool execution.

use serde_json::{json, Value};

pub(crate) fn execution_audit(
    execution_id: &str,
    planned_tool_names: &[&str],
    results: &[Value],
    duration_ms: u128,
) -> Value {
    let ready_tools = tools_with_status(results, "ready");
    let skipped_tools = tools_with_status(results, "skipped");
    let failed_tools = tools_with_status(results, "failed");
    let unavailable_tools = tools_with_status(results, "unavailable");
    let source_id_count = results
        .iter()
        .filter(|result| result.get("status").and_then(Value::as_str) == Some("ready"))
        .filter_map(|result| result.get("source_ids").and_then(Value::as_array))
        .map(Vec::len)
        .sum::<usize>();
    let has_current_user_only_result = results.iter().any(result_has_current_user_order_data);
    let grounded_result_count = count_grounding_status(results, "grounded");
    let weak_result_count = count_grounding_status(results, "weak");
    let unsafe_result_count = count_grounding_status(results, "unsafe");
    let grounding_warnings = grounding_warnings(results);

    json!({
        "schema": "external_app.tool_execution_audit.v1",
        "execution_id": execution_id,
        "planned_count": planned_tool_names.len(),
        "result_count": results.len(),
        "ready_count": ready_tools.len(),
        "skipped_count": skipped_tools.len(),
        "failed_count": failed_tools.len(),
        "unavailable_count": unavailable_tools.len(),
        "planned_tools": planned_tool_names,
        "ready_tools": ready_tools,
        "skipped_tools": skipped_tools,
        "failed_tools": failed_tools,
        "unavailable_tools": unavailable_tools,
        "source_id_count": source_id_count,
        "has_current_user_only_result": has_current_user_only_result,
        "grounded_result_count": grounded_result_count,
        "weak_result_count": weak_result_count,
        "unsafe_result_count": unsafe_result_count,
        "grounding_warnings": grounding_warnings,
        "duration_ms": duration_ms,
        "answer_grounding": {
            "facts_allowed_from_ready_results": ready_tools.len() > 0,
            "facts_allowed_from_grounded_results": grounded_result_count > 0,
            "requires_source_ids_when_available": true,
            "weak_results_require_caveat": weak_result_count > 0,
            "unsafe_results_must_not_ground_facts": unsafe_result_count > 0,
            "current_user_only_data_present": has_current_user_only_result
        }
    })
}

pub(crate) fn execution_status(results: &[Value]) -> &'static str {
    let ready_count = count_status(results, "ready");
    if ready_count == results.len() && !results.is_empty() {
        "ready"
    } else if ready_count > 0 {
        "partial"
    } else if results
        .iter()
        .all(|result| result.get("status").and_then(Value::as_str) == Some("skipped"))
    {
        "skipped"
    } else {
        "unavailable"
    }
}

fn count_status(results: &[Value], status: &str) -> usize {
    results
        .iter()
        .filter(|result| result.get("status").and_then(Value::as_str) == Some(status))
        .count()
}

fn tools_with_status(results: &[Value], status: &str) -> Vec<String> {
    results
        .iter()
        .filter(|result| result.get("status").and_then(Value::as_str) == Some(status))
        .filter_map(|result| {
            result
                .get("tool_name")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned)
        })
        .collect()
}

fn count_grounding_status(results: &[Value], status: &str) -> usize {
    results
        .iter()
        .filter(|result| {
            result
                .get("grounding")
                .and_then(|grounding| grounding.get("status"))
                .and_then(Value::as_str)
                == Some(status)
        })
        .count()
}

fn grounding_warnings(results: &[Value]) -> Vec<String> {
    let mut warnings = Vec::new();
    for warning in results
        .iter()
        .filter_map(|result| result.get("grounding"))
        .filter_map(|grounding| grounding.get("warnings"))
        .filter_map(Value::as_array)
        .flat_map(|items| items.iter().filter_map(Value::as_str))
    {
        if !warnings.iter().any(|existing| existing == warning) {
            warnings.push(warning.to_string());
        }
    }
    warnings
}

fn result_has_current_user_order_data(result: &Value) -> bool {
    if result.get("status").and_then(Value::as_str) != Some("ready") {
        return false;
    }
    match result.get("visibility").and_then(Value::as_str) {
        Some("current_user_only") => true,
        Some("match_focused_brief") => result
            .get("data")
            .and_then(|data| data.get("user_orders"))
            .and_then(Value::as_array)
            .map(|orders| !orders.is_empty())
            .unwrap_or(false),
        _ => false,
    }
}

#[cfg(test)]
#[path = "external_app_context_tool_audit_tests.rs"]
mod tests;
