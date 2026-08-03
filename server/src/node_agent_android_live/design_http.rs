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
        Ok(artifact) => {
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
        Err(error) => json_error(StatusCode::BAD_REQUEST, error),
    }
}

async fn call(runtime: &Arc<NodeRuntime>, name: &str, mut arguments: Value) -> Response {
    let session = match project_session(runtime, &mut arguments).await {
        Ok(session) => session,
        Err(error) => return json_error(StatusCode::BAD_REQUEST, error),
    };
    match super::design_targets::call(&session, name, arguments).await {
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
