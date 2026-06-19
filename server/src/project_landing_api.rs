use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::Deserialize;
use serde_json::Value;
use std::sync::Arc;

use crate::{
    project_auth::{auth_from_headers, can_edit, json_error, project_access},
    project_landing,
    types::AppState,
};

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
    let user = match auth_from_headers(&state, &headers) {
        Ok(user) => user,
        Err(error) => return json_error(StatusCode::UNAUTHORIZED, error.to_string()),
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
