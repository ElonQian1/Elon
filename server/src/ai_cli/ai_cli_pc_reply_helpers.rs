use crate::cli_usage::CliTokenUsage;

use super::{
    ai_cli_output::{extract_json_agent_message, truncate_chars},
    ai_cli_types::{AiCliRequestMode, NativeSessionScope},
    pc_passthrough_reply::{
        extract_codex_reply, extract_marker_lightweight_reply, pc_lightweight_no_readable_diagnostic,
        sanitize_lightweight_pc_reply, strip_terminal_control_sequences,
    },
    PcAgentRunOutcome,
};

pub(super) fn abort_pc_progress(handle: &mut Option<tokio::task::JoinHandle<()>>) {
    if let Some(handle) = handle.take() {
        handle.abort();
    }
}

pub(super) fn extract_lightweight_pc_chat_reply(output: &str, is_codex: bool) -> String {
    let clean = strip_terminal_control_sequences(output);
    if let Some(reply) = extract_json_agent_message(&clean) {
        let reply = sanitize_lightweight_pc_reply(&reply);
        if !reply.is_empty() {
            return reply;
        }
    }

    if is_codex {
        let reply = sanitize_lightweight_pc_reply(&extract_codex_reply(&clean));
        if !reply.is_empty() {
            return reply;
        }
    }

    extract_marker_lightweight_reply(&clean)
}

pub(super) fn no_readable_lightweight_reply(output: &str, cli_name: &str) -> PcAgentRunOutcome {
    PcAgentRunOutcome::NoReadableLightweightReply {
        diagnostic: pc_lightweight_no_readable_diagnostic(output, cli_name),
    }
}

pub(super) fn lightweight_pc_reply_delta(
    output: &str,
    is_codex: bool,
    streamed_reply: &mut String,
) -> Option<String> {
    let reply = extract_lightweight_pc_chat_reply(output, is_codex);
    lightweight_reply_text_delta(&reply, streamed_reply)
}

pub(super) fn lightweight_reply_text_delta(reply: &str, streamed_reply: &mut String) -> Option<String> {
    let reply = reply.trim();
    if reply.is_empty() || reply == streamed_reply.trim() {
        return None;
    }

    if streamed_reply.trim().is_empty() {
        streamed_reply.clear();
        streamed_reply.push_str(reply);
        return Some(reply.to_string());
    }

    if let Some(delta) = reply.strip_prefix(streamed_reply.as_str()) {
        if delta.is_empty() {
            return None;
        }
        streamed_reply.push_str(delta);
        return Some(delta.to_string());
    }

    None
}

pub(super) fn extract_lightweight_pc_chat_timeout_reply(output: &str, is_codex: bool) -> Option<String> {
    let reply = extract_lightweight_pc_chat_reply(output, is_codex);
    (!reply.trim().is_empty()).then_some(reply)
}

pub(super) fn sanitize_pc_development_reply(reply: &str, apk_url: Option<&str>) -> String {
    let clean = strip_terminal_control_sequences(reply);
    let mut lines = Vec::<String>::new();
    let mut in_code_block = false;

    for raw in clean.lines() {
        let line = raw.trim();
        if line.starts_with("```") {
            in_code_block = !in_code_block;
            continue;
        }
        if in_code_block {
            continue;
        }
        if is_pc_development_reply_boundary(line) {
            break;
        }
        if line.is_empty() || is_pc_development_reply_noise_line(line) {
            continue;
        }
        lines.push(sanitize_user_reply_line(line));
        if lines.len() >= 4 {
            break;
        }
    }

    let mut text = lines
        .into_iter()
        .filter(|line| !line.trim().is_empty())
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string();
    text = truncate_chars(text.as_str(), 220).trim().to_string();

    if text.is_empty() {
        text = if apk_url.is_some() {
            "新的安装包已生成。".to_string()
        } else {
            "开发助手已结束，但没有返回可展示的总结。请查看进度日志确认结果。".to_string()
        };
    }

    if apk_url.is_some()
        && !text.contains("项目空间")
        && !text.contains("安装按钮")
        && !text.contains("点击「安装」")
    {
        if !text.ends_with('。') && !text.ends_with('！') && !text.ends_with('!') {
            text.push('。');
        }
        text.push_str("\n请到项目空间点击「安装」下载体验。");
    }

    text
}

pub(super) fn is_pc_development_reply_boundary(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    line.starts_with("diff --git")
        || line.starts_with("```")
        || line.starts_with("安装命令")
        || lower.starts_with("adb ")
        || lower.contains("adb.exe")
        || lower.starts_with("powershell")
        || lower.starts_with("git diff")
}

pub(super) fn is_pc_development_reply_noise_line(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    lower.contains("c:\\")
        || lower.contains("c:/")
        || lower.contains("\\users\\")
        || lower.contains("/users/")
        || lower.contains("conversation-worktrees")
        || lower.contains("app/build/outputs/apk")
        || lower.contains("app\\build\\outputs\\apk")
        || lower.contains("platform-tools")
        || lower.contains("adb.exe")
        || lower.contains("diff --git")
        || lower.starts_with("index ")
        || lower.starts_with("--- ")
        || lower.starts_with("+++ ")
        || lower.starts_with("@@")
        || lower.starts_with("apply patch")
        || (line.contains(".apk](") && (lower.contains("c:/") || lower.contains("c:\\")))
        || (line.contains("安装包") && line.contains("这里"))
}

pub(super) fn sanitize_user_reply_line(line: &str) -> String {
    line.replace('`', "")
        .replace("APK 已重新构建成功", "新的安装包已生成")
        .trim()
        .to_string()
}

pub(super) fn pc_codex_progress_hint(text: &str, display_model: &str) -> Option<(&'static str, String)> {
    let clean = strip_terminal_control_sequences(text);
    let lower = clean.to_ascii_lowercase();

    if lower.contains("stream disconnected - retrying sampling request")
        || lower.contains("reconnecting...")
    {
        let attempt = extract_codex_reconnect_attempt(&clean)
            .map(|value| format!("（第 {value} 次）"))
            .unwrap_or_default();
        return Some((
            "codex_reconnecting",
            format!("Codex ({display_model}) 流式连接不稳定，正在自动重连{attempt}。"),
        ));
    }

    if lower.contains("falling back to http") {
        return Some((
            "codex_http_fallback",
            format!("Codex ({display_model}) 已切换到 HTTP fallback 继续生成。"),
        ));
    }

    if lower.contains("failed to refresh remote installed plugins cache")
        || lower.contains("curated plugin sync")
        || lower.contains("git sync failed for curated plugin")
    {
        return Some((
            "codex_plugin_cache",
            "Codex 插件远程同步不可达，已继续使用本地缓存。".to_string(),
        ));
    }

    None
}

pub(super) fn extract_codex_reconnect_attempt(text: &str) -> Option<String> {
    let lower = text.to_ascii_lowercase();
    let marker = "reconnecting...";
    if let Some(index) = lower.find(marker) {
        let rest = &text[index + marker.len()..];
        return extract_retry_fraction(rest);
    }

    let marker = "sampling request (";
    let index = lower.find(marker)?;
    let rest = &text[index + marker.len()..];
    extract_retry_fraction(rest.split(')').next().unwrap_or(rest))
}

pub(super) fn extract_retry_fraction(text: &str) -> Option<String> {
    text.split(|ch: char| !(ch.is_ascii_digit() || ch == '/'))
        .find(|part| {
            let mut split = part.split('/');
            matches!((split.next(), split.next()), (Some(a), Some(b)) if !a.is_empty() && !b.is_empty())
        })
        .map(str::to_string)
}

pub(super) fn pc_dispatch_started_event(
    pc_req_id: &str,
    agent_id: &str,
    node_display_name: &str,
    cli_name: &str,
    cwd: Option<&str>,
    native_session_scope: Option<&NativeSessionScope>,
    request_mode: AiCliRequestMode,
) -> String {
    serde_json::json!({
        "type": "pc_dispatch_started",
        "pc_req_id": pc_req_id,
        "req_id": pc_req_id,
        "agent_id": agent_id,
        "node_display_name": node_display_name,
        "cli": cli_name,
        "cwd_configured": cwd.is_some(),
        "project_id": native_session_scope.map(|scope| scope.project_id.as_str()),
        "conversation_id": native_session_scope.map(|scope| scope.conversation_id.as_str()),
        "runtime_permission": native_session_scope.map(|scope| scope.runtime_permission.as_str()),
        "mode": if request_mode.is_plan() {
            "plan"
        } else if request_mode.is_passthrough() {
            "passthrough"
        } else {
            "execute"
        }
    })
    .to_string()
}

pub(super) fn pc_cli_model_id(model: Option<&str>) -> String {
    model
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| format!("pc-cli/{value}"))
        .unwrap_or_else(|| "pc-cli/unknown".to_string())
}

pub(super) fn pc_cli_usage_tokens(usage: &crate::cli_usage::CliTokenUsage) -> (i64, i64) {
    let prompt_tokens = usage.input_tokens.max(0);
    let total_tokens = usage
        .total_tokens
        .max(usage.input_tokens.max(0) + usage.output_tokens.max(0));
    let completion_tokens = (total_tokens - usage.input_tokens.max(0)).max(0);
    (prompt_tokens, completion_tokens)
}

pub(super) fn pc_cli_price_per_1k_credits() -> f64 {
    std::env::var("ELON_PC_CLI_PRICE_PER_1K_CREDITS")
        .ok()
        .and_then(|value| value.trim().parse::<f64>().ok())
        .filter(|value| *value >= 0.0)
        .unwrap_or(0.1)
}
