//! Route C remote runtime operations API.

use axum::{
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;

use crate::{
    admin::check_auth, project_auth::json_error,
    server_agent_runtime_budget::server_runtime_budget_status, types::AppState,
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

    Json(json!({
        "ok": true,
        "budgetStatus": server_runtime_budget_status(&state.store),
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
