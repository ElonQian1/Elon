use tauri::Url;

const MAX_URL_LENGTH: usize = 8_192;
const CONVERSATION_QUERY_KEYS: [&str; 5] = ["q", "udm", "aep", "hl", "csuir"];

/// Returns the bounded canonical URL of a durable Google AI Mode conversation.
///
/// Google also uses `/search?...&udm=50` as the executable URL for a prompt.
/// That transient URL must never become a restart pointer or conversation-cache
/// identity. A non-empty `csuir` is the upstream durable thread marker.
pub(super) fn sanitize_conversation_url(raw_url: &str) -> Option<String> {
    if raw_url.len() > MAX_URL_LENGTH {
        return None;
    }
    let url = raw_url.parse::<Url>().ok()?;
    if url.scheme() != "https"
        || !url.username().is_empty()
        || url.password().is_some()
        || url.port().is_some_and(|port| port != 443)
        || url.fragment().is_some()
        || !matches!(url.host_str(), Some("google.com" | "www.google.com"))
        || url.path() != "/search"
    {
        return None;
    }

    let mut values = std::collections::HashMap::new();
    for (key, value) in url.query_pairs() {
        if CONVERSATION_QUERY_KEYS.contains(&key.as_ref()) {
            values.entry(key.into_owned()).or_insert_with(|| value.into_owned());
        }
    }
    let ai_mode = values.get("udm").is_some_and(|value| value == "50")
        || values.get("aep").is_some_and(|value| value == "11");
    if !ai_mode || values.get("csuir").is_none_or(|value| value.trim().is_empty()) {
        return None;
    }

    let mut canonical = Url::parse("https://www.google.com/search").ok()?;
    {
        let mut query = canonical.query_pairs_mut();
        for key in CONVERSATION_QUERY_KEYS {
            if let Some(value) = values.get(key) {
                query.append_pair(key, value);
            }
        }
    }
    let canonical = canonical.to_string();
    (canonical.len() <= MAX_URL_LENGTH).then_some(canonical)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_only_durable_google_ai_mode_conversations() {
        assert_eq!(
            sanitize_conversation_url(
                "https://google.com/search?ved=drop&q=BTC&udm=50&csuir=thread_1234567890&hl=zh-CN",
            )
            .as_deref(),
            Some(
                "https://www.google.com/search?q=BTC&udm=50&hl=zh-CN&csuir=thread_1234567890",
            )
        );
        assert_eq!(
            sanitize_conversation_url(
                "https://www.google.com/search?aep=11&csuir=thread_1234567890",
            )
            .as_deref(),
            Some("https://www.google.com/search?aep=11&csuir=thread_1234567890")
        );
    }

    #[test]
    fn rejects_prompt_execution_and_blank_surfaces_as_restart_pointers() {
        assert!(sanitize_conversation_url(
            "https://www.google.com/search?q=private&udm=50",
        )
        .is_none());
        assert!(sanitize_conversation_url("https://www.google.com/aimode").is_none());
        assert!(sanitize_conversation_url(
            "https://www.google.com/search?q=private&udm=50&csuir=",
        )
        .is_none());
    }
}
