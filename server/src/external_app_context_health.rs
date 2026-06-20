//! Read-only configuration health for external app context integrations.

use serde_json::{json, Value};

pub(crate) fn public_context_health(app_id: &str) -> Value {
    match app_id {
        "fb2" => fb2_context_health(),
        _ => json!({
            "app_id": app_id,
            "status": "not_configured",
            "checks": [],
            "warnings": ["unknown_external_app_context_health"]
        }),
    }
}

fn fb2_context_health() -> Value {
    let base_url = first_non_empty_env(&[
        "ELON_EXTERNAL_APP_FB2_BASE_URL",
        "ELON_FB2_BASE_URL",
        "FB2_BASE_URL",
    ]);
    let token = first_non_empty_env(&[
        "ELON_EXTERNAL_APP_FB2_CONTEXT_TOKEN",
        "ELON_FB2_AI_CENTER_TOKEN",
        "ELON_EXTERNAL_APP_FB2_TOKEN",
        "FB2_MAIN_PROJECT_SHARED_SECRET",
    ]);
    let max_chars = env_usize("ELON_EXTERNAL_APP_CONTEXT_MAX_CHARS", 16_000, 4_000, 48_000);

    let mut warnings = Vec::new();
    if base_url.is_none() {
        warnings.push("missing_base_url");
    }
    if token.is_none() {
        warnings.push("missing_context_token");
    }
    if !env_flag("ELON_EXTERNAL_APP_FB2_CONTEXT_PACK_ENABLED", true) {
        warnings.push("context_pack_disabled");
    }

    let status = if warnings.is_empty() {
        "ready"
    } else {
        "degraded"
    };

    json!({
        "app_id": "fb2",
        "status": status,
        "checks": {
            "base_url_configured": base_url.is_some(),
            "context_token_configured": token.is_some(),
            "context_pack_enabled": env_flag("ELON_EXTERNAL_APP_FB2_CONTEXT_PACK_ENABLED", true),
            "platform_order_context_enabled": env_flag("ELON_EXTERNAL_APP_FB2_PLATFORM_ORDER_CONTEXT", false),
            "max_context_chars": max_chars,
            "timeout_secs": env_u64("ELON_EXTERNAL_APP_FB2_CONTEXT_TIMEOUT_SECS", 6, 2, 30),
            "match_limit": env_u32("ELON_EXTERNAL_APP_FB2_MATCH_CONTEXT_LIMIT", 30, 1, 100),
            "discussion_limit": env_u32("ELON_EXTERNAL_APP_FB2_DISCUSSION_CONTEXT_LIMIT", 80, 1, 200),
            "order_limit": env_u32("ELON_EXTERNAL_APP_FB2_ORDER_CONTEXT_LIMIT", 20, 1, 100)
        },
        "warnings": warnings,
        "safe_to_expose": true,
        "secret_values_exposed": false
    })
}

fn first_non_empty_env(names: &[&str]) -> Option<String> {
    names
        .iter()
        .find_map(|name| std::env::var(name).ok())
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn env_flag(name: &str, default_value: bool) -> bool {
    std::env::var(name)
        .ok()
        .map(|value| {
            matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
        .unwrap_or(default_value)
}

fn env_u32(name: &str, default_value: u32, min: u32, max: u32) -> u32 {
    std::env::var(name)
        .ok()
        .and_then(|value| value.trim().parse::<u32>().ok())
        .unwrap_or(default_value)
        .clamp(min, max)
}

fn env_u64(name: &str, default_value: u64, min: u64, max: u64) -> u64 {
    std::env::var(name)
        .ok()
        .and_then(|value| value.trim().parse::<u64>().ok())
        .unwrap_or(default_value)
        .clamp(min, max)
}

fn env_usize(name: &str, default_value: usize, min: usize, max: usize) -> usize {
    std::env::var(name)
        .ok()
        .and_then(|value| value.trim().parse::<usize>().ok())
        .unwrap_or(default_value)
        .clamp(min, max)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fb2_health_never_exposes_secret_values() {
        let health = public_context_health("fb2");
        assert_eq!(health["secret_values_exposed"], false);
        assert!(health["checks"].get("context_token_configured").is_some());
        assert!(health.get("ELON_EXTERNAL_APP_FB2_CONTEXT_TOKEN").is_none());
    }

    #[test]
    fn unknown_app_reports_not_configured() {
        let health = public_context_health("unknown");
        assert_eq!(health["status"], "not_configured");
    }
}
