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
    use super::{load_session_plan, stale_resume_failure};
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
