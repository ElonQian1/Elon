use crate::{pc_agent_runtime_choice::PcRuntimeRoutePreference, types::UserAgentConfig};

/// Decide whether the server may try another platform API when the preferred
/// model selected by the PC UI is temporarily unavailable.
///
/// The PC UI sends the selected model name even when the user chose “自动选择”.
/// Treating every non-empty `agent` as an explicit, non-fallback choice made an
/// automatic Codex/GPT selection fail hard on the second message as soon as its
/// upstream request timed out. A direct user API configuration remains pinned
/// to that user's endpoint, while automatic/platform routes can fail over to
/// the server's other eligible API agents.
pub(crate) fn allow_server_agent_fallback(
    route: Option<PcRuntimeRoutePreference>,
    user_config: Option<&UserAgentConfig>,
) -> bool {
    match route {
        Some(PcRuntimeRoutePreference::RouteB) => false,
        Some(
            PcRuntimeRoutePreference::RouteA
            | PcRuntimeRoutePreference::RouteC2
            | PcRuntimeRoutePreference::RouteC3,
        ) => false,
        Some(PcRuntimeRoutePreference::RouteC) => true,
        None => !user_config.map(has_user_api_override).unwrap_or(false),
    }
}

fn has_user_api_override(config: &UserAgentConfig) -> bool {
    config.has_direct_custom_api()
        || config.has_api_key_reference()
        || config
            .api_base
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty())
        || config
            .model
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty())
}

#[cfg(test)]
mod tests {
    use super::{allow_server_agent_fallback, has_user_api_override};
    use crate::{pc_agent_runtime_choice::PcRuntimeRoutePreference, types::UserAgentConfig};

    #[test]
    fn automatic_route_can_fail_over_even_with_selected_model_name() {
        assert!(allow_server_agent_fallback(None, None));
        assert!(allow_server_agent_fallback(
            Some(PcRuntimeRoutePreference::RouteC),
            None
        ));
        assert!(!allow_server_agent_fallback(
            Some(PcRuntimeRoutePreference::RouteB),
            None
        ));
    }

    #[test]
    fn automatic_route_stays_on_user_api_when_user_overrides_endpoint() {
        let config = UserAgentConfig {
            api_base: Some("https://user.example/v1".to_string()),
            model: Some("user-model".to_string()),
            api_key: Some("user-key".to_string()),
            ..UserAgentConfig::default()
        };
        assert!(has_user_api_override(&config));
        assert!(!allow_server_agent_fallback(None, Some(&config)));
    }
}
