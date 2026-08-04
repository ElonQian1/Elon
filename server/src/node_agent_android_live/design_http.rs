use std::sync::Arc;

use anyhow::{anyhow, Context, Result};
use axum::{
    body::Body,
    extract::{Path, State},
    http::{header, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::post,
    Json, Router,
};
use serde_json::{json, Value};

use crate::NodeRuntime;

use super::broker::LiveUiSession;

pub(super) fn routes() -> Router<Arc<NodeRuntime>> {
    Router::new()
        .route("/api/android-live/design/targets", post(list_targets))
        .route(
            "/api/android-live/design/capabilities",
            post(get_capabilities),
        )
        .route(
            "/api/android-live/design/sessions/list",
            post(list_sessions),
        )
        .route("/api/android-live/design/sessions", post(open_session))
        .route(
            "/api/android-live/design/sessions/:design_session_id/capture",
            post(capture_session),
        )
        .route(
            "/api/android-live/design/sessions/:design_session_id/surface",
            post(get_surface),
        )
        .route(
            "/api/android-live/design/sessions/:design_session_id/artifact",
            post(get_artifact),
        )
        .route(
            "/api/android-live/design/sessions/:design_session_id/browser/prepare",
            post(prepare_browser),
        )
        .route(
            "/api/android-live/design/sessions/:design_session_id/browser/interact",
            post(interact_browser),
        )
        .route(
            "/api/android-live/design/sessions/:design_session_id/browser/stop",
            post(stop_browser),
        )
        .route(
            "/api/android-live/design/sessions/:design_session_id/tauri/prepare",
            post(prepare_tauri),
        )
        .route(
            "/api/android-live/design/sessions/:design_session_id/tauri/capture",
            post(capture_tauri),
        )
        .route(
            "/api/android-live/design/sessions/:design_session_id/tauri/stop",
            post(stop_tauri),
        )
        .route(
            "/api/android-live/design/sessions/:design_session_id/tauri/artifact",
            post(get_tauri_artifact),
        )
        .route(
            "/api/android-live/design/sessions/:design_session_id/tauri/behavior",
            post(capture_tauri_behavior),
        )
        .route("/api/android-live/design/drafts/list", post(list_drafts))
        .route("/api/android-live/design/drafts", post(create_draft))
        .route("/api/android-live/design/drafts/:draft_id", post(get_draft))
        .route(
            "/api/android-live/design/drafts/:draft_id/update",
            post(update_draft),
        )
        .route(
            "/api/android-live/design/drafts/:draft_id/undo",
            post(undo_draft),
        )
        .route(
            "/api/android-live/design/drafts/:draft_id/writeback/begin",
            post(begin_draft_writeback),
        )
        .route(
            "/api/android-live/design/drafts/:draft_id/writeback/complete",
            post(complete_draft_writeback),
        )
        .route(
            "/api/android-live/design/drafts/:draft_id/verification-matrix",
            post(get_verification_matrix),
        )
        .route(
            "/api/android-live/design/drafts/:draft_id/preview",
            post(preview_draft),
        )
        .route(
            "/api/android-live/design/drafts/:draft_id/preview/restore",
            post(restore_draft_preview),
        )
        .route(
            "/api/android-live/design/drafts/:draft_id/source-binding/candidates",
            post(suggest_source_binding),
        )
        .route(
            "/api/android-live/design/tasks/:task_id/bind",
            post(bind_design_task),
        )
        .route(
            "/api/android-live/design/tasks/:task_id/binding",
            post(get_design_task_binding),
        )
        .route(
            "/api/android-live/design/tasks/:task_id/renew",
            post(renew_design_task_binding),
        )
        .route(
            "/api/android-live/design/tasks/:task_id/settle",
            post(settle_design_task_binding),
        )
        .route("/api/android-live/design/events", post(list_design_events))
        .merge(super::design_planning_http::routes())
}

async fn get_capabilities(
    State(runtime): State<Arc<NodeRuntime>>,
    Json(arguments): Json<Value>,
) -> Response {
    call(&runtime, "ui_get_design_capabilities", arguments).await
}

async fn list_targets(
    State(runtime): State<Arc<NodeRuntime>>,
    Json(arguments): Json<Value>,
) -> Response {
    call(&runtime, "ui_list_design_targets", arguments).await
}

async fn list_sessions(
    State(runtime): State<Arc<NodeRuntime>>,
    Json(arguments): Json<Value>,
) -> Response {
    call(&runtime, "ui_list_design_sessions", arguments).await
}

async fn open_session(
    State(runtime): State<Arc<NodeRuntime>>,
    Json(arguments): Json<Value>,
) -> Response {
    call(&runtime, "ui_open_design_target", arguments).await
}

async fn capture_session(
    State(runtime): State<Arc<NodeRuntime>>,
    Path(design_session_id): Path<String>,
    Json(mut arguments): Json<Value>,
) -> Response {
    arguments["designSessionId"] = json!(design_session_id);
    call(&runtime, "ui_capture_design_surface", arguments).await
}

async fn get_surface(
    State(runtime): State<Arc<NodeRuntime>>,
    Path(design_session_id): Path<String>,
    Json(mut arguments): Json<Value>,
) -> Response {
    arguments["designSessionId"] = json!(design_session_id);
    call(&runtime, "ui_get_design_surface", arguments).await
}

async fn get_artifact(
    State(runtime): State<Arc<NodeRuntime>>,
    Path(design_session_id): Path<String>,
    Json(mut arguments): Json<Value>,
) -> Response {
    let session = match project_session(&runtime, &mut arguments).await {
        Ok(session) => session,
        Err(error) => return json_error(StatusCode::BAD_REQUEST, error),
    };
    match super::design_targets::pixel_artifact(&session, &design_session_id) {
        Ok(artifact) => artifact_response(artifact),
        Err(error) => json_error(StatusCode::BAD_REQUEST, error),
    }
}

async fn prepare_browser(
    State(runtime): State<Arc<NodeRuntime>>,
    Path(id): Path<String>,
    Json(mut arguments): Json<Value>,
) -> Response {
    arguments["designSessionId"] = json!(id);
    call(&runtime, "ui_prepare_design_browser", arguments).await
}

async fn interact_browser(
    State(runtime): State<Arc<NodeRuntime>>,
    Path(id): Path<String>,
    Json(mut arguments): Json<Value>,
) -> Response {
    arguments["designSessionId"] = json!(id);
    call(&runtime, "ui_interact_design_browser", arguments).await
}

async fn stop_browser(
    State(runtime): State<Arc<NodeRuntime>>,
    Path(id): Path<String>,
    Json(mut arguments): Json<Value>,
) -> Response {
    arguments["designSessionId"] = json!(id);
    call(&runtime, "ui_stop_design_browser", arguments).await
}

async fn prepare_tauri(
    State(runtime): State<Arc<NodeRuntime>>,
    Path(id): Path<String>,
    Json(mut arguments): Json<Value>,
) -> Response {
    arguments["designSessionId"] = json!(id);
    call(&runtime, "ui_prepare_tauri_runtime", arguments).await
}

async fn capture_tauri(
    State(runtime): State<Arc<NodeRuntime>>,
    Path(id): Path<String>,
    Json(mut arguments): Json<Value>,
) -> Response {
    arguments["designSessionId"] = json!(id);
    call(&runtime, "ui_capture_tauri_host", arguments).await
}

async fn stop_tauri(
    State(runtime): State<Arc<NodeRuntime>>,
    Path(id): Path<String>,
    Json(mut arguments): Json<Value>,
) -> Response {
    arguments["designSessionId"] = json!(id);
    call(&runtime, "ui_stop_tauri_runtime", arguments).await
}

async fn get_tauri_artifact(
    State(runtime): State<Arc<NodeRuntime>>,
    Path(design_session_id): Path<String>,
    Json(mut arguments): Json<Value>,
) -> Response {
    let session = match project_session(&runtime, &mut arguments).await {
        Ok(session) => session,
        Err(error) => return json_error(StatusCode::BAD_REQUEST, error),
    };
    match super::tauri_host_runtime::native_artifact(&session, &design_session_id) {
        Ok(artifact) => artifact_response(artifact),
        Err(error) => json_error(StatusCode::BAD_REQUEST, error),
    }
}

async fn capture_tauri_behavior(
    State(runtime): State<Arc<NodeRuntime>>,
    Path(id): Path<String>,
    Json(mut arguments): Json<Value>,
) -> Response {
    arguments["designSessionId"] = json!(id);
    call(&runtime, "ui_capture_tauri_behavior", arguments).await
}

async fn list_drafts(
    State(runtime): State<Arc<NodeRuntime>>,
    Json(arguments): Json<Value>,
) -> Response {
    call(&runtime, "ui_list_design_drafts", arguments).await
}

async fn create_draft(
    State(runtime): State<Arc<NodeRuntime>>,
    Json(arguments): Json<Value>,
) -> Response {
    call(&runtime, "ui_create_design_draft", arguments).await
}

async fn get_draft(
    State(runtime): State<Arc<NodeRuntime>>,
    Path(draft_id): Path<String>,
    Json(mut arguments): Json<Value>,
) -> Response {
    arguments["draftId"] = json!(draft_id);
    call(&runtime, "ui_get_design_draft", arguments).await
}

async fn update_draft(
    State(runtime): State<Arc<NodeRuntime>>,
    Path(draft_id): Path<String>,
    Json(mut arguments): Json<Value>,
) -> Response {
    arguments["draftId"] = json!(draft_id);
    call(&runtime, "ui_update_design_draft", arguments).await
}

async fn undo_draft(
    State(runtime): State<Arc<NodeRuntime>>,
    Path(draft_id): Path<String>,
    Json(mut arguments): Json<Value>,
) -> Response {
    arguments["draftId"] = json!(draft_id);
    call(&runtime, "ui_undo_design_draft", arguments).await
}

async fn begin_draft_writeback(
    State(runtime): State<Arc<NodeRuntime>>,
    Path(draft_id): Path<String>,
    Json(mut arguments): Json<Value>,
) -> Response {
    arguments["draftId"] = json!(draft_id);
    call(&runtime, "ui_begin_design_writeback", arguments).await
}

async fn complete_draft_writeback(
    State(runtime): State<Arc<NodeRuntime>>,
    Path(draft_id): Path<String>,
    Json(mut arguments): Json<Value>,
) -> Response {
    arguments["draftId"] = json!(draft_id);
    call(&runtime, "ui_complete_design_writeback", arguments).await
}

async fn get_verification_matrix(
    State(runtime): State<Arc<NodeRuntime>>,
    Path(draft_id): Path<String>,
    Json(mut arguments): Json<Value>,
) -> Response {
    arguments["draftId"] = json!(draft_id);
    call(&runtime, "ui_get_design_verification_matrix", arguments).await
}

async fn preview_draft(
    State(runtime): State<Arc<NodeRuntime>>,
    Path(draft_id): Path<String>,
    Json(mut arguments): Json<Value>,
) -> Response {
    arguments["draftId"] = json!(draft_id);
    call(&runtime, "ui_preview_design_draft", arguments).await
}

async fn restore_draft_preview(
    State(runtime): State<Arc<NodeRuntime>>,
    Path(draft_id): Path<String>,
    Json(mut arguments): Json<Value>,
) -> Response {
    arguments["draftId"] = json!(draft_id);
    call(&runtime, "ui_restore_design_draft_preview", arguments).await
}

async fn suggest_source_binding(
    State(runtime): State<Arc<NodeRuntime>>,
    Path(draft_id): Path<String>,
    Json(mut arguments): Json<Value>,
) -> Response {
    arguments["draftId"] = json!(draft_id);
    call(&runtime, "ui_suggest_design_source_binding", arguments).await
}

async fn bind_design_task(
    State(runtime): State<Arc<NodeRuntime>>,
    Path(task_id): Path<String>,
    Json(mut arguments): Json<Value>,
) -> Response {
    arguments["taskId"] = json!(task_id);
    call(&runtime, "ui_bind_design_task", arguments).await
}

async fn get_design_task_binding(
    State(runtime): State<Arc<NodeRuntime>>,
    Path(task_id): Path<String>,
    Json(mut arguments): Json<Value>,
) -> Response {
    arguments["taskId"] = json!(task_id);
    call(&runtime, "ui_get_design_task_binding", arguments).await
}

async fn renew_design_task_binding(
    State(runtime): State<Arc<NodeRuntime>>,
    Path(task_id): Path<String>,
    Json(mut arguments): Json<Value>,
) -> Response {
    arguments["taskId"] = json!(task_id);
    call(&runtime, "ui_renew_design_task_binding", arguments).await
}

async fn settle_design_task_binding(
    State(runtime): State<Arc<NodeRuntime>>,
    Path(task_id): Path<String>,
    Json(mut arguments): Json<Value>,
) -> Response {
    arguments["taskId"] = json!(task_id);
    call(&runtime, "ui_settle_design_task_binding", arguments).await
}

async fn list_design_events(
    State(runtime): State<Arc<NodeRuntime>>,
    Json(arguments): Json<Value>,
) -> Response {
    call(&runtime, "ui_list_design_events", arguments).await
}

fn artifact_response(artifact: super::design_session_store::VerifiedPixelArtifact) -> Response {
    let mut response = Response::new(Body::from(artifact.bytes));
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_str(&artifact.media_type)
            .unwrap_or_else(|_| HeaderValue::from_static("image/png")),
    );
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    if let Ok(etag) = HeaderValue::from_str(&format!("\"{}\"", artifact.sha256)) {
        response.headers_mut().insert(header::ETAG, etag);
    }
    response
}

pub(super) async fn call(runtime: &Arc<NodeRuntime>, name: &str, mut arguments: Value) -> Response {
    let session = match project_session(runtime, &mut arguments).await {
        Ok(session) => session,
        Err(error) => return json_error(StatusCode::BAD_REQUEST, error),
    };
    match super::design_tools::call(&session, name, arguments).await {
        Ok(result) => Json(json!({"ok":true,"result":result})).into_response(),
        Err(error) => json_error(StatusCode::BAD_REQUEST, error),
    }
}

async fn project_session(
    runtime: &Arc<NodeRuntime>,
    arguments: &mut Value,
) -> Result<Arc<LiveUiSession>> {
    let project_root = arguments
        .get("projectRoot")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| anyhow!("缺少 projectRoot"))?;
    let root = std::path::PathBuf::from(project_root)
        .canonicalize()
        .with_context(|| format!("项目目录不存在: {project_root}"))?;
    if !root.join(".git").exists() {
        return Err(anyhow!("projectRoot 不是 Git 工作区"));
    }
    arguments
        .as_object_mut()
        .ok_or_else(|| anyhow!("请求参数必须是对象"))?
        .remove("projectRoot");
    let host_port = crate::node_agent_admin_open::admin_port_from_env();
    let descriptor =
        super::mcp::descriptor_for_project(&runtime.live_ui, &root.to_string_lossy(), host_port)
            .await?
            .context("无法创建项目 UI MCP 会话")?;
    let session_id = descriptor
        .get("sessionId")
        .and_then(Value::as_str)
        .context("项目 UI MCP 没有返回 sessionId")?;
    runtime.live_ui.session(session_id).await
}

fn json_error(status: StatusCode, error: impl std::fmt::Display) -> Response {
    (status, Json(json!({"ok":false,"error":error.to_string()}))).into_response()
}
