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
mod tests {
    use super::{
        extract_session_id_from_text, load_session_plan, stale_resume_failure,
        strip_session_id_lines, CodexSessionCapture,
    };
    use crate::node_agent_task_journal::TaskJournal;

    #[test]
    fn stale_resume_failure_requires_session_context_and_stale_reason() {
        assert!(stale_resume_failure(
            "",
            "Error: could not resume session abc: not found"
        ));
        assert!(stale_resume_failure("", "thread expired"));
        assert!(!stale_resume_failure("", "network disconnected"));
        assert!(!stale_resume_failure("", "not found"));
    }

    #[test]
    fn load_session_plan_prefers_task_journal_over_legacy_file() {
        let dir = std::env::temp_dir().join(format!(
            "elon-codex-session-plan-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let journal = TaskJournal::new(&dir);
        journal
            .record_codex_session("req-1", "scope-a", "journal-session")
            .expect("journal session should persist");
        let legacy = dir.join("legacy.json");
        std::fs::write(&legacy, r#"{"scope-a":"legacy-session"}"#).unwrap();

        let plan = load_session_plan(&journal, &legacy, Some("scope-a".to_string()));

        assert_eq!(plan.scope_key.as_deref(), Some("scope-a"));
        assert_eq!(plan.session_id.as_deref(), Some("journal-session"));
        assert!(plan.is_resume());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn extracts_codex_session_from_text_variants() {
        assert_eq!(
            extract_session_id_from_text("session id: 019f172c-2d52-7e33-8ce5-5af73dada2bf\n")
                .as_deref(),
            Some("019f172c-2d52-7e33-8ce5-5af73dada2bf")
        );
        assert_eq!(
            extract_session_id_from_text("\u{1b}[32mSession ID: codex-session_123\u{1b}[0m")
                .as_deref(),
            Some("codex-session_123")
        );
        assert_eq!(
            extract_session_id_from_text(
                r#"{"type":"thread.started","thread_id":"thread-json-123"}"#
            )
            .as_deref(),
            Some("thread-json-123")
        );
    }

    #[test]
    fn strips_session_id_lines_without_dropping_adjacent_output() {
        let (session_id, visible) = strip_session_id_lines(
            "before\n\u{1b}[36mSession ID: 019f172c-2d52-7e33-8ce5-5af73dada2bf\u{1b}[0m\nafter\n",
        );

        assert_eq!(
            session_id.as_deref(),
            Some("019f172c-2d52-7e33-8ce5-5af73dada2bf")
        );
        assert_eq!(visible, "before\nafter\n");
    }

    #[test]
    fn capture_finds_session_split_across_chunks() {
        let mut capture = CodexSessionCapture::default();
        assert_eq!(capture.observe("session "), None);
        assert_eq!(
            capture
                .observe("id: 019f172c-2d52-7e33-8ce5-5af73dada2bf\n")
                .as_deref(),
            Some("019f172c-2d52-7e33-8ce5-5af73dada2bf")
        );
        assert_eq!(
            capture.observe("session id: different-session"),
            None,
            "capture should only persist the first session id"
        );
    }

    #[tokio::test]
    async fn clear_stale_session_removes_journal_and_legacy_cache() {
        let dir = std::env::temp_dir().join(format!(
            "elon-codex-session-clear-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let journal = TaskJournal::new(&dir);
        journal
            .record_codex_session("req-1", "scope-a", "journal-session")
            .expect("journal session should persist");
        let legacy = dir.join("legacy.json");
        std::fs::write(
            &legacy,
            r#"{"scope-a":"legacy-session","scope-b":"keep-session"}"#,
        )
        .unwrap();

        super::clear_stale_session(&journal, &legacy, "req-1", "scope-a").await;

        assert_eq!(journal.load_codex_session("scope-a").unwrap(), None);
        let legacy_text = std::fs::read_to_string(&legacy).unwrap();
        assert!(!legacy_text.contains("scope-a"));
        assert!(legacy_text.contains("scope-b"));
        let _ = std::fs::remove_dir_all(dir);
    }
}
