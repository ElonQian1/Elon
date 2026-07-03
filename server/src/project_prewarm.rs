//! 项目 Codex CLI 原生 session 预热接口（从 project_chat.rs 抽出）。
//!
//! 提供两个 axum HTTP handler：
//! - `prewarm_project`：APK 内部接口，需要 `auth_from_headers` 鉴权。
//! - `prewarm_user_project`：移动端外部接口，使用 `ensure_mobile_project` 自动建库。
//!
//! 二者共享 `prewarm_project_response` 实际执行预热逻辑。

use axum::{
    extract::{Path as AxumPath, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use std::{collections::HashMap, sync::Arc, time::Duration};

use crate::{
    agent,
    agent_routing::is_local_cli_option,
    ai_cli,
    project_auth::{auth_from_headers, can_edit, json_error, project_access},
    project_conversation_workspace::{
        prepare_project_conversation_workspace, ProjectConversationWorkspace,
    },
    project_keys::{clean_trace_id, codex_prewarm_key},
    project_mobile::ensure_mobile_project,
    project_ws_protocol::ProjectPrewarmRequest,
    store::{ProjectAccess, PublicUser},
    types::AppState,
};

pub async fn prewarm_project(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    AxumPath(project_id): AxumPath<String>,
    Json(req): Json<ProjectPrewarmRequest>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(user) => user,
        Err(e) => return json_error(StatusCode::UNAUTHORIZED, e.to_string()),
    };
    let project = match project_access(&state, &user.id, &project_id) {
        Ok(project) => project,
        Err(e) => return json_error(StatusCode::FORBIDDEN, e.to_string()),
    };
    prewarm_project_response(state, user, project, req).await
}

pub async fn prewarm_user_project(
    State(state): State<Arc<AppState>>,
    AxumPath((user_id, project_id)): AxumPath<(String, String)>,
    Query(query): Query<HashMap<String, String>>,
    Json(req): Json<ProjectPrewarmRequest>,
) -> Response {
    let (user, project) = match ensure_mobile_project(
        &state,
        &user_id,
        &project_id,
        query.get("title").map(String::as_str),
    ) {
        Ok(pair) => pair,
        Err(e) => return json_error(StatusCode::BAD_REQUEST, e.to_string()),
    };
    prewarm_project_response(state, user, project, req).await
}

async fn prewarm_project_response(
    state: Arc<AppState>,
    user: PublicUser,
    project: ProjectAccess,
    req: ProjectPrewarmRequest,
) -> Response {
    if !can_edit(&project.role) {
        return json_error(
            StatusCode::FORBIDDEN,
            "current user cannot edit this project",
        );
    }
    if let Err(msg) = crate::billing::check_can_call(&state.store, &user.id) {
        return json_error(StatusCode::PAYMENT_REQUIRED, msg);
    }

    let conversation_id = match state.store.ensure_conversation(
        &project.id,
        &user.id,
        req.conversation_id.as_deref(),
        req.conversation_title.as_deref(),
    ) {
        Ok(id) => id,
        Err(e) => return json_error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    };

    let trace_id = req
        .trace_id
        .as_deref()
        .map(|value| clean_trace_id(Some(value)))
        .filter(|value| !value.is_empty());
    let requested_agent = req
        .agent
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned);

    if should_use_route_a_runtime_prewarm(&project) {
        if let Some(trace_id) = trace_id.as_deref() {
            state.server_traces.record(
                trace_id,
                "route_a_runtime_prewarm_request",
                serde_json::json!({
                    "project_id": &project.id,
                    "user_id": &user.id,
                    "conversation_id": &conversation_id,
                    "node_id": project.node_id.as_deref(),
                }),
            );
        }
        if let Some(result) =
            agent::prewarm_route_a_runtime_for_project(&state, &user.id, &project, &conversation_id)
                .await
        {
            if let Some(trace_id) = trace_id.as_deref() {
                state.server_traces.record(
                    trace_id,
                    "route_a_runtime_prewarm_ready",
                    serde_json::json!({
                        "project_id": &project.id,
                        "conversation_id": &conversation_id,
                        "agent_id": &result.agent_id,
                        "workspace": &result.workspace,
                        "reused": result.reused,
                    }),
                );
            }
            return Json(serde_json::json!({
                "status": "accepted",
                "mode": "route_a_runtime",
                "project_id": project.id,
                "conversation_id": conversation_id,
                "agent_id": result.agent_id,
                "workspace": result.workspace,
                "lease_ttl_secs": result.ttl_secs,
                "reused": result.reused,
            }))
            .into_response();
        }
        if let Some(trace_id) = trace_id.as_deref() {
            state.server_traces.record(
                trace_id,
                "route_a_runtime_prewarm_skipped",
                serde_json::json!({
                    "reason": "pc_binding_unavailable",
                    "project_id": &project.id,
                    "conversation_id": &conversation_id,
                }),
            );
        }
        return Json(serde_json::json!({
            "status": "skipped",
            "mode": "route_a_runtime",
            "reason": "pc_binding_unavailable",
            "project_id": project.id,
            "conversation_id": conversation_id,
        }))
        .into_response();
    }

    let base_workspace =
        state.resolve_project_workspace(&project.workspace_key, project.workspace_path.as_deref());
    let conversation_workspace =
        match prepare_project_conversation_workspace(&state, &project, &conversation_id) {
            Ok(workspace) => workspace,
            Err(error) => {
                tracing::warn!(
                    project_id = %project.id,
                    conversation_id = %conversation_id,
                    error = %error,
                    "conversation worktree prewarm fell back to base workspace"
                );
                ProjectConversationWorkspace::shared(base_workspace.clone())
            }
        };
    let workspace = conversation_workspace.active_path().to_path_buf();
    let agent = if state.ai_cli.codex_cli_only {
        requested_agent
            .as_deref()
            .filter(|name| is_local_cli_option(&state, name))
            .map(ToOwned::to_owned)
    } else {
        requested_agent
    };
    let workspace_key = workspace.display().to_string();
    let throttle_key = codex_prewarm_key(
        &project.id,
        &user.id,
        &conversation_id,
        agent.as_deref(),
        &workspace_key,
    );
    if let Some(trace_id) = trace_id.as_deref() {
        state.server_traces.record(
            trace_id,
            "codex_prewarm_request",
            serde_json::json!({
                "project_id": &project.id,
                "user_id": &user.id,
                "conversation_id": &conversation_id,
                "workspace": &workspace_key,
                "agent": agent.as_deref(),
            }),
        );
    }
    if !state
        .codex_prewarm
        .start_if_allowed(&throttle_key, Duration::from_secs(120))
        .await
    {
        if let Some(trace_id) = trace_id.as_deref() {
            state.server_traces.record(
                trace_id,
                "codex_prewarm_skipped",
                serde_json::json!({
                    "reason": "cooldown",
                    "project_id": &project.id,
                    "conversation_id": &conversation_id,
                }),
            );
        }
        return Json(serde_json::json!({
            "status": "skipped",
            "reason": "cooldown",
            "project_id": project.id,
            "conversation_id": conversation_id,
        }))
        .into_response();
    }

    let scope = ai_cli::NativeSessionScope {
        project_id: project.id.clone(),
        user_id: user.id.clone(),
        conversation_id: conversation_id.clone(),
        runtime_permission: project.runtime_permission.clone(),
    };
    let state_for_task = state.clone();
    let workspace_for_task = workspace.clone();
    let agent_for_task = agent.clone();
    let project_id_for_log = project.id.clone();
    let conversation_id_for_log = conversation_id.clone();
    let prewarm_key_for_task = throttle_key.clone();
    let trace_id_for_task = trace_id.clone();
    tokio::spawn(async move {
        match ai_cli::prewarm_codex_session(
            &workspace_for_task,
            agent_for_task.as_deref(),
            scope,
            trace_id_for_task.as_deref(),
            &state_for_task,
        )
        .await
        {
            Ok(result) => tracing::info!(
                project_id = %project_id_for_log,
                conversation_id = %conversation_id_for_log,
                reused = result.reused,
                thread_id = ?result.thread_id,
                elapsed_ms = result.elapsed_ms,
                "Codex CLI session prewarm completed"
            ),
            Err(error) => tracing::warn!(
                project_id = %project_id_for_log,
                conversation_id = %conversation_id_for_log,
                error = %error,
                "Codex CLI session prewarm failed"
            ),
        }
        let accepted = state_for_task
            .codex_prewarm
            .finish(&prewarm_key_for_task)
            .await;
        if !accepted {
            tracing::info!(
                project_id = %project_id_for_log,
                conversation_id = %conversation_id_for_log,
                "Codex CLI session prewarm finished after real request started"
            );
        }
    });

    Json(serde_json::json!({
        "status": "accepted",
        "project_id": project.id,
        "conversation_id": conversation_id,
        "workspace": workspace_key,
    }))
    .into_response()
}

fn should_use_route_a_runtime_prewarm(project: &ProjectAccess) -> bool {
    project.source_type == "pc_managed"
        || project
            .node_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .is_some()
}
