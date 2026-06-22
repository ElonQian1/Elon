// server/src/server_agent_runtime_policy.rs

use serde::Serialize;

pub(crate) const SERVER_AGENT_RUNTIME_ENABLED_ENV: &str = "ELON_SERVER_AGENT_RUNTIME_ENABLED";

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
        "Route C 服务器模型当前已由平台运营开关暂停，请稍后再试或改用本机 CLI / 自带 API Key。"
            .to_string()
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
    use super::{ServerAgentRuntimePolicy, SERVER_AGENT_RUNTIME_ENABLED_ENV};

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
            assert!(policy.public_disabled_message().contains("Route C"));
        }
    }

    #[test]
    fn unknown_values_fail_open_to_avoid_accidental_outage() {
        let policy = ServerAgentRuntimePolicy::from_env_value(Some("maybe"));

        assert!(policy.enabled);
        assert_eq!(policy.source, SERVER_AGENT_RUNTIME_ENABLED_ENV);
        assert!(policy.reason.is_none());
    }
}
