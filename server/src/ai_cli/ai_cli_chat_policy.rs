use crate::intent_router;

pub(crate) const PROJECT_LIGHTWEIGHT_CHAT_ENABLED_ENV: &str = "AI_PROJECT_LIGHTWEIGHT_CHAT_ENABLED";

pub(crate) fn project_lightweight_chat_split_enabled() -> bool {
    project_lightweight_chat_split_enabled_from(|name| std::env::var(name).ok())
}

pub(crate) fn project_lightweight_chat_split_enabled_from<F>(lookup: F) -> bool
where
    F: Fn(&str) -> Option<String>,
{
    env_bool_from(
        lookup(PROJECT_LIGHTWEIGHT_CHAT_ENABLED_ENV).as_deref(),
        false,
    )
}

pub(crate) fn should_use_project_lightweight_chat(
    split_enabled: bool,
    request_mode_is_plan: bool,
    route: intent_router::CapabilityRoute,
    user_message: &str,
) -> bool {
    split_enabled
        && !request_mode_is_plan
        && route == intent_router::CapabilityRoute::ChatAgent
        && !intent_router::looks_like_development_request(user_message)
}

pub(crate) fn prompt_route_for_project_chat(
    split_enabled: bool,
    route: intent_router::CapabilityRoute,
) -> intent_router::CapabilityRoute {
    if split_enabled || route != intent_router::CapabilityRoute::ChatAgent {
        route
    } else {
        intent_router::CapabilityRoute::CodeAgent
    }
}

fn env_bool_from(value: Option<&str>, default: bool) -> bool {
    let Some(value) = value else {
        return default;
    };
    match value.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "y" | "on" => true,
        "0" | "false" | "no" | "n" | "off" => false,
        _ => default,
    }
}
