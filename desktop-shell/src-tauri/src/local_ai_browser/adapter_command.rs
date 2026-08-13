use serde_json::{Map, Value};

const CHATGPT_ACTIONS: &[&str] = &[
    "snapshot",
    "send_prompt",
    "stop_generation",
    "regenerate_response",
    "new_conversation",
    "list_conversations",
    "open_conversation",
    "open_project",
    "start_google_login",
    "list_model_options",
    "list_composer_tools",
    "collect_model_options",
    "collect_composer_tools",
    "select_model_option",
    "select_composer_tool",
];

const GOOGLE_AI_MODE_ACTIONS: &[&str] = &[
    "snapshot",
    "send_prompt",
    "stop_generation",
    "new_conversation",
];

pub fn build(
    provider_id: &str,
    provider_name: &str,
    google_ai_mode_id: &str,
    action: &str,
    value: Option<String>,
    expected_draft: Option<String>,
) -> Result<Value, String> {
    let actions = if provider_id == google_ai_mode_id {
        GOOGLE_AI_MODE_ACTIONS
    } else {
        CHATGPT_ACTIONS
    };
    if !actions.contains(&action) {
        return Err(format!("不支持的 {provider_name} 原生界面动作。"));
    }
    if matches!(action, "open_conversation" | "open_project") {
        let path = value.as_deref().unwrap_or_default();
        let safe = if action == "open_project" {
            is_safe_project_path(path)
        } else {
            is_safe_conversation_path(path)
        };
        if !safe {
            return Err(format!("{provider_name} 网页导航地址无效。"));
        }
    }
    let mut command = Map::new();
    command.insert("action".to_string(), Value::String(action.to_string()));
    if let Some(value) = value {
        if value.chars().count() > 20_000 {
            return Err(format!("{provider_name} 输入内容过长。"));
        }
        command.insert("value".to_string(), Value::String(value));
    }
    if let Some(expected_draft) = expected_draft {
        if expected_draft.chars().count() > 20_000 {
            return Err(format!("{provider_name} 网页草稿过长。"));
        }
        command.insert("expectedDraft".to_string(), Value::String(expected_draft));
    }
    Ok(Value::Object(command))
}

fn is_safe_conversation_path(path: &str) -> bool {
    let segments = path
        .strip_prefix('/')
        .map(|value| value.split('/').collect::<Vec<_>>());
    match segments.as_deref() {
        Some(["c", conversation_id]) => is_safe_route_id(conversation_id, 160),
        Some(["g", project_id, "c", conversation_id]) => {
            is_safe_project_id(project_id) && is_safe_route_id(conversation_id, 160)
        }
        _ => false,
    }
}

fn is_safe_project_path(path: &str) -> bool {
    let segments = path
        .strip_prefix('/')
        .map(|value| value.split('/').collect::<Vec<_>>());
    match segments.as_deref() {
        Some(["g", project_id]) | Some(["g", project_id, "project"]) => {
            is_safe_project_id(project_id)
        }
        _ => false,
    }
}

fn is_safe_project_id(value: &str) -> bool {
    value
        .strip_prefix("g-p-")
        .is_some_and(|suffix| is_safe_route_id(suffix, 160))
}

fn is_safe_route_id(value: &str, max: usize) -> bool {
    !value.is_empty()
        && value.len() <= max
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_and_conversation_navigation_only_accept_safe_chatgpt_paths() {
        assert!(build(
            "chatgpt",
            "ChatGPT",
            "google_ai_mode",
            "open_project",
            Some("/g/g-p-roadmap/project".into()),
            None
        )
        .is_ok());
        assert!(build(
            "chatgpt",
            "ChatGPT",
            "google_ai_mode",
            "open_conversation",
            Some("/g/g-p-roadmap/c/chat-1".into()),
            None
        )
        .is_ok());
        assert!(build(
            "chatgpt",
            "ChatGPT",
            "google_ai_mode",
            "open_project",
            Some("https://evil.example".into()),
            None
        )
        .is_err());
    }
}
