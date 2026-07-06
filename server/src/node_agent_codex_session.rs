// server/src/node_agent_codex_session.rs

use std::path::Path;

use crate::node_agent_task_journal::TaskJournal;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CodexSessionPlan {
    pub scope_key: Option<String>,
    pub session_id: Option<String>,
}

impl CodexSessionPlan {
    pub(crate) fn is_resume(&self) -> bool {
        self.session_id.is_some()
    }
}

pub(crate) fn load_session_plan(
    task_journal: &TaskJournal,
    legacy_sessions_file: &Path,
    scope_key: Option<String>,
) -> CodexSessionPlan {
    // 优先信任新 journal；旧版临时文件只作为升级兼容兜底，避免老缓存覆盖新 session。
    let session_id = scope_key.as_ref().and_then(|key| {
        task_journal
            .load_codex_session(key)
            .ok()
            .flatten()
            .or_else(|| load_legacy_session(legacy_sessions_file, key))
    });
    CodexSessionPlan {
        scope_key,
        session_id,
    }
}

pub(crate) fn stale_resume_failure(stdout: &str, stderr: &str) -> bool {
    let combined = format!("{stdout}\n{stderr}").to_ascii_lowercase();
    let mentions_resume_target =
        combined.contains("session") || combined.contains("thread") || combined.contains("resume");
    let stale = combined.contains("not found")
        || combined.contains("no such")
        || combined.contains("invalid")
        || combined.contains("expired")
        || combined.contains("unknown")
        || combined.contains("could not resume")
        || combined.contains("failed to resume");
    mentions_resume_target && stale
}

#[derive(Debug, Clone, Default)]
pub(crate) struct CodexSessionCapture {
    captured: Option<String>,
    recent_output: String,
}

impl CodexSessionCapture {
    pub(crate) fn observe(&mut self, text: &str) -> Option<String> {
        if self.captured.is_some() {
            return None;
        }
        self.recent_output
            .push_str(&strip_cli_control_sequences(text));
        const MAX_RECENT_OUTPUT: usize = 8192;
        if self.recent_output.len() > MAX_RECENT_OUTPUT {
            let keep_from = self
                .recent_output
                .char_indices()
                .rev()
                .take(MAX_RECENT_OUTPUT)
                .last()
                .map(|(idx, _)| idx)
                .unwrap_or(0);
            self.recent_output = self.recent_output[keep_from..].to_string();
        }
        let session_id = extract_session_id_from_text(&self.recent_output)?;
        self.captured = Some(session_id.clone());
        Some(session_id)
    }
}

pub(crate) fn extract_session_id_from_text(text: &str) -> Option<String> {
    let clean = strip_cli_control_sequences(text);
    for line in clean.lines() {
        if let Some(thread_id) = extract_thread_started_id(line) {
            return Some(thread_id);
        }
        if let Some(session_id) = extract_session_id_from_line(line) {
            return Some(session_id);
        }
    }
    extract_session_id_from_line(&clean)
}

pub(crate) fn strip_session_id_lines(text: &str) -> (Option<String>, String) {
    let mut session_id = None;
    let mut visible = String::new();
    for segment in text.split_inclusive('\n') {
        let clean_segment = strip_cli_control_sequences(segment);
        let found = extract_thread_started_id(clean_segment.trim())
            .or_else(|| extract_session_id_from_line(&clean_segment));
        if let Some(found) = found {
            session_id.get_or_insert(found);
            continue;
        }
        visible.push_str(segment);
    }
    (session_id, visible)
}

pub(crate) fn persist_session_compat(
    task_journal: &TaskJournal,
    legacy_sessions_file: Option<&Path>,
    req_id: &str,
    scope_key: &str,
    session_id: &str,
) {
    if let Err(error) = task_journal.record_codex_session(req_id, scope_key, session_id) {
        tracing::warn!("PC 任务 journal 写入 Codex session 失败: {error}");
    }
    if let Some(path) = legacy_sessions_file {
        persist_legacy_session(path, scope_key, session_id);
    }
    tracing::info!("🔖 Codex session saved: {} → {}", scope_key, session_id);
}

pub(crate) async fn clear_stale_session(
    task_journal: &TaskJournal,
    legacy_sessions_file: &Path,
    req_id: &str,
    scope_key: &str,
) {
    // resume 失败后必须同时清理新旧两处缓存，否则下一轮还会继续命中过期 session。
    if let Err(error) = task_journal.clear_codex_session(req_id, scope_key) {
        tracing::warn!("PC 任务 journal 清理失效 Codex session 失败: {error}");
    }
    clear_legacy_session(legacy_sessions_file, scope_key).await;
}

fn persist_legacy_session(path: &Path, scope_key: &str, session_id: &str) {
    let mut map: serde_json::Map<String, serde_json::Value> = std::fs::read_to_string(path)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default();
    map.insert(scope_key.to_string(), serde_json::json!(session_id));
    if let Ok(text) = serde_json::to_string(&map) {
        let _ = std::fs::write(path, text);
    }
}

fn load_legacy_session(path: &Path, scope_key: &str) -> Option<String> {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|text| {
            serde_json::from_str::<serde_json::Map<String, serde_json::Value>>(&text).ok()
        })
        .and_then(|map| {
            map.get(scope_key)
                .and_then(|value| value.as_str())
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string)
        })
}

fn extract_thread_started_id(line: &str) -> Option<String> {
    let value = serde_json::from_str::<serde_json::Value>(line).ok()?;
    if value.get("type").and_then(serde_json::Value::as_str) != Some("thread.started") {
        return None;
    }
    value
        .get("thread_id")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| plausible_session_id(value))
        .map(ToOwned::to_owned)
}

fn extract_session_id_from_line(line: &str) -> Option<String> {
    let lower = line.to_ascii_lowercase();
    let marker = "session id:";
    let marker_start = lower.find(marker)?;
    let after = line[marker_start + marker.len()..].trim_start();
    let token = read_session_token(after)?;
    if plausible_session_id(&token) {
        Some(token)
    } else {
        None
    }
}

fn read_session_token(value: &str) -> Option<String> {
    let value = value.trim_start_matches(['"', '\'', '`']);
    let mut token = String::new();
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.' | ':' | '/') {
            token.push(ch);
        } else {
            break;
        }
    }
    let token = token
        .trim_matches(|ch: char| matches!(ch, '"' | '\'' | '`' | ',' | ';'))
        .to_string();
    if token.is_empty() {
        None
    } else {
        Some(token)
    }
}

fn plausible_session_id(value: &str) -> bool {
    let value = value.trim();
    if value.len() < 8 || value.len() > 256 {
        return false;
    }
    if let Some(thread_id) = value.strip_prefix("codex://threads/") {
        return plausible_session_id(thread_id);
    }
    value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.' | ':' | '/'))
        && value.chars().any(|ch| ch.is_ascii_alphanumeric())
        && !matches!(
            value.to_ascii_lowercase().as_str(),
            "unknown" | "none" | "null"
        )
}

fn strip_cli_control_sequences(input: &str) -> String {
    let chars: Vec<char> = input.chars().collect();
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
        if ch.is_control() && ch != '\n' && ch != '\r' && ch != '\t' {
            i += 1;
            continue;
        }
        out.push(ch);
        i += 1;
    }
    out
}

async fn clear_legacy_session(path: &Path, scope_key: &str) {
    let Some(mut map) = tokio::fs::read_to_string(path).await.ok().and_then(|text| {
        serde_json::from_str::<serde_json::Map<String, serde_json::Value>>(&text).ok()
    }) else {
        return;
    };
    if map.remove(scope_key).is_none() {
        return;
    }
    if let Err(error) =
        tokio::fs::write(path, serde_json::to_string(&map).unwrap_or_default()).await
    {
        tracing::warn!("清理旧版 Codex session 缓存失败: {error}");
    }
}


#[cfg(test)]
#[path = "node_agent_codex_session_tests.rs"]
mod tests;
