/// project_join_requests.rs — 项目加入申请审批 API
///
/// 路由（均需登录）：
///   POST  /api/projects/:id/request-join          提交加入申请（join_mode=approval）
///   GET   /api/projects/:id/join-requests         owner/admin 查看项目申请列表
///   PATCH /api/projects/:id/join-requests/:req_id owner/admin 审批（approve/reject）
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
    store::PERMISSION_INVITE_MEMBERS,
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
            // 查询项目 owner/admin，推送通知
            if let Ok(members) = state.store.list_project_members(&project_id) {
                for manager in members.iter().filter(|m| {
                    state
                        .store
                        .project_role_has_permission(
                            &project_id,
                            &m.role,
                            PERMISSION_INVITE_MEMBERS,
                        )
                        .unwrap_or(false)
                }) {
                    join_request_events::publish_new_request(
                        &manager.user_id,
                        &record.id,
                        &record.project_id,
                        &record.project_name,
                        &user.account,
                    );
                }
            }
            Json(serde_json::json!({
                "ok": true,
                "message": "申请已提交，等待项目管理员审核",
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

/// GET /api/projects/:id/join-requests — owner/admin 查看申请列表
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

    // 仅有邀请/审批权限的角色可查看申请列表
    match state.store.get_project_access(&user.id, &project_id) {
        Ok(access)
            if state
                .store
                .project_role_has_permission(&project_id, &access.role, PERMISSION_INVITE_MEMBERS)
                .unwrap_or(false) => {}
        Ok(_) => return json_error(StatusCode::FORBIDDEN, "当前角色无权管理加入申请"),
        Err(_) => return json_error(StatusCode::FORBIDDEN, "项目不存在或无权访问"),
    }

    let only_pending = q.pending_only.unwrap_or(true);
    match state.store.list_join_requests(&project_id, only_pending) {
        Ok(requests) => Json(serde_json::json!({
            "requests": requests,
            "total": requests.len(),
            "project_id": project_id,
        }))
        .into_response(),
        Err(e) => json_error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    }
}

/// PATCH /api/projects/:id/join-requests/:req_id — owner/admin 审批
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

    // 仅有邀请/审批权限的角色可审批
    match state.store.get_project_access(&user.id, &project_id) {
        Ok(access)
            if state
                .store
                .project_role_has_permission(&project_id, &access.role, PERMISSION_INVITE_MEMBERS)
                .unwrap_or(false) => {}
        Ok(_) => return json_error(StatusCode::FORBIDDEN, "当前角色无权审批加入申请"),
        Err(_) => return json_error(StatusCode::FORBIDDEN, "项目不存在或无权访问"),
    }

    let review_action = req.action.trim();
    let result = match review_action {
        "approve" => state
            .store
            .approve_join_request(&req_id, &project_id, &user.id),
        "reject" => state
            .store
            .reject_join_request(&req_id, &project_id, &user.id),
        _ => return json_error(StatusCode::BAD_REQUEST, "action 必须为 approve 或 reject"),
    };

    match result {
        Ok(record) => {
            let action = if review_action == "approve" {
                "approve_join"
            } else {
                "reject_join"
            };
            let new_role = if review_action == "approve" {
                Some("member")
            } else {
                None
            };
            if let Err(err) = state.store.record_project_member_audit(
                &project_id,
                Some(&user.id),
                Some(&record.user_id),
                action,
                None,
                new_role,
                Some(&record.id),
            ) {
                tracing::warn!(?err, project_id = %project_id, "记录加入申请审计日志失败");
            }
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
            } else if msg.contains("不属于当前项目") || msg.contains("仅项目 owner") {
                StatusCode::FORBIDDEN
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
pub async fn my_join_requests(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Response {
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

/// DELETE /api/me/join-requests/:req_id — 用户取消自己的加入申请
pub async fn cancel_my_join_request(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(req_id): Path<String>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(u) => u,
        Err(e) => return json_error(StatusCode::UNAUTHORIZED, e.to_string()),
    };

    match state.store.cancel_my_join_request(&req_id, &user.id) {
        Ok(()) => Json(serde_json::json!({ "ok": true })).into_response(),
        Err(e) => json_error(StatusCode::BAD_REQUEST, e.to_string()),
    }
}

/// GET /api/me/owned-projects/pending-counts — 查看我拥有的项目中各项目的待审批数量
pub async fn owned_projects_pending_counts(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(u) => u,
        Err(e) => return json_error(StatusCode::UNAUTHORIZED, e.to_string()),
    };

    match state
        .store
        .list_owned_projects_with_pending_counts(&user.id)
    {
        Ok(rows) => Json(serde_json::json!({ "projects": rows })).into_response(),
        Err(e) => json_error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    }
}
