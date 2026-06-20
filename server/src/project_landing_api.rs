use axum::{
    extract::{Path, State},
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::Deserialize;
use serde_json::Value;
use std::sync::Arc;
use uuid::Uuid;

use crate::{
    project_auth::{
        auth_from_headers, can_edit, can_manage_project_members, json_error, project_access,
    },
    project_landing,
    types::AppState,
};

const PROJECT_LANDING_TOKEN_HEADER: &str = "x-elon-project-landing-token";

#[derive(Debug, Default, Deserialize)]
pub(crate) struct SyncProjectLandingRequest {
    pub landing: Option<Value>,
}

pub async fn sync_project_landing(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
    body: Option<Json<SyncProjectLandingRequest>>,
) -> Response {
    let req = body.map(|Json(req)| req).unwrap_or_default();
    if let Some(token) = project_landing_token_header(&headers) {
        return sync_project_landing_with_upload_token(state, project_id, req, token).await;
    }

    let user = match auth_from_headers(&state, &headers) {
        Ok(user) => user,
        Err(user_error) => {
            if let Some(token) = bearer_token(&headers) {
                return sync_project_landing_with_upload_token(state, project_id, req, token).await;
            }
            return json_error(StatusCode::UNAUTHORIZED, user_error.to_string());
        }
    };
    let access = match project_access(&state, &user.id, &project_id) {
        Ok(access) => access,
        Err(error) => return json_error(StatusCode::NOT_FOUND, error.to_string()),
    };
    if !can_edit(&access.role) {
        return json_error(StatusCode::FORBIDDEN, "当前用户无权同步项目首页");
    }

    let landing = if let Some(landing) = req.landing {
        landing
    } else {
        let workspace = state
            .resolve_project_workspace(&access.workspace_key, access.workspace_path.as_deref());
        match project_landing::load_workspace_landing(&workspace)
            .filter(project_landing::has_display_content)
        {
            Some(landing) => landing,
            None if access.node_id.is_some() => {
                return json_error(
                    StatusCode::BAD_REQUEST,
                    "这是 PC 节点本地项目，请在安装了 node-agent 的电脑端同步首页",
                )
            }
            None => {
                return json_error(
                    StatusCode::BAD_REQUEST,
                    "项目工作区没有可用的 .elon/project-landing.json",
                )
            }
        }
    };

    let snapshot =
        match state
            .store
            .update_project_landing_snapshot(&user.id, &project_id, &landing)
        {
            Ok(Some(snapshot)) => snapshot,
            Ok(None) => {
                return json_error(StatusCode::BAD_REQUEST, "项目首页 manifest 为空或格式无效")
            }
            Err(error) => return json_error(StatusCode::BAD_REQUEST, error.to_string()),
        };

    Json(serde_json::json!({
        "ok": true,
        "project_id": project_id,
        "landing": snapshot,
    }))
    .into_response()
}

async fn sync_project_landing_with_upload_token(
    state: Arc<AppState>,
    project_id: String,
    req: SyncProjectLandingRequest,
    token: &str,
) -> Response {
    let landing = match req.landing {
        Some(landing) => landing,
        None => {
            return json_error(
                StatusCode::BAD_REQUEST,
                "项目首页上传凭证只能提交请求体 landing，不能读取工作区",
            )
        }
    };
    let record = match state
        .store
        .authenticate_project_landing_upload_token(&project_id, token)
    {
        Ok(Some(record)) => record,
        Ok(None) => return json_error(StatusCode::UNAUTHORIZED, "项目首页上传凭证无效"),
        Err(error) => return json_error(StatusCode::BAD_REQUEST, error.to_string()),
    };
    let snapshot = match state
        .store
        .update_project_landing_snapshot_with_upload_token(&project_id, &record.id, &landing)
    {
        Ok(Some(snapshot)) => snapshot,
        Ok(None) => return json_error(StatusCode::BAD_REQUEST, "项目首页 manifest 为空或格式无效"),
        Err(error) => return json_error(StatusCode::BAD_REQUEST, error.to_string()),
    };

    Json(serde_json::json!({
        "ok": true,
        "project_id": project_id,
        "auth": "project_landing_upload_token",
        "token_id": record.id,
        "landing": snapshot,
    }))
    .into_response()
}

pub async fn rotate_project_landing_token(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(user) => user,
        Err(error) => return json_error(StatusCode::UNAUTHORIZED, error.to_string()),
    };
    let access = match project_access(&state, &user.id, &project_id) {
        Ok(access) => access,
        Err(error) => return json_error(StatusCode::NOT_FOUND, error.to_string()),
    };
    if !can_manage_project_members(&access.role) {
        return json_error(
            StatusCode::FORBIDDEN,
            "只有项目 owner/admin 可以生成首页上传凭证",
        );
    }

    let token = new_project_landing_token();
    let record =
        match state
            .store
            .rotate_project_landing_upload_token(&project_id, &user.id, &token)
        {
            Ok(record) => record,
            Err(error) => return json_error(StatusCode::BAD_REQUEST, error.to_string()),
        };

    Json(serde_json::json!({
        "ok": true,
        "project_id": record.project_id,
        "token_id": record.id,
        "token": token,
        "header": "X-Elon-Project-Landing-Token",
        "env": "ELON_MAIN_PROJECT_TOKEN",
        "created_at": record.created_at,
        "last_used_at": record.last_used_at,
        "warning": "token 只在本次响应明文返回，请保存到子项目 CI Secret 或本机环境变量",
    }))
    .into_response()
}

fn new_project_landing_token() -> String {
    format!("plt_{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple())
}

fn project_landing_token_header(headers: &HeaderMap) -> Option<&str> {
    headers
        .get(PROJECT_LANDING_TOKEN_HEADER)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn bearer_token(headers: &HeaderMap) -> Option<&str> {
    headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.trim().strip_prefix("Bearer "))
        .map(str::trim)
        .filter(|value| !value.is_empty())
}
