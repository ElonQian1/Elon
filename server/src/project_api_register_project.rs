use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use std::sync::Arc;
use crate::{
    project_auth::{auth_from_headers, json_error},
    project_landing,
    types::AppState,
};
use super::{RegisterExternalProjectRequest, clean_optional_string};

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

    let dev_profile = match req.dev_profile.as_ref() {
        Some(profile) => match state.store.upsert_project_dev_profile(
            &user.id,
            &create_result.project.id,
            profile,
        ) {
            Ok(profile) => profile,
            Err(error) => {
                return json_error(StatusCode::INTERNAL_SERVER_ERROR, error.to_string());
            }
        },
        None => match state
            .store
            .get_project_dev_profile_for_user(&user.id, &create_result.project.id)
        {
            Ok(profile) => profile,
            Err(error) => {
                tracing::warn!(
                    project_id = %create_result.project.id,
                    "读取项目开发命令 profile 失败: {error}"
                );
                None
            }
        },
    };

    let workspace_landing = if req.landing.is_none() && node_id.is_none() {
        project_landing::load_workspace_landing(std::path::Path::new(workspace_path))
    } else {
        None
    };
    let landing_snapshot = req
        .landing
        .as_ref()
        .or(workspace_landing.as_ref())
        .and_then(|landing| {
            match state.store.update_project_landing_snapshot(
                &user.id,
                &create_result.project.id,
                landing,
            ) {
                Ok(snapshot) => snapshot,
                Err(error) => {
                    tracing::warn!(
                        project_id = %create_result.project.id,
                        "保存项目首页 landing 快照失败: {error}"
                    );
                    None
                }
            }
        });

    let join_mode = req.join_mode.as_deref().unwrap_or("readonly").trim();
    if !["open", "approval", "invite", "readonly"].contains(&join_mode) {
        return json_error(
            StatusCode::BAD_REQUEST,
            "join_mode 必须为 open / approval / invite / readonly",
        );
    }
    let is_elon_self = create_result.project.id == "elon-self";
    let effective_is_public = req.is_public.unwrap_or(false) || is_elon_self;
    let effective_join_mode = if is_elon_self { "approval" } else { join_mode };
    if effective_is_public {
        if let Err(e) =
            state
                .store
                .set_project_visibility(&create_result.project.id, true, effective_join_mode)
        {
            return json_error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string());
        }
    }

    Json(serde_json::json!({
        "project": create_result.project,
        "reused_existing": create_result.reused_existing,
        "node_id": node_id,
        "is_public": effective_is_public,
        "join_mode": if effective_is_public { effective_join_mode } else { "invite" },
        "landing": landing_snapshot,
        "dev_profile": dev_profile,
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

