/// project_store.rs — 项目商店：浏览公开项目
///
/// 路由（无需登录即可浏览，加入需要登录）：
///   GET  /api/store/projects          浏览公开项目（分页 + 搜索）
///   GET  /api/store/projects/:id      单个项目详情预览
///   GET  /api/store/joined            我已加入（非 owner）的项目列表
use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::Deserialize;
use std::sync::Arc;

use crate::{
    project_auth::{auth_from_headers, json_error},
    types::AppState,
};

// ─── 请求参数 ─────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct StoreQuery {
    /// 关键词搜索（匹配项目名 / 描述）
    pub q: Option<String>,
    /// 加入方式过滤：open / approval / readonly
    pub join_mode: Option<String>,
    /// 是否只看已有可安装 APK 的项目
    pub has_apk: Option<bool>,
    /// 排序：updated / created / members
    pub sort: Option<String>,
    /// 每页数量，默认 20，最大 50
    pub limit: Option<i64>,
    /// 偏移量，默认 0
    pub offset: Option<i64>,
}

// ─── Handlers ─────────────────────────────────────────────────────────────────

/// GET /api/store/projects — 浏览公开项目（无需登录）
pub async fn list_store_projects(
    State(state): State<Arc<AppState>>,
    Query(q): Query<StoreQuery>,
) -> Response {
    let limit = q.limit.unwrap_or(20).clamp(1, 50);
    let offset = q.offset.unwrap_or(0).max(0);

    match state.store.list_public_projects(
        q.q.as_deref(),
        q.join_mode.as_deref(),
        q.has_apk,
        q.sort.as_deref(),
        limit,
        offset,
    ) {
        Ok(projects) => Json(serde_json::json!({
            "projects": projects,
            "total": projects.len(),
            "limit": limit,
            "offset": offset,
        }))
        .into_response(),
        Err(e) => json_error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    }
}

/// GET /api/store/projects/:id — 公开项目详情（无需登录）
pub async fn get_store_project(
    State(state): State<Arc<AppState>>,
    Path(project_id): Path<String>,
) -> Response {
    match state.store.get_public_project(&project_id) {
        Ok(project) => Json(serde_json::json!({ "project": project })).into_response(),
        Err(e) => json_error(StatusCode::NOT_FOUND, e.to_string()),
    }
}

/// GET /api/store/joined — 我已加入（非 owner）的项目列表（需登录）
pub async fn list_joined_projects(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(u) => u,
        Err(e) => return json_error(StatusCode::UNAUTHORIZED, e.to_string()),
    };
    match state.store.list_joined_projects(&user.id) {
        Ok(projects) => Json(serde_json::json!({ "projects": projects })).into_response(),
        Err(e) => json_error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    }
}
