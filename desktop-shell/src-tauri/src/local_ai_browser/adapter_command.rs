use serde_json::{Map, Value};

const CHATGPT_ACTIONS: &[&str] = &[
    "snapshot",
    "send_prompt",
    "stop_generation",
    "regenerate_response",
    "new_conversation",
    "list_conversations",
    "open_conversation",
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
