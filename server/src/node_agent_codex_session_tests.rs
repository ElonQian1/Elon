use super::{
    extract_session_id_from_text, load_session_plan, stale_resume_failure, strip_session_id_lines,
    CodexSessionCapture,
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
        extract_session_id_from_text("\u{1b}[32mSession ID: codex-session_123\u{1b}[0m").as_deref(),
        Some("codex-session_123")
    );
    assert_eq!(
        extract_session_id_from_text(r#"{"type":"thread.started","thread_id":"thread-json-123"}"#)
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
