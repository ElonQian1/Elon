//! 用户记忆管理 REST API。
//!
//! 路由（在 router.rs 注册）：
//!   GET    /api/memories         → 查看当前用户的记忆列表（分页）
//!   POST   /api/memories         → 手动添加一条记忆
//!   DELETE /api/memories/:id     → 删除指定记忆条目

use axum::{
    extract::{Path as AxumPath, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::{
    project_auth::{auth_from_headers, json_error},
    types::AppState,
};

// ── 查询参数 ──────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct ListMemoriesQuery {
    #[serde(default = "default_page")]
    pub page: i64,
    #[serde(default = "default_page_size")]
    pub page_size: i64,
}

fn default_page() -> i64 {
    1
}
fn default_page_size() -> i64 {
    20
}

// ── 处理函数 ──────────────────────────────────────────────────────────────────

/// GET /api/memories — 返回当前登录用户的记忆列表（按 importance 降序）
pub async fn list_memories(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(q): Query<ListMemoriesQuery>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(u) => u,
        Err(e) => return json_error(StatusCode::UNAUTHORIZED, e.to_string()),
    };

    let page_size = q.page_size.clamp(1, 100);
    let memories = match state.store.list_user_memories(&user.id, q.page, page_size) {
        Ok(m) => m,
        Err(e) => return json_error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    };

    Json(serde_json::json!({
        "memories": memories,
        "page": q.page,
        "page_size": page_size,
    }))
    .into_response()
}

/// DELETE /api/memories/:id — 删除指定记忆（仅限本人）
pub async fn delete_memory(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    AxumPath(memory_id): AxumPath<String>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(u) => u,
        Err(e) => return json_error(StatusCode::UNAUTHORIZED, e.to_string()),
    };

    match state.store.delete_user_memory(&memory_id, &user.id) {
        Ok(true) => Json(serde_json::json!({ "ok": true })).into_response(),
        Ok(false) => json_error(StatusCode::NOT_FOUND, "记忆不存在或无权删除"),
        Err(e) => json_error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    }
}

// ── 手动添加 ─────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct CreateMemoryBody {
    pub content: String,
    #[serde(default = "default_category")]
    pub category: String,
    #[serde(default = "default_importance")]
    pub importance: i64,
}

#[derive(Debug, Serialize)]
struct CreateMemoryResp {
    ok: bool,
    message: &'static str,
}

fn default_category() -> String {
    "fact".to_string()
}
fn default_importance() -> i64 {
    5
}

/// POST /api/memories — 手动添加一条记忆（用户主动告知 AI）
pub async fn create_memory(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<CreateMemoryBody>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(u) => u,
        Err(e) => return json_error(StatusCode::UNAUTHORIZED, e.to_string()),
    };

    let content = body.content.trim().to_string();
    if content.is_empty() {
        return json_error(StatusCode::BAD_REQUEST, "content 不能为空");
    }
    if content.chars().count() > 200 {
        return json_error(StatusCode::BAD_REQUEST, "content 不能超过 200 字");
    }

    let valid_categories = ["preference", "profile", "goal", "fact"];
    let category = if valid_categories.contains(&body.category.as_str()) {
        body.category.clone()
    } else {
        "fact".to_string()
    };

    let importance = body.importance.clamp(1, 10);

    match state
        .store
        .insert_user_memory(&user.id, &content, &category, importance, None)
    {
        Ok(()) => Json(CreateMemoryResp { ok: true, message: "已添加" }).into_response(),
        Err(e) => json_error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    }
}
