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
            "agentPolicy": {"mode": "default_agent_only", "source": "default"},
            "budget": {"enabled": true, "dailyCallLimit": 100, "remainingCallsToday": 99},
            "admission": {"remainingRequestsPerMinute": 11},
            "admissionAvailability": {"ready": true},
            "ignored": "not forwarded"
        }));

        assert!(server_runtime_ready_from_status_value(&status));
        assert_eq!(status["limits"]["maxRequestsPerMinute"], 12);
        assert_eq!(status["budget"]["remainingCallsToday"], 99);
        assert_eq!(status["admission"]["remainingRequestsPerMinute"], 11);
        assert_eq!(status["admissionAvailability"]["ready"], true);
        assert_eq!(status["agentPolicy"]["mode"], "default_agent_only");
        assert_eq!(status["blockingReasons"], json!([]));
        assert!(status.get("ignored").is_none());
    }

    #[test]
    fn normalizes_snake_case_admission_availability_for_older_servers() {
        let status = normalize_status_value(&json!({
            "ready": false,
            "status": "limited",
            "admission_availability": {
                "ready": false,
                "reason": "rate_limited",
                "publicMessage": "当前用户平台AI请求频率已达上限",
                "retryAfterSecs": 17
            }
        }));

        assert!(!server_runtime_ready_from_status_value(&status));
        assert_eq!(status["status"], "limited");
        assert_eq!(status["admissionAvailability"]["reason"], "rate_limited");
        assert_eq!(status["admissionAvailability"]["retryAfterSecs"], 17);
    }

    #[test]
    fn explicit_admission_or_policy_blocks_route_c_ready() {
        assert!(!server_runtime_ready_from_status_value(&json!({
            "ready": true,
            "status": "limited"
        })));
        assert!(!server_runtime_ready_from_status_value(&json!({
            "ready": true,
            "status": "ready",
            "policy": {"enabled": false}
        })));
        assert!(!server_runtime_ready_from_status_value(&json!({
            "ready": true,
            "status": "ready",
            "admissionAvailability": {
                "ready": false,
                "reason": "user_budget_exhausted"
            }
        })));
        assert!(!server_runtime_ready_from_status_value(&json!({
            "ready": true,
            "status": "ready",
            "agentPolicy": {
                "mode": "allowlist",
                "ready": false,
                "reason": "no_server_api_key_agent"
            }
        })));
        assert!(!server_runtime_ready_from_status_value(&json!({
            "ready": true,
            "status": "ready",
            "blockingReasons": [{
                "code": "platform_budget_exhausted",
                "scope": "budget",
                "message": "平台AI今日平台预算已用完"
            }]
        })));
    }

    #[test]
    fn missing_optional_protection_fields_keep_legacy_ready_status() {
        assert!(server_runtime_ready_from_status_value(&json!({
            "ready": true,
            "status": "ready",
            "policy": null,
            "agentPolicy": null,
            "admissionAvailability": null
        })));
    }

    #[test]
    fn normalizes_snake_case_agent_policy_for_older_servers() {
        let status = normalize_status_value(&json!({
            "ready": true,
            "status": "ready",
            "agent_policy": {
                "mode": "allowlist",
                "source": "ELON_SERVER_AGENT_RUNTIME_ALLOWED_AGENTS"
            }
        }));

        assert_eq!(status["agentPolicy"]["mode"], "allowlist");
        assert_eq!(
            status["agentPolicy"]["source"],
            "ELON_SERVER_AGENT_RUNTIME_ALLOWED_AGENTS"
        );
    }

    #[test]
    fn normalizes_blocking_reasons_for_node_profile() {
        let status = normalize_status_value(&json!({
            "ready": false,
            "status": "budget_exhausted",
            "blockingReasons": [{
                "code": "platform_budget_exhausted",
                "scope": "budget",
                "message": "平台AI今日平台预算已用完",
                "retryAfterSecs": 3600
            }]
        }));

        assert!(!server_runtime_ready_from_status_value(&status));
        assert_eq!(
            status["blockingReasons"][0]["code"],
            "platform_budget_exhausted"
        );
        assert_eq!(status["blockingReasons"][0]["retryAfterSecs"], 3600);
    }

    #[test]
    fn unavailable_status_is_not_ready() {
        let status = unavailable_status("cloud_unreachable");

        assert!(!status.ready);
        assert_eq!(status.status["status"], "unavailable");
        assert_eq!(status.status["reason"], "cloud_unreachable");
    }
}
