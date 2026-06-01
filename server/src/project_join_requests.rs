/// project_join_requests.rs — 项目加入申请审批 API
///
/// 路由（均需登录）：
///   POST  /api/projects/:id/request-join          提交加入申请（join_mode=approval）
///   GET   /api/projects/:id/join-requests         owner 查看项目申请列表
///   PATCH /api/projects/:id/join-requests/:req_id owner 审批（approve/reject）
///   GET   /api/me/join-requests                   用户查看自己的申请状态
use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::Deserialize;
use std::sync::Arc;

use crate::{
    join_request_events,
    project_auth::{auth_from_headers, json_error},
    types::AppState,
};

// ─── 请求体 ───────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct RequestJoinBody {
    /// 申请留言（可选）
    pub message: Option<String>,
}

#[derive(Deserialize)]
pub struct ReviewBody {
    /// "approve" 或 "reject"
    pub action: String,
}

#[derive(Deserialize)]
pub struct ListQuery {
    /// true = 只显示 pending，false/不传 = 全部
    pub pending_only: Option<bool>,
}

// ─── Handlers ─────────────────────────────────────────────────────────────────

/// POST /api/projects/:id/request-join — 提交加入申请
pub async fn request_join(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
    Json(req): Json<RequestJoinBody>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(u) => u,
        Err(e) => return json_error(StatusCode::UNAUTHORIZED, e.to_string()),
    };

    let message = req.message.as_deref().filter(|m| !m.trim().is_empty());
    match state
        .store
        .create_join_request(&user.id, &project_id, message)
    {
        Ok(record) => {
            // 查询项目 owner，推送通知
            if let Ok(members) = state.store.list_project_members(&project_id) {
                let owner = members.iter().find(|m| m.role == "owner");
                if let Some(owner) = owner {
                    join_request_events::publish_new_request(
                        &owner.user_id,
                        &record.id,
                        &record.project_id,
                        &record.project_name,
                        &user.account,
                    );
                }
            }
            Json(serde_json::json!({
                "ok": true,
                "message": "申请已提交，等待 owner 审核",
                "request": record,
            }))
            .into_response()
        }
        Err(e) => {
            let msg = e.to_string();
            let status = if msg.contains("不存在") {
                StatusCode::NOT_FOUND
            } else if msg.contains("已经是") || msg.contains("不对外公开") {
                StatusCode::CONFLICT
            } else if msg.contains("不需要申请") {
                StatusCode::BAD_REQUEST
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };
            json_error(status, msg)
        }
    }
}

/// GET /api/projects/:id/join-requests — owner 查看申请列表
pub async fn list_join_requests(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
    Query(q): Query<ListQuery>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(u) => u,
        Err(e) => return json_error(StatusCode::UNAUTHORIZED, e.to_string()),
    };

    // 仅 owner/editor 可查看申请列表
    match state.store.get_project_access(&user.id, &project_id) {
        Ok(access) if access.role == "owner" || access.role == "editor" => {}
        Ok(_) => return json_error(StatusCode::FORBIDDEN, "只有项目 owner 才可管理加入申请"),
        Err(_) => return json_error(StatusCode::FORBIDDEN, "项目不存在或无权访问"),
    }

    let only_pending = q.pending_only.unwrap_or(true);
    match state
        .store
        .list_join_requests(&project_id, only_pending)
    {
        Ok(requests) => Json(serde_json::json!({
            "requests": requests,
            "total": requests.len(),
            "project_id": project_id,
        }))
        .into_response(),
        Err(e) => json_error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    }
}

/// PATCH /api/projects/:id/join-requests/:req_id — owner 审批
pub async fn review_join_request(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, req_id)): Path<(String, String)>,
    Json(req): Json<ReviewBody>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(u) => u,
        Err(e) => return json_error(StatusCode::UNAUTHORIZED, e.to_string()),
    };

    // 仅 owner 可审批
    match state.store.get_project_access(&user.id, &project_id) {
        Ok(access) if access.role == "owner" => {}
        Ok(_) => return json_error(StatusCode::FORBIDDEN, "只有项目 owner 才可审批加入申请"),
        Err(_) => return json_error(StatusCode::FORBIDDEN, "项目不存在或无权访问"),
    }

    let result = match req.action.as_str() {
        "approve" => state.store.approve_join_request(&req_id, &user.id),
        "reject" => state.store.reject_join_request(&req_id, &user.id),
        _ => return json_error(StatusCode::BAD_REQUEST, "action 必须为 approve 或 reject"),
    };

    match result {
        Ok(record) => {
            // 通知申请人
            join_request_events::publish_review_result(
                &record.user_id,
                &record.id,
                &record.project_id,
                &record.project_name,
                &record.status,
            );
            Json(serde_json::json!({
                "ok": true,
                "request": record,
            }))
            .into_response()
        }
        Err(e) => {
            let msg = e.to_string();
            let status = if msg.contains("不存在") {
                StatusCode::NOT_FOUND
            } else if msg.contains("已处理") {
                StatusCode::CONFLICT
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };
            json_error(status, msg)
        }
    }
}

/// GET /api/me/join-requests — 当前用户查看自己的申请
pub async fn my_join_requests(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(u) => u,
        Err(e) => return json_error(StatusCode::UNAUTHORIZED, e.to_string()),
    };

    match state.store.list_my_join_requests(&user.id) {
        Ok(requests) => Json(serde_json::json!({
            "requests": requests,
            "total": requests.len(),
        }))
        .into_response(),
        Err(e) => json_error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    }
}
