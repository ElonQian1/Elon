// server/src/server_agent_runtime_policy.rs

use serde::Serialize;

pub(crate) const SERVER_AGENT_RUNTIME_ENABLED_ENV: &str = "ELON_SERVER_AGENT_RUNTIME_ENABLED";
pub(crate) const SERVER_AGENT_RUNTIME_ALLOWED_AGENTS_ENV: &str =
    "ELON_SERVER_AGENT_RUNTIME_ALLOWED_AGENTS";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ServerAgentRuntimePolicy {
    pub enabled: bool,
    pub source: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

impl ServerAgentRuntimePolicy {
    pub(crate) fn current() -> Self {
        Self::from_env_value(
            std::env::var(SERVER_AGENT_RUNTIME_ENABLED_ENV)
                .ok()
                .as_deref(),
        )
    }

    pub(crate) fn from_env_value(value: Option<&str>) -> Self {
        let Some(raw) = value.map(str::trim).filter(|value| !value.is_empty()) else {
            return Self {
                enabled: true,
                source: "default",
                reason: None,
            };
        };

        if disables_runtime(raw) {
            return Self {
                enabled: false,
                source: SERVER_AGENT_RUNTIME_ENABLED_ENV,
                reason: Some(format!(
                    "Route C disabled by {SERVER_AGENT_RUNTIME_ENABLED_ENV}={raw}"
                )),
            };
        }

        Self {
            enabled: true,
            source: SERVER_AGENT_RUNTIME_ENABLED_ENV,
            reason: None,
        }
    }

    pub(crate) fn public_disabled_message(&self) -> String {
        "平台AI当前已由运营暂停，请稍后再试，或改用本机AI / 我的 API key。".to_string()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ServerAgentRuntimeAgentPolicy {
    pub mode: &'static str,
    pub source: &'static str,
    #[serde(skip_serializing)]
    allow_any: bool,
    #[serde(skip_serializing)]
    allowed_agents: Vec<String>,
}

impl ServerAgentRuntimeAgentPolicy {
    pub(crate) fn current() -> Self {
        Self::from_env_value(
            std::env::var(SERVER_AGENT_RUNTIME_ALLOWED_AGENTS_ENV)
                .ok()
                .as_deref(),
        )
    }

    pub(crate) fn from_env_value(value: Option<&str>) -> Self {
        let Some(raw) = value.map(str::trim).filter(|value| !value.is_empty()) else {
            return Self {
                mode: "default_agent_only",
                source: "default",
                allow_any: false,
                allowed_agents: Vec::new(),
            };
        };

        let allowed_agents = raw
            .split(',')
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| value.to_ascii_lowercase())
            .collect::<Vec<_>>();
        let allow_any = allowed_agents.iter().any(|value| value == "*");

        Self {
            mode: if allow_any { "any" } else { "allowlist" },
            source: SERVER_AGENT_RUNTIME_ALLOWED_AGENTS_ENV,
            allow_any,
            allowed_agents,
        }
    }

    pub(crate) fn allows_requested_agent(
        &self,
        requested_agent: Option<&str>,
        default_agent: &str,
    ) -> bool {
        let Some(requested_agent) = requested_agent
            .map(str::trim)
            .filter(|value| !value.is_empty())
        else {
            return true;
        };

        let requested = requested_agent.to_ascii_lowercase();
        if !default_agent.trim().is_empty()
            && requested == default_agent.trim().to_ascii_lowercase()
        {
            return true;
        }
        self.allow_any || self.allowed_agents.iter().any(|agent| agent == &requested)
    }

    pub(crate) fn public_denied_message(&self) -> String {
        "平台AI暂不允许选择这个模型通道；请使用默认平台AI，或等待运营开放。".to_string()
    }
}

fn disables_runtime(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "0" | "false" | "off" | "no" | "disabled" | "disable"
    )
}

#[cfg(test)]
mod tests {
    use super::{
        ServerAgentRuntimeAgentPolicy, ServerAgentRuntimePolicy,
        SERVER_AGENT_RUNTIME_ALLOWED_AGENTS_ENV, SERVER_AGENT_RUNTIME_ENABLED_ENV,
    };

    #[test]
    fn defaults_to_enabled_when_unset() {
        let policy = ServerAgentRuntimePolicy::from_env_value(None);

        assert!(policy.enabled);
        assert_eq!(policy.source, "default");
        assert!(policy.reason.is_none());
    }

    #[test]
    fn recognizes_operator_disable_values() {
        for value in ["0", "false", "OFF", "no", "disabled", " disable "] {
            let policy = ServerAgentRuntimePolicy::from_env_value(Some(value));

            assert!(!policy.enabled, "{value} should disable Route C");
            assert_eq!(policy.source, SERVER_AGENT_RUNTIME_ENABLED_ENV);
            assert!(policy
                .reason
                .as_deref()
                .is_some_and(|reason| reason.contains(SERVER_AGENT_RUNTIME_ENABLED_ENV)));
            assert!(policy.public_disabled_message().contains("平台AI"));
        }
    }

    #[test]
    fn unknown_values_fail_open_to_avoid_accidental_outage() {
        let policy = ServerAgentRuntimePolicy::from_env_value(Some("maybe"));

        assert!(policy.enabled);
        assert_eq!(policy.source, SERVER_AGENT_RUNTIME_ENABLED_ENV);
        assert!(policy.reason.is_none());
    }

    #[test]
    fn route_c_agent_policy_defaults_to_default_agent_only() {
        let policy = ServerAgentRuntimeAgentPolicy::from_env_value(None);

        assert_eq!(policy.mode, "default_agent_only");
        assert_eq!(policy.source, "default");
        assert!(policy.allows_requested_agent(None, "main"));
        assert!(policy.allows_requested_agent(Some("main"), "main"));
        assert!(!policy.allows_requested_agent(Some("expensive"), "main"));
        assert!(policy.public_denied_message().contains("平台AI"));
    }

    #[test]
    fn route_c_agent_policy_accepts_operator_allowlist() {
        let policy = ServerAgentRuntimeAgentPolicy::from_env_value(Some("cheap, route-c-fast"));

        assert_eq!(policy.mode, "allowlist");
        assert_eq!(policy.source, SERVER_AGENT_RUNTIME_ALLOWED_AGENTS_ENV);
        assert!(policy.allows_requested_agent(Some("cheap"), "main"));
        assert!(policy.allows_requested_agent(Some("ROUTE-C-FAST"), "main"));
        assert!(!policy.allows_requested_agent(Some("expensive"), "main"));
    }

    #[test]
    fn route_c_agent_policy_can_be_explicitly_opened_by_operator() {
        let policy = ServerAgentRuntimeAgentPolicy::from_env_value(Some("*"));

        assert_eq!(policy.mode, "any");
        assert!(policy.allows_requested_agent(Some("expensive"), "main"));
    }
}
