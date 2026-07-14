// server/src/node_agent_route_c_status.rs

use std::time::Duration;

use serde_json::{json, Value};
use tracing::warn;

#[derive(Debug, Clone)]
pub(crate) struct ServerRuntimeCloudStatus {
    pub ready: bool,
    pub status: Value,
}

pub(crate) async fn server_runtime_status_from_cloud(
    cloud_http_url: &str,
    user_token: Option<&str>,
) -> ServerRuntimeCloudStatus {
    let Some(token) = user_token.map(str::trim).filter(|value| !value.is_empty()) else {
        return ServerRuntimeCloudStatus {
            ready: false,
            status: json!({
                "ready": false,
                "status": "missing_token",
                "reason": "win_client_not_logged_in"
            }),
        };
    };
    let client = match crate::node_agent_cloud_net::direct_cloud_client(Duration::from_secs(5)) {
        Ok(client) => client,
        Err(e) => {
            warn!("Route C 服务器模型预检无法创建 HTTP client: {e}");
            return unavailable_status("client_build_failed");
        }
    };
    let url = format!(
        "{}/api/agent/runtime/status",
        cloud_http_url.trim_end_matches('/')
    );
    let response = match client.get(url).bearer_auth(token).send().await {
        Ok(response) => response,
        Err(e) => {
            warn!("Route C 服务器模型预检失败: {e}");
            return unavailable_status("cloud_unreachable");
        }
    };
    let status = response.status();
    if !status.is_success() {
        warn!("Route C 服务器模型预检返回 {status}");
        return ServerRuntimeCloudStatus {
            ready: false,
            status: json!({
                "ready": false,
                "status": "http_error",
                "httpStatus": status.as_u16()
            }),
        };
    }
    match response.json::<serde_json::Value>().await {
        Ok(value) => {
            let status = normalize_status_value(&value);
            ServerRuntimeCloudStatus {
                ready: server_runtime_ready_from_status_value(&status),
                status,
            }
        }
        Err(e) => {
            warn!("Route C 服务器模型预检响应不是 JSON: {e}");
            unavailable_status("invalid_json")
        }
    }
}

fn server_runtime_ready_from_status_value(value: &serde_json::Value) -> bool {
    if !value
        .get("ready")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false)
    {
        return false;
    }
    if value
        .get("status")
        .and_then(Value::as_str)
        .is_some_and(route_c_status_is_blocking)
    {
        return false;
    }
    if object_bool_is_false(value.get("policy"), "enabled")
        || object_bool_is_false(value.get("admissionAvailability"), "ready")
        || object_bool_is_false(value.get("admissionAvailability"), "available")
        || object_bool_is_false(value.get("agentPolicy"), "ready")
    {
        return false;
    }
    if object_status_is_blocking(value.get("admissionAvailability"))
        || object_status_is_blocking(value.get("agentPolicy"))
    {
        return false;
    }
    if value
        .get("blockingReasons")
        .or_else(|| value.get("blocking_reasons"))
        .and_then(Value::as_array)
        .is_some_and(|reasons| !reasons.is_empty())
    {
        return false;
    }
    true
}

fn object_bool_is_false(object: Option<&Value>, key: &str) -> bool {
    object
        .and_then(|value| value.get(key))
        .and_then(Value::as_bool)
        == Some(false)
}

fn object_status_is_blocking(object: Option<&Value>) -> bool {
    let Some(status) = object
        .and_then(|value| value.get("status").or_else(|| value.get("reason")))
        .and_then(Value::as_str)
    else {
        return false;
    };
    route_c_status_is_blocking(status)
}

fn route_c_status_is_blocking(status: &str) -> bool {
    matches!(
        status.trim().to_ascii_lowercase().as_str(),
        "disabled"
            | "blocked"
            | "limited"
            | "rate_limited"
            | "budget_exhausted"
            | "platform_budget_exhausted"
            | "user_budget_exhausted"
            | "agent_policy_blocked"
            | "no_server_api_key_agent"
            | "unavailable"
    )
}

fn normalize_status_value(value: &Value) -> Value {
    json!({
        "ready": value.get("ready").and_then(Value::as_bool).unwrap_or(false),
        "status": value.get("status").and_then(Value::as_str).unwrap_or("unknown"),
        "agent": value.get("agent").cloned().unwrap_or(Value::Null),
        "limits": value.get("limits").cloned().unwrap_or(Value::Null),
        "protection": value.get("protection").cloned().unwrap_or(Value::Null),
        "policy": value.get("policy").cloned().unwrap_or(Value::Null),
        "agentPolicy": value
            .get("agentPolicy")
            .or_else(|| value.get("agent_policy"))
            .cloned()
            .unwrap_or(Value::Null),
        "budget": value.get("budget").cloned().unwrap_or(Value::Null),
        "admission": value.get("admission").cloned().unwrap_or(Value::Null),
        "admissionAvailability": value
            .get("admissionAvailability")
            .or_else(|| value.get("admission_availability"))
            .cloned()
            .unwrap_or(Value::Null),
        "blockingReasons": value
            .get("blockingReasons")
            .or_else(|| value.get("blocking_reasons"))
            .cloned()
            .unwrap_or_else(|| json!([])),
    })
}

fn unavailable_status(reason: &'static str) -> ServerRuntimeCloudStatus {
    ServerRuntimeCloudStatus {
        ready: false,
        status: json!({
            "ready": false,
            "status": "unavailable",
            "reason": reason
        }),
    }
}

#[cfg(test)]
#[path = "node_agent_route_c_status_tests.rs"]
mod tests;
