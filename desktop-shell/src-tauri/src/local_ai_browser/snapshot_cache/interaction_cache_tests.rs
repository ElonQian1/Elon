use super::*;
use serde_json::json;

#[test]
fn cacheable_snapshot_strips_draft_and_rejects_streaming_content() {
    let complete = cacheable_envelope(
        "chatgpt",
        Some(&json!({
            "type": "message_snapshot",
            "draft": "private unfinished text",
            "streaming": false,
            "messages": [{"state": "completed", "content": [{"type": "text", "text": "answer"}]}]
        })),
        None,
        &BTreeMap::new(),
        None,
        &[],
        0,
        42,
    )
    .unwrap();
    assert_eq!(complete.semantic_event.unwrap()["draft"], "");

    let streaming = json!({
        "type": "message_snapshot",
        "draft": "",
        "streaming": true,
        "messages": [{"state": "streaming"}]
    });
    assert!(cacheable_envelope(
        "chatgpt",
        Some(&streaming),
        None,
        &BTreeMap::new(),
        None,
        &[],
        0,
        43,
    )
    .is_none());
}

#[test]
fn google_cache_migration_drops_prompt_urls_and_canonicalizes_durable_threads() {
    let updated_at_ms = now_ms();
    let snapshot = |id: &str, restorable_url: &str| StoredConversationSnapshot {
        id: id.to_string(),
        title: "Google conversation".to_string(),
        restorable_url: restorable_url.to_string(),
        semantic_event: json!({"type":"message_snapshot","messages":[]}),
        updated_at_ms,
    };
    let values = sanitize_stored_conversations_with_ttl(
        "google-ai-mode",
        vec![
            snapshot(
                "0000000000000001",
                "https://www.google.com/search?q=old-prompt&udm=50",
            ),
            snapshot(
                "0000000000000002",
                "https://google.com/search?ved=drop&q=durable&udm=50&csuir=thread_1234567890",
            ),
        ],
        CACHE_TTL_MS,
    );

    assert_eq!(values.len(), 1);
    assert_eq!(values[0].id, "0000000000000002");
    assert_eq!(
        values[0].restorable_url,
        "https://www.google.com/search?q=durable&udm=50&csuir=thread_1234567890",
    );
}
