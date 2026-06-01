use std::time::{SystemTime, UNIX_EPOCH};

pub fn project_ws_job_key(
    project_id: &str,
    user_id: &str,
    conversation_id: &str,
    client_request_id: &str,
) -> String {
    format!(
        "{}\u{1f}{}\u{1f}{}\u{1f}{}",
        project_id, user_id, conversation_id, client_request_id
    )
}

pub fn project_ws_fingerprint(
    conversation_id: &str,
    agent_name: Option<&str>,
    execution_mode: &str,
    message: &str,
) -> String {
    format!(
        "{}\u{1f}{}\u{1f}{}\u{1f}{}",
        conversation_id,
        agent_name.unwrap_or(""),
        execution_mode,
        message
    )
}

pub fn clean_trace_id(input: Option<&str>) -> String {
    let cleaned = input
        .unwrap_or_default()
        .trim()
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.' | ':'))
        .take(120)
        .collect::<String>();
    if cleaned.is_empty() {
        format!("srv_{}", current_wall_time_ms())
    } else {
        cleaned
    }
}

pub fn codex_prewarm_key(
    project_id: &str,
    user_id: &str,
    conversation_id: &str,
    agent: Option<&str>,
    workspace_key: &str,
) -> String {
    format!(
        "{}|{}|{}|{}|{}",
        project_id,
        user_id,
        conversation_id,
        agent.unwrap_or("default"),
        workspace_key
    )
}

fn current_wall_time_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cleans_trace_id_to_safe_ascii() {
        assert_eq!(
            clean_trace_id(Some(" abc-DEF_123.:中文/unsafe ")),
            "abc-DEF_123.:unsafe"
        );
    }

    #[test]
    fn uses_default_agent_for_prewarm_key() {
        assert_eq!(
            codex_prewarm_key("p", "u", "c", None, "workspace"),
            "p|u|c|default|workspace"
        );
    }

    #[test]
    fn builds_ws_job_key_with_unit_separator() {
        assert_eq!(
            project_ws_job_key("p", "u", "c", "r"),
            "p\u{1f}u\u{1f}c\u{1f}r"
        );
    }
}
