//! Admin-only reports for external app tool execution quality.

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
    admin::check_auth,
    external_app_registry::{external_app_by_id, public_external_app_config},
    project_auth::json_error,
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
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("fb2");
    let app = match external_app_by_id(app_id) {
        Some(app) => app,
        None => return json_error(StatusCode::NOT_FOUND, format!("未知外部应用：{app_id}")),
    };
    let days = q.days.clamp(1, 365);
    let limit = q.limit.clamp(1, 500);
    let external_group_id = q
        .external_group_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let status = q
        .status
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());

    match state.store.admin_external_app_tool_execution_report(
        app.id,
        days,
        limit,
        external_group_id,
        status,
    ) {
        Ok(report) => Json(json!({
            "schema": "external_app.tool_execution_report.v1",
            "app": public_external_app_config(app),
            "filters": {
                "app_id": app.id,
                "days": days,
                "limit": limit,
                "external_group_id": external_group_id,
                "status": status
            },
            "summary": report.summary,
            "recent_executions": report.rows,
            "privacy": {
                "raw_payloads_exposed": false,
                "raw_payloads_retained_in_table": "external_app_tool_executions",
                "note": "管理列表只返回执行元数据和质量统计，不返回 results_json 中的订单或票据明细。"
            }
        }))
        .into_response(),
        Err(error) => {
            tracing::warn!(
                app_id = app.id,
                error = %error,
                "admin external app tool execution report failed"
            );
            json_error(StatusCode::INTERNAL_SERVER_ERROR, "查询外部应用工具执行报告失败")
        }
    }
}
