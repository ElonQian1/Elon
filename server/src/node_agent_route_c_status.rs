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
    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
    {
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
    value
        .get("ready")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false)
}

fn normalize_status_value(value: &Value) -> Value {
    json!({
        "ready": value.get("ready").and_then(Value::as_bool).unwrap_or(false),
        "status": value.get("status").and_then(Value::as_str).unwrap_or("unknown"),
        "agent": value.get("agent").cloned().unwrap_or(Value::Null),
        "limits": value.get("limits").cloned().unwrap_or(Value::Null),
        "protection": value.get("protection").cloned().unwrap_or(Value::Null),
        "policy": value.get("policy").cloned().unwrap_or(Value::Null),
        "admission": value.get("admission").cloned().unwrap_or(Value::Null),
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
mod tests {
    use super::{
        normalize_status_value, server_runtime_ready_from_status_value, unavailable_status,
    };
    use serde_json::json;

    #[test]
    fn parses_server_runtime_ready_status() {
        assert!(server_runtime_ready_from_status_value(
            &json!({"ready": true})
        ));
        assert!(!server_runtime_ready_from_status_value(
            &json!({"ready": false})
        ));
        assert!(!server_runtime_ready_from_status_value(&json!({
            "ready": false,
            "status": "disabled",
            "policy": {"enabled": false}
        })));
        assert!(!server_runtime_ready_from_status_value(&json!({
            "status": "ready"
        })));
    }

    #[test]
    fn normalizes_route_c_status_for_node_profile() {
        let status = normalize_status_value(&json!({
            "ready": true,
            "status": "ready",
            "agent": {"name": "pc-route-c", "model": "gpt-test"},
            "limits": {"maxRequestsPerMinute": 12, "maxConcurrentPerUser": 2},
            "protection": {"admissionControl": "global and per-user concurrency"},
            "policy": {"enabled": true},
            "admission": {"remainingRequestsPerMinute": 11},
            "ignored": "not forwarded"
        }));

        assert!(server_runtime_ready_from_status_value(&status));
        assert_eq!(status["limits"]["maxRequestsPerMinute"], 12);
        assert_eq!(status["admission"]["remainingRequestsPerMinute"], 11);
        assert!(status.get("ignored").is_none());
    }

    #[test]
    fn unavailable_status_is_not_ready() {
        let status = unavailable_status("cloud_unreachable");

        assert!(!status.ready);
        assert_eq!(status.status["status"], "unavailable");
        assert_eq!(status.status["reason"], "cloud_unreachable");
    }
}
