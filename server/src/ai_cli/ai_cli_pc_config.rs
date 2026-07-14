use sha2::Digest;

use super::{
    ai_cli_chat::is_tiny_chat_message,
    ai_cli_types::{AiCliRequestMode, NativeSessionScope},
};

pub(super) const PC_CODEX_PROJECT_DEFAULT_REASONING_EFFORT: &str = "medium";
pub(super) const PC_AGENT_CLI_RECV_TIMEOUT_ENV: &str = "ELON_PC_AGENT_CLI_RECV_TIMEOUT_SECS";
pub(super) const PC_AGENT_CLI_RECV_TIMEOUT_GRACE_SECS: u64 = 45;

fn normalize_codex_reasoning_effort(value: &str, fallback: &str) -> String {
    match value.trim().to_ascii_lowercase().as_str() {
        "none" => "none".to_string(),
        "minimal" => "minimal".to_string(),
        "low" => "low".to_string(),
        "medium" => "medium".to_string(),
        "high" => "high".to_string(),
        "xhigh" | "max" | "ultra" | "extra_high" => "xhigh".to_string(),
        _ => fallback.to_string(),
    }
}

pub(super) fn pc_lightweight_chat_reasoning_effort(
    cli_name: &str,
    requested_effort: Option<&str>,
) -> Option<String> {
    if cli_name != "codex" {
        return None;
    }

    let clean = requested_effort
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| normalize_codex_reasoning_effort(value, "low"))
        .unwrap_or_else(|| "low".to_string());

    match clean.as_str() {
        "high" | "xhigh" => Some("low".to_string()),
        _ => Some(clean),
    }
}

pub(super) fn pc_project_reasoning_effort(
    cli_name: &str,
    requested_effort: Option<&str>,
    request_mode: AiCliRequestMode,
) -> Option<String> {
    if cli_name != "codex" {
        return None;
    }

    let clean = requested_effort
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| {
            normalize_codex_reasoning_effort(value, PC_CODEX_PROJECT_DEFAULT_REASONING_EFFORT)
        });

    clean.or_else(|| {
        if request_mode.is_passthrough() {
            None
        } else {
            Some(
                if request_mode.is_plan() {
                    "low"
                } else {
                    PC_CODEX_PROJECT_DEFAULT_REASONING_EFFORT
                }
                .to_string(),
            )
        }
    })
}

pub(super) fn pc_runtime_full_access(runtime_permission: Option<&str>) -> bool {
    matches!(
        runtime_permission.map(str::trim),
        Some("project_write" | "full_access" | "danger_full_access")
    )
}

pub(super) fn pc_agent_cli_node_timeout_secs(
    cli_name: &str,
    runtime_permission: Option<&str>,
) -> u64 {
    match cli_name.trim().to_ascii_lowercase().as_str() {
        "codex" if pc_runtime_full_access(runtime_permission) => 1200,
        "codex" => 300,
        _ => 180,
    }
}

pub(super) fn pc_agent_cli_recv_timeout_secs(
    cli_name: &str,
    request_mode: AiCliRequestMode,
    scope: Option<&NativeSessionScope>,
) -> u64 {
    if let Ok(value) = std::env::var(PC_AGENT_CLI_RECV_TIMEOUT_ENV) {
        if let Ok(parsed) = value.trim().parse::<u64>() {
            return parsed.clamp(60, 3600);
        }
    }
    let runtime_permission = if request_mode.is_plan() {
        Some("read_only")
    } else {
        scope.map(|scope| scope.runtime_permission.as_str())
    };
    pc_agent_cli_node_timeout_secs(cli_name, runtime_permission)
        .saturating_add(PC_AGENT_CLI_RECV_TIMEOUT_GRACE_SECS)
}

pub(super) fn should_skip_pc_chat_native_session(user_message: &str) -> bool {
    if is_tiny_chat_message(user_message) {
        return true;
    }

    let compact: String = user_message
        .trim()
        .to_lowercase()
        .chars()
        .filter(|ch| {
            !ch.is_whitespace()
                && !matches!(
                    ch,
                    '!' | '！'
                        | '?'
                        | '？'
                        | '.'
                        | '。'
                        | ','
                        | '，'
                        | ';'
                        | '；'
                        | ':'
                        | '：'
                        | '~'
                        | '～'
                )
        })
        .take(32)
        .collect();

    matches!(
        compact.as_str(),
        "我有一个想法"
            | "我有个想法"
            | "有一个想法"
            | "有个想法"
            | "我刚有个想法"
            | "我刚刚有个想法"
            | "我有一个需求"
            | "我有个需求"
            | "有一个需求"
            | "有个需求"
    )
}

pub(super) fn pc_display_model_label(
    cli_name: &str,
    requested_label: Option<&str>,
    codex_reasoning_effort: Option<&str>,
    lightweight_pc_chat: bool,
    fallback: &str,
) -> String {
    let base = requested_label
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(fallback);

    if lightweight_pc_chat && cli_name == "codex" {
        if let Some(effort) = codex_reasoning_effort
            .map(str::trim)
            .filter(|v| !v.is_empty())
        {
            let model = base
                .split_once(" · 推理 ")
                .map(|(model, _)| model)
                .unwrap_or(base);
            return format!("{model} · 轻量 {effort}");
        }
    }

    base.to_string()
}

pub(super) fn native_session_uuid(cli_name: &str, scope: &NativeSessionScope) -> String {
    use sha2::Digest;

    let cli_prefix = match cli_name {
        "copilot" => "copilot-session",
        "codex" => "codex-session",
        other => other,
    };
    let seed = format!(
        "{}/{}/{}/{}",
        cli_prefix, scope.project_id, scope.user_id, scope.conversation_id
    );
    let hash = sha2::Sha256::digest(seed.as_bytes());
    format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-4{:x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        hash[0],
        hash[1],
        hash[2],
        hash[3],
        hash[4],
        hash[5],
        hash[6] & 0x0f, // 单个十六进制位，用 {:x} 避免零填充为两位
        hash[7],
        (hash[8] & 0x3f) | 0x80,
        hash[9],
        hash[10],
        hash[11],
        hash[12],
        hash[13],
        hash[14],
        hash[15]
    )
}

pub(super) fn pc_route_a_extra_args(
    cli_name: &str,
    native_session_id: Option<&str>,
    model: Option<&str>,
    codex_reasoning_effort: Option<&str>,
) -> Vec<String> {
    match cli_name {
        "copilot" => {
            let mut args = native_session_id
                .map(|sid| vec![format!("--session-id={}", sid)])
                .unwrap_or_default();
            if let Some(model) = model {
                if !model.is_empty() && model != "auto" {
                    args.push("--model".into());
                    args.push(model.to_string());
                }
            }
            args
        }
        "codex" => {
            let mut args = native_session_id
                .map(|sid| vec![format!("--session-id={}", sid)])
                .unwrap_or_default();
            if let Some(model) = model {
                if !model.is_empty() && model != "auto" {
                    args.push(format!("--codex-model={}", model));
                }
            }
            if let Some(effort) = codex_reasoning_effort {
                if !effort.is_empty() {
                    args.push(format!("--codex-effort={}", effort));
                }
            }
            args
        }
        _ => vec![],
    }
}

pub(super) fn pc_route_a_ui_args(
    cli_name: &str,
    native_session_id: Option<&str>,
    model: Option<&str>,
    codex_reasoning_effort: Option<&str>,
    prompt: &str,
    public_url: &str,
) -> Vec<String> {
    let mut args =
        pc_route_a_extra_args(cli_name, native_session_id, model, codex_reasoning_effort);
    if matches!(cli_name, "codex" | "copilot" | "claude" | "gemini") {
        for url in crate::ui_design_tasks::ui_design_image_attachment_urls(prompt, public_url) {
            args.push("--attachment".to_string());
            args.push(url);
        }
    }
    args
}
