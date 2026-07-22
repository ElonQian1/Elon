use std::sync::Arc;

use axum::{
    extract::{Path, Query, State, WebSocketUpgrade},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use serde_json::json;

use crate::NodeRuntime;

use super::adb_session::{start_runtime, stop_runtime, DEFAULT_DEVICE_PORT};
use super::build_verify::{
    build_and_verify, prepare_debug_runtime, BuildVerifyRequest, PrepareDebugRuntimeRequest,
};
use super::capability_gap::{check_capabilities, control_gap, get_gap};
use super::design_diff_regions::{analyze_session_design_diff, DesignDiffRegionRequest};
use super::frame::capture_frame;
use super::mcp::{
    cleanup_descriptor, descriptor as mcp_descriptor, descriptor_for_project,
    handle_request as handle_mcp_request, McpQuery, McpRequest,
};
use super::preview::{open_preview, PreviewOpenRequest};
use super::protocol::{
    LiveStylePatch, RuntimeSocketQuery, StartLiveSessionRequest, PROTOCOL_VERSION,
};
use super::source_commit::{build_source_commit_plan, commit_source, SourceCommitRequest};
use super::ui_ir::{
    bind_ui_ir, load_or_build_ui_ir, persist_target_design, BindUiIrRequest, TargetDesignUpload,
};
use super::visual_diff::{compare_images, VisualDiffRequest};
use super::visual_solver::{solve_visual_style, VisualSolverRequest};

pub(crate) fn protected_routes() -> Router<Arc<NodeRuntime>> {
    Router::new()
        .route(
            "/api/android-live/debug-runtime/prepare",
            post(prepare_debug_runtime_handler),
        )
        .route(
            "/api/android-live/debug-runtime/integration-status",
            get(debug_integration_status_handler),
        )
        .route("/api/android-live/sessions", post(create_session_handler))
        .route(
            "/api/android-live/sessions/:session_id",
            get(session_handler).delete(stop_session_handler),
        )
        .route(
            "/api/android-live/sessions/:session_id/tree",
            get(tree_handler),
        )
        .route(
            "/api/android-live/sessions/:session_id/frame",
            get(frame_handler),
        )
        .route(
            "/api/android-live/sessions/:session_id/reconnect",
            post(reconnect_session_handler),
        )
        .route(
            "/api/android-live/sessions/:session_id/preview",
            post(preview_handler),
        )
        .route(
            "/api/android-live/sessions/:session_id/patch",
            post(patch_handler),
        )
        .route(
            "/api/android-live/sessions/:session_id/undo",
            post(undo_handler),
        )
        .route(
            "/api/android-live/sessions/:session_id/redo",
            post(redo_handler),
        )
        .route(
            "/api/android-live/sessions/:session_id/commit-plan",
            get(commit_plan_handler),
        )
        .route(
            "/api/android-live/sessions/:session_id/commit",
            post(commit_source_handler),
        )
        .route(
            "/api/android-live/sessions/:session_id/build-verify",
            post(build_verify_handler),
        )
        .route(
            "/api/android-live/sessions/:session_id/ui-ir",
            get(ui_ir_handler).post(bind_ui_ir_handler),
        )
        .route(
            "/api/android-live/sessions/:session_id/target-design",
            post(target_design_handler),
        )
        .route(
            "/api/android-live/sessions/:session_id/design-diff-regions",
            post(design_diff_regions_handler),
        )
        .route(
            "/api/android-live/sessions/:session_id/visual-diff",
            post(visual_diff_handler),
        )
        .route(
            "/api/android-live/sessions/:session_id/visual-solver",
            post(visual_solver_handler),
        )
        .route(
            "/api/android-live/sessions/:session_id/mcp-descriptor",
            get(mcp_descriptor_handler),
        )
        .route(
            "/api/android-live/sessions/:session_id/capability-gaps",
            get(capability_gaps_handler),
        )
        .route(
            "/api/android-live/sessions/:session_id/capabilities",
            get(capabilities_handler),
        )
        .route(
            "/api/android-live/sessions/:session_id/capability-gaps/:gap_id",
            get(capability_gap_handler).post(capability_gap_command_handler),
        )
        .merge(super::fit_run::protected_routes())
}

async fn capabilities_handler(
    State(runtime): State<Arc<NodeRuntime>>,
    Path(session_id): Path<String>,
) -> Response {
    let session = match runtime.live_ui.session(&session_id).await {
        Ok(session) => session,
        Err(error) => return json_error(StatusCode::NOT_FOUND, format!("{error:#}")),
    };
    match check_capabilities(&session, &json!({})).await {
        Ok(result) => Json(json!({ "ok": true, "result": result })).into_response(),
        Err(error) => json_error(StatusCode::BAD_REQUEST, format!("{error:#}")),
    }
}

async fn capability_gaps_handler(
    State(runtime): State<Arc<NodeRuntime>>,
    Path(session_id): Path<String>,
) -> Response {
    let session = match runtime.live_ui.session(&session_id).await {
        Ok(session) => session,
        Err(error) => return json_error(StatusCode::NOT_FOUND, format!("{error:#}")),
    };
    match get_gap(&session, &json!({})) {
        Ok(result) => Json(json!({ "ok": true, "result": result })).into_response(),
        Err(error) => json_error(StatusCode::BAD_REQUEST, format!("{error:#}")),
    }
}

async fn capability_gap_handler(
    State(runtime): State<Arc<NodeRuntime>>,
    Path((session_id, gap_id)): Path<(String, String)>,
) -> Response {
    let session = match runtime.live_ui.session(&session_id).await {
        Ok(session) => session,
        Err(error) => return json_error(StatusCode::NOT_FOUND, format!("{error:#}")),
    };
    match get_gap(&session, &json!({ "gapId": gap_id })) {
        Ok(result) => Json(json!({ "ok": true, "result": result })).into_response(),
        Err(error) => json_error(StatusCode::NOT_FOUND, format!("{error:#}")),
    }
}

async fn capability_gap_command_handler(
    State(runtime): State<Arc<NodeRuntime>>,
    Path((session_id, gap_id)): Path<(String, String)>,
    Json(mut command): Json<serde_json::Value>,
) -> Response {
    let session = match runtime.live_ui.session(&session_id).await {
        Ok(session) => session,
        Err(error) => return json_error(StatusCode::NOT_FOUND, format!("{error:#}")),
    };
    command["gapId"] = json!(gap_id);
    match control_gap(&session, &command) {
        Ok(result) => Json(json!({ "ok": true, "result": result })).into_response(),
        Err(error) => json_error(StatusCode::BAD_REQUEST, format!("{error:#}")),
    }
}

async fn prepare_debug_runtime_handler(
    State(runtime): State<Arc<NodeRuntime>>,
    Json(mut request): Json<PrepareDebugRuntimeRequest>,
) -> Response {
    if let Err(error) = crate::node_agent_android_device_lease::validate_android_device_lease(
        &runtime,
        request.lease.as_ref(),
    )
    .await
    {
        return json_error(StatusCode::CONFLICT, format!("{error:#}"));
    }
    let host_port = crate::node_agent_admin_open::admin_port_from_env();
    match prepare_debug_runtime(&runtime.live_ui, request, host_port).await {
        Ok(result) => Json(json!({ "ok": true, "result": result })).into_response(),
        Err(error) => json_error(StatusCode::BAD_REQUEST, format!("{error:#}")),
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DebugIntegrationStatusQuery {
    project_root: String,
    project_id: String,
    device_identity: String,
}

async fn debug_integration_status_handler(
    State(runtime): State<Arc<NodeRuntime>>,
    Query(query): Query<DebugIntegrationStatusQuery>,
) -> Response {
    match runtime.live_ui.debug_integration.status_for(
        &query.project_root,
        &query.project_id,
        &query.device_identity,
    ) {
        Ok(status) => Json(json!({ "ok": true, "status": status })).into_response(),
        Err(error) => json_error(StatusCode::BAD_REQUEST, format!("{error:#}")),
    }
}

pub(crate) fn runtime_routes() -> Router<Arc<NodeRuntime>> {
    Router::new()
        .route("/api/android-live/runtime", get(runtime_socket_handler))
        .route(
            "/api/android-live/project-mcp/bootstrap",
            post(project_mcp_bootstrap_handler),
        )
        .route("/api/android-live/mcp/:session_id", post(mcp_handler))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProjectMcpBootstrapRequest {
    project_root: String,
}

/// Codex 桌面端的仓库级 STDIO bridge 只通过 loopback 调用此入口。
///
/// 浏览器仍受本机 CORS 限制；这里不返回本机管理员令牌，只签发一个短生命周期
/// Android Live MCP 会话令牌。调用方还必须证明目标是现存 Git 工作区，避免把
/// 本机节点变成任意目录浏览器。
async fn project_mcp_bootstrap_handler(
    State(runtime): State<Arc<NodeRuntime>>,
    Json(request): Json<ProjectMcpBootstrapRequest>,
) -> Response {
    let project_root = match std::path::PathBuf::from(request.project_root.trim()).canonicalize() {
        Ok(root) if root.join(".git").exists() => root,
        Ok(_) => return json_error(StatusCode::BAD_REQUEST, "projectRoot 不是 Git 工作区"),
        Err(error) => {
            return json_error(
                StatusCode::BAD_REQUEST,
                format!("projectRoot 不存在或不可访问: {error}"),
            )
        }
    };
    let project_root = project_root.to_string_lossy().to_string();
    let host_port = crate::node_agent_admin_open::admin_port_from_env();
    match descriptor_for_project(&runtime.live_ui, &project_root, host_port).await {
        Ok(Some(mcp)) => Json(json!({
            "ok": true,
            "projectRoot": project_root,
            "mcp": mcp,
        }))
        .into_response(),
        Ok(None) => json_error(StatusCode::SERVICE_UNAVAILABLE, "无法创建项目 UI MCP 会话"),
        Err(error) => json_error(StatusCode::BAD_REQUEST, format!("{error:#}")),
    }
}

async fn create_session_handler(
    State(runtime): State<Arc<NodeRuntime>>,
    Json(req): Json<StartLiveSessionRequest>,
) -> Response {
    if let Err(error) = crate::node_agent_android_device_lease::validate_android_device_lease(
        &runtime,
        req.lease.as_ref(),
    )
    .await
    {
        return json_error(StatusCode::CONFLICT, format!("{error:#}"));
    }
    let device_id = req.device_id.trim().to_string();
    let package_name = match super::normalize_debug_package_name(
        req.package_name.trim(),
        &runtime.install_id,
        &device_id,
    ) {
        Ok(package) => package,
        Err(error) => return json_error(StatusCode::BAD_REQUEST, format!("{error:#}")),
    };
    let project_root = req
        .project_root
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let device_identity = req
        .lease
        .as_ref()
        .map(|lease| lease.hardware_serial.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| device_id.clone());
    let debug_project_id = req
        .lease
        .as_ref()
        .map(|lease| lease.project_id.trim().to_string())
        .filter(|value| !value.is_empty())
        .or_else(|| project_root.clone())
        .unwrap_or_default();
    if device_id.is_empty() || package_name.is_empty() {
        return json_error(StatusCode::BAD_REQUEST, "deviceId 和 packageName 不能为空");
    }
    let session = runtime
        .live_ui
        .create_session_with_identity(
            device_id,
            device_identity,
            package_name,
            project_root,
            debug_project_id,
            DEFAULT_DEVICE_PORT,
        )
        .await;
    let host_port = crate::node_agent_admin_open::admin_port_from_env();
    match start_runtime(&session, host_port).await {
        Ok(output) => match mcp_descriptor(&session, host_port) {
            Ok(mcp) => Json(json!({
                "ok": true,
                "protocolVersion": PROTOCOL_VERSION,
                "session": session.view().await,
                "mcp": mcp,
                "adbOutput": output,
            }))
            .into_response(),
            Err(error) => {
                let _ = stop_runtime(&session).await;
                runtime.live_ui.remove_session(&session.id).await;
                json_error(StatusCode::INTERNAL_SERVER_ERROR, format!("{error:#}"))
            }
        },
        Err(error) => {
            let _ = stop_runtime(&session).await;
            runtime.live_ui.remove_session(&session.id).await;
            json_error(StatusCode::BAD_GATEWAY, format!("{error:#}"))
        }
    }
}

async fn reconnect_session_handler(
    State(runtime): State<Arc<NodeRuntime>>,
    Path(session_id): Path<String>,
) -> Response {
    let session = match runtime.live_ui.session(&session_id).await {
        Ok(session) => session,
        Err(error) => return json_error(StatusCode::NOT_FOUND, format!("{error:#}")),
    };
    let host_port = crate::node_agent_admin_open::admin_port_from_env();
    match start_runtime(&session, host_port).await {
        Ok(output) => Json(json!({
            "ok": true,
            "session": session.view().await,
            "adbOutput": output,
        }))
        .into_response(),
        Err(error) => json_error(StatusCode::BAD_GATEWAY, format!("{error:#}")),
    }
}

async fn ui_ir_handler(
    State(runtime): State<Arc<NodeRuntime>>,
    Path(session_id): Path<String>,
) -> Response {
    match load_or_build_ui_ir(&runtime.live_ui, &session_id).await {
        Ok(document) => Json(json!({ "ok": true, "document": document })).into_response(),
        Err(error) => json_error(StatusCode::BAD_REQUEST, format!("{error:#}")),
    }
}

async fn bind_ui_ir_handler(
    State(runtime): State<Arc<NodeRuntime>>,
    Path(session_id): Path<String>,
    Json(request): Json<BindUiIrRequest>,
) -> Response {
    match bind_ui_ir(&runtime.live_ui, &session_id, request).await {
        Ok(document) => Json(json!({ "ok": true, "document": document })).into_response(),
        Err(error) => json_error(StatusCode::BAD_REQUEST, format!("{error:#}")),
    }
}

async fn target_design_handler(
    State(runtime): State<Arc<NodeRuntime>>,
    Path(session_id): Path<String>,
    Json(upload): Json<TargetDesignUpload>,
) -> Response {
    match persist_target_design(&runtime.live_ui, &session_id, upload).await {
        Ok(target) => Json(json!({ "ok": true, "targetDesign": target })).into_response(),
        Err(error) => json_error(StatusCode::BAD_REQUEST, format!("{error:#}")),
    }
}

async fn design_diff_regions_handler(
    State(runtime): State<Arc<NodeRuntime>>,
    Path(session_id): Path<String>,
    Json(request): Json<DesignDiffRegionRequest>,
) -> Response {
    match analyze_session_design_diff(&runtime.live_ui, &session_id, request).await {
        Ok(analysis) => Json(json!({ "ok": true, "analysis": analysis })).into_response(),
        Err(error) => json_error(StatusCode::BAD_REQUEST, format!("{error:#}")),
    }
}

async fn visual_diff_handler(
    State(runtime): State<Arc<NodeRuntime>>,
    Path(session_id): Path<String>,
    Json(request): Json<VisualDiffRequest>,
) -> Response {
    if let Err(error) = runtime.live_ui.session(&session_id).await {
        return json_error(StatusCode::NOT_FOUND, format!("{error:#}"));
    }
    match tokio::task::spawn_blocking(move || compare_images(&request)).await {
        Ok(Ok(diff)) => Json(json!({ "ok": true, "diff": diff })).into_response(),
        Ok(Err(error)) => json_error(StatusCode::BAD_REQUEST, format!("{error:#}")),
        Err(error) => json_error(StatusCode::INTERNAL_SERVER_ERROR, format!("{error:#}")),
    }
}

async fn visual_solver_handler(
    State(runtime): State<Arc<NodeRuntime>>,
    Path(session_id): Path<String>,
    Json(request): Json<VisualSolverRequest>,
) -> Response {
    match solve_visual_style(&runtime.live_ui, &session_id, request).await {
        Ok(result) => Json(json!({ "ok": true, "result": result })).into_response(),
        Err(error) => json_error(StatusCode::BAD_REQUEST, format!("{error:#}")),
    }
}

async fn mcp_descriptor_handler(
    State(runtime): State<Arc<NodeRuntime>>,
    Path(session_id): Path<String>,
) -> Response {
    match runtime.live_ui.session(&session_id).await {
        Ok(session) => {
            let host_port = crate::node_agent_admin_open::admin_port_from_env();
            match mcp_descriptor(&session, host_port) {
                Ok(mcp) => Json(json!({ "ok": true, "mcp": mcp })).into_response(),
                Err(error) => json_error(StatusCode::INTERNAL_SERVER_ERROR, format!("{error:#}")),
            }
        }
        Err(error) => json_error(StatusCode::NOT_FOUND, format!("{error:#}")),
    }
}

async fn mcp_handler(
    State(runtime): State<Arc<NodeRuntime>>,
    Path(session_id): Path<String>,
    Query(query): Query<McpQuery>,
    Json(request): Json<McpRequest>,
) -> Response {
    if let Err(error) = runtime
        .live_ui
        .authorize_session(&session_id, &query.token)
        .await
    {
        return json_error(StatusCode::UNAUTHORIZED, format!("{error:#}"));
    }
    match handle_mcp_request(&runtime.live_ui, &runtime.ui_fit_runs, &session_id, request).await {
        Some(response) => Json(response).into_response(),
        None => StatusCode::ACCEPTED.into_response(),
    }
}

async fn session_handler(
    State(runtime): State<Arc<NodeRuntime>>,
    Path(session_id): Path<String>,
) -> Response {
    match runtime.live_ui.session_view(&session_id).await {
        Ok(session) => Json(json!({ "ok": true, "session": session })).into_response(),
        Err(error) => json_error(StatusCode::NOT_FOUND, format!("{error:#}")),
    }
}

async fn tree_handler(
    State(runtime): State<Arc<NodeRuntime>>,
    Path(session_id): Path<String>,
) -> Response {
    match runtime.live_ui.tree(&session_id).await {
        Ok((tree_revision, nodes)) => Json(json!({
            "ok": true,
            "protocolVersion": PROTOCOL_VERSION,
            "treeRevision": tree_revision,
            "nodes": nodes,
        }))
        .into_response(),
        Err(error) => json_error(StatusCode::NOT_FOUND, format!("{error:#}")),
    }
}

async fn frame_handler(
    State(runtime): State<Arc<NodeRuntime>>,
    Path(session_id): Path<String>,
) -> Response {
    let session = match runtime.live_ui.session(&session_id).await {
        Ok(session) => session,
        Err(error) => return json_error(StatusCode::NOT_FOUND, format!("{error:#}")),
    };
    match capture_frame(&session).await {
        Ok(frame) => Json(json!({ "ok": true, "frame": frame })).into_response(),
        Err(error) => json_error(StatusCode::BAD_GATEWAY, format!("{error:#}")),
    }
}

async fn preview_handler(
    State(runtime): State<Arc<NodeRuntime>>,
    Path(session_id): Path<String>,
    Json(request): Json<PreviewOpenRequest>,
) -> Response {
    let session = match runtime.live_ui.session(&session_id).await {
        Ok(session) => session,
        Err(error) => return json_error(StatusCode::NOT_FOUND, format!("{error:#}")),
    };
    match open_preview(&session, request).await {
        Ok(result) => Json(json!({ "ok": true, "preview": result })).into_response(),
        Err(error) => json_error(StatusCode::BAD_REQUEST, format!("{error:#}")),
    }
}

async fn patch_handler(
    State(runtime): State<Arc<NodeRuntime>>,
    Path(session_id): Path<String>,
    Json(patch): Json<LiveStylePatch>,
) -> Response {
    match runtime.live_ui.apply_patch(&session_id, patch).await {
        Ok(ack) => Json(json!({ "ok": true, "ack": ack })).into_response(),
        Err(error) => json_error(StatusCode::BAD_REQUEST, format!("{error:#}")),
    }
}

async fn undo_handler(
    State(runtime): State<Arc<NodeRuntime>>,
    Path(session_id): Path<String>,
) -> Response {
    match runtime.live_ui.undo(&session_id).await {
        Ok(ack) => Json(json!({ "ok": true, "ack": ack })).into_response(),
        Err(error) => json_error(StatusCode::BAD_REQUEST, format!("{error:#}")),
    }
}

async fn redo_handler(
    State(runtime): State<Arc<NodeRuntime>>,
    Path(session_id): Path<String>,
) -> Response {
    match runtime.live_ui.redo(&session_id).await {
        Ok(ack) => Json(json!({ "ok": true, "ack": ack })).into_response(),
        Err(error) => json_error(StatusCode::BAD_REQUEST, format!("{error:#}")),
    }
}

async fn commit_plan_handler(
    State(runtime): State<Arc<NodeRuntime>>,
    Path(session_id): Path<String>,
) -> Response {
    let session = match runtime.live_ui.session(&session_id).await {
        Ok(session) => session,
        Err(error) => return json_error(StatusCode::NOT_FOUND, format!("{error:#}")),
    };
    match build_source_commit_plan(session).await {
        Ok(plan) => Json(json!({ "ok": true, "plan": plan })).into_response(),
        Err(error) => json_error(StatusCode::BAD_REQUEST, format!("{error:#}")),
    }
}

async fn commit_source_handler(
    State(runtime): State<Arc<NodeRuntime>>,
    Path(session_id): Path<String>,
    Json(request): Json<SourceCommitRequest>,
) -> Response {
    let session = match runtime.live_ui.session(&session_id).await {
        Ok(session) => session,
        Err(error) => return json_error(StatusCode::NOT_FOUND, format!("{error:#}")),
    };
    match commit_source(session, request).await {
        Ok(result) => Json(json!({ "ok": true, "result": result })).into_response(),
        Err(error) => json_error(StatusCode::CONFLICT, format!("{error:#}")),
    }
}

async fn build_verify_handler(
    State(runtime): State<Arc<NodeRuntime>>,
    Path(session_id): Path<String>,
    Json(request): Json<BuildVerifyRequest>,
) -> Response {
    let host_port = crate::node_agent_admin_open::admin_port_from_env();
    match build_and_verify(&runtime.live_ui, &session_id, request, host_port).await {
        Ok(result) => Json(json!({ "ok": true, "result": result })).into_response(),
        Err(error) => json_error(StatusCode::BAD_REQUEST, format!("{error:#}")),
    }
}

async fn stop_session_handler(
    State(runtime): State<Arc<NodeRuntime>>,
    Path(session_id): Path<String>,
) -> Response {
    let Some(session) = runtime.live_ui.remove_session(&session_id).await else {
        return json_error(StatusCode::NOT_FOUND, "Live UI 会话不存在或已结束");
    };
    if let Err(error) = stop_runtime(&session).await {
        return json_error(StatusCode::BAD_GATEWAY, format!("{error:#}"));
    }
    cleanup_descriptor(&session_id);
    Json(json!({ "ok": true, "stopped": session_id })).into_response()
}

async fn runtime_socket_handler(
    State(runtime): State<Arc<NodeRuntime>>,
    Query(query): Query<RuntimeSocketQuery>,
    upgrade: WebSocketUpgrade,
) -> Response {
    let broker = runtime.live_ui.clone();
    let session_id = query.session_id;
    let token = query.token;
    if broker.session(&session_id).await.is_err() {
        return json_error(StatusCode::NOT_FOUND, "Live UI 会话不存在或已结束");
    }
    upgrade
        .on_upgrade(move |socket| async move {
            if let Err(error) = broker.attach_runtime(&session_id, &token, socket).await {
                tracing::warn!(session_id, error = %error, "Android Live Runtime 连接结束");
            }
        })
        .into_response()
}

fn json_error(status: StatusCode, error: impl Into<String>) -> Response {
    (status, Json(json!({ "ok": false, "error": error.into() }))).into_response()
}
