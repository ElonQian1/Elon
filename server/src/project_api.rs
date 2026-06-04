use axum::{
    extract::{
        ws::{WebSocketUpgrade},
        Path as AxumPath, Query, State,
    },
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::Deserialize;
use std::{
    collections::HashMap,
    sync::Arc,
};

use crate::{
    project_auth::{auth_from_headers, can_edit, json_error, project_access},
    project_mobile::ensure_mobile_project,
    project_ws_session::handle_project_ws,
    tools,
    types::AppState,
};

#[derive(Deserialize)]
pub struct CreateProjectRequest {
    pub name: String,
    pub description: Option<String>,
    pub template: Option<String>,
}

#[derive(Deserialize)]
pub struct RegisterExternalProjectRequest {
    pub name: String,
    pub workspace_path: String,
    pub description: Option<String>,
    /// 若提供，表示项目位于该 PC 节点上（workspace_path 是 PC 上的绝对路径），
    /// 服务器不会做本地路径存在性校验。
    pub node_id: Option<String>,
}

#[derive(Deserialize)]
pub struct UpdateProfileRequest {
    pub nickname: Option<String>,
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

pub async fn list_my_projects(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(user) => user,
        Err(e) => return json_error(StatusCode::UNAUTHORIZED, e.to_string()),
    };
    if let Err(e) = ensure_mobile_project(&state, &user.id, "elon-self", Some("一龙项目")) {
        return json_error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string());
    }

    match state.store.list_projects_for_user(&user.id) {
        Ok(projects) => Json(serde_json::json!({ "projects": projects })).into_response(),
        Err(e) => json_error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    }
}

pub async fn create_project(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(req): Json<CreateProjectRequest>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(user) => user,
        Err(e) => return json_error(StatusCode::UNAUTHORIZED, e.to_string()),
    };

    let create_result = match state.store.create_project(
        &user.id,
        &req.name,
        req.description.as_deref(),
        req.template.as_deref(),
    ) {
        Ok(result) => result,
        Err(e) => return json_error(StatusCode::BAD_REQUEST, e.to_string()),
    };

    let reused_existing = create_result.reused_existing;
    let project = create_result.project;
    if !reused_existing {
        let workspace = state.get_project_workspace(&project.workspace_key);
        if let Err(e) =
            tools::create_project_workspace(&workspace, &project.template, &project.name, &user.id)
        {
            return json_error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string());
        }
    }

    Json(serde_json::json!({ "project": project, "reused_existing": reused_existing }))
        .into_response()
}

/// 注册一个指向外部本地路径的项目（如本机 D:\rust\active-projects\bb64a）。
/// 不会自动创建/初始化目录，仅在 DB 中登记，并验证路径已存在。
pub async fn register_external_project(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(req): Json<RegisterExternalProjectRequest>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(user) => user,
        Err(e) => return json_error(StatusCode::UNAUTHORIZED, e.to_string()),
    };

    let workspace_path = req.workspace_path.trim();
    if workspace_path.is_empty() {
        return json_error(StatusCode::BAD_REQUEST, "workspace_path 不能为空");
    }
    // 仅当请求方未声明 node_id（即项目应在服务器本机）时才校验路径存在；
    // PC 节点项目的路径在用户 PC 上，服务器看不到，跳过校验。
    if req.node_id.as_deref().map(str::trim).filter(|s| !s.is_empty()).is_none() {
        let pb = std::path::Path::new(workspace_path);
        if !pb.exists() {
            return json_error(
                StatusCode::BAD_REQUEST,
                format!("workspace_path 不存在: {}", workspace_path),
            );
        }
        if !pb.is_dir() {
            return json_error(
                StatusCode::BAD_REQUEST,
                "workspace_path 必须指向一个目录",
            );
        }
    }

    let create_result = match state.store.register_external_project(
        &user.id,
        &req.name,
        req.description.as_deref(),
        workspace_path,
    ) {
        Ok(r) => r,
        Err(e) => return json_error(StatusCode::BAD_REQUEST, e.to_string()),
    };

    Json(serde_json::json!({
        "project": create_result.project,
        "reused_existing": create_result.reused_existing,
    }))
    .into_response()
}

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
