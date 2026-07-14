use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;

use crate::{
    project_auth::{auth_from_headers, can_edit, json_error, project_access},
    store::{AcquireAndroidDeviceLease, PublicUser},
    types::AppState,
};

pub(crate) fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/api/projects/:project_id/modules/ui-tuner/android-device-leases",
            get(list_leases),
        )
        .route(
            "/api/projects/:project_id/modules/ui-tuner/android-device-leases/:hardware_serial/acquire",
            post(acquire_lease),
        )
        .route(
            "/api/projects/:project_id/modules/ui-tuner/android-device-leases/:hardware_serial/heartbeat",
            post(heartbeat_lease),
        )
        .route(
            "/api/projects/:project_id/modules/ui-tuner/android-device-leases/:hardware_serial/release",
            post(release_lease),
        )
        .route(
            "/api/me/modules/ui-tuner/android-device-lease/validate",
            post(validate_lease),
        )
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ClientBody {
    client_instance_id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct OwnedLeaseBody {
    lease_id: String,
    client_instance_id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ValidateLeaseBody {
    lease_id: String,
    project_id: String,
    hardware_serial: String,
}

async fn list_leases(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
) -> Response {
    if let Err(response) = authorized_developer(&state, &headers, &project_id) {
        return response;
    }
    match state.store.list_project_android_device_leases(&project_id) {
        Ok(leases) => Json(serde_json::json!({ "leases": leases })).into_response(),
        Err(error) => json_error(StatusCode::INTERNAL_SERVER_ERROR, error.to_string()),
    }
}

async fn acquire_lease(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, hardware_serial)): Path<(String, String)>,
    Json(body): Json<ClientBody>,
) -> Response {
    let user = match authorized_developer(&state, &headers, &project_id) {
        Ok(user) => user,
        Err(response) => return response,
    };
    if !valid_client_id(&body.client_instance_id) {
        return json_error(StatusCode::BAD_REQUEST, "clientInstanceId 格式不合法");
    }
    let known = match state.store.list_project_android_devices(&project_id) {
        Ok(devices) => devices
            .iter()
            .any(|device| device.hardware_serial == hardware_serial),
        Err(error) => return json_error(StatusCode::INTERNAL_SERVER_ERROR, error.to_string()),
    };
    if !known {
        return json_error(StatusCode::NOT_FOUND, "当前项目没有共享这台手机");
    }
    let display_name = user.nickname.as_deref().unwrap_or(&user.account);
    match state.store.acquire_project_android_device_lease(
        &project_id,
        &hardware_serial,
        &user.id,
        display_name,
        body.client_instance_id.trim(),
    ) {
        Ok(AcquireAndroidDeviceLease::Acquired(lease)) => Json(lease).into_response(),
        Ok(AcquireAndroidDeviceLease::Occupied(lease)) => (
            StatusCode::CONFLICT,
            Json(serde_json::json!({
                "error": format!("{} 正在使用这台手机，请切换设备或等待占用自动释放", lease.owner_display_name),
                "lease": lease,
            })),
        ).into_response(),
        Err(error) => json_error(StatusCode::INTERNAL_SERVER_ERROR, error.to_string()),
    }
}

async fn heartbeat_lease(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, hardware_serial)): Path<(String, String)>,
    Json(body): Json<OwnedLeaseBody>,
) -> Response {
    let user = match authorized_developer(&state, &headers, &project_id) {
        Ok(user) => user,
        Err(response) => return response,
    };
    match state.store.heartbeat_project_android_device_lease(
        &project_id,
        &hardware_serial,
        &user.id,
        body.lease_id.trim(),
        body.client_instance_id.trim(),
    ) {
        Ok(Some(lease)) => Json(lease).into_response(),
        Ok(None) => json_error(StatusCode::CONFLICT, "设备占用已失效或已被释放"),
        Err(error) => json_error(StatusCode::INTERNAL_SERVER_ERROR, error.to_string()),
    }
}

async fn release_lease(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, hardware_serial)): Path<(String, String)>,
    Json(body): Json<OwnedLeaseBody>,
) -> Response {
    let user = match authorized_developer(&state, &headers, &project_id) {
        Ok(user) => user,
        Err(response) => return response,
    };
    match state.store.release_project_android_device_lease(
        &project_id,
        &hardware_serial,
        &user.id,
        body.lease_id.trim(),
        body.client_instance_id.trim(),
    ) {
        Ok(released) => Json(serde_json::json!({ "released": released })).into_response(),
        Err(error) => json_error(StatusCode::INTERNAL_SERVER_ERROR, error.to_string()),
    }
}

async fn validate_lease(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<ValidateLeaseBody>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(user) => user,
        Err(error) => return json_error(StatusCode::UNAUTHORIZED, error.to_string()),
    };
    let project = match project_access(&state, &user.id, body.project_id.trim()) {
        Ok(project) if can_edit(&project.role) => project,
        Ok(_) => return json_error(StatusCode::FORBIDDEN, "当前节点账号不能使用项目测试设备"),
        Err(error) => return json_error(StatusCode::FORBIDDEN, error.to_string()),
    };
    let _ = project;
    match state.store.validate_project_android_device_lease(
        body.project_id.trim(),
        body.hardware_serial.trim(),
        body.lease_id.trim(),
    ) {
        Ok(true) => Json(serde_json::json!({ "valid": true })).into_response(),
        Ok(false) => json_error(
            StatusCode::CONFLICT,
            "设备占用不存在、已过期或不属于当前用户",
        ),
        Err(error) => json_error(StatusCode::INTERNAL_SERVER_ERROR, error.to_string()),
    }
}

fn authorized_developer(
    state: &AppState,
    headers: &HeaderMap,
    project_id: &str,
) -> Result<PublicUser, Response> {
    let user = auth_from_headers(state, headers)
        .map_err(|error| json_error(StatusCode::UNAUTHORIZED, error.to_string()))?;
    let project = project_access(state, &user.id, project_id)
        .map_err(|error| json_error(StatusCode::FORBIDDEN, error.to_string()))?;
    if !can_edit(&project.role) {
        return Err(json_error(
            StatusCode::FORBIDDEN,
            "当前角色不能使用项目测试设备",
        ));
    }
    Ok(user)
}

fn valid_client_id(value: &str) -> bool {
    let value = value.trim();
    (8..=80).contains(&value.len())
        && value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_'))
}

#[cfg(test)]
mod tests {
    use super::valid_client_id;

    #[test]
    fn client_ids_are_bounded_and_safe() {
        assert!(valid_client_id("uit_abcd-1234"));
        assert!(!valid_client_id("short"));
        assert!(!valid_client_id("uit_<script>"));
    }
}
