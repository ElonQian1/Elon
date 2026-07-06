use super::{ai_cli_output::truncate_chars, ai_cli_pc_prompt::pc_cli_progress_label};

pub(crate) fn pc_passthrough_empty_reply_diagnostic(
    output: &str,
    cli_name: &str,
    display_model: &str,
) -> String {
    pc_lightweight_no_readable_diagnostic(output, cli_name).unwrap_or_else(|| {
        format!(
            "{} 已结束，但没有返回可展示的正文；本轮结果无法确认完成。请直接重发一次，或在节点页重新检测 Codex CLI 后再试。",
            display_model
        )
    })
}

pub(crate) fn pc_lightweight_no_readable_diagnostic(
    output: &str,
    cli_name: &str,
) -> Option<String> {
    let clean = strip_terminal_control_sequences(output);
    let lower = clean.to_ascii_lowercase();
    let cli_label = pc_cli_progress_label(cli_name);

    if lower.contains("usage limit") || lower.contains("hit your usage limit") {
        return Some(format!(
            "{}达到使用额度或限流，未返回可读内容；请稍后重发或检查本机 Codex 登录额度。",
            cli_label
        ));
    }

    if lower.contains("request timed out")
        || lower.contains("stream disconnected")
        || lower.contains("reconnecting")
    {
        let reconnect_count = lower.matches("reconnecting").count();
        let timeout_count = lower.matches("request timed out").count();
        let mut detail = format!(
            "{}网络请求超时，未返回可读内容；已观察到 {} 次重连、{} 次 request timed out，请稍后直接重发一次。",
            cli_label, reconnect_count, timeout_count
        );
        if lower.contains("falling back to http") {
            detail.push_str(" Codex 已尝试 fallback HTTP。");
        }
        return Some(detail);
    }

    if lower.contains("canceled") || clean.contains("用户已停止 PC CLI 任务") {
        return Some(format!(
            "{}任务被取消，未返回可读内容；请稍后直接重发一次。",
            cli_label
        ));
    }

    last_non_noise_cli_line(&clean).map(|line| {
        format!(
            "{}未返回可读内容；最后的本机 CLI 输出是：{}",
            cli_label,
            truncate_chars(line.trim(), 160)
        )
    })
}

fn last_non_noise_cli_line(output: &str) -> Option<String> {
    output
        .lines()
        .rev()
        .map(str::trim)
        .find(|line| {
            !line.is_empty()
                && !is_lightweight_cli_noise_line(line)
                && !is_codex_output_boundary(line)
                && !line.starts_with("{\"type\":\"turn.started\"")
        })
        .map(ToOwned::to_owned)
}

pub(crate) fn extract_codex_reply(output: &str) -> String {
    let clean = strip_terminal_control_sequences(output);
    if let Some(reply) = extract_codex_json_agent_reply(&clean) {
        return reply;
    }
    let mut in_codex_reply = false;
    let mut reply_lines: Vec<String> = Vec::new();
    let mut replies: Vec<String> = Vec::new();

    for line in clean.lines() {
        let trimmed = line.trim();
        if is_lightweight_reply_marker(trimmed) {
            if !reply_lines.is_empty() {
                replies.push(reply_lines.join("\n").trim().to_string());
                reply_lines.clear();
            }
            in_codex_reply = true;
            continue;
        }
        if in_codex_reply {
            if trimmed == "tokens used"
                || (!trimmed.is_empty() && trimmed.chars().all(|c| c.is_ascii_digit() || c == ','))
            {
                if !reply_lines.is_empty() {
                    replies.push(reply_lines.join("\n").trim().to_string());
                    reply_lines.clear();
                }
                in_codex_reply = false;
                continue;
            }
            if is_codex_output_boundary(trimmed) {
                if !reply_lines.is_empty() {
                    replies.push(reply_lines.join("\n").trim().to_string());
                    reply_lines.clear();
                }
                in_codex_reply = false;
                continue;
            }
            if is_lightweight_cli_noise_line(trimmed) {
                continue;
            }
            reply_lines.push(trimmed.to_string());
        }
    }

    if !reply_lines.is_empty() {
        replies.push(reply_lines.join("\n").trim().to_string());
    }

    replies
        .into_iter()
        .rev()
        .find(|reply| is_useful_codex_reply(reply))
        .unwrap_or_default()
}

fn extract_codex_json_agent_reply(output: &str) -> Option<String> {
    let mut replies = Vec::new();
    for line in output.lines() {
        let Ok(value) = serde_json::from_str::<serde_json::Value>(line.trim()) else {
            continue;
        };
        if value.get("type").and_then(serde_json::Value::as_str) != Some("item.completed") {
            continue;
        }
        let Some(item) = value.get("item") else {
            continue;
        };
        if item.get("type").and_then(serde_json::Value::as_str) != Some("agent_message") {
            continue;
        }
        let Some(text) = item.get("text").and_then(serde_json::Value::as_str) else {
            continue;
        };
        let display = text
            .trim()
            .strip_prefix("用户可见：")
            .unwrap_or(text.trim())
            .trim();
        if is_useful_codex_reply(display) {
            replies.push(display.to_string());
        }
    }
    replies.into_iter().rev().next()
}

fn is_useful_codex_reply(reply: &str) -> bool {
    let trimmed = reply.trim();
    if trimmed.is_empty() {
        return false;
    }
    let lower = trimmed.to_ascii_lowercase();
    !lower.contains("codex cli 执行完成，但没有返回可解析输出")
        && !lower.contains("codex cli 执行完成，但输出里没有可解析的 codex 回复段")
}

pub(crate) fn extract_marker_lightweight_reply(output: &str) -> String {
    let mut collecting = false;
    let mut reply_lines = Vec::<String>::new();

    for raw in output.lines() {
        let line = raw.trim();
        if is_lightweight_reply_marker(line) {
            collecting = true;
            reply_lines.clear();
            continue;
        }
        if !collecting {
            continue;
        }
        if is_codex_output_boundary(line) {
            if reply_lines.is_empty() {
                continue;
            }
            break;
        }
        if !is_lightweight_cli_noise_line(line) {
            reply_lines.push(line.to_string());
        }
    }

    reply_lines.join("\n").trim().to_string()
}

pub(crate) fn sanitize_lightweight_pc_reply(reply: &str) -> String {
    let clean = strip_terminal_control_sequences(reply);
    clean
        .lines()
        .map(str::trim)
        .filter(|line| {
            !line.is_empty()
                && !is_lightweight_cli_noise_line(line)
                && !is_codex_output_boundary(line)
        })
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}

#[cfg(test)]
pub(crate) fn clean_codex_stream_chunk(text: &str) -> String {
    let clean = strip_terminal_control_sequences(text);
    let mut lines = Vec::<String>::new();

    for raw in clean.lines() {
        let trimmed = raw.trim();
        if trimmed.is_empty()
            || is_lightweight_cli_noise_line(trimmed)
            || is_codex_output_boundary(trimmed)
            || trimmed == "--------"
        {
            continue;
        }
        lines.push(raw.trim_end().to_string());
    }

    let mut out = lines.join("\n");
    if !out.is_empty() && clean.ends_with('\n') {
        out.push('\n');
    }
    out
}

pub(crate) fn strip_terminal_control_sequences(input: &str) -> String {
    let chars = input.chars().collect::<Vec<_>>();
    let mut out = String::with_capacity(input.len());
    let mut i = 0;

    while i < chars.len() {
        let ch = chars[i];
        if ch == '\u{1b}' {
            i += 1;
            if i < chars.len() && chars[i] == '[' {
                i += 1;
                while i < chars.len() {
                    let next = chars[i];
                    i += 1;
                    if ('@'..='~').contains(&next) {
                        break;
                    }
                }
                continue;
            }
            if i < chars.len() && chars[i] == ']' {
                i += 1;
                while i < chars.len() {
                    if chars[i] == '\u{7}' {
                        i += 1;
                        break;
                    }
                    if chars[i] == '\u{1b}' && i + 1 < chars.len() && chars[i + 1] == '\\' {
                        i += 2;
                        break;
                    }
                    i += 1;
                }
                continue;
            }
            continue;
        }

        if let Some(end) = orphan_csi_end(&chars, i) {
            i = end;
            continue;
        }

        if ch.is_control() && ch != '\n' && ch != '\r' && ch != '\t' {
            i += 1;
            continue;
        }

        out.push(ch);
        i += 1;
    }

    out
}

fn orphan_csi_end(chars: &[char], start: usize) -> Option<usize> {
    if chars.get(start) != Some(&'[') {
        return None;
    }

    let mut i = start + 1;
    if chars.get(i) == Some(&'?') {
        i += 1;
    }

    let mut saw_param = false;
    while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == ';') {
        saw_param = true;
        i += 1;
    }

    if i < chars.len() && chars[i].is_ascii_alphabetic() && (saw_param || chars[i] == 'm') {
        Some(i + 1)
    } else {
        None
    }
}

fn is_lightweight_reply_marker(line: &str) -> bool {
    matches!(
        line.trim().to_ascii_lowercase().as_str(),
        "codex" | "assistant"
    )
}

fn is_codex_output_boundary(line: &str) -> bool {
    let lower = line.trim().to_ascii_lowercase();
    matches!(
        lower.as_str(),
        "user" | "exec" | "tokens used" | "tool" | "system" | "output:"
    ) || lower.starts_with("openai codex")
        || lower.starts_with("workdir:")
        || lower.starts_with("model:")
        || lower.starts_with("provider:")
        || lower.starts_with("approval:")
        || lower.starts_with("sandbox:")
        || lower.starts_with("reasoning")
        || lower.starts_with("session id:")
        || lower.starts_with("wall time:")
        || lower.starts_with("process exited")
        || lower.starts_with("original token count:")
        || lower.starts_with("succeeded in")
        || lower.starts_with("failed in")
        || lower.starts_with("error:")
        || lower == "assistant"
}

fn is_lightweight_cli_noise_line(line: &str) -> bool {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return true;
    }
    let lower = trimmed.to_ascii_lowercase();

    is_lightweight_reply_marker(trimmed)
        || lower.starts_with("]0;")
        || lower.starts_with("[warn")
        || lower.starts_with("warn ")
        || lower.starts_with("warning:")
        || lower.starts_with("2026-")
        || lower.starts_with('{') && lower.contains("\"type\"")
        || lower.contains("cmd.exe")
        || lower.contains("windows\\system32")
        || lower.contains("unc ")
        || trimmed.contains("路径不受支持")
        || trimmed.contains("默认值设为")
        || lower.contains("sqlx::query")
        || lower.contains("slow statement")
        || lower.contains("delete from logs")
        || lower.contains("rows_affected")
        || lower.contains("rows_returned")
        || lower.contains("db.statement")
        || lower.contains("elapsed")
        || lower.contains("event.timestamp=")
        || lower.contains("mcp_server=")
        || lower.contains("model_client.")
        || lower.contains("memories startup: error returned from database")
        || lower.contains("no such table: stage1_outputs")
        || lower.contains("responses_websocket")
        || lower.contains("feedback_tags")
        || lower.contains("auth_header")
        || lower.starts_with(r"\\?\")
}
