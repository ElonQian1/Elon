use anyhow::{Result, anyhow};
use serde_json::Value;

use crate::{ai_cli::IntentGateResult, intent_router};

pub(crate) fn parse_intent_gate_result(stdout: &str) -> Result<IntentGateResult> {
    let text = extract_json_agent_message(stdout).unwrap_or_else(|| stdout.trim().to_string());
    let value = parse_json_object_from_text(&text)
        .ok_or_else(|| anyhow!("Codex CLI 意图确认没有返回有效 JSON"))?;
    let route_text = value
        .get("route")
        .and_then(Value::as_str)
        .unwrap_or("chat")
        .trim()
        .to_ascii_lowercase();
    let route = match route_text.as_str() {
        "development" | "code" | "codeagent" | "dev" => intent_router::CapabilityRoute::CodeAgent,
        _ => intent_router::CapabilityRoute::ChatAgent,
    };
    let confidence = value
        .get("confidence")
        .and_then(Value::as_f64)
        .unwrap_or(0.0)
        .clamp(0.0, 1.0);
    let reason = value
        .get("reason")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .to_string();
    let chat_reply = value
        .get("chat_reply")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|reply| !reply.is_empty())
        .map(ToOwned::to_owned);

    Ok(IntentGateResult {
        route,
        confidence,
        reason,
        chat_reply,
    })
}

fn parse_json_object_from_text(text: &str) -> Option<Value> {
    serde_json::from_str::<Value>(text).ok().or_else(|| {
        let start = text.find('{')?;
        let end = text.rfind('}')?;
        serde_json::from_str::<Value>(&text[start..=end]).ok()
    })
}

pub(crate) fn format_cli_reply(stdout: &str, stderr: &str, success: bool) -> String {
    let extracted;
    let primary = if stdout.trim().is_empty() {
        extracted = extract_codex_answer(stderr);
        extracted.as_deref().unwrap_or(stderr)
    } else if let Some(answer) = extract_json_agent_message(stdout) {
        extracted = Some(answer);
        extracted.as_deref().unwrap_or(stdout)
    } else {
        stdout
    };
    let clean = truncate_chars(strip_ansi(primary).trim(), 8000);

    if clean.is_empty() {
        if success {
            "本地 AI CLI 已完成处理。".into()
        } else {
            "AI 助手尝试处理这个流程，但没有返回可读的失败原因。请稍后重试；如果问题持续出现，需要人工确认当前 Git 工作区状态后再继续。".into()
        }
    } else if success {
        clean
    } else {
        format!(
            "{}\n\n这次 AI 助手已经尝试自行处理，但流程没有正常完成。请根据上面的原因确认下一步，或稍后重试。",
            clean
        )
    }
}

pub(crate) fn extract_thread_id(stdout: &str) -> Option<String> {
    stdout.lines().find_map(|line| {
        let value: Value = serde_json::from_str(line).ok()?;
        if value.get("type").and_then(Value::as_str) == Some("thread.started") {
            value
                .get("thread_id")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned)
        } else {
            None
        }
    })
}

pub(crate) fn extract_json_agent_message(stdout: &str) -> Option<String> {
    let mut latest = None;
    for line in stdout.lines() {
        let Ok(value) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        if value.get("type").and_then(Value::as_str) != Some("item.completed") {
            continue;
        }
        let Some(item) = value.get("item") else {
            continue;
        };
        if item.get("type").and_then(Value::as_str) != Some("agent_message") {
            continue;
        }
        if let Some(text) = item.get("text").and_then(Value::as_str) {
            latest = Some(text.to_string());
        }
    }
    latest
}

fn extract_codex_answer(stderr: &str) -> Option<String> {
    let clean = strip_ansi(stderr);
    let mut answers = Vec::<String>::new();
    let mut collecting = false;
    let mut current = Vec::<String>::new();

    for raw in clean.lines() {
        let line = raw.trim();
        if line == "codex" {
            if !current.is_empty() {
                answers.push(current.join("\n").trim().to_string());
                current.clear();
            }
            collecting = true;
            continue;
        }

        if collecting && is_codex_block_boundary(line) {
            if !current.is_empty() {
                answers.push(current.join("\n").trim().to_string());
                current.clear();
            }
            collecting = false;
            continue;
        }

        if collecting && !is_noisy_codex_answer_line(line) {
            current.push(line.to_string());
        }
    }

    if !current.is_empty() {
        answers.push(current.join("\n").trim().to_string());
    }

    answers
        .into_iter()
        .rev()
        .find(|answer| !answer.trim().is_empty())
}

fn is_codex_block_boundary(line: &str) -> bool {
    if line.is_empty() {
        return false;
    }
    let lower = line.to_ascii_lowercase();
    matches!(
        lower.as_str(),
        "user" | "exec" | "tokens used" | "tool" | "system" | "assistant" | "output:"
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
        || lower.starts_with("/bin/")
        || lower.starts_with("succeeded in")
        || lower.starts_with("failed in")
        || lower.starts_with("error:")
        || lower.starts_with("warn")
        || lower.contains(" event.timestamp=")
        || lower.contains("mcp_server=")
}

fn is_noisy_codex_answer_line(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    line.is_empty()
        || lower.contains("feedback_tags")
        || lower.contains("model_client.")
        || lower.contains("responses_websocket")
        || lower.contains("event.timestamp=")
        || lower.contains("mcp_server=")
        || lower.contains("auth_header")
}

fn strip_ansi(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch != '\u{1b}' {
            out.push(ch);
            continue;
        }

        if chars.peek() == Some(&'[') {
            chars.next();
            for next in chars.by_ref() {
                if next.is_ascii_alphabetic() {
                    break;
                }
            }
        }
    }

    out
}

pub(crate) fn truncate_chars(value: &str, max_chars: usize) -> String {
    let mut iter = value.chars();
    let truncated: String = iter.by_ref().take(max_chars).collect();
    if iter.next().is_some() {
        format!("{}...\n\n（输出过长，已截断）", truncated)
    } else {
        truncated
    }
}
