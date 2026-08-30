use super::*;

#[test]
fn realtime_voice_state_keeps_only_bounded_structural_evidence() {
    let raw = json!({
        "schema": "yilong.ai.ui.v1",
        "providerId": "chatgpt",
        "adapterVersion": ADAPTER_VERSION,
        "documentToken": "doc_win_voice_state",
        "event": {
            "type": "realtime_voice_state",
            "version": 1,
            "active": true,
            "observedChannelCount": 99,
            "openChannelCount": 1,
            "observedFrameCount": 12,
            "acceptedEventCount": 7,
            "streamCount": 2,
            "revision": 9,
            "transcript": "must not cross the native boundary"
        }
    });
    let event = sanitize_event(&raw.to_string()).unwrap();
    assert_eq!(event.kind, "realtime_voice_state");
    assert_eq!(event.payload["active"], true);
    assert_eq!(event.payload["observedChannelCount"], 32);
    assert_eq!(event.payload["openChannelCount"], 1);
    assert!(event.payload.get("transcript").is_none());
}
