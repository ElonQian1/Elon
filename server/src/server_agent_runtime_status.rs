// server/src/server_agent_runtime_status.rs

use serde::Serialize;

use crate::{
    server_agent_runtime_budget::ServerRuntimeBudgetStatus,
    server_agent_runtime_guard::{
        ServerRuntimeAdmissionAvailability, ServerRuntimeAdmissionSnapshot,
        ServerRuntimeProtectionStatus,
    },
    server_agent_runtime_limits::ServerAgentRuntimeLimits,
    server_agent_runtime_policy::{ServerAgentRuntimeAgentPolicy, ServerAgentRuntimePolicy},
};

#[derive(Debug, Serialize)]
pub(crate) struct ServerAgentRuntimeStatus {
    pub ready: bool,
    pub status: &'static str,
    pub agent: Option<ServerAgentRuntimeAgentStatus>,
    pub limits: ServerAgentRuntimeLimits,
    pub protection: ServerRuntimeProtectionStatus,
    pub policy: ServerAgentRuntimePolicy,
    #[serde(rename = "agentPolicy")]
    pub agent_policy: ServerAgentRuntimeAgentPolicy,
    pub budget: ServerRuntimeBudgetStatus,
    pub admission: ServerRuntimeAdmissionSnapshot,
    #[serde(rename = "admissionAvailability")]
    pub admission_availability: ServerRuntimeAdmissionAvailability,
    #[serde(rename = "blockingReasons", skip_serializing_if = "Vec::is_empty")]
    pub blocking_reasons: Vec<ServerAgentRuntimeBlockingReason>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ServerAgentRuntimeAgentStatus {
    pub name: String,
    pub model: String,
    pub usage_mode: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ServerAgentRuntimeBlockingReason {
    pub code: &'static str,
    pub scope: &'static str,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry_after_secs: Option<u64>,
}

pub(crate) fn route_c_status_code(
    policy: &ServerAgentRuntimePolicy,
    agent_available: bool,
    agent_status: &'static str,
    budget: &ServerRuntimeBudgetStatus,
    admission_availability: &ServerRuntimeAdmissionAvailability,
) -> &'static str {
    if !policy.enabled {
        "disabled"
    } else if !agent_available {
        agent_status
    } else if budget.status == "user_exhausted" {
        "user_budget_exhausted"
    } else if !budget.ready() {
        "budget_exhausted"
    } else if !admission_availability.ready {
        "limited"
    } else {
        "ready"
    }
}

pub(crate) fn route_c_blocking_reasons(
    policy: &ServerAgentRuntimePolicy,
    agent_available: bool,
    agent_status: &'static str,
    budget: &ServerRuntimeBudgetStatus,
    admission_availability: &ServerRuntimeAdmissionAvailability,
) -> Vec<ServerAgentRuntimeBlockingReason> {
    let mut reasons = Vec::new();
    if !policy.enabled {
        reasons.push(ServerAgentRuntimeBlockingReason {
            code: "operator_disabled",
            scope: "policy",
            message: policy.public_disabled_message(),
            retry_after_secs: None,
        });
    }

    if !agent_available {
        reasons.push(agent_blocking_reason(agent_status));
    }

    if let Some(reason) = budget_blocking_reason(budget) {
        reasons.push(reason);
    }

    if let Some(reason) = admission_blocking_reason(admission_availability) {
        reasons.push(reason);
    }

    reasons
}

fn agent_blocking_reason(agent_status: &'static str) -> ServerAgentRuntimeBlockingReason {
    match agent_status {
        "agent_not_allowed" => ServerAgentRuntimeBlockingReason {
            code: "agent_policy_blocked",
            scope: "agentPolicy",
            message: "Route C 服务器模型不允许当前 agent 策略".to_string(),
            retry_after_secs: None,
        },
        "unsupported_agent_usage_mode" => ServerAgentRuntimeBlockingReason {
            code: "no_server_api_key_agent",
            scope: "agentPolicy",
            message: "Route C 只允许 server_api_key agent；当前 agent 模式不会被调用".to_string(),
            retry_after_secs: None,
        },
        _ => ServerAgentRuntimeBlockingReason {
            code: "agent_unavailable",
            scope: "agent",
            message: "服务器未配置可用 Route C agent".to_string(),
            retry_after_secs: None,
        },
    }
}

fn budget_blocking_reason(
    budget: &ServerRuntimeBudgetStatus,
) -> Option<ServerAgentRuntimeBlockingReason> {
    match budget.status {
        "user_exhausted" => Some(ServerAgentRuntimeBlockingReason {
            code: "user_budget_exhausted",
            scope: "budget",
            message: "Route C 今日个人额度已用完".to_string(),
            retry_after_secs: Some(budget.reset_after_secs),
        }),
        "exhausted" => Some(ServerAgentRuntimeBlockingReason {
            code: "platform_budget_exhausted",
            scope: "budget",
            message: "Route C 今日平台预算已用完".to_string(),
            retry_after_secs: Some(budget.reset_after_secs),
        }),
        "unavailable" => Some(ServerAgentRuntimeBlockingReason {
            code: "budget_unavailable",
            scope: "budget",
            message: "Route C 预算系统暂时不可用".to_string(),
            retry_after_secs: Some(budget.reset_after_secs),
        }),
        _ => None,
    }
}

fn admission_blocking_reason(
    admission_availability: &ServerRuntimeAdmissionAvailability,
) -> Option<ServerAgentRuntimeBlockingReason> {
    if admission_availability.ready {
        return None;
    }
    Some(ServerAgentRuntimeBlockingReason {
        code: admission_availability.reason.unwrap_or("admission_limited"),
        scope: "admission",
        message: admission_availability
            .public_message
            .unwrap_or("Route C 当前容量受限")
            .to_string(),
        retry_after_secs: admission_availability.retry_after_secs,
    })
}

#[cfg(test)]
mod tests {
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
    use crate::server_agent_runtime_policy::{
        ServerAgentRuntimeAgentPolicy, ServerAgentRuntimePolicy,
    };
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
            public_message: Some("当前用户 Route C 远程模型请求频率已达上限"),
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
        assert!(reasons[0].message.contains("运营开关"));
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
}
