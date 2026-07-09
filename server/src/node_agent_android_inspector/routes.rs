use std::sync::Arc;

use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde_json::json;

use crate::NodeRuntime;

use super::adb_capture::{adb_status, capture_snapshot, connect_device, launch_app, list_devices};
use super::types::{CaptureRequest, ConnectRequest, LaunchAppRequest};

pub(crate) fn routes() -> Router<Arc<NodeRuntime>> {
    Router::new()
        .route("/api/android-inspector/status", get(status_handler))
        .route("/api/android-inspector/devices", get(devices_handler))
        .route("/api/android-inspector/connect", post(connect_handler))
        .route(
            "/api/android-inspector/launch-app",
            post(launch_app_handler),
        )
        .route("/api/android-inspector/capture", post(capture_handler))
}

async fn status_handler(State(_runtime): State<Arc<NodeRuntime>>) -> Response {
    Json(json!({ "ok": true, "adb": adb_status().await })).into_response()
}

async fn devices_handler(State(_runtime): State<Arc<NodeRuntime>>) -> Response {
    match list_devices().await {
        Ok(devices) => Json(json!({ "ok": true, "adb": adb_status().await, "devices": devices }))
            .into_response(),
        Err(error) => json_error(StatusCode::BAD_GATEWAY, format!("{error:#}")),
    }
}

async fn connect_handler(
    State(_runtime): State<Arc<NodeRuntime>>,
    Json(req): Json<ConnectRequest>,
) -> Response {
    match connect_device(&req.address).await {
        Ok(output) => Json(json!({ "ok": true, "output": output })).into_response(),
        Err(error) => json_error(StatusCode::BAD_REQUEST, format!("{error:#}")),
    }
}

async fn launch_app_handler(
    State(_runtime): State<Arc<NodeRuntime>>,
    Json(req): Json<LaunchAppRequest>,
) -> Response {
    let package = req.package_name.as_deref().unwrap_or("com.elon.app");
    match launch_app(&req.device_id, package).await {
        Ok(output) => Json(json!({ "ok": true, "output": output })).into_response(),
        Err(error) => json_error(StatusCode::BAD_REQUEST, format!("{error:#}")),
    }
}

async fn capture_handler(
    State(_runtime): State<Arc<NodeRuntime>>,
    Json(req): Json<CaptureRequest>,
) -> Response {
    match capture_snapshot(req).await {
        Ok(snapshot) => Json(snapshot).into_response(),
        Err(error) => json_error(StatusCode::BAD_GATEWAY, format!("{error:#}")),
    }
}

fn json_error(status: StatusCode, error: String) -> Response {
    (status, Json(json!({ "ok": false, "error": error }))).into_response()
}
