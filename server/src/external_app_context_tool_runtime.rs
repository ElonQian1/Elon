//! Runtime execution for external app context tools.

use chrono::{SecondsFormat, Utc};
use futures::future::join_all;
use serde_json::{json, Value};
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
use tracing::{info, warn};

use crate::{
    external_app_context_config::{
        fb2_base_url, fb2_context_token, tool_execution_enabled, tool_execution_timeout_secs,
        FB2_APP_ID, FB2_CONTEXT_HEADER,
    },
    external_app_context_response::compact_error,
    external_app_context_tool_audit::{execution_audit, execution_status},
    external_app_context_tool_planner::{plan_fb2_tools, PlannedTool},
    external_app_registry::external_group_by_group_id,
    types::AppState,
};

const FB2_TOOL_EXECUTE_PATH: &str = "/api/main-project/tools/execute";

pub(crate) async fn group_tool_results_for_chat(
    state: &Arc<AppState>,
    user_id: &str,
    group_id: &str,
    context: &Value,
    topic_hint: Option<&str>,
) -> Option<Value> {
    if !tool_execution_enabled() {
        return None;
    }

    let (app, group) = external_group_by_group_id(group_id)?;
    if app.id != FB2_APP_ID {
        return None;
    }

    let started_at = Instant::now();
    let execution_id = format!("fb2_exec_{}", uuid::Uuid::new_v4().simple());
    let planned_tools = plan_fb2_tools(context, topic_hint);
    if planned_tools.is_empty() {
        return None;
    }
    let planned_tool_names = planned_tools
        .iter()
        .map(|tool| tool.name)
        .collect::<Vec<_>>();

    let Some(base_url) = fb2_base_url() else {
        return Some(unavailable_execution(
            &execution_id,
            app.id,
            group.external_group_id,
            &planned_tool_names,
            planned_tools,
            "missing_fb2_base_url",
            started_at.elapsed().as_millis(),
        ));
    };
    let Some(token) = fb2_context_token() else {
        return Some(unavailable_execution(
            &execution_id,
            app.id,
            group.external_group_id,
            &planned_tool_names,
            planned_tools,
            "missing_fb2_context_token",
            started_at.elapsed().as_millis(),
        ));
    };

    let external_user_id = state
        .store
        .external_account_for_main_user(app.id, user_id)
        .map_err(|error| {
            warn!("fb2 tool external account lookup failed: {}", error);
            error
        })
        .ok()
        .flatten()
        .map(|account| account.external_user_id);

    let context_audit_id = context
        .get("context_audit_id")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned);

    let mut executable = Vec::new();
    let mut results = Vec::new();
    for plan in planned_tools {
        if plan.requires_external_user && external_user_id.is_none() {
            results.push(json!({
                "tool_name": plan.name,
                "request_id": Value::Null,
                "status": "skipped",
                "success": false,
                "error": "missing_external_user_id",
                "reason": plan.reason
            }));
            continue;
        }
        executable.push(plan);
    }

    let executions = executable.into_iter().map(|plan| {
        execute_fb2_tool(
            state,
            app.id,
            group.external_group_id,
            external_user_id.as_deref(),
            context_audit_id.as_deref(),
            &base_url,
            &token,
            plan,
        )
    });
    results.extend(join_all(executions).await);

    if results.is_empty() {
        return None;
    }

    let duration_ms = started_at.elapsed().as_millis();
    let audit = execution_audit(&execution_id, &planned_tool_names, &results, duration_ms);
    let status = execution_status(&results);
    let ready_count = audit
        .get("ready_count")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    info!(
        app_id = app.id,
        group_id,
        external_group_id = group.external_group_id,
        user_id,
        execution_id,
        status,
        ready_count,
        duration_ms,
        "external app tools executed"
    );

    Some(json!({
        "schema": "external_app.executed_tools.v1",
        "execution_id": execution_id,
        "app_id": app.id,
        "status": status,
        "executed_at": Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true),
        "group_id": group.external_group_id,
        "context_audit_id": context_audit_id,
        "results": results,
        "ready_count": ready_count,
        "audit": audit
    }))
}

async fn execute_fb2_tool(
    state: &Arc<AppState>,
    app_id: &str,
    external_group_id: &str,
    external_user_id: Option<&str>,
    context_audit_id: Option<&str>,
    base_url: &str,
    token: &str,
    plan: PlannedTool,
) -> Value {
    let request_id = format!("fb2_tool_{}", uuid::Uuid::new_v4().simple());
    let url = format!("{base_url}{FB2_TOOL_EXECUTE_PATH}");
    let payload = json!({
        "request_id": request_id,
        "tool_name": plan.name,
        "group_id": external_group_id,
        "external_user_id": external_user_id,
        "context_audit_id": context_audit_id,
        "arguments": plan.arguments,
        "reason": plan.reason,
        "limits": {
            "max_items": 12,
            "max_chars": 4000
        }
    });

    let response = state
        .http_client
        .post(&url)
        .header(FB2_CONTEXT_HEADER, token)
        .json(&payload)
        .timeout(Duration::from_secs(tool_execution_timeout_secs()))
        .send()
        .await;

    match response {
        Ok(response) => {
            normalize_tool_response(app_id, plan.name, plan.reason, &request_id, response).await
        }
        Err(error) => json!({
            "tool_name": plan.name,
            "request_id": request_id,
            "status": "unavailable",
            "success": false,
            "error": compact_error(&error.to_string()),
            "reason": plan.reason
        }),
    }
}

async fn normalize_tool_response(
    app_id: &str,
    tool_name: &str,
    reason: &str,
    request_id: &str,
    response: reqwest::Response,
) -> Value {
    let status_code = response.status().as_u16();
    let text = match response.text().await {
        Ok(text) => text,
        Err(error) => {
            return json!({
                "tool_name": tool_name,
                "request_id": request_id,
                "status": "unavailable",
                "success": false,
                "error": compact_error(&error.to_string()),
                "reason": reason
            });
        }
    };
    if !(200..300).contains(&status_code) {
        return json!({
            "tool_name": tool_name,
            "request_id": request_id,
            "status": "unavailable",
            "success": false,
            "status_code": status_code,
            "error": compact_error(&text),
            "reason": reason
        });
    }

    let parsed = match serde_json::from_str::<Value>(&text) {
        Ok(value) => value,
        Err(error) => {
            return json!({
                "tool_name": tool_name,
                "request_id": request_id,
                "status": "failed",
                "success": false,
                "error": format!("{} tool response JSON parse failed: {}", app_id, compact_error(&error.to_string())),
                "reason": reason
            });
        }
    };

    let success = parsed.get("success").and_then(Value::as_bool) == Some(true);
    json!({
        "tool_name": tool_name,
        "request_id": parsed.get("request_id").cloned().unwrap_or_else(|| json!(request_id)),
        "status": if success { "ready" } else { "failed" },
        "success": success,
        "data": parsed.get("data").cloned().unwrap_or(Value::Null),
        "error": parsed.get("error").cloned().unwrap_or(Value::Null),
        "generated_at": parsed.get("generated_at").cloned().unwrap_or(Value::Null),
        "source_ids": parsed.get("source_ids").cloned().unwrap_or(Value::Array(Vec::new())),
        "visibility": parsed.get("visibility").cloned().unwrap_or(Value::Null),
        "metrics": parsed.get("metrics").cloned().unwrap_or_else(|| json!({})),
        "reason": reason
    })
}

fn unavailable_execution(
    execution_id: &str,
    app_id: &str,
    external_group_id: &str,
    planned_tool_names: &[&str],
    planned_tools: Vec<PlannedTool>,
    reason: &str,
    duration_ms: u128,
) -> Value {
    let results = planned_tools
        .into_iter()
        .map(|tool| {
            json!({
                "tool_name": tool.name,
                "request_id": Value::Null,
                "status": "skipped",
                "success": false,
                "error": reason,
                "reason": tool.reason
            })
        })
        .collect::<Vec<_>>();
    let audit = execution_audit(execution_id, planned_tool_names, &results, duration_ms);

    json!({
        "schema": "external_app.executed_tools.v1",
        "execution_id": execution_id,
        "app_id": app_id,
        "status": "not_configured",
        "executed_at": Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true),
        "group_id": external_group_id,
        "results": results,
        "ready_count": 0,
        "audit": audit
    })
}
