use super::*;

#[test]
fn exposes_readiness_without_identity_or_page_content() {
    let runtime = LocalAiBrowserRuntime::default();
    runtime.ensure_session("local-ai-chatgpt-owner-secret", "chatgpt", "connecting");
    let page_url = Url::parse("https://chatgpt.com/c/private-conversation-id").unwrap();
    runtime.mark_navigation("local-ai-chatgpt-owner-secret", &page_url, true, None);
    runtime.mark_page_finished("local-ai-chatgpt-owner-secret", &page_url);
    runtime.record_adapter_event(
        "local-ai-chatgpt-owner-secret",
        "message_snapshot",
        json!({
            "type": "message_snapshot",
            "composerReady": true,
            "pageKind": "home",
            "draft": "private prompt",
            "privateRichRecovery": {
                "version": 1, "generation": 9, "active": true,
                "conversationBound": true, "turnBound": true,
                "richKinds": ["finance", "private-secret-kind"],
                "acceptedCount": 2, "lastOutcome": "accepted",
                "secret": "private recovery material"
            },
            "messages": [
                {"role":"user","content":[{"type":"text","text":"private prompt"}]},
                {"role":"assistant","content":[
                    {"type":"markdown","text":"private answer"},
                    {"type":"citation","text":"private source","url":"https://example.com/private","iconUrl":"https://example.com/favicon.ico","markerText":"private marker +1","citationId":"citation_control_1","groupSize":2},
                    {"type":"rich_card","kind":"finance","text":"private finance","richContent":{"schema":"yilong.rich-content.v1","kind":"finance","source":"official_dom","payload":{"title":"private asset","primaryValue":"private value","trend":"neutral"}}}
                ]}
            ],
        }),
    );
    runtime.record_adapter_event(
        "local-ai-chatgpt-owner-secret",
        "realtime_voice_state",
        json!({
            "type":"realtime_voice_state","version":2,"active":true,
            "observedChannelCount":2,"openChannelCount":1,"acceptedEventCount":7,
            "managedPhase":"active","managedActive":true,"microphoneActive":true,
            "remoteAudio":true,"muted":false,"routeBound":true,"lifecycleRevision":9,
            "authorization":"private voice credential"
        }),
    );
    runtime.record_adapter_event(
        "local-ai-chatgpt-owner-secret",
        "attachment_transport",
        json!({
            "type":"attachment_transport","transportVersion":1,"sequence":4,
            "state":"completed","completedCount":1,"url":"https://example.com/private-upload"
        }),
    );
    runtime.record_adapter_event(
        "local-ai-chatgpt-owner-secret",
        "conversation_snapshot",
        json!({
            "type":"conversation_snapshot",
            "conversations":[
                {"path":"/c/private-one","title":"private title","pinned":true},
                {"path":"/c/private-two","title":"private second title","pinned":false}
            ],
            "projects":[{"path":"/g/g-p-private/project","title":"private project"}],
            "collection":{"complete":false,"observedCount":1,"availableCount":2}
        }),
    );
    runtime.record_adapter_event(
        "local-ai-chatgpt-owner-secret",
        "browser_diagnostic",
        json!({"kind":"adapter_bootstrap_failed","detail":"private exception detail"}),
    );

    let diagnostic = runtime.diagnostic_for_provider("chatgpt").unwrap();
    assert_eq!(diagnostic["adapter_connected"], true);
    assert_eq!(diagnostic["semantic_snapshot_ready"], true);
    assert_eq!(diagnostic["composer_ready"], true);
    assert_eq!(diagnostic["context_ready"], true);
    assert_eq!(diagnostic["navigation_snapshot_ready"], true);
    assert_eq!(diagnostic["navigation_live"], true);
    assert_eq!(diagnostic["directory_complete"], false);
    assert_eq!(diagnostic["directory_observed_count"], 1);
    assert_eq!(diagnostic["directory_available_count"], 2);
    assert_eq!(diagnostic["conversation_count"], 2);
    assert_eq!(diagnostic["project_count"], 1);
    assert_eq!(diagnostic["pinned_count"], 1);
    assert_eq!(diagnostic["local_conversation_count"], 1);
    assert_eq!(diagnostic["active_conversation"], true);
    assert_eq!(diagnostic["message_count"], 2);
    assert_eq!(diagnostic["assistant_message_count"], 1);
    assert_eq!(diagnostic["content_part_counts"]["text"], 1);
    assert_eq!(diagnostic["content_part_counts"]["markdown"], 1);
    assert_eq!(diagnostic["content_part_counts"]["citation"], 1);
    assert_eq!(diagnostic["content_part_counts"]["rich_card"], 1);
    assert_eq!(diagnostic["rich_card_kind_counts"]["finance"], 1);
    assert_eq!(diagnostic["citation_count"], 1);
    assert_eq!(diagnostic["linked_citation_count"], 1);
    assert_eq!(diagnostic["citation_logo_count"], 1);
    assert_eq!(diagnostic["private_rich_recovery"]["active"], true);
    assert_eq!(
        diagnostic["private_rich_recovery"]["richKinds"],
        json!(["finance"])
    );
    assert_eq!(diagnostic["realtime_voice"]["active"], true);
    assert_eq!(diagnostic["realtime_voice"]["acceptedEventCount"], 7);
    assert_eq!(diagnostic["realtime_voice"]["managedPhase"], "active");
    assert_eq!(diagnostic["realtime_voice"]["managedActive"], true);
    assert_eq!(diagnostic["realtime_voice"]["remoteAudio"], true);
    assert_eq!(diagnostic["attachment_transport"]["state"], "completed");
    assert_eq!(diagnostic["attachment_transport"]["completedCount"], 1);
    assert_eq!(diagnostic["last_error_code"], "adapter_bootstrap_failed");
    let session = runtime.snapshot("local-ai-chatgpt-owner-secret").unwrap();
    assert_eq!(session.diagnostics["realtimeVoice"]["openChannelCount"], 1);
    assert_eq!(session.diagnostics["attachmentTransport"]["sequence"], 4);

    let encoded = diagnostic.to_string();
    for secret in [
        "owner-secret", "private-conversation-id", "private prompt", "private answer",
        "private source", "private marker", "example.com", "private asset",
        "private value", "private title", "private project", "private exception detail",
        "private recovery material", "private-secret-kind", "private voice credential",
        "private-upload",
    ] {
        assert!(!encoded.contains(secret), "diagnostic leaked {secret}");
    }
}
