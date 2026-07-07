use axum::{
    extract::{ws::WebSocketUpgrade, Path as AxumPath, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::Deserialize;
use std::{collections::HashMap, sync::Arc};

use crate::{
    project_auth::{auth_from_headers, can_edit, json_error, project_access},
    project_landing,
    project_mobile::ensure_mobile_project,
    project_storage, project_workspace_provision,
    project_ws_session::handle_project_ws,
    store::{is_system_project_source_type, ProjectDevProfile},
    types::AppState,
    user_archive_profile::build_user_archive_project_response,
};

#[derive(Deserialize)]
pub struct CreateProjectRequest {
    pub name: String,
    pub description: Option<String>,
    pub template: Option<String>,
    /// Optional Git remote. When present, the selected PC node clones/fetches this repository.
    pub repo_url: Option<String>,
    /// Optional branch to check out on the PC node.
    pub branch: Option<String>,
    /// 新项目可创建到任意在线 PC CLI 节点；多节点在线时必须显式指定。
    pub node_id: Option<String>,
    /// 可选：项目代码母仓所在的 PC 硬盘节点；不传时优先自动使用当前用户的在线硬盘节点。
    pub storage_node_id: Option<String>,
    /// PC 工作台新建项目默认跳过代码存储，先让用户拿到可用项目；高级场景再显式启用。
    pub skip_storage: Option<bool>,
    /// 兼容未来扩展。当前只允许 "pc_node" / "pc" / 空值。
    pub execution_target: Option<String>,
}

#[derive(Deserialize)]
pub struct RegisterExternalProjectRequest {
    /// 可选：重绑定一个已存在项目，而不是按 name 创建/复用当前 owner 的项目。
    /// 用于平台自身项目 elon-self 这类共享项目绑定到 PC 节点。
    pub project_id: Option<String>,
    pub name: String,
    pub workspace_path: String,
    pub description: Option<String>,
    /// 若提供，表示项目位于该 PC 节点上（workspace_path 是 PC 上的绝对路径），
    /// 服务器不会做本地路径存在性校验。
    pub node_id: Option<String>,
    /// true 时注册后立即发布到项目广场。
    pub is_public: Option<bool>,
    /// 可选 Git 远端；用于之后把该项目迁移到其它 PC 节点时重建工作区。
    pub repo_url: Option<String>,
    /// 可选 Git 分支；不传则使用本地/远端默认分支。
    pub branch: Option<String>,
    /// "open" | "approval" | "invite" | "readonly"；默认 "readonly"。
    pub join_mode: Option<String>,
    /// 可选：PC node-agent 从项目根目录 `.elon/project-landing.json` 读取到的项目首页快照。
    pub landing: Option<serde_json::Value>,
    /// 可选：PC node-agent 从项目根目录自动识别到的运行/测试/构建命令。
    pub dev_profile: Option<ProjectDevProfile>,
}

#[derive(Deserialize)]
pub struct UpdateProfileRequest {
    pub nickname: Option<String>,
}

#[derive(Deserialize, Default)]
pub struct ListMyProjectsQuery {
    pub include_system: Option<bool>,
}

pub async fn update_profile(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(req): Json<UpdateProfileRequest>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(user) => user,
        Err(e) => return json_error(StatusCode::UNAUTHORIZED, e.to_string()),
    };
    let nickname = match req.nickname {
        Some(value) => {
            let value = value.trim().to_string();
            if value.is_empty() {
                return json_error(StatusCode::BAD_REQUEST, "昵称不能为空");
            }
            value
        }
        None => return json_error(StatusCode::BAD_REQUEST, "缺少昵称"),
    };
    match state.store.update_user_nickname(&user.id, &nickname) {
        Ok(user) => Json(serde_json::json!({ "user": user })).into_response(),
        Err(e) => json_error(StatusCode::BAD_REQUEST, e.to_string()),
    }
}

pub async fn list_my_projects(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(query): Query<ListMyProjectsQuery>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(user) => user,
        Err(e) => return json_error(StatusCode::UNAUTHORIZED, e.to_string()),
    };

    match state.store.list_projects_for_user(&user.id) {
        Ok(projects) => {
            let include_system = query.include_system.unwrap_or(false);
            let projects = if include_system {
                projects
            } else {
                projects
                    .into_iter()
                    .filter(|project| !is_system_project_source_type(&project.source_type))
                    .collect()
            };
            Json(serde_json::json!({ "projects": projects })).into_response()
        }
        Err(e) => json_error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    }
}

// Sub-modules extracted to reduce file size
#[path = "project_api_create_project.rs"]
mod create_project_mod;
pub use create_project_mod::create_project;

#[path = "project_api_register_project.rs"]
mod register_project_mod;
pub use register_project_mod::register_external_project;

async fn archive_project_payload(
    state: &AppState,
    user_id: &str,
    project_id: &str,
) -> Option<crate::store::UserArchiveProject> {
    match build_user_archive_project_response(state, user_id, project_id).await {
        Ok(project) => project,
        Err(e) => {
            tracing::warn!("构造项目归档响应失败 project_id={project_id}: {e}");
            None
        }
    }
}

fn clean_optional_string(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

/// 注册一个指向外部本地路径的项目（如本机 D:\rust\active-projects\bb64a）。
/// 不会自动创建/初始化目录，仅在 DB 中登记，并验证路径已存在。

pub async fn ws_project_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
    AxumPath(project_id): AxumPath<String>,
    Query(query): Query<HashMap<String, String>>,
) -> Response {
    let token = query.get("token").map(String::as_str).unwrap_or("");
    let user = match state.store.authenticate_token(token) {
        Ok(user) => user,
        Err(e) => return json_error(StatusCode::UNAUTHORIZED, e.to_string()),
    };
    let project = match project_access(&state, &user.id, &project_id) {
        Ok(project) => project,
        Err(e) => return json_error(StatusCode::FORBIDDEN, e.to_string()),
    };
    if !can_edit(&project.role) {
        return json_error(StatusCode::FORBIDDEN, "当前用户没有修改项目的权限");
    }

    let download_base = format!("{}/api/projects/{}/download", state.public_url, project.id);
    let client_version_code = query
        .get("app_version_code")
        .and_then(|value| value.parse::<i64>().ok());
    ws.on_upgrade(move |socket| {
        handle_project_ws(
            socket,
            state,
            user,
            project,
            download_base,
            client_version_code,
        )
    })
    .into_response()
}

pub async fn ws_user_project_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
    AxumPath((user_id, project_id)): AxumPath<(String, String)>,
    Query(query): Query<HashMap<String, String>>,
) -> Response {
    // ── APK 最低版本门控 ────────────────────────────────────────────────────
    if state.min_apk_version_code > 0 {
        let client_vc = query
            .get("app_version_code")
            .and_then(|v| v.parse::<i64>().ok())
            .unwrap_or(0);
        if client_vc > 0 && client_vc < state.min_apk_version_code {
            return json_error(
                StatusCode::UPGRADE_REQUIRED,
                format!(
                    "您的 APP 版本过低（当前 {}，最低要求 {}），请前往 {}/app/download 升级后再使用",
                    client_vc, state.min_apk_version_code, state.public_url
                ),
            );
        }
    }

    // ── 身份验证（REQUIRE_LOGIN=true 时强制） ───────────────────────────────
    let (user, project) = if state.require_login {
        let token = query.get("token").map(String::as_str).unwrap_or("");
        let authed_user = match state.store.authenticate_token(token) {
            Ok(u) => u,
            Err(e) => {
                return json_error(
                    StatusCode::UNAUTHORIZED,
                    format!("请先登录后再使用（{}）", e),
                );
            }
        };
        // token 验证通过后，用 token 对应的用户身份获取/创建项目
        match ensure_mobile_project(
            &state,
            &authed_user.id,
            &project_id,
            query.get("title").map(String::as_str),
        ) {
            Ok(pair) => pair,
            Err(e) => return json_error(StatusCode::BAD_REQUEST, e.to_string()),
        }
    } else {
        // 旧兼容模式：device UUID 直接当 user_id，自动创建账号
        match ensure_mobile_project(
            &state,
            &user_id,
            &project_id,
            query.get("title").map(String::as_str),
        ) {
            Ok(pair) => pair,
            Err(e) => return json_error(StatusCode::BAD_REQUEST, e.to_string()),
        }
    };

    let download_base = format!(
        "{}/api/user/{}/projects/{}/download",
        state.public_url, user.id, project.id
    );
    let client_version_code = query
        .get("app_version_code")
        .and_then(|value| value.parse::<i64>().ok());
    ws.on_upgrade(move |socket| {
        handle_project_ws(
            socket,
            state,
            user,
            project,
            download_base,
            client_version_code,
        )
    })
    .into_response()
}