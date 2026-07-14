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
            message: "平台AI不允许当前模型通道策略".to_string(),
            retry_after_secs: None,
        },
        "unsupported_agent_usage_mode" => ServerAgentRuntimeBlockingReason {
            code: "no_server_api_key_agent",
            scope: "agentPolicy",
            message: "平台AI只允许平台 API key 模式；当前模型通道不会被调用".to_string(),
            retry_after_secs: None,
        },
        _ => ServerAgentRuntimeBlockingReason {
            code: "agent_unavailable",
            scope: "agent",
            message: "服务器未配置可用的平台AI模型通道".to_string(),
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
            message: "平台AI今日个人额度已用完".to_string(),
            retry_after_secs: Some(budget.reset_after_secs),
        }),
        "exhausted" => Some(ServerAgentRuntimeBlockingReason {
            code: "platform_budget_exhausted",
            scope: "budget",
            message: "平台AI今日平台预算已用完".to_string(),
            retry_after_secs: Some(budget.reset_after_secs),
        }),
        "unavailable" => Some(ServerAgentRuntimeBlockingReason {
            code: "budget_unavailable",
            scope: "budget",
            message: "平台AI预算系统暂时不可用".to_string(),
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
            .unwrap_or("平台AI当前容量受限")
            .to_string(),
        retry_after_secs: admission_availability.retry_after_secs,
    })
}

#[cfg(test)]
#[path = "server_agent_runtime_status_tests.rs"]
mod tests;
