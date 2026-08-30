use super::*;

#[test]
fn lifecycle_is_memory_only_and_resets_with_adapter_document() {
    let runtime = LocalAiBrowserRuntime::default();
    runtime.ensure_session("session", "chatgpt", "active");
    runtime.record_adapter_event(
        "session",
        "attachment_transport",
        json!({"type":"attachment_transport","transportVersion":1,"sequence":2,"state":"completed","completedCount":1}),
    );
    assert_eq!(
        runtime.snapshot("session").unwrap().attachment_transport_event.unwrap()["state"],
        "completed"
    );

    runtime.record_adapter_event("session", "adapter_ready", json!({"type":"adapter_ready"}));
    assert!(runtime.snapshot("session").unwrap().attachment_transport_event.is_none());
}
