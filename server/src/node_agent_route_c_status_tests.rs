use super::{normalize_status_value, server_runtime_ready_from_status_value, unavailable_status};
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
