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
    pub route_day: Option<String>,
    pub user_id: Option<String>,
}

fn default_days() -> i64 {
    14
}

fn default_limit() -> i64 {
    100
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
    let route_day = clean_optional(q.route_day);
    let user_id = clean_optional(q.user_id);

    let summaries = match state.store.route_c_budget_day_summaries(days) {
        Ok(rows) => rows,
        Err(e) => {
            tracing::warn!("admin route c budget summary error: {e}");
            return json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "查询 Route C 预算摘要失败",
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

    Json(json!({
        "ok": true,
        "budgetStatus": server_runtime_budget_status(&state.store),
        "query": {
            "days": days,
            "limit": limit,
            "routeDay": route_day,
            "userId": user_id,
        },
        "dailySummaries": summaries,
        "recentEvents": events,
    }))
    .into_response()
}

fn clean_optional(value: Option<String>) -> Option<String> {
    value
        .map(|raw| raw.trim().to_string())
        .filter(|value| !value.is_empty())
}
