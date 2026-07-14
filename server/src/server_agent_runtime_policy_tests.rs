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
