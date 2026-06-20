//! Runtime execution for external app context tools.

use chrono::{SecondsFormat, Utc};
use futures::future::join_all;
use serde_json::{json, Value};
use std::{sync::Arc, time::Duration};
use tracing::warn;

use crate::{
    external_app_context_config::{
        fb2_base_url, fb2_context_token, infer_lottery_type, tool_execution_enabled,
        tool_execution_timeout_secs, FB2_APP_ID, FB2_CONTEXT_HEADER,
    },
    external_app_context_response::compact_error,
    external_app_registry::external_group_by_group_id,
    types::AppState,
};

const FB2_TOOL_EXECUTE_PATH: &str = "/api/main-project/tools/execute";
const MAX_PROMPT_TOOL_JSON_CHARS: usize = 6_000;

#[derive(Clone)]
struct PlannedTool {
    name: &'static str,
    reason: &'static str,
    arguments: Value,
    requires_external_user: bool,
}

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

    let planned_tools = plan_fb2_tools(context, topic_hint);
    if planned_tools.is_empty() {
        return None;
    }

    let Some(base_url) = fb2_base_url() else {
        return Some(unavailable_execution(
            app.id,
            group.external_group_id,
            planned_tools,
            "missing_fb2_base_url",
        ));
    };
    let Some(token) = fb2_context_token() else {
        return Some(unavailable_execution(
            app.id,
            group.external_group_id,
            planned_tools,
            "missing_fb2_context_token",
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

    let ready_count = results
        .iter()
        .filter(|result| result.get("status").and_then(Value::as_str) == Some("ready"))
        .count();
    let status = if ready_count == results.len() {
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
    };

    Some(json!({
        "schema": "external_app.executed_tools.v1",
        "app_id": app.id,
        "status": status,
        "executed_at": Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true),
        "group_id": group.external_group_id,
        "context_audit_id": context_audit_id,
        "results": results,
        "ready_count": ready_count
    }))
}

pub(crate) fn prompt_executed_tools_block(execution: Option<&Value>) -> String {
    let Some(execution) = execution else {
        return String::new();
    };
    let status = execution
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let app_id = execution
        .get("app_id")
        .and_then(Value::as_str)
        .unwrap_or("external");
    let executed_at = execution
        .get("executed_at")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let mut body = serde_json::to_string_pretty(execution).unwrap_or_else(|_| "{}".to_string());
    if body.chars().count() > MAX_PROMPT_TOOL_JSON_CHARS {
        body = body
            .chars()
            .take(MAX_PROMPT_TOOL_JSON_CHARS)
            .collect::<String>();
        body.push_str("\n... [external app tool results truncated]");
    }

    format!(
        "按需执行的外部项目工具：\n\
         <executed_external_app_tools app_id=\"{app_id}\" status=\"{status}\" executed_at=\"{executed_at}\">\n\
         {body}\n\
         <tool_result_rules>\n\
         - 只有 status=ready 且 success=true 的单项结果可以当作已查询事实引用。\n\
         - skipped、failed、unavailable 结果只能作为数据缺口说明，不能编造成比赛、赔率、订单或群友观点事实。\n\
         - 引用工具结果时优先带 match_id、order_id、ticket_id、message_id 或 context_audit_id。\n\
         - 当前用户订单只允许基于 current_user_only 结果分析，不能推断或暴露其他用户明细。\n\
         </tool_result_rules>\n\
         </executed_external_app_tools>"
    )
}

fn plan_fb2_tools(context: &Value, topic_hint: Option<&str>) -> Vec<PlannedTool> {
    let query = topic_hint.unwrap_or("").trim();
    let mut plans = Vec::new();

    if should_search_matches(context, query) {
        let mut arguments = json!({
            "query": query,
            "date": "today",
            "include_odds": true
        });
        if let Some(lottery_type) = infer_lottery_type(Some(query)) {
            arguments["lottery_type"] = json!(lottery_type);
        }
        plans.push(PlannedTool {
            name: "search_matches",
            reason: "用户在 fb2 群聊中询问比赛、赔率、预测或今日场次，需要补充可引用比赛候选。",
            arguments,
            requires_external_user: false,
        });
    }

    if contains_any(
        query,
        &[
            "订单",
            "我的票",
            "我的单",
            "票据",
            "方案",
            "下单",
            "已买",
            "串关",
        ],
    ) {
        plans.push(PlannedTool {
            name: "search_user_orders",
            reason: "用户提到自己的订单、票据或方案，需要只查询当前用户可见数据。",
            arguments: json!({
                "query": query,
                "scope": "current_user"
            }),
            requires_external_user: true,
        });
    }

    if contains_any(
        query,
        &[
            "群友", "大家", "观点", "讨论", "分歧", "群里", "建议", "采纳",
        ],
    ) {
        plans.push(PlannedTool {
            name: "search_group_opinions",
            reason: "用户要求总结群友观点、讨论分歧或采纳建议，需要检索群聊观点来源。",
            arguments: json!({
                "query": query
            }),
            requires_external_user: false,
        });
    }

    if should_query_audit(context) {
        if let Some(audit_id) = context.get("context_audit_id").and_then(Value::as_str) {
            plans.push(PlannedTool {
                name: "get_context_audit",
                reason: "当前 fb2 context pack 质量或预算状态有风险，需要回查来源和裁剪指标。",
                arguments: json!({
                    "context_audit_id": audit_id
                }),
                requires_external_user: false,
            });
        }
    }

    plans.truncate(3);
    plans
}

fn should_search_matches(context: &Value, query: &str) -> bool {
    contains_any(
        query,
        &[
            "今天",
            "比赛",
            "赛事",
            "场次",
            "赔率",
            "预测",
            "推荐",
            "竞彩",
            "北单",
            "足球",
            "篮球",
            "NBA",
            "让球",
            "大小分",
            "胜负",
        ],
    ) || context_has_warning(context, "empty_matches")
        || context_has_warning(context, "fb2_budget_too_large")
}

fn should_query_audit(context: &Value) -> bool {
    context_has_warning(context, "fb2_budget_empty")
        || context_has_warning(context, "fb2_budget_too_large")
        || context_has_warning(context, "missing_context_pack")
}

fn context_has_warning(context: &Value, warning: &str) -> bool {
    context
        .get("context_quality")
        .and_then(|quality| quality.get("warnings"))
        .and_then(Value::as_array)
        .map(|warnings| warnings.iter().any(|value| value.as_str() == Some(warning)))
        .unwrap_or(false)
}

fn contains_any(text: &str, needles: &[&str]) -> bool {
    if text.is_empty() {
        return false;
    }
    let lower = text.to_ascii_lowercase();
    needles.iter().any(|needle| {
        let needle_lower = needle.to_ascii_lowercase();
        lower.contains(&needle_lower) || text.contains(needle)
    })
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
        Ok(response) => normalize_tool_response(app_id, plan.name, plan.reason, response).await,
        Err(error) => json!({
            "tool_name": plan.name,
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
    response: reqwest::Response,
) -> Value {
    let status_code = response.status().as_u16();
    let text = match response.text().await {
        Ok(text) => text,
        Err(error) => {
            return json!({
                "tool_name": tool_name,
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
    app_id: &str,
    external_group_id: &str,
    planned_tools: Vec<PlannedTool>,
    reason: &str,
) -> Value {
    json!({
        "schema": "external_app.executed_tools.v1",
        "app_id": app_id,
        "status": "not_configured",
        "executed_at": Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true),
        "group_id": external_group_id,
        "results": planned_tools.into_iter().map(|tool| json!({
            "tool_name": tool.name,
            "status": "skipped",
            "success": false,
            "error": reason,
            "reason": tool.reason
        })).collect::<Vec<_>>(),
        "ready_count": 0
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plans_match_order_and_opinion_tools_from_user_request() {
        let plans = plan_fb2_tools(
            &json!({
                "context_quality": {"warnings": []}
            }),
            Some("今天比赛怎么预测？顺便看看我的票和群友观点"),
        );

        let names = plans.iter().map(|plan| plan.name).collect::<Vec<_>>();
        assert_eq!(
            names,
            vec![
                "search_matches",
                "search_user_orders",
                "search_group_opinions"
            ]
        );
        assert!(plans[1].requires_external_user);
    }

    #[test]
    fn plans_audit_when_context_pack_quality_is_blocking() {
        let plans = plan_fb2_tools(
            &json!({
                "context_audit_id": "audit-1",
                "context_quality": {"warnings": ["missing_context_pack"]}
            }),
            Some("帮我看看"),
        );

        assert!(plans.iter().any(|plan| plan.name == "get_context_audit"));
    }

    #[test]
    fn rendered_tool_block_marks_success_as_required_for_facts() {
        let block = prompt_executed_tools_block(Some(&json!({
            "schema": "external_app.executed_tools.v1",
            "app_id": "fb2",
            "status": "ready",
            "executed_at": "2026-06-21T00:00:00Z",
            "results": [{"tool_name": "search_matches", "status": "ready", "success": true}]
        })));

        assert!(block.contains("<executed_external_app_tools"));
        assert!(block.contains("success=true"));
        assert!(block.contains("不能编造"));
    }
}
