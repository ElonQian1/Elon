//! Configuration helpers for external app context fetching.

pub(crate) const FB2_APP_ID: &str = "fb2";
pub(crate) const FB2_CONTEXT_HEADER: &str = "X-FB2-AI-CENTER-TOKEN";
pub(crate) const FB2_CONTEXT_USER_ID_HEADER: &str = "X-FB2-AI-CONTEXT-USER-ID";
pub(crate) const FB2_CONTEXT_SCOPE_HEADER: &str = "X-FB2-AI-CONTEXT-SCOPE";
pub(crate) const FB2_PLATFORM_ORDER_SUMMARY_SCOPE: &str = "platform_order_summary";

const DEFAULT_MATCH_LIMIT: u32 = 30;
const DEFAULT_DISCUSSION_LIMIT: u32 = 80;
const DEFAULT_ORDER_LIMIT: u32 = 20;
const DEFAULT_TIMEOUT_SECS: u64 = 6;
const DEFAULT_TOOL_EXECUTION_TIMEOUT_SECS: u64 = 6;

pub(crate) fn fb2_base_url() -> Option<String> {
    first_non_empty_env(&[
        "ELON_EXTERNAL_APP_FB2_BASE_URL",
        "ELON_FB2_BASE_URL",
        "FB2_BASE_URL",
    ])
    .map(|value| value.trim_end_matches('/').to_string())
}

pub(crate) fn fb2_context_token() -> Option<String> {
    first_non_empty_env(&[
        "ELON_EXTERNAL_APP_FB2_CONTEXT_TOKEN",
        "ELON_FB2_AI_CENTER_TOKEN",
        "ELON_EXTERNAL_APP_FB2_TOKEN",
        "FB2_MAIN_PROJECT_SHARED_SECRET",
    ])
}

pub(crate) fn match_limit() -> u32 {
    std::env::var("ELON_EXTERNAL_APP_FB2_MATCH_CONTEXT_LIMIT")
        .ok()
        .and_then(|value| value.trim().parse::<u32>().ok())
        .unwrap_or(DEFAULT_MATCH_LIMIT)
        .clamp(1, 100)
}

pub(crate) fn discussion_limit() -> u32 {
    std::env::var("ELON_EXTERNAL_APP_FB2_DISCUSSION_CONTEXT_LIMIT")
        .ok()
        .and_then(|value| value.trim().parse::<u32>().ok())
        .unwrap_or(DEFAULT_DISCUSSION_LIMIT)
        .clamp(1, 200)
}

pub(crate) fn order_limit() -> u32 {
    std::env::var("ELON_EXTERNAL_APP_FB2_ORDER_CONTEXT_LIMIT")
        .ok()
        .and_then(|value| value.trim().parse::<u32>().ok())
        .unwrap_or(DEFAULT_ORDER_LIMIT)
        .clamp(1, 100)
}

pub(crate) fn context_pack_enabled() -> bool {
    env_flag("ELON_EXTERNAL_APP_FB2_CONTEXT_PACK_ENABLED", true)
}

pub(crate) fn platform_order_summary_enabled() -> bool {
    env_flag("ELON_EXTERNAL_APP_FB2_PLATFORM_ORDER_CONTEXT", false)
}

pub(crate) fn fb2_request_context_headers(
    external_user_id: Option<&str>,
    include_platform_order_summary: bool,
) -> Vec<(&'static str, String)> {
    let mut headers = Vec::new();
    if let Some(user_id) = external_user_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        headers.push((FB2_CONTEXT_USER_ID_HEADER, user_id.to_string()));
    }
    if include_platform_order_summary {
        headers.push((
            FB2_CONTEXT_SCOPE_HEADER,
            FB2_PLATFORM_ORDER_SUMMARY_SCOPE.to_string(),
        ));
    }
    headers
}

pub(crate) fn timeout_secs() -> u64 {
    std::env::var("ELON_EXTERNAL_APP_FB2_CONTEXT_TIMEOUT_SECS")
        .ok()
        .and_then(|value| value.trim().parse::<u64>().ok())
        .unwrap_or(DEFAULT_TIMEOUT_SECS)
        .clamp(2, 30)
}

pub(crate) fn tool_execution_enabled() -> bool {
    env_flag("ELON_EXTERNAL_APP_FB2_TOOL_EXECUTION_ENABLED", true)
}

pub(crate) fn tool_execution_timeout_secs() -> u64 {
    std::env::var("ELON_EXTERNAL_APP_FB2_TOOL_EXECUTION_TIMEOUT_SECS")
        .ok()
        .and_then(|value| value.trim().parse::<u64>().ok())
        .unwrap_or(DEFAULT_TOOL_EXECUTION_TIMEOUT_SECS)
        .clamp(1, 15)
}

pub(crate) fn infer_lottery_type(topic_hint: Option<&str>) -> Option<String> {
    let text = topic_hint?.to_ascii_lowercase();
    if text.contains("北单") || text.contains("beidan") {
        Some("BeiDan".to_string())
    } else if text.contains("竞彩") || text.contains("jingcai") {
        Some("JingCai".to_string())
    } else {
        None
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn infers_fb2_lottery_type_from_topic_hint() {
        assert_eq!(
            infer_lottery_type(Some("今天竞彩怎么看")),
            Some("JingCai".into())
        );
        assert_eq!(infer_lottery_type(Some("北单赛事")), Some("BeiDan".into()));
        assert_eq!(infer_lottery_type(Some("足球比赛")), None);
    }

    #[test]
    fn env_flag_defaults_when_missing() {
        assert!(env_flag("__ELON_TEST_MISSING_FLAG__", true));
        assert!(!env_flag("__ELON_TEST_MISSING_FLAG__", false));
    }

    #[test]
    fn builds_fb2_permission_headers_for_user_and_platform_scope() {
        assert_eq!(
            fb2_request_context_headers(Some("  user-1  "), true),
            vec![
                (FB2_CONTEXT_USER_ID_HEADER, "user-1".to_string()),
                (
                    FB2_CONTEXT_SCOPE_HEADER,
                    FB2_PLATFORM_ORDER_SUMMARY_SCOPE.to_string()
                )
            ]
        );
        assert_eq!(fb2_request_context_headers(Some(""), false), Vec::new());
    }
}
