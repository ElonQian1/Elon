//! Project-scoped discovery and relay for shared Android device hosts.

use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    time::Duration,
};

use axum::{
    body::Body,
    extract::{Path, Request, State},
    http::{HeaderMap, HeaderName, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::{any, get},
    Json, Router,
};
use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use futures::{stream, StreamExt};
use homecli_proto::{AgentToServer, CAP_ANDROID_DEVICE_HOST_V1};
use serde::Serialize;

use crate::{
    homecli_agent::AgentSummary,
    project_auth::{auth_from_headers, can_edit, json_error, project_access},
    store::PublicUser,
    types::AppState,
};

const LEASE_ID_HEADER: &str = "x-elon-device-lease-id";
const HARDWARE_SERIAL_HEADER: &str = "x-elon-device-hardware-serial";
const DISCOVERY_TIMEOUT: Duration = Duration::from_secs(8);
const RECONNECT_TIMEOUT: Duration = Duration::from_secs(25);
const RELAY_TIMEOUT: Duration = Duration::from_secs(45);
const BUILD_RELAY_TIMEOUT: Duration = Duration::from_secs(20 * 60 + 30);

pub(crate) fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/api/projects/:project_id/modules/ui-tuner/android-device-hosts",
            get(list_device_hosts),
        )
        .route(
            "/api/projects/:project_id/modules/ui-tuner/android-device-hosts/:agent_id/relay/*path",
            any(relay_device_host),
        )
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AndroidDeviceHost {
    agent_id: String,
    display_name: String,
    device_name: Option<String>,
    version: String,
    devices: Vec<serde_json::Value>,
}

async fn list_device_hosts(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
) -> Response {
    if let Err(response) = authorized_developer(&state, &headers, &project_id) {
        return response;
    }
    let shared_serials: HashSet<String> =
        match state.store.list_project_android_devices(&project_id) {
            Ok(devices) => devices
                .into_iter()
                .map(|device| device.hardware_serial)
                .collect(),
            Err(error) => return json_error(StatusCode::INTERNAL_SERVER_ERROR, error.to_string()),
        };
    if shared_serials.is_empty() {
        return Json(serde_json::json!({ "hosts": [] })).into_response();
    }
    let credentials: HashMap<String, (String, Option<String>)> =
        match state.store.list_public_dev_node_credentials() {
            Ok(items) => items
                .into_iter()
                .map(|credential| {
                    (
                        credential.agent_id,
                        (credential.label, credential.device_name),
                    )
                })
                .collect(),
            Err(error) => return json_error(StatusCode::INTERNAL_SERVER_ERROR, error.to_string()),
        };
    let candidates = state
        .agent_manager
        .list()
        .await
        .into_iter()
        .filter(|agent| credentials.contains_key(&agent.agent_id))
        .filter(|agent| {
            agent
                .capabilities
                .iter()
                .any(|capability| capability == CAP_ANDROID_DEVICE_HOST_V1)
        })
        .collect::<Vec<_>>();
    let hosts = stream::iter(candidates.into_iter().map(|agent| {
        let state = state.clone();
        let shared_serials = shared_serials.clone();
        let credential = credentials.get(&agent.agent_id).cloned();
        async move { probe_host(state, agent, credential, &shared_serials).await }
    }))
    .buffer_unordered(8)
    .filter_map(|host| async move { host })
    .collect::<Vec<_>>()
    .await;
    Json(serde_json::json!({ "hosts": hosts })).into_response()
}

async fn probe_host(
    state: Arc<AppState>,
    agent: AgentSummary,
    credential: Option<(String, Option<String>)>,
    shared_serials: &HashSet<String>,
) -> Option<AndroidDeviceHost> {
    let mut devices = fetch_devices(&state, &agent.agent_id).await;
    if matching_devices(&devices, shared_serials).is_empty() {
        let _ = state
            .agent_manager
            .dispatch_android_device_host_http(
                &agent.agent_id,
                "POST".to_string(),
                "/api/android-inspector/wireless/reconnect".to_string(),
                vec![("content-type".to_string(), "application/json".to_string())],
                Some(B64.encode(b"{}")),
                RECONNECT_TIMEOUT,
            )
            .await;
        devices = fetch_devices(&state, &agent.agent_id).await;
    }
    let devices = matching_devices(&devices, shared_serials);
    if devices.is_empty() {
        return None;
    }
    let device_name = agent
        .device_name
        .clone()
        .or_else(|| credential.as_ref().and_then(|item| item.1.clone()));
    let display_name = credential
        .as_ref()
        .map(|item| item.0.trim())
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .or_else(|| device_name.clone())
        .unwrap_or_else(|| format!("设备主机 {}", short_id(&agent.agent_id)));
    Some(AndroidDeviceHost {
        agent_id: agent.agent_id,
        display_name,
        device_name,
        version: agent.version,
        devices,
    })
}

async fn fetch_devices(state: &AppState, agent_id: &str) -> Vec<serde_json::Value> {
    let response = state
        .agent_manager
        .dispatch_android_device_host_http(
            agent_id,
            "GET".to_string(),
            "/api/android-inspector/devices".to_string(),
            Vec::new(),
            None,
            DISCOVERY_TIMEOUT,
        )
        .await;
    let Ok(AgentToServer::HttpResponse {
        status: 200,
        body_b64: Some(body),
        ..
    }) = response
    else {
        return Vec::new();
    };
    B64.decode(body)
        .ok()
        .and_then(|body| serde_json::from_slice::<serde_json::Value>(&body).ok())
        .and_then(|body| {
            body.get("devices")
                .and_then(serde_json::Value::as_array)
                .cloned()
        })
        .unwrap_or_default()
}

fn matching_devices(
    devices: &[serde_json::Value],
    shared_serials: &HashSet<String>,
) -> Vec<serde_json::Value> {
    devices
        .iter()
        .filter(|device| device.get("state").and_then(serde_json::Value::as_str) == Some("device"))
        .filter(|device| {
            device
                .get("hardwareSerial")
                .and_then(serde_json::Value::as_str)
                .is_some_and(|serial| shared_serials.contains(serial))
        })
        .cloned()
        .collect()
}

async fn relay_device_host(
    State(state): State<Arc<AppState>>,
    Path((project_id, agent_id, path)): Path<(String, String, String)>,
    request: Request,
) -> Response {
    let user = match authorized_developer(&state, request.headers(), &project_id) {
        Ok(user) => user,
        Err(response) => return response,
    };
    let relay_path = format!("/{path}");
    if !browser_relay_target_allowed(request.method().as_str(), &relay_path) {
        return json_error(StatusCode::FORBIDDEN, "该真机接口不允许远程访问");
    }
    if let Err(response) = validate_host(&state, &agent_id).await {
        return response;
    }
    if let Err(response) = validate_owned_lease(&state, &user, &project_id, request.headers()) {
        return response;
    }
    let method = request.method().to_string();
    let query = request
        .uri()
        .query()
        .map(|query| format!("?{query}"))
        .unwrap_or_default();
    let path = format!("{relay_path}{query}");
    let content_type = request
        .headers()
        .get("content-type")
        .and_then(|value| value.to_str().ok())
        .map(|value| ("content-type".to_string(), value.to_string()))
        .into_iter()
        .collect();
    let body = match axum::body::to_bytes(request.into_body(), 32 * 1024 * 1024).await {
        Ok(body) => body,
        Err(error) => return json_error(StatusCode::BAD_REQUEST, error.to_string()),
    };
    let timeout = if relay_path.ends_with("/build-verify")
        || relay_path.ends_with("/debug-runtime/prepare")
    {
        BUILD_RELAY_TIMEOUT
    } else {
        RELAY_TIMEOUT
    };
    match state
        .agent_manager
        .dispatch_android_device_host_http(
            &agent_id,
            method,
            path,
            content_type,
            (!body.is_empty()).then(|| B64.encode(body)),
            timeout,
        )
        .await
    {
        Ok(AgentToServer::HttpResponse {
            status,
            headers,
            body_b64,
            ..
        }) => relay_response(status, headers, body_b64),
        Ok(AgentToServer::HttpError { message, .. }) => json_error(
            StatusCode::BAD_GATEWAY,
            format!("设备主机请求失败：{message}"),
        ),
        Ok(_) => json_error(StatusCode::BAD_GATEWAY, "设备主机返回了无法识别的响应"),
        Err(error) => json_error(
            StatusCode::BAD_GATEWAY,
            format!("设备主机离线或响应超时：{error}"),
        ),
    }
}

async fn validate_host(state: &AppState, agent_id: &str) -> Result<(), Response> {
    let public = state
        .store
        .get_node_credential(agent_id)
        .map_err(|error| json_error(StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?
        .is_some_and(|credential| credential.public_dev_enabled);
    let capable = state
        .agent_manager
        .agent_has_capability(agent_id, CAP_ANDROID_DEVICE_HOST_V1)
        .await;
    if !public || !capable {
        return Err(json_error(
            StatusCode::NOT_FOUND,
            "设备主机未公开、未在线或版本过旧",
        ));
    }
    Ok(())
}

fn validate_owned_lease(
    state: &AppState,
    user: &PublicUser,
    project_id: &str,
    headers: &HeaderMap,
) -> Result<(), Response> {
    let lease_id = required_header(headers, LEASE_ID_HEADER)?;
    let hardware_serial = required_header(headers, HARDWARE_SERIAL_HEADER)?;
    let owned = state
        .store
        .list_project_android_device_leases(project_id)
        .map_err(|error| json_error(StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?
        .into_iter()
        .any(|lease| {
            lease.lease_id == lease_id
                && lease.hardware_serial == hardware_serial
                && lease.owner_user_id == user.id
        });
    if !owned {
        return Err(json_error(
            StatusCode::CONFLICT,
            "公共测试手机使用权不存在、已过期或属于其他用户",
        ));
    }
    Ok(())
}

fn required_header<'a>(headers: &'a HeaderMap, name: &str) -> Result<&'a str, Response> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| json_error(StatusCode::CONFLICT, format!("缺少 {name}")))
}

fn relay_response(
    status: u16,
    headers: Vec<(String, String)>,
    body_b64: Option<String>,
) -> Response {
    let mut response =
        Response::builder().status(StatusCode::from_u16(status).unwrap_or(StatusCode::BAD_GATEWAY));
    for (name, value) in headers {
        if name.eq_ignore_ascii_case("content-type") || name.eq_ignore_ascii_case("cache-control") {
            if let (Ok(name), Ok(value)) =
                (name.parse::<HeaderName>(), value.parse::<HeaderValue>())
            {
                response = response.header(name, value);
            }
        }
    }
    response
        .body(Body::from(
            body_b64
                .and_then(|body| B64.decode(body).ok())
                .unwrap_or_default(),
        ))
        .unwrap_or_else(|error| json_error(StatusCode::BAD_GATEWAY, error.to_string()))
}

fn browser_relay_target_allowed(method: &str, path: &str) -> bool {
    let inspector = matches!(
        path,
        "/api/android-inspector/capture" | "/api/android-inspector/selection-artifact"
    );
    let live = path.starts_with("/api/android-live/")
        && !path.starts_with("/api/android-live/runtime")
        && !path.contains("/mcp/")
        && !path.starts_with("/api/android-live/project-mcp/");
    matches!(method, "GET" | "POST" | "DELETE") && (inspector || live)
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

fn short_id(agent_id: &str) -> String {
    agent_id.chars().take(8).collect()
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::{browser_relay_target_allowed, matching_devices};

    #[test]
    fn discovery_filters_non_project_and_offline_devices() {
        let devices = vec![
            serde_json::json!({"serial":"one", "hardwareSerial":"xiaomi", "state":"device"}),
            serde_json::json!({"serial":"two", "hardwareSerial":"other", "state":"device"}),
            serde_json::json!({"serial":"three", "hardwareSerial":"xiaomi", "state":"offline"}),
        ];
        let shared = HashSet::from(["xiaomi".to_string()]);
        let matching = matching_devices(&devices, &shared);
        assert_eq!(matching.len(), 1);
        assert_eq!(matching[0]["serial"], "one");
    }

    #[test]
    fn browser_relay_is_narrower_than_node_internal_relay() {
        assert!(browser_relay_target_allowed(
            "POST",
            "/api/android-inspector/capture"
        ));
        assert!(browser_relay_target_allowed(
            "GET",
            "/api/android-live/sessions/one/frame"
        ));
        assert!(!browser_relay_target_allowed(
            "POST",
            "/api/android-inspector/wireless/reconnect"
        ));
        assert!(!browser_relay_target_allowed(
            "GET",
            "/api/android-inspector/devices"
        ));
        assert!(!browser_relay_target_allowed(
            "POST",
            "/api/android-live/mcp/one"
        ));
    }
}
