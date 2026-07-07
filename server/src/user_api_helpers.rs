use super::*;

pub(super) fn is_cli_selection(state: &AppState, name: &str) -> bool {
    is_cli_alias(name) || state.ai_cli.has_option(name)
}

pub(super) fn is_cli_alias(name: &str) -> bool {
    matches!(
        name.trim().to_ascii_lowercase().as_str(),
        "codex"
            | "codex_cli"
            | "copilot"
            | "copilot_cli"
            | "claude"
            | "claude_cli"
            | "gemini"
            | "gemini_cli"
            | "cli"
            | "local"
            | "local_cli"
    )
}

/// 将代理名转换为 (provider, 用户可见模型名)。
/// - `copilot:gpt-4o`  → ("copilot", "GPT-4o")
/// - `openai`          → ("openai", "GPT-4o")
pub(super) fn agent_display_meta(name: &str, model: &str) -> (String, String) {
    if let Some(model_id) = name.strip_prefix("copilot:") {
        (
            "copilot".to_string(),
            copilot_model_friendly_name(model_id).to_string(),
        )
    } else {
        (name.to_string(), direct_model_label(model, name))
    }
}

pub(super) fn cli_option_display_label(option: &AiCliOption) -> String {
    option.display_label()
}

pub(super) fn dedupe_available_agents(agents: Vec<serde_json::Value>) -> Vec<serde_json::Value> {
    let mut deduped: Vec<serde_json::Value> = Vec::new();
    for agent in agents {
        let key = available_agent_key(&agent);
        if key.is_empty() {
            deduped.push(agent);
            continue;
        }

        if let Some(existing) = deduped
            .iter_mut()
            .find(|existing| available_agent_key(existing) == key)
        {
            if available_agent_priority(&agent) > available_agent_priority(existing) {
                *existing = agent;
            }
        } else {
            deduped.push(agent);
        }
    }
    deduped
}

pub(super) fn available_agent_key(agent: &serde_json::Value) -> String {
    agent["name"]
        .as_str()
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase()
}

pub(super) fn available_agent_priority(agent: &serde_json::Value) -> u8 {
    match agent["backend"]
        .as_str()
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "cli" => 2,
        "api" => 1,
        _ => 0,
    }
}

pub(super) fn direct_model_label(model: &str, fallback: &str) -> String {
    let model = model.trim();
    if model.is_empty() || model.eq_ignore_ascii_case("default") {
        strip_provider_prefix(fallback)
    } else {
        copilot_model_friendly_name(model).to_string()
    }
}

pub(super) fn strip_provider_prefix(label: &str) -> String {
    let label = label.trim();
    if let Some((_, model)) = label.rsplit_once('/') {
        let model = model.trim();
        if !model.is_empty() {
            return model.to_string();
        }
    }
    if let Some(start) = label.rfind('[') {
        if label.ends_with(']') && start + 1 < label.len() - 1 {
            let model = label[start + 1..label.len() - 1].trim();
            if !model.is_empty() {
                return model.to_string();
            }
        }
    }
    label.to_string()
}

/// 将 Copilot / GitHub Models 的模型 ID 映射为用户可读名称。
pub(super) fn copilot_model_friendly_name(model: &str) -> &str {
    match model {
        // GPT 系列
        "gpt-4o" => "GPT-4o",
        "gpt-4o-mini" => "GPT-4o mini",
        "gpt-4.1" => "GPT-4.1",
        "gpt-4.1-mini" => "GPT-4.1 mini",
        "gpt-4.1-nano" => "GPT-4.1 nano",
        "gpt-4.5-preview" => "GPT-4.5 Preview",
        "gpt-5" | "gpt-5.0" => "GPT-5",
        "gpt-5-mini" => "GPT-5 mini",
        "gpt-5.3-codex" => "GPT-5.3 Codex",
        "gpt-5.4" => "GPT-5.4",
        "gpt-5.4-mini" => "GPT-5.4 mini",
        "gpt-5.5" => "GPT-5.5",
        // Claude 系列（Copilot CLI 实际使用的 model ID 格式：claude-{role}-{major}.{minor}）
        "claude-haiku-4.5" => "Claude Haiku 4.5",
        "claude-sonnet-4" | "claude-sonnet-4.0" => "Claude Sonnet 4",
        "claude-sonnet-4.5" | "claude-3-sonnet-4-5" => "Claude Sonnet 4.5",
        "claude-sonnet-4.6" => "Claude Sonnet 4.6",
        "claude-opus-4" | "claude-opus-4.0" => "Claude Opus 4",
        "claude-opus-4.7" => "Claude Opus 4.7",
        "claude-opus-4.8" => "Claude Opus 4.8",
        // 旧版 Claude（向后兼容）
        "claude-3.5-sonnet" | "claude-3-5-sonnet-20241022" => "Claude 3.5 Sonnet",
        "claude-3.7-sonnet" | "claude-3-7-sonnet-20250219" => "Claude 3.7 Sonnet",
        // 推理模型
        "o1" => "o1",
        "o1-mini" => "o1 mini",
        "o1-preview" => "o1 preview",
        "o3" => "o3",
        "o3-mini" => "o3 mini",
        "o4-mini" => "o4 mini",
        // Gemini 系列
        "gemini-2.0-flash" | "gemini-2.0-flash-001" => "Gemini 2.0 Flash",
        "gemini-2.5-pro" | "gemini-2.5-pro-preview" => "Gemini 2.5 Pro",
        "gemini-2.5-flash" | "gemini-2.5-flash-preview" => "Gemini 2.5 Flash",
        "gemini-3.1-pro-preview" => "Gemini 3.1 Pro",
        "gemini-3.5-flash" => "Gemini 3.5 Flash",
        // 混元
        "hunyuan-turbo" => "混元 Turbo",
        "hunyuan-2.0-instruct-20251111" => "混元 2.0 Instruct",
        "hy-image-v3.0" => "混元生图 3.0",
        other => other,
    }
}
