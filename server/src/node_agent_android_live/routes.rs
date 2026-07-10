use std::sync::Arc;

use axum::{
    extract::{Path, Query, State, WebSocketUpgrade},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde_json::json;

use crate::NodeRuntime;

use super::adb_session::{start_runtime, stop_runtime, DEFAULT_DEVICE_PORT};
use super::protocol::{
    LiveStylePatch, RuntimeSocketQuery, StartLiveSessionRequest, PROTOCOL_VERSION,
};

pub(crate) fn protected_routes() -> Router<Arc<NodeRuntime>> {
    Router::new()
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
}

pub(crate) fn runtime_routes() -> Router<Arc<NodeRuntime>> {
    Router::new().route("/api/android-live/runtime", get(runtime_socket_handler))
}

async fn create_session_handler(
    State(runtime): State<Arc<NodeRuntime>>,
    Json(req): Json<StartLiveSessionRequest>,
) -> Response {
    let device_id = req.device_id.trim().to_string();
    let package_name = req.package_name.trim().to_string();
    let project_root = req
        .project_root
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    if device_id.is_empty() || package_name.is_empty() {
        return json_error(StatusCode::BAD_REQUEST, "deviceId 和 packageName 不能为空");
    }
    let session = runtime
        .live_ui
        .create_session(device_id, package_name, project_root, DEFAULT_DEVICE_PORT)
        .await;
    let host_port = crate::node_agent_admin_open::admin_port_from_env();
    match start_runtime(&session, host_port).await {
        Ok(output) => Json(json!({
            "ok": true,
            "protocolVersion": PROTOCOL_VERSION,
            "session": session.view().await,
            "adbOutput": output,
        }))
        .into_response(),
        Err(error) => {
            let _ = stop_runtime(&session).await;
            runtime.live_ui.remove_session(&session.id).await;
            json_error(StatusCode::BAD_GATEWAY, format!("{error:#}"))
        }
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
