use super::*;

#[test]
fn realtime_voice_state_is_memory_only_and_clears_on_new_document() {
    let runtime = LocalAiBrowserRuntime::default();
    runtime.ensure_session("voice", "chatgpt", "active");
    runtime.record_adapter_event(
        "voice",
        "realtime_voice_state",
        json!({"type":"realtime_voice_state","version":1,"active":true,"openChannelCount":1}),
    );
    assert_eq!(
        runtime
            .snapshot("voice")
            .unwrap()
            .realtime_voice_event
            .unwrap()["active"],
        true
    );
    runtime.record_adapter_event(
        "voice",
        "adapter_ready",
        json!({"type":"adapter_ready","capabilities":[]}),
    );
    assert!(runtime
        .snapshot("voice")
        .unwrap()
        .realtime_voice_event
        .is_none());
}
