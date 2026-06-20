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
    let has_current_user_only_result = results.iter().any(|result| {
        result.get("status").and_then(Value::as_str) == Some("ready")
            && result.get("visibility").and_then(Value::as_str) == Some("current_user_only")
    });

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
        "duration_ms": duration_ms,
        "answer_grounding": {
            "facts_allowed_from_ready_results": ready_tools.len() > 0,
            "requires_source_ids_when_available": true,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audit_counts_statuses_and_sources() {
        let results = vec![
            json!({
                "tool_name": "search_matches",
                "status": "ready",
                "source_ids": ["match-1", "match-2"],
                "visibility": "group_context"
            }),
            json!({
                "tool_name": "search_user_orders",
                "status": "skipped"
            }),
        ];
        let audit = execution_audit(
            "exec-1",
            &["search_matches", "search_user_orders"],
            &results,
            42,
        );

        assert_eq!(audit["ready_count"], 1);
        assert_eq!(audit["skipped_count"], 1);
        assert_eq!(audit["source_id_count"], 2);
        assert_eq!(execution_status(&results), "partial");
    }
}
