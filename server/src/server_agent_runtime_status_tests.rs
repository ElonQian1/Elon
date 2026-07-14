use super::{
    route_c_blocking_reasons, route_c_status_code, ServerAgentRuntimeAgentStatus,
    ServerAgentRuntimeStatus,
};
use crate::server_agent_runtime_budget::ServerRuntimeBudgetStatus;
use crate::server_agent_runtime_guard::{
    admission_availability, protection_status, ServerRuntimeAdmissionAvailability,
    ServerRuntimeAdmissionSnapshot,
};
use crate::server_agent_runtime_limits::ServerAgentRuntimeLimits;
use crate::server_agent_runtime_policy::{ServerAgentRuntimeAgentPolicy, ServerAgentRuntimePolicy};
use serde_json::json;

fn unlimited_budget() -> ServerRuntimeBudgetStatus {
    ServerRuntimeBudgetStatus {
        enabled: false,
        status: "unlimited",
        source: "default",
        used_calls_today: 0,
        daily_call_limit: None,
        remaining_calls_today: None,
        per_user_enabled: false,
        per_user_source: "default",
        used_calls_today_for_user: None,
        per_user_daily_call_limit: None,
        remaining_calls_today_for_user: None,
        reset_after_secs: 60,
    }
}

fn ready_admission() -> ServerRuntimeAdmissionAvailability {
    ServerRuntimeAdmissionAvailability {
        ready: true,
        reason: None,
        public_message: None,
        retry_after_secs: None,
    }
}

#[test]
fn runtime_status_serializes_agent_policy_for_operations() {
    let status = ServerAgentRuntimeStatus {
        ready: true,
        status: "ready",
        agent: Some(ServerAgentRuntimeAgentStatus {
            name: "main".to_string(),
            model: "route-c-model".to_string(),
            usage_mode: "server_api_key".to_string(),
        }),
        limits: ServerAgentRuntimeLimits::current(),
        protection: protection_status(),
        policy: ServerAgentRuntimePolicy::from_env_value(None),
        agent_policy: ServerAgentRuntimeAgentPolicy::from_env_value(Some("route-c-fast")),
        budget: unlimited_budget(),
        admission: ServerRuntimeAdmissionSnapshot {
            in_flight_global: 0,
            max_concurrent_global: 24,
            remaining_concurrent_global: 24,
            in_flight_for_user: 0,
            max_concurrent_per_user: 2,
            remaining_concurrent_for_user: 2,
            recent_requests_per_minute: 0,
            max_requests_per_minute: 12,
            remaining_requests_per_minute: 12,
            rate_limit_retry_after_secs: None,
            duplicate_request_window_secs: 5,
            recent_duplicate_fingerprints: 0,
        },
        admission_availability: ready_admission(),
        blocking_reasons: Vec::new(),
    };

    let value = serde_json::to_value(status).unwrap();

    assert_eq!(value["agentPolicy"]["mode"], "allowlist");
    assert_eq!(
        value["agentPolicy"]["source"],
        "ELON_SERVER_AGENT_RUNTIME_ALLOWED_AGENTS"
    );
    assert!(value.get("agent_policy").is_none());
    assert!(value["agentPolicy"].get("allowedAgents").is_none());
    assert!(value.get("blockingReasons").is_none());
}

#[test]
fn blocking_reasons_cover_budget_admission_and_agent_policy() {
    let mut budget = unlimited_budget();
    budget.enabled = true;
    budget.status = "user_exhausted";
    budget.per_user_enabled = true;
    budget.per_user_daily_call_limit = Some(2);
    budget.remaining_calls_today_for_user = Some(0);
    budget.reset_after_secs = 1800;

    let admission = ServerRuntimeAdmissionAvailability {
        ready: false,
        reason: Some("rate_limited"),
        public_message: Some("当前用户平台AI请求频率已达上限"),
        retry_after_secs: Some(17),
    };
    let reasons = route_c_blocking_reasons(
        &ServerAgentRuntimePolicy::from_env_value(None),
        false,
        "unsupported_agent_usage_mode",
        &budget,
        &admission,
    );

    assert_eq!(
        reasons.iter().map(|reason| reason.code).collect::<Vec<_>>(),
        vec![
            "no_server_api_key_agent",
            "user_budget_exhausted",
            "rate_limited"
        ]
    );
    assert_eq!(reasons[1].retry_after_secs, Some(1800));
    assert_eq!(reasons[2].retry_after_secs, Some(17));
}

#[test]
fn disabled_policy_status_has_public_blocking_reason() {
    let policy = ServerAgentRuntimePolicy::from_env_value(Some("off"));
    let budget = unlimited_budget();
    let admission = ready_admission();
    let status = route_c_status_code(&policy, true, "ready", &budget, &admission);
    let reasons = route_c_blocking_reasons(&policy, true, "ready", &budget, &admission);

    assert_eq!(status, "disabled");
    assert_eq!(reasons[0].code, "operator_disabled");
    assert!(reasons[0].message.contains("运营暂停"));
}

#[test]
fn admission_snapshot_reason_is_reused_as_blocking_reason() {
    let mut snapshot = ServerRuntimeAdmissionSnapshot {
        in_flight_global: 0,
        max_concurrent_global: 24,
        remaining_concurrent_global: 24,
        in_flight_for_user: 0,
        max_concurrent_per_user: 2,
        remaining_concurrent_for_user: 2,
        recent_requests_per_minute: 12,
        max_requests_per_minute: 12,
        remaining_requests_per_minute: 0,
        rate_limit_retry_after_secs: Some(23),
        duplicate_request_window_secs: 5,
        recent_duplicate_fingerprints: 0,
    };
    let availability = admission_availability(&snapshot);
    let reason = route_c_blocking_reasons(
        &ServerAgentRuntimePolicy::from_env_value(None),
        true,
        "ready",
        &unlimited_budget(),
        &availability,
    )
    .pop()
    .unwrap();

    assert_eq!(reason.code, "rate_limited");
    assert_eq!(reason.scope, "admission");
    assert_eq!(reason.retry_after_secs, Some(23));

    snapshot.remaining_requests_per_minute = 1;
    assert!(admission_availability(&snapshot).ready);
}

#[test]
fn blocking_reasons_serialize_without_prompt_or_agent_allowlist() {
    let mut budget = unlimited_budget();
    budget.status = "exhausted";
    budget.reset_after_secs = 3600;
    let reasons = route_c_blocking_reasons(
        &ServerAgentRuntimePolicy::from_env_value(None),
        true,
        "ready",
        &budget,
        &ready_admission(),
    );
    let value = serde_json::to_value(json!({ "blockingReasons": reasons })).unwrap();
    let text = serde_json::to_string(&value).unwrap();

    assert!(text.contains("platform_budget_exhausted"));
    assert!(!text.contains("sk-"));
    assert!(!text.contains("allowedAgents"));
}
