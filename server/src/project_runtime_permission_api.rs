use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::Deserialize;
use std::sync::Arc;

use crate::{
    project_auth::{auth_from_headers, can_manage_project_members, json_error},
    store::{
        normalize_project_runtime_permission, PROJECT_RUNTIME_PERMISSION_DANGER_FULL_ACCESS,
        PROJECT_RUNTIME_PERMISSION_FULL_ACCESS, PROJECT_RUNTIME_PERMISSION_PROJECT_WRITE,
    },
    types::AppState,
};

#[derive(Debug, Deserialize)]
pub struct RuntimePermissionRequest {
    pub mode: String,
    #[serde(default, alias = "confirmFullAccess")]
    pub confirm_full_access: bool,
    #[serde(default, alias = "confirmDangerFullAccess")]
    pub confirm_danger_full_access: bool,
}

/// GET /api/projects/:id/runtime-permission
pub async fn get_runtime_permission(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(u) => u,
        Err(e) => return json_error(StatusCode::UNAUTHORIZED, e.to_string()),
    };
    let access = match state.store.get_project_access(&user.id, &project_id) {
        Ok(a) => a,
        Err(_) => return json_error(StatusCode::FORBIDDEN, "项目不存在或无权访问"),
    };
    let record = match state.store.get_project_runtime_permission(&project_id) {
        Ok(record) => record,
        Err(e) => return json_error(StatusCode::BAD_REQUEST, e.to_string()),
    };

    Json(serde_json::json!({
        "ok": true,
        "project_id": project_id,
        "mode": record.mode,
        "updated_by": record.updated_by,
        "updated_at": record.updated_at,
        "can_manage": can_manage_project_members(&access.role),
        "allowed_modes": [
            PROJECT_RUNTIME_PERMISSION_PROJECT_WRITE,
            PROJECT_RUNTIME_PERMISSION_FULL_ACCESS,
            PROJECT_RUNTIME_PERMISSION_DANGER_FULL_ACCESS
        ],
    }))
    .into_response()
}

/// PATCH /api/projects/:id/runtime-permission
pub async fn update_runtime_permission(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
    Json(req): Json<RuntimePermissionRequest>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(u) => u,
        Err(e) => return json_error(StatusCode::UNAUTHORIZED, e.to_string()),
    };
    let access = match state.store.get_project_access(&user.id, &project_id) {
        Ok(a) => a,
        Err(_) => return json_error(StatusCode::FORBIDDEN, "项目不存在或无权访问"),
    };
    if !can_manage_project_members(&access.role) {
        return json_error(
            StatusCode::FORBIDDEN,
            "只有项目 owner 或管理员才可修改 AI 运行权限",
        );
    }

    let Some(mode) = normalize_project_runtime_permission(&req.mode) else {
        return json_error(
            StatusCode::BAD_REQUEST,
            "mode 必须为 project_write、full_access 或 danger_full_access",
        );
    };
    if mode == PROJECT_RUNTIME_PERMISSION_FULL_ACCESS && !req.confirm_full_access {
        return json_error(
            StatusCode::BAD_REQUEST,
            "开启完全访问前必须显式确认 full_access 授权",
        );
    }
    if mode == PROJECT_RUNTIME_PERMISSION_DANGER_FULL_ACCESS
        && !(req.confirm_full_access || req.confirm_danger_full_access)
    {
        return json_error(
            StatusCode::BAD_REQUEST,
            "开启完整本机命令行前必须显式确认 danger_full_access 授权",
        );
    }

    match state
        .store
        .set_project_runtime_permission(&project_id, &user.id, mode)
    {
        Ok(record) => Json(serde_json::json!({
            "ok": true,
            "project_id": record.project_id,
            "mode": record.mode,
            "updated_by": record.updated_by,
            "updated_at": record.updated_at,
        }))
        .into_response(),
        Err(e) => {
            let msg = e.to_string();
            let status = if msg.contains("不存在") {
                StatusCode::NOT_FOUND
            } else if msg.contains("系统归档项目") {
                StatusCode::FORBIDDEN
            } else {
                StatusCode::BAD_REQUEST
            };
            json_error(status, msg)
        }
    }
}
