//! Reports for external app tool execution quality.

use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;

use crate::{
    admin::check_auth,
    external_app_api::require_external_app_service_token,
    external_app_registry::{
        external_app_by_id, public_external_app_config, ExternalAppDefinition,
    },
    project_auth::json_error,
    store::AdminExternalAppToolExecutionSummary,
    types::AppState,
};

#[derive(Debug, Deserialize)]
pub struct ExternalAppToolExecutionReportQuery {
    pub app_id: Option<String>,
    #[serde(default = "default_days")]
    pub days: i64,
    #[serde(default = "default_limit")]
    pub limit: i64,
    pub external_group_id: Option<String>,
    pub status: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
struct ToolExecutionRecommendation {
    severity: &'static str,
    code: &'static str,
    metric: &'static str,
    message: &'static str,
    next_action: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ToolExecutionReportFilters<'a> {
    days: i64,
    limit: i64,
    external_group_id: Option<&'a str>,
    status: Option<&'a str>,
}

fn default_days() -> i64 {
    7
}

fn default_limit() -> i64 {
    50
}

/// GET /api/admin/external-apps/tool-executions?app_id=fb2&days=7&limit=50
pub async fn get_tool_execution_report(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(q): Query<ExternalAppToolExecutionReportQuery>,
) -> Response {
    if !check_auth(&headers, &state.admin_token) {
        return json_error(StatusCode::UNAUTHORIZED, "无效的管理员令牌");
    }
    let app_id = q
        .app_id
        .as_deref()
        .and_then(trimmed_non_empty)
        .unwrap_or("fb2");
    let app = match external_app_by_id(app_id) {
        Some(app) => app,
        None => return json_error(StatusCode::NOT_FOUND, format!("未知外部应用：{app_id}")),
    };
    tool_execution_report_response(
        state.as_ref(),
        app,
        report_filters_from_query(&q),
        json!({
            "mode": "admin_token",
            "scope": "selected_app",
            "app_id_source": "query_or_default"
        }),
    )
}

/// GET /api/external/apps/:app_id/tool-executions?days=7&limit=50
pub async fn get_external_app_tool_execution_report(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(app_id): Path<String>,
    Query(q): Query<ExternalAppToolExecutionReportQuery>,
) -> Response {
    let app_id = app_id.trim();
    let app = match external_app_by_id(app_id) {
        Some(app) => app,
        None => return json_error(StatusCode::NOT_FOUND, format!("未知外部应用：{app_id}")),
    };
    if let Err(response) = require_external_app_service_token(app.id, &headers) {
        return response;
    }
    tool_execution_report_response(
        state.as_ref(),
        app,
        report_filters_from_query(&q),
        json!({
            "mode": "external_app_service_token",
            "scope": "own_app_only",
            "app_id_source": "path",
            "query_app_id_ignored": q.app_id.as_deref().and_then(trimmed_non_empty)
        }),
    )
}

fn tool_execution_report_response(
    state: &AppState,
    app: &ExternalAppDefinition,
    filters: ToolExecutionReportFilters<'_>,
    access: serde_json::Value,
) -> Response {
    match state.store.admin_external_app_tool_execution_report(
        app.id,
        filters.days,
        filters.limit,
        filters.external_group_id,
        filters.status,
    ) {
        Ok(report) => {
            let recommendations = quality_recommendations(&report.summary);
            Json(json!({
                "schema": "external_app.tool_execution_report.v1",
                "app": public_external_app_config(app),
                "filters": {
                    "app_id": app.id,
                    "days": filters.days,
                    "limit": filters.limit,
                    "external_group_id": filters.external_group_id,
                    "status": filters.status
                },
                "access": access,
                "summary": report.summary,
                "recent_executions": report.rows,
                "recommendations": recommendations,
                "privacy": {
                    "raw_payloads_exposed": false,
                    "raw_payloads_retained_in_table": "external_app_tool_executions",
                    "note": "该报告只返回执行元数据和质量统计，不返回 results_json 中的订单或票据明细。"
                }
            }))
            .into_response()
        }
        Err(error) => {
            tracing::warn!(
                app_id = app.id,
                error = %error,
                "external app tool execution report failed"
            );
            json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "查询外部应用工具执行报告失败",
            )
        }
    }
}

fn report_filters_from_query(
    q: &ExternalAppToolExecutionReportQuery,
) -> ToolExecutionReportFilters<'_> {
    ToolExecutionReportFilters {
        days: q.days.clamp(1, 365),
        limit: q.limit.clamp(1, 500),
        external_group_id: q.external_group_id.as_deref().and_then(trimmed_non_empty),
        status: q.status.as_deref().and_then(trimmed_non_empty),
    }
}

fn trimmed_non_empty(value: &str) -> Option<&str> {
    let value = value.trim();
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

fn quality_recommendations(
    summary: &AdminExternalAppToolExecutionSummary,
) -> Vec<ToolExecutionRecommendation> {
    if summary.total_executions == 0 {
        return vec![recommendation(
            "info",
            "no_tool_execution_data",
            "total_executions",
            "当前筛选范围内还没有外部应用工具执行记录，暂时无法评估 fb2 数据工具质量。",
            "先让 fb2 群聊触发一次比赛、订单或群观点类 AI 问答，再回到该报告观察 grounding 和来源覆盖。",
        )];
    }

    let mut items = Vec::new();
    if summary.unsafe_result_count > 0 {
        items.push(recommendation(
            "critical",
            "unsafe_tool_results_present",
            "unsafe_result_count",
            "存在主项目明确不能用于事实回答的工具结果。",
            "优先检查 fb2 工具返回的 visibility、source_ids 和权限边界，修复后再扩展 AI 自动采纳范围。",
        ));
    }
    if summary.unavailable_executions > 0 {
        items.push(recommendation(
            "warning",
            "tool_runtime_unavailable",
            "unavailable_executions",
            "有工具执行因为配置、服务或鉴权不可用而没有拿到 fb2 数据。",
            "检查 FB2_CONTEXT_BASE_URL、FB2_CONTEXT_SHARED_TOKEN 以及 fb2 /api/main-project/tools/execute 健康状态。",
        ));
    }
    if summary.ready_result_count > 0 && summary.grounding_rate < 0.8 {
        items.push(recommendation(
            "warning",
            "low_grounding_rate",
            "grounding_rate",
            "可用工具结果中，强证据 grounded 比例偏低。",
            "让 fb2 为比赛、订单、群观点结果稳定返回 match_id、order_id、ticket_id、message_id 等 source_ids。",
        ));
    }
    if summary.ready_result_count > 0 && summary.source_id_count < summary.ready_result_count {
        items.push(recommendation(
            "warning",
            "low_source_id_coverage",
            "source_id_count",
            "工具结果的来源 ID 覆盖不足，AI 难以把关键判断追溯到具体比赛、订单或群消息。",
            "优先补齐 search_matches、search_user_orders、search_group_opinions 的 source_ids 和更新时间字段。",
        ));
    }
    if summary.weak_rate > 0.25 {
        items.push(recommendation(
            "info",
            "weak_result_rate_high",
            "weak_rate",
            "弱证据结果占比较高，AI 回答会更保守并需要提示不确定性。",
            "在 fb2 返回中标记清楚数据新鲜度、截断状态和聚合口径，减少主项目对结果可信度的降级。",
        ));
    }
    if summary.partial_executions > summary.ready_executions {
        items.push(recommendation(
            "info",
            "partial_executions_dominate",
            "partial_executions",
            "部分成功的工具执行多于完全成功，说明 planner 能触发工具但 fb2 返回还不稳定或不完整。",
            "按 recent_executions 里的 status 和 topic_hint 抽样复盘，优先修复高频失败工具。",
        ));
    }
    if summary.avg_duration_ms > 3000.0 {
        items.push(recommendation(
            "warning",
            "tool_latency_high",
            "avg_duration_ms",
            "fb2 工具平均执行耗时偏高，会拖慢群聊 AI 回复。",
            "为高频查询增加 fb2 侧索引、缓存或裁剪参数，主项目侧继续保留短超时和部分结果策略。",
        ));
    }
    if items.is_empty() {
        items.push(recommendation(
            "info",
            "tool_quality_healthy",
            "grounding_rate",
            "当前筛选范围内没有明显工具质量告警。",
            "继续观察趋势；下一步可以把该报告接入自动评测或运营面板。",
        ));
    }
    items
}

fn recommendation(
    severity: &'static str,
    code: &'static str,
    metric: &'static str,
    message: &'static str,
    next_action: &'static str,
) -> ToolExecutionRecommendation {
    ToolExecutionRecommendation {
        severity,
        code,
        metric,
        message,
        next_action,
    }
}

#[cfg(test)]
#[path = "external_app_tool_report_api_tests.rs"]
mod tests;
