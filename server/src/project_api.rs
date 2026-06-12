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
    project_mobile::ensure_mobile_project,
    project_storage, project_workspace_provision,
    project_ws_session::handle_project_ws,
    store::is_system_project_source_type,
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

pub async fn create_project(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(req): Json<CreateProjectRequest>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(user) => user,
        Err(e) => return json_error(StatusCode::UNAUTHORIZED, e.to_string()),
    };

    let execution_target = req
        .execution_target
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("pc_node");
    if !["pc_node", "pc"].contains(&execution_target) {
        return json_error(
            StatusCode::BAD_REQUEST,
            "服务器磁盘不再承载新代码项目，请选择在线 PC 节点创建项目",
        );
    }
    let requested_repo_url = clean_optional_string(req.repo_url.as_deref());
    let requested_branch = clean_optional_string(req.branch.as_deref());
    if requested_repo_url.is_some()
        && req
            .storage_node_id
            .as_deref()
            .map(str::trim)
            .is_some_and(|value| !value.is_empty())
    {
        return json_error(
            StatusCode::BAD_REQUEST,
            "repo_url 和 storage_node_id 不能同时指定：外部 Git 远端与平台硬盘节点二选一",
        );
    }

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
    let mut project = create_result.project;
    if requested_repo_url.is_some() || requested_branch.is_some() {
        project = match state.store.update_project_git_metadata(
            &user.id,
            &project.id,
            requested_repo_url.as_deref(),
            requested_branch.as_deref(),
        ) {
            Ok(project) => project,
            Err(e) => {
                if !reused_existing {
                    let _ = state.store.purge_project_records(&user.id, &project.id);
                }
                return json_error(StatusCode::BAD_REQUEST, e.to_string());
            }
        };
    }
    if reused_existing {
        if project
            .node_id
            .as_deref()
            .is_some_and(|value| !value.is_empty())
            && project
                .workspace_path
                .as_deref()
                .is_some_and(|value| !value.trim().is_empty())
        {
            let existing_node_id = project.node_id.clone();
            let archive_project = archive_project_payload(&state, &user.id, &project.id).await;
            return Json(serde_json::json!({
                "project": project,
                "archive_project": archive_project,
                "reused_existing": true,
                "node_id": existing_node_id,
                "provisioned": false,
            }))
            .into_response();
        }
        return json_error(
            StatusCode::CONFLICT,
            "同名项目已存在但尚未绑定 PC 工作区，请先绑定 PC 节点或更换项目名称",
        );
    }

    let node_id = match project_workspace_provision::resolve_pc_project_node(
        &state,
        &user.id,
        req.node_id.as_deref(),
    )
    .await
    {
        Ok(node_id) => node_id,
        // 节点暂时离线 → 保留项目记录，返回 pending 状态。
        // 用户可以先进入项目，等节点上线后工作区会在首次发起任务时自动初始化。
        Err((StatusCode::SERVICE_UNAVAILABLE, _)) => {
            let archive_project = archive_project_payload(&state, &user.id, &project.id).await;
            return (
                StatusCode::ACCEPTED,
                Json(serde_json::json!({
                    "project": project,
                    "archive_project": archive_project,
                    "reused_existing": false,
                    "workspace_status": "pending",
                    "node_id": req.node_id,
                    "message": "项目已创建，PC 节点上线后工作区将自动初始化",
                })),
            )
                .into_response();
        }
        // 其他错误（如节点配置错误、不支持 CLI）→ 回滚项目记录，返回错误
        Err((status, message)) => {
            let _ = state.store.purge_project_records(&user.id, &project.id);
            return json_error(status, message);
        }
    };

    let mut provision_repo_url = project.repo_url.clone();
    let mut provision_branch = project.branch.clone();
    let mut storage_repo_created = None;
    let mut local_storage_clone_path = None;
    if provision_repo_url
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .is_none()
    {
        let prepared_storage = match project_storage::maybe_prepare_project_storage_repo(
            &state,
            &user.id,
            &project.id,
            &project.name,
            project.branch.as_deref(),
            req.storage_node_id.as_deref(),
            Some(&node_id),
        )
        .await
        {
            Ok(repo) => repo,
            Err((status, message)) => {
                let _ = state.store.purge_project_records(&user.id, &project.id);
                return json_error(status, message);
            }
        };
        if let Some(storage) = prepared_storage {
            let clone_url = match project_storage::clone_url_for_prepared_storage(
                &storage, &node_id,
            ) {
                Some(url) => url,
                None => {
                    let _ = state.store.purge_project_records(&user.id, &project.id);
                    return json_error(
                        StatusCode::BAD_REQUEST,
                        "硬盘节点已创建项目仓库，但没有可跨 PC clone 的 Git 地址。请在硬盘节点管理页配置 Git 服务基础地址，或选择同一台 PC 同时作为硬盘和计算节点。",
                    );
                }
            };
            if storage.storage_repo_url.is_none() && clone_url == storage.storage_repo_path {
                local_storage_clone_path = Some(clone_url.clone());
            }
            storage_repo_created = Some(storage.created);
            project = match state.store.bind_project_storage_repo(
                &user.id,
                &project.id,
                &storage.node_id,
                &storage.storage_repo_path,
                storage.storage_repo_url.as_deref(),
                storage.branch.as_deref(),
            ) {
                Ok(project) => project,
                Err(e) => {
                    let _ = state.store.purge_project_records(&user.id, &project.id);
                    return json_error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string());
                }
            };
            provision_repo_url = Some(clone_url);
            provision_branch = storage.branch.or(project.branch.clone());
        }
    }

    let provisioned = match project_workspace_provision::provision_project_workspace(
        &state,
        &node_id,
        &user.id,
        &project.id,
        &project.name,
        &project.template,
        provision_repo_url.as_deref(),
        provision_branch.as_deref(),
    )
    .await
    {
        Ok(workspace) => workspace,
        Err(e) => {
            let _ = state.store.purge_project_records(&user.id, &project.id);
            return json_error(
                StatusCode::SERVICE_UNAVAILABLE,
                format!("PC 节点创建项目工作区失败：{e}"),
            );
        }
    };

    let persisted_remote_origin = provisioned
        .git_remote_origin
        .as_deref()
        .filter(|origin| Some(*origin) != local_storage_clone_path.as_deref())
        .or(project.repo_url.as_deref());
    let project = match state.store.bind_project_to_pc_workspace(
        &user.id,
        &project.id,
        &provisioned.workspace_path,
        &node_id,
        provisioned.git_head.as_deref(),
        persisted_remote_origin,
        provisioned
            .git_branch
            .as_deref()
            .or(provision_branch.as_deref())
            .or(project.branch.as_deref()),
    ) {
        Ok(project) => project,
        Err(e) => {
            let _ = state.store.purge_project_records(&user.id, &project.id);
            return json_error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string());
        }
    };

    let storage_node_id = project.storage_node_id.clone();
    let archive_project = archive_project_payload(&state, &user.id, &project.id).await;
    Json(serde_json::json!({
        "project": project,
        "archive_project": archive_project,
        "reused_existing": false,
        "node_id": node_id,
        "storage_node_id": storage_node_id,
        "provisioned": true,
        "workspace_created": provisioned.created,
        "storage_repo_created": storage_repo_created,
    }))
    .into_response()
}

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
    let repo_url = clean_optional_string(req.repo_url.as_deref());
    let branch = clean_optional_string(req.branch.as_deref());

    let node_id = match resolve_external_project_node(
        &state,
        &user.id,
        req.node_id.as_deref(),
        workspace_path,
    )
    .await
    {
        Ok(node_id) => node_id,
        Err((status, message)) => return json_error(status, message),
    };

    // 仅服务器本机项目需要校验服务端路径；PC 节点项目的路径在用户 PC 上。
    if node_id.is_none() {
        let pb = std::path::Path::new(workspace_path);
        if !pb.exists() {
            return json_error(
                StatusCode::BAD_REQUEST,
                format!("workspace_path 不存在: {}", workspace_path),
            );
        }
        if !pb.is_dir() {
            return json_error(StatusCode::BAD_REQUEST, "workspace_path 必须指向一个目录");
        }
    }

    let create_result = match state.store.register_external_project(
        &user.id,
        req.project_id.as_deref(),
        &req.name,
        req.description.as_deref(),
        workspace_path,
        node_id.as_deref(),
        repo_url.as_deref(),
        branch.as_deref(),
    ) {
        Ok(r) => r,
        Err(e) => return json_error(StatusCode::BAD_REQUEST, e.to_string()),
    };

    let join_mode = req.join_mode.as_deref().unwrap_or("readonly").trim();
    if !["open", "approval", "invite", "readonly"].contains(&join_mode) {
        return json_error(
            StatusCode::BAD_REQUEST,
            "join_mode 必须为 open / approval / invite / readonly",
        );
    }
    if req.is_public.unwrap_or(false) {
        if let Err(e) =
            state
                .store
                .set_project_visibility(&create_result.project.id, true, join_mode)
        {
            return json_error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string());
        }
    }

    Json(serde_json::json!({
        "project": create_result.project,
        "reused_existing": create_result.reused_existing,
        "node_id": node_id,
        "is_public": req.is_public.unwrap_or(false),
        "join_mode": if req.is_public.unwrap_or(false) { join_mode } else { "invite" },
    }))
    .into_response()
}

async fn resolve_external_project_node(
    state: &AppState,
    user_id: &str,
    requested_node_id: Option<&str>,
    workspace_path: &str,
) -> Result<Option<String>, (StatusCode, String)> {
    let requested_node_id = requested_node_id
        .map(str::trim)
        .filter(|value| !value.is_empty());
    if let Some(node_id) = requested_node_id {
        ensure_node_belongs_to_user_or_legacy(state, user_id, node_id).await?;
        return Ok(Some(node_id.to_string()));
    }

    let server_path = std::path::Path::new(workspace_path);
    if server_path.exists() {
        return Ok(None);
    }

    let cli_nodes: Vec<_> = state
        .agent_manager
        .list()
        .await
        .into_iter()
        .filter(|agent| {
            agent
                .allowed_clis
                .iter()
                .any(|cli| cli.eq_ignore_ascii_case("copilot") || cli.eq_ignore_ascii_case("codex"))
        })
        .collect();

    let mut owned = Vec::new();
    for agent in &cli_nodes {
        if matches!(
            state.store.get_node_credential_owner(&agent.agent_id),
            Ok(Some(owner)) if owner == user_id
        ) {
            owned.push(agent.agent_id.clone());
        }
    }

    match owned.len() {
        1 => return Ok(owned.into_iter().next()),
        n if n > 1 => {
            return Err((
                StatusCode::CONFLICT,
                "检测到多个属于你的在线 PC CLI 节点，请在请求中指定 node_id".into(),
            ));
        }
        _ => {}
    }

    match cli_nodes.len() {
        1 => Ok(Some(cli_nodes[0].agent_id.clone())),
        0 => Err((
            StatusCode::BAD_REQUEST,
            format!(
                "workspace_path 不存在于服务器本机，且当前没有在线 PC CLI 节点可接管: {}",
                workspace_path
            ),
        )),
        _ => Err((
            StatusCode::CONFLICT,
            "检测到多个在线 PC CLI 节点，请在请求中指定 node_id".into(),
        )),
    }
}

async fn ensure_node_belongs_to_user_or_legacy(
    state: &AppState,
    user_id: &str,
    node_id: &str,
) -> Result<(), (StatusCode, String)> {
    match state.store.get_node_credential_owner(node_id) {
        Ok(Some(owner)) if owner == user_id => Ok(()),
        Ok(Some(_)) => Err((StatusCode::FORBIDDEN, "该 PC 节点不属于当前用户".into())),
        Ok(None) => {
            let connected = state
                .agent_manager
                .list()
                .await
                .into_iter()
                .any(|agent| agent.agent_id == node_id);
            if connected {
                Ok(())
            } else {
                Err((
                    StatusCode::NOT_FOUND,
                    format!("PC 节点未注册或不在线: {}", node_id),
                ))
            }
        }
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
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
