use super::*;

#[test]
fn chatgpt_document_generation_rejects_late_events_after_navigation() {
    assert_document_generation_isolated(
        "chatgpt",
        "https://chatgpt.com/c/first",
        "https://chatgpt.com/c/second",
    );
}

#[test]
fn google_document_generation_rejects_late_events_after_navigation() {
    assert_document_generation_isolated(
        "google-ai-mode",
        "https://www.google.com/search?udm=50&q=first",
        "https://www.google.com/search?udm=50&q=second",
    );
}

fn assert_document_generation_isolated(provider_id: &str, first_url: &str, second_url: &str) {
    let runtime = LocalAiBrowserRuntime::default();
    runtime.ensure_session("session", provider_id, "active");
    let first_url = Url::parse(first_url).unwrap();
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

    let second_url = Url::parse(second_url).unwrap();
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
