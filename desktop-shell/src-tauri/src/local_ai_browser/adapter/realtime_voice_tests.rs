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
            "version": 2,
            "active": true,
            "observedChannelCount": 99,
            "openChannelCount": 1,
            "observedFrameCount": 12,
            "acceptedEventCount": 7,
            "streamCount": 2,
            "revision": 9,
            "managedPhase": "active",
            "managedActive": true,
            "microphoneActive": true,
            "remoteAudio": true,
            "muted": false,
            "routeBound": true,
            "fallbackCode": "",
            "lifecycleRevision": 12,
            "transcript": "must not cross the native boundary"
        }
    });
    let event = sanitize_event(&raw.to_string()).unwrap();
    assert_eq!(event.kind, "realtime_voice_state");
    assert_eq!(event.payload["active"], true);
    assert_eq!(event.payload["observedChannelCount"], 32);
    assert_eq!(event.payload["openChannelCount"], 1);
    assert_eq!(event.payload["managedPhase"], "active");
    assert_eq!(event.payload["managedActive"], true);
    assert_eq!(event.payload["microphoneActive"], true);
    assert_eq!(event.payload["remoteAudio"], true);
    assert_eq!(event.payload["routeBound"], true);
    assert_eq!(event.payload["lifecycleRevision"], 12);
    assert!(event.payload.get("transcript").is_none());
}

#[test]
fn realtime_voice_state_rejects_unstructured_managed_details() {
    let raw = json!({
        "schema": "yilong.ai.ui.v1",
        "providerId": "chatgpt",
        "adapterVersion": ADAPTER_VERSION,
        "documentToken": "doc_win_voice_redaction",
        "event": {
            "type": "realtime_voice_state",
            "version": 2,
            "managedPhase": "active<script>",
            "fallbackCode": "relay_failed bearer private-secret",
            "offer": "v=0 private sdp",
            "answer": "v=0 private answer",
            "authorization": "private credential"
        }
    });
    let event = sanitize_event(&raw.to_string()).unwrap();
    assert_eq!(event.payload["managedPhase"], "activescript");
    assert_eq!(event.payload["fallbackCode"], "relay_failedbearerprivatesecret");
    let encoded = event.payload.to_string();
    for secret in ["v=0", "authorization", "credential", "<script>"] {
        assert!(!encoded.contains(secret));
    }
}
