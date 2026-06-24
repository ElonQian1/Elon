//! Route C remote runtime operations API.

use axum::{
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;

use crate::{
    admin::check_auth,
    project_auth::json_error,
    server_agent_runtime_budget::{server_runtime_budget_status, ServerRuntimeBudgetStatus},
    store::route_c_budget::{RouteCBudgetEventRow, RouteCBudgetOutcomeSummary},
    types::AppState,
};

#[derive(Deserialize)]
pub struct RouteCBudgetQuery {
    #[serde(default = "default_days")]
    pub days: i64,
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(
        default = "default_stale_pending_after_secs",
        alias = "stalePendingAfterSecs"
    )]
    pub stale_pending_after_secs: i64,
    pub route_day: Option<String>,
    pub user_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RouteCOperationActionItem {
    pub severity: &'static str,
    pub kind: &'static str,
    pub message: String,
    pub count: i64,
    pub operator_action: &'static str,
}

fn default_days() -> i64 {
    14
}

fn default_limit() -> i64 {
    100
}

fn default_stale_pending_after_secs() -> i64 {
    15 * 60
}

pub async fn budget_report(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(q): Query<RouteCBudgetQuery>,
) -> Response {
    if !check_auth(&headers, &state.admin_token) {
        return json_error(StatusCode::UNAUTHORIZED, "无效的管理员令牌");
    }

    let days = q.days.clamp(1, 90);
    let limit = q.limit.clamp(1, 500);
    let stale_pending_after_secs = q.stale_pending_after_secs.clamp(60, 86_400);
    let stale_pending_before =
        (chrono::Utc::now() - chrono::Duration::seconds(stale_pending_after_secs)).to_rfc3339();
    let route_day = clean_optional(q.route_day);
    let user_id = clean_optional(q.user_id);

    let summaries = match state
        .store
        .route_c_budget_day_summaries_with_stale(days, Some(&stale_pending_before))
    {
        Ok(rows) => rows,
        Err(e) => {
            tracing::warn!("admin route c budget summary error: {e}");
            return json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "查询 Route C 预算摘要失败",
            );
        }
    };
    let outcome_summaries = match state.store.route_c_budget_outcome_summaries_with_stale(
        route_day.as_deref(),
        user_id.as_deref(),
        Some(&stale_pending_before),
    ) {
        Ok(rows) => rows,
        Err(e) => {
            tracing::warn!("admin route c budget outcome summary error: {e}");
            return json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "查询 Route C 调用结果摘要失败",
            );
        }
    };
    let events = match state.store.route_c_budget_recent_events(
        route_day.as_deref(),
        user_id.as_deref(),
        limit,
    ) {
        Ok(rows) => rows,
        Err(e) => {
            tracing::warn!("admin route c budget events error: {e}");
            return json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "查询 Route C 调用审计失败",
            );
        }
    };
    let stale_pending_events = match state.store.route_c_budget_stale_pending_events(
        route_day.as_deref(),
        user_id.as_deref(),
        &stale_pending_before,
        limit,
    ) {
        Ok(rows) => rows,
        Err(e) => {
            tracing::warn!("admin route c budget stale pending events error: {e}");
            return json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "查询 Route C 卡住调用审计失败",
            );
        }
    };

    let budget_status = server_runtime_budget_status(&state.store);
    let action_items =
        route_c_operation_action_items(&budget_status, &outcome_summaries, &stale_pending_events);

    Json(json!({
        "ok": true,
        "budgetStatus": budget_status,
        "query": {
            "days": days,
            "limit": limit,
            "stalePendingAfterSecs": stale_pending_after_secs,
            "stalePendingBefore": stale_pending_before,
            "routeDay": route_day,
            "userId": user_id,
        },
        "dailySummaries": summaries,
        "outcomeSummaries": outcome_summaries,
        "actionItems": action_items,
        "recentEvents": events,
        "stalePendingEvents": stale_pending_events,
    }))
    .into_response()
}

fn clean_optional(value: Option<String>) -> Option<String> {
    value
        .map(|raw| raw.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn route_c_operation_action_items(
    budget_status: &ServerRuntimeBudgetStatus,
    outcome_summaries: &[RouteCBudgetOutcomeSummary],
    stale_pending_events: &[RouteCBudgetEventRow],
) -> Vec<RouteCOperationActionItem> {
    let mut items = Vec::new();

    match budget_status.status {
        "exhausted" => items.push(RouteCOperationActionItem {
            severity: "blocker",
            kind: "budget_exhausted",
            message: "Route C platform daily budget is exhausted.".to_string(),
            count: budget_status.used_calls_today as i64,
            operator_action:
                "Raise ELON_SERVER_AGENT_RUNTIME_DAILY_CALL_LIMIT, wait for UTC reset, or route users to local CLI/API keys.",
        }),
        "user_exhausted" => items.push(RouteCOperationActionItem {
            severity: "warning",
            kind: "user_budget_exhausted",
            message: "Route C per-user daily budget is exhausted for the selected user.".to_string(),
            count: budget_status.used_calls_today_for_user.unwrap_or_default() as i64,
            operator_action:
                "Raise ELON_SERVER_AGENT_RUNTIME_PER_USER_DAILY_CALL_LIMIT, wait for UTC reset, or route this user to local CLI/API keys.",
        }),
        "unavailable" => items.push(RouteCOperationActionItem {
            severity: "blocker",
            kind: "budget_unavailable",
            message: "Route C budget ledger is unavailable while limits are configured.".to_string(),
            count: 1,
            operator_action:
                "Inspect server storage and database health before allowing Route C remote-model traffic.",
        }),
        _ => {
            if let (Some(limit), Some(remaining)) = (
                budget_status.daily_call_limit,
                budget_status.remaining_calls_today,
            ) {
                let warning_floor = (limit / 10).max(1);
                if remaining <= warning_floor {
                    items.push(RouteCOperationActionItem {
                        severity: "warning",
                        kind: "budget_low",
                        message: format!(
                            "Route C platform daily budget is low: {remaining} of {limit} calls remain."
                        ),
                        count: remaining as i64,
                        operator_action:
                            "Monitor demand, raise the daily limit, or temporarily prefer local CLI/API-key routes.",
                    });
                }
            }
        }
    }

    let stale_count = stale_pending_events.len() as i64;
    if stale_count > 0 {
        items.push(RouteCOperationActionItem {
            severity: "warning",
            kind: "stale_pending_calls",
            message: format!("{stale_count} Route C calls are still pending past the stale cutoff."),
            count: stale_count,
            operator_action:
                "Inspect stalePendingEvents, provider latency, and server logs; reconcile or retry stuck runs.",
        });
    }

    let server_errors = completed_outcome_count(outcome_summaries, "server_error");
    if server_errors > 0 {
        items.push(RouteCOperationActionItem {
            severity: "blocker",
            kind: "server_errors",
            message: format!("{server_errors} Route C calls failed inside the server runtime."),
            count: server_errors,
            operator_action:
                "Inspect server runtime logs, deployment health, and model request serialization before increasing traffic.",
        });
    }

    let provider_errors = completed_outcome_count(outcome_summaries, "provider_error");
    if provider_errors > 0 {
        items.push(RouteCOperationActionItem {
            severity: "warning",
            kind: "provider_errors",
            message: format!("{provider_errors} Route C calls failed at the model provider boundary."),
            count: provider_errors,
            operator_action:
                "Check provider status, rate limits, credentials, and fallback users to local CLI/API-key routes if errors continue.",
        });
    }

    let output_rejections = completed_outcome_count(outcome_summaries, "output_rejected");
    if output_rejections > 0 {
        items.push(RouteCOperationActionItem {
            severity: "warning",
            kind: "output_rejections",
            message: format!("{output_rejections} Route C responses were rejected by output validation."),
            count: output_rejections,
            operator_action:
                "Review validation policy and redact-sensitive output handling before relaxing any Route C guardrails.",
        });
    }

    let known_failure_outcomes = ["provider_error", "server_error", "output_rejected"];
    let other_failures = outcome_summaries
        .iter()
        .filter(|row| {
            row.outcome != "success"
                && row.outcome != "admitted"
                && !known_failure_outcomes.contains(&row.outcome.as_str())
        })
        .map(|row| row.completed_calls.max(0))
        .sum::<i64>();
    if other_failures > 0 {
        items.push(RouteCOperationActionItem {
            severity: "warning",
            kind: "other_failures",
            message: format!("{other_failures} Route C calls completed with non-success outcomes."),
            count: other_failures,
            operator_action:
                "Inspect outcomeSummaries and recentEvents for the failing outcome class before expanding Route C usage.",
        });
    }

    items
}

fn completed_outcome_count(outcome_summaries: &[RouteCBudgetOutcomeSummary], outcome: &str) -> i64 {
    outcome_summaries
        .iter()
        .filter(|row| row.outcome == outcome)
        .map(|row| row.completed_calls.max(0))
        .sum()
}

#[cfg(test)]
mod tests {
    use super::{route_c_operation_action_items, RouteCBudgetEventRow, RouteCBudgetOutcomeSummary};
    use crate::server_agent_runtime_budget::ServerRuntimeBudgetStatus;

    fn budget_status(status: &'static str) -> ServerRuntimeBudgetStatus {
        ServerRuntimeBudgetStatus {
            enabled: true,
            status,
            source: "test",
            used_calls_today: 100,
            daily_call_limit: Some(100),
            remaining_calls_today: Some(0),
            per_user_enabled: true,
            per_user_source: "test",
            used_calls_today_for_user: Some(3),
            per_user_daily_call_limit: Some(3),
            remaining_calls_today_for_user: Some(0),
            reset_after_secs: 3600,
        }
    }

    fn outcome(outcome: &str, completed_calls: i64) -> RouteCBudgetOutcomeSummary {
        RouteCBudgetOutcomeSummary {
            outcome: outcome.to_string(),
            total_calls: completed_calls,
            completed_calls,
            pending_calls: 0,
            stale_pending_calls: 0,
            unique_users: 1,
            total_tokens: None,
            first_created_at: None,
            last_created_at: None,
        }
    }

    fn stale_event() -> RouteCBudgetEventRow {
        RouteCBudgetEventRow {
            id: "evt_1".to_string(),
            user_id: "user_1".to_string(),
            request_fingerprint: "fp_1".to_string(),
            route_day: "2026-06-24".to_string(),
            created_at: "2026-06-24T00:00:00+00:00".to_string(),
            outcome: "admitted".to_string(),
            completed_at: None,
            model: None,
            total_tokens: None,
            error_summary: Some("secret prompt text should not leave event rows".to_string()),
        }
    }

    #[test]
    fn route_c_operation_action_items_surface_budget_and_stale_pending() {
        let actions = route_c_operation_action_items(
            &budget_status("exhausted"),
            &[outcome("provider_error", 2)],
            &[stale_event()],
        );
        let kinds = actions.iter().map(|item| item.kind).collect::<Vec<_>>();

        assert_eq!(
            kinds,
            vec!["budget_exhausted", "stale_pending_calls", "provider_errors"]
        );
        assert_eq!(actions[0].severity, "blocker");
        assert_eq!(actions[1].count, 1);
        assert_eq!(actions[2].count, 2);

        let serialized = serde_json::to_string(&actions).unwrap();
        assert!(!serialized.contains("secret prompt text"));
        assert!(!serialized.contains("fp_1"));
        assert!(!serialized.contains("user_1"));
    }

    #[test]
    fn route_c_operation_action_items_warn_when_budget_is_low() {
        let mut status = budget_status("available");
        status.used_calls_today = 95;
        status.remaining_calls_today = Some(5);

        let actions = route_c_operation_action_items(&status, &[], &[]);

        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].kind, "budget_low");
        assert_eq!(actions[0].severity, "warning");
        assert_eq!(actions[0].count, 5);
    }
}
