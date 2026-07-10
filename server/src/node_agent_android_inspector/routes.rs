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

use super::adb_capture::{adb_status, capture_snapshot, launch_app, list_devices};
use super::adb_wireless::{
    connect_and_remember, enable_tcpip, forget_device, pair_device, reconnect_devices,
    register_device, wireless_status,
};
use super::types::{
    CaptureRequest, ConnectRequest, EnableTcpIpRequest, ForgetDeviceRequest, LaunchAppRequest,
    PairDeviceRequest, ReconnectRequest, RegisterDeviceRequest,
};

pub(crate) fn routes() -> Router<Arc<NodeRuntime>> {
    Router::new()
        .route("/api/android-inspector/status", get(status_handler))
        .route("/api/android-inspector/devices", get(devices_handler))
        .route("/api/android-inspector/connect", post(connect_handler))
        .route(
            "/api/android-inspector/wireless/status",
            get(wireless_status_handler),
        )
        .route(
            "/api/android-inspector/wireless/register",
            post(register_device_handler),
        )
        .route(
            "/api/android-inspector/wireless/pair",
            post(pair_device_handler),
        )
        .route(
            "/api/android-inspector/wireless/reconnect",
            post(reconnect_devices_handler),
        )
        .route(
            "/api/android-inspector/wireless/enable-tcpip",
            post(enable_tcpip_handler),
        )
        .route(
            "/api/android-inspector/wireless/forget",
            post(forget_device_handler),
        )
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
    match connect_and_remember(&req.address, req.profile_id.as_deref()).await {
        Ok(output) => Json(json!({ "ok": true, "output": output })).into_response(),
        Err(error) => json_error(StatusCode::BAD_REQUEST, format!("{error:#}")),
    }
}

async fn wireless_status_handler(State(_runtime): State<Arc<NodeRuntime>>) -> Response {
    match wireless_status().await {
        Ok(status) => Json(status).into_response(),
        Err(error) => json_error(StatusCode::BAD_GATEWAY, format!("{error:#}")),
    }
}

async fn register_device_handler(
    State(_runtime): State<Arc<NodeRuntime>>,
    Json(req): Json<RegisterDeviceRequest>,
) -> Response {
    match register_device(req).await {
        Ok(profile) => match wireless_status().await {
            Ok(status) => {
                Json(json!({ "ok": true, "profile": profile, "status": status })).into_response()
            }
            Err(error) => json_error(StatusCode::BAD_GATEWAY, format!("{error:#}")),
        },
        Err(error) => json_error(StatusCode::BAD_REQUEST, format!("{error:#}")),
    }
}

async fn pair_device_handler(
    State(_runtime): State<Arc<NodeRuntime>>,
    Json(req): Json<PairDeviceRequest>,
) -> Response {
    match pair_device(req).await {
        Ok((output, status)) => {
            Json(json!({ "ok": true, "output": output, "status": status })).into_response()
        }
        Err(error) => json_error(StatusCode::BAD_REQUEST, format!("{error:#}")),
    }
}

async fn reconnect_devices_handler(
    State(_runtime): State<Arc<NodeRuntime>>,
    Json(req): Json<ReconnectRequest>,
) -> Response {
    match reconnect_devices(req).await {
        Ok(status) => Json(status).into_response(),
        Err(error) => json_error(StatusCode::BAD_GATEWAY, format!("{error:#}")),
    }
}

async fn enable_tcpip_handler(
    State(_runtime): State<Arc<NodeRuntime>>,
    Json(req): Json<EnableTcpIpRequest>,
) -> Response {
    match enable_tcpip(req).await {
        Ok((output, status)) => {
            Json(json!({ "ok": true, "output": output, "status": status })).into_response()
        }
        Err(error) => json_error(StatusCode::BAD_REQUEST, format!("{error:#}")),
    }
}

async fn forget_device_handler(
    State(_runtime): State<Arc<NodeRuntime>>,
    Json(req): Json<ForgetDeviceRequest>,
) -> Response {
    match forget_device(&req.profile_id) {
        Ok(removed) => Json(json!({ "ok": true, "removed": removed })).into_response(),
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
