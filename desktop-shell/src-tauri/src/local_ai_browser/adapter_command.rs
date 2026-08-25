use serde_json::{Map, Value};

pub(super) const CHATGPT_ACTIONS: &[&str] = &[
    "snapshot",
    "refresh_current_conversation",
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
    "request_attachment_upload",
    "open_model_selector",
    "open_composer_tools",
    "start_dictation",
    "cancel_dictation",
    "submit_dictation",
    "remove_attachment",
    "dismiss_composer_menu",
    "list_navigation",
    "collect_navigation",
    "select_navigation",
    "dismiss_navigation",
    "snapshot_ui_manifest",
    "invoke_ui_control",
];

pub(super) const GOOGLE_AI_MODE_ACTIONS: &[&str] = &[
    "snapshot",
    "send_prompt",
    "stop_generation",
    "new_conversation",
    "list_conversations",
    "open_conversation",
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum PageCommandBinding {
    None,
    ChatGptDocument,
}

pub fn build(
    provider_name: &str,
    supported_actions: &[&str],
    action: &str,
    value: Option<String>,
    expected_draft: Option<String>,
    request_id: Option<String>,
) -> Result<Value, String> {
    if !supported_actions.contains(&action) {
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
    if action == "list_conversations" {
        command.insert("fastDirectoryAck".to_string(), Value::Bool(true));
    }
    if let Some(request_id) = request_id {
        if !is_safe_request_id(&request_id) {
            return Err(format!("{provider_name} 命令回执标识无效。"));
        }
        command.insert("requestId".to_string(), Value::String(request_id));
    }
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

fn is_safe_request_id(value: &str) -> bool {
    value.len() >= 5
        && value.len() <= 36
        && value.starts_with("mcp_")
        && value
            .bytes()
            .skip(4)
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
}

pub(super) fn page_invocation_script(
    bridge: &str,
    binding: PageCommandBinding,
    raw_command: &str,
) -> Result<String, String> {
    if bridge.is_empty()
        || !bridge
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
    {
        return Err("本地 AI 页面桥名称无效。".to_string());
    }
    let encoded = serde_json::to_string(raw_command).map_err(|error| error.to_string())?;
    Ok(match binding {
        PageCommandBinding::None => {
            format!("window.{bridge}&&window.{bridge}.command({encoded});")
        }
        PageCommandBinding::ChatGptDocument => format!(
            r#"(function(raw){{
var bridge=window.{bridge};
if(!bridge||typeof bridge.command!=='function')return;
var command=JSON.parse(raw);
command.documentToken=String(window.__elonChatGptDocumentToken||'');
bridge.command(JSON.stringify(command));
}})({encoded});"#
        ),
    })
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
            "ChatGPT",
            CHATGPT_ACTIONS,
            "open_project",
            Some("/g/g-p-roadmap/project".into()),
            None,
            Some("mcp_roadmap1".into())
        )
        .is_ok());
        assert!(build(
            "ChatGPT",
            CHATGPT_ACTIONS,
            "open_conversation",
            Some("/g/g-p-roadmap/c/chat-1".into()),
            None,
            None
        )
        .is_ok());
        assert!(build(
            "ChatGPT",
            CHATGPT_ACTIONS,
            "open_project",
            Some("https://evil.example".into()),
            None,
            None
        )
        .is_err());
    }

    #[test]
    fn provider_action_matrix_is_explicit_and_provider_scoped() {
        assert!(CHATGPT_ACTIONS.contains(&"list_conversations"));
        assert!(CHATGPT_ACTIONS.contains(&"refresh_current_conversation"));
        assert!(!GOOGLE_AI_MODE_ACTIONS.contains(&"refresh_current_conversation"));
        assert!(CHATGPT_ACTIONS.contains(&"start_google_login"));
        assert!(GOOGLE_AI_MODE_ACTIONS.contains(&"list_conversations"));
        assert!(GOOGLE_AI_MODE_ACTIONS.contains(&"open_conversation"));
        assert!(CHATGPT_ACTIONS.contains(&"request_attachment_upload"));
        assert!(CHATGPT_ACTIONS.contains(&"start_dictation"));
    }

    #[test]
    fn command_receipts_require_bounded_lowercase_ids() {
        let command = build(
            "ChatGPT",
            CHATGPT_ACTIONS,
            "snapshot",
            None,
            None,
            Some("mcp_abc123".into()),
        )
        .unwrap();
        assert_eq!(command["requestId"], "mcp_abc123");
        assert!(build(
            "ChatGPT",
            CHATGPT_ACTIONS,
            "snapshot",
            None,
            None,
            Some("unsafe-request".into()),
        )
        .is_err());
    }

    #[test]
    fn desktop_directory_commands_request_fast_non_destructive_acknowledgement() {
        let command = build(
            "ChatGPT",
            CHATGPT_ACTIONS,
            "list_conversations",
            None,
            None,
            Some("mcp_directory1".into()),
        )
        .unwrap();
        assert_eq!(command["fastDirectoryAck"], true);
    }

    #[test]
    fn chatgpt_commands_bind_the_live_document_without_exposing_it_to_frontend() {
        let raw = r#"{"action":"send_prompt","value":"hello"}"#;
        let chatgpt = page_invocation_script(
            "__elonChatGptBridge",
            PageCommandBinding::ChatGptDocument,
            raw,
        )
        .unwrap();
        assert!(chatgpt.contains("window.__elonChatGptDocumentToken"));
        assert!(chatgpt.contains("bridge.command(JSON.stringify(command))"));
        assert!(!chatgpt.contains("hello\"") || chatgpt.contains("\\\"hello\\\""));

        let google = page_invocation_script(
            "__elonGoogleWebBridge",
            PageCommandBinding::None,
            r#"{"action":"snapshot"}"#,
        )
        .unwrap();
        assert!(!google.contains("documentToken"));
        assert!(page_invocation_script("bad-bridge", PageCommandBinding::None, "{}").is_err());
    }
}
