use super::*;

#[test]
fn chatgpt_document_generation_rejects_late_events_after_navigation() {
    let runtime = LocalAiBrowserRuntime::default();
    runtime.ensure_session("session", "chatgpt", "active");
    let first_url = Url::parse("https://chatgpt.com/c/first").unwrap();
    runtime.mark_navigation("session", &first_url, true, None);

    assert!(!runtime.accept_adapter_document_event(
        "session",
        "message_snapshot",
        Some("doc_first"),
    ));
    assert!(runtime.accept_adapter_document_event(
        "session",
        "adapter_ready",
        Some("doc_first"),
    ));
    assert!(runtime.accept_adapter_document_event(
        "session",
        "message_snapshot",
        Some("doc_first"),
    ));

    let second_url = Url::parse("https://chatgpt.com/c/second").unwrap();
    runtime.mark_navigation("session", &second_url, true, None);
    assert!(!runtime.accept_adapter_document_event(
        "session",
        "message_snapshot",
        Some("doc_first"),
    ));
    assert!(!runtime.accept_adapter_document_event(
        "session",
        "message_snapshot",
        Some("doc_second"),
    ));
    assert!(runtime.accept_adapter_document_event(
        "session",
        "adapter_ready",
        Some("doc_second"),
    ));
    assert!(runtime.accept_adapter_document_event(
        "session",
        "message_snapshot",
        Some("doc_second"),
    ));
    assert!(!runtime.accept_adapter_document_event(
        "session",
        "message_snapshot",
        Some("doc_first"),
    ));
    assert_eq!(
        runtime.snapshot("session").unwrap().diagnostics["lastEventKind"],
        "stale_document_event_ignored",
    );
}
