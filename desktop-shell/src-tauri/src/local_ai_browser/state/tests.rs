use super::*;
use serde_json::json;

#[path = "tests/document.rs"]
mod document;
#[path = "tests/new_conversation.rs"]
mod new_conversation;
#[path = "tests/provider_diagnostics.rs"]
mod provider_diagnostics;
#[path = "tests/realtime_voice.rs"]
mod realtime_voice;
#[path = "tests/attachment_transport.rs"]
mod attachment_transport;

#[test]
fn private_stream_state_is_authoritative_over_stale_dom_streaming() {
    assert!(private_stream::is_streaming(Some(&json!({
        "streaming": false,
        "privateStreamObserved": true,
        "privateStreamState": "streaming"
    }))));
    assert!(!private_stream::is_streaming(Some(&json!({
        "streaming": true,
        "privateStreamObserved": true,
        "privateStreamState": "completed"
    }))));
    assert!(private_stream::is_streaming(Some(&json!({
        "streaming": true,
        "privateStreamObserved": false,
        "privateStreamState": "completed"
    }))));
}

#[test]
fn parked_session_labels_excludes_visible_and_closed_sessions() {
    let runtime = LocalAiBrowserRuntime::default();
    runtime.ensure_session("parked", "chatgpt", "active");
    runtime.mark_opening("parked", false);
    runtime.mark_window_status("parked", "ready");

    runtime.ensure_session("visible", "chatgpt", "active");
    runtime.mark_opening("visible", true);
    runtime.mark_window_status("visible", "ready");

    runtime.ensure_session("closed", "chatgpt", "active");
    runtime.mark_window_status("closed", "closed");

    let mut parked = runtime.parked_session_labels();
    parked.sort();
    assert_eq!(parked, vec!["parked".to_string()]);
}

#[test]
fn message_and_navigation_snapshots_do_not_overwrite_each_other() {
    let runtime = LocalAiBrowserRuntime::default();
    runtime.ensure_session("session", "chatgpt", "reserved");
    runtime.record_adapter_event(
        "session",
        "message_snapshot",
        json!({"type": "message_snapshot", "messages": [{"id": "answer"}]}),
    );
    let semantic_updated_at_ms = runtime.snapshot("session").unwrap().semantic_updated_at_ms;
    assert!(semantic_updated_at_ms > 0);
    std::thread::sleep(std::time::Duration::from_millis(2));
    runtime.record_adapter_event(
        "session",
        "conversation_snapshot",
        json!({"type": "conversation_snapshot", "projects": [{"id": "project"}]}),
    );

    let snapshot = runtime.snapshot("session").unwrap();
    assert_eq!(snapshot.semantic_event.unwrap()["type"], "message_snapshot");
    assert_eq!(
        snapshot.navigation_event.unwrap()["type"],
        "conversation_snapshot"
    );
    assert_eq!(snapshot.semantic_updated_at_ms, semantic_updated_at_ms);
    assert!(snapshot.cache_updated_at_ms > snapshot.semantic_updated_at_ms);
}

#[test]
fn partial_official_directory_updates_do_not_erase_cached_sidebar_items() {
    let runtime = LocalAiBrowserRuntime::default();
    runtime.ensure_session("session", "chatgpt", "active");
    runtime.record_adapter_event(
        "session",
        "conversation_snapshot",
        json!({
            "type":"conversation_snapshot",
            "conversations":[
                {"path":"/c/one","title":"One","pinned":true},
                {"path":"/c/two","title":"Two","pinned":false}
            ],
            "projects":[{"path":"/g/g-p-roadmap/project","id":"g-p-roadmap","title":"Roadmap"}],
            "collection":{"complete":false,"observedCount":2}
        }),
    );
    runtime.record_adapter_event(
        "session",
        "conversation_snapshot",
        json!({
            "type":"conversation_snapshot",
            "conversations":[{"path":"/c/one","title":"One updated","pinned":false}],
            "projects":[],
            "collection":{"complete":false,"observedCount":1}
        }),
    );

    let snapshot = runtime.snapshot("session").unwrap();
    let directory = snapshot.navigation_event.unwrap();
    assert_eq!(directory["conversations"].as_array().unwrap().len(), 2);
    assert_eq!(directory["conversations"][0]["pinned"], true);
    assert_eq!(directory["projects"].as_array().unwrap().len(), 1);
    assert_eq!(directory["collection"]["source"], "official_partial");
    assert_eq!(snapshot.navigation_updated_at_ms, 0);
}

#[test]
fn only_complete_official_directory_advances_verified_freshness() {
    let runtime = LocalAiBrowserRuntime::default();
    runtime.ensure_session("session", "chatgpt", "active");
    runtime.record_adapter_event(
        "session",
        "conversation_snapshot",
        json!({
            "type":"conversation_snapshot",
            "conversations":[{"path":"/c/one","title":"One"}],
            "projects":[],
            "collection":{"complete":false,"observedCount":1}
        }),
    );
    assert_eq!(runtime.snapshot("session").unwrap().navigation_updated_at_ms, 0);

    runtime.record_adapter_event(
        "session",
        "conversation_snapshot",
        json!({
            "type":"conversation_snapshot",
            "conversations":[{"path":"/c/one","title":"One"}],
            "projects":[],
            "collection":{"complete":true,"observedCount":1}
        }),
    );
    let verified_at = runtime.snapshot("session").unwrap().navigation_updated_at_ms;
    assert!(verified_at > 0);

    runtime.record_adapter_event(
        "session",
        "conversation_snapshot",
        json!({
            "type":"conversation_snapshot",
            "conversations":[{"path":"/c/one","title":"One partial"}],
            "projects":[],
            "collection":{"complete":false,"observedCount":1}
        }),
    );
    assert_eq!(
        runtime.snapshot("session").unwrap().navigation_updated_at_ms,
        verified_at
    );

    runtime.mark_command_pending("session", "new_conversation", Some("new-chat"));
    assert_eq!(runtime.snapshot("session").unwrap().navigation_updated_at_ms, 0);
}

#[test]
fn composer_feature_and_command_receipts_remain_independent() {
    let runtime = LocalAiBrowserRuntime::default();
    runtime.ensure_session("session", "chatgpt", "reserved");
    runtime.record_adapter_event(
        "session",
        "composer_controls_snapshot",
        json!({"type":"composer_controls_snapshot","section":"model","options":[]}),
    );
    runtime.record_adapter_event(
        "session",
        "navigation_snapshot",
        json!({"type":"navigation_snapshot","features":[]}),
    );
    runtime.mark_command_pending("session", "send_prompt", Some("mcp_receipt1"));
    runtime.record_adapter_event(
        "session",
        "command_result",
        json!({"type":"command_result","action":"send_prompt","requestId":"mcp_receipt1","ok":true}),
    );

    let snapshot = runtime.snapshot("session").unwrap();
    assert_eq!(snapshot.composer_event.unwrap()["section"], "model");
    assert_eq!(
        snapshot.feature_event.unwrap()["type"],
        "navigation_snapshot"
    );
    assert_eq!(
        snapshot.command_result.unwrap()["requestId"],
        "mcp_receipt1"
    );
    assert_eq!(snapshot.diagnostics["lastCommandRequestId"], "mcp_receipt1");
}

#[test]
fn interleaved_command_receipts_remain_available_by_request_id() {
    let runtime = LocalAiBrowserRuntime::default();
    runtime.ensure_session("session", "chatgpt", "active");
    for index in 0..10 {
        let request_id = format!("mcp_receipt{index}");
        runtime.mark_command_pending("session", "collect_model_options", Some(&request_id));
        runtime.record_adapter_event(
            "session",
            "command_result",
            json!({"type":"command_result","action":"collect_model_options","requestId":request_id,"ok":true}),
        );
    }

    let snapshot = runtime.snapshot("session").unwrap();
    assert_eq!(snapshot.command_results.len(), 8);
    assert_eq!(snapshot.command_results[0]["requestId"], "mcp_receipt2");
    assert_eq!(snapshot.command_results[7]["requestId"], "mcp_receipt9");
    assert_eq!(
        snapshot.command_result.unwrap()["requestId"],
        "mcp_receipt9"
    );
}

#[test]
fn background_opening_never_reports_a_visible_official_window() {
    let runtime = LocalAiBrowserRuntime::default();
    runtime.ensure_session("session", "chatgpt", "connecting");
    runtime.mark_opening("session", false);

    let background = runtime.snapshot("session").unwrap();
    assert_eq!(background.window_status, "opening");
    assert!(!background.window_visible);

    runtime.mark_opening("session", true);
    assert!(runtime.snapshot("session").unwrap().window_visible);
}

#[test]
fn cached_messages_are_not_sendable_until_the_official_page_is_live_and_bound() {
    let runtime = LocalAiBrowserRuntime::default();
    runtime.ensure_session("session", "chatgpt", "active");
    let url = Url::parse("https://chatgpt.com/c/context").unwrap();
    runtime.mark_navigation("session", &url, true, None);
    runtime.mark_page_finished("session", &url);
    runtime.record_adapter_event_with_context(
        "session",
        "message_snapshot",
        json!({"type":"message_snapshot","composerReady":true,"messages":[
            {"id":"user","role":"user","state":"completed","content":[{"type":"text","text":"first"}]},
            {"id":"answer","role":"assistant","state":"completed","content":[{"type":"text","text":"answer"}]}
        ]}),
        semantic_context::page_context_key("chatgpt", url.as_str()).as_deref(),
    );
    let bound = runtime.snapshot("session").unwrap();
    assert!(bound.context_ready);
    assert_eq!(bound.context_status, "bound");
    assert!(runtime.require_bound_context("session").is_ok());

    runtime.mark_navigation("session", &url, true, None);
    let restoring = runtime.snapshot("session").unwrap();
    assert!(!restoring.context_ready);
    assert_eq!(restoring.context_status, "restoring");
    assert!(runtime.require_bound_context("session").is_err());
}

#[test]
fn stale_pending_context_self_heals_after_timeout_without_a_matching_snapshot() {
    let runtime = LocalAiBrowserRuntime::default();
    runtime.ensure_session("session", "chatgpt", "active");
    let url = Url::parse("https://chatgpt.com/c/context").unwrap();
    runtime.mark_navigation("session", &url, true, None);
    runtime.mark_page_finished("session", &url);
    runtime.record_adapter_event_with_context(
        "session",
        "message_snapshot",
        json!({"type":"message_snapshot","composerReady":true,"messages":[
            {"id":"user","role":"user","state":"completed","content":[{"type":"text","text":"first"}]},
            {"id":"answer","role":"assistant","state":"completed","content":[{"type":"text","text":"answer"}]}
        ]}),
        semantic_context::page_context_key("chatgpt", url.as_str()).as_deref(),
    );
    assert!(runtime.snapshot("session").unwrap().context_ready);

    // 官网从未回过匹配这次命令的可见快照（例如提取逻辑判定内容未变化而丢弃了事件）。
    runtime.mark_command_pending("session", "send_prompt", Some("mcp_stall1"));
    let stalled = runtime.snapshot("session").unwrap();
    assert!(!stalled.context_ready);
    assert_eq!(stalled.context_status, "restoring");
    assert!(runtime.require_bound_context("session").is_err());

    runtime.backdate_pending_context_for_test("session", 9_500);
    let recovered = runtime.snapshot("session").unwrap();
    assert!(recovered.context_ready);
    assert_eq!(recovered.context_status, "bound");
    assert!(runtime.require_bound_context("session").is_ok());
}

#[test]
fn send_snapshot_can_advance_a_chatgpt_spa_route_without_losing_context() {
    let runtime = LocalAiBrowserRuntime::default();
    runtime.ensure_session("session", "chatgpt", "active");
    let home = Url::parse("https://chatgpt.com/").unwrap();
    let home_key = semantic_context::page_context_key("chatgpt", home.as_str());
    runtime.mark_navigation("session", &home, true, None);
    runtime.mark_page_finished("session", &home);
    runtime.record_adapter_event_with_context(
        "session",
        "message_snapshot",
        json!({"type":"message_snapshot","composerReady":true,"messages":[]}),
        home_key.as_deref(),
    );
    let conversation_id = runtime
        .snapshot("session")
        .unwrap()
        .active_conversation_id
        .unwrap();

    runtime.mark_command_pending_with_value(
        "session",
        "send_prompt",
        Some("mcp_first"),
        Some("first"),
    );
    assert!(!runtime.snapshot("session").unwrap().context_ready);
    runtime.record_adapter_event_with_context(
        "session",
        "message_snapshot",
        json!({"type":"message_snapshot","composerReady":true,"messages":[]}),
        home_key.as_deref(),
    );
    let pending = runtime.snapshot("session").unwrap();
    assert!(!pending.context_ready);
    assert_eq!(
        pending.diagnostics["lastEventKind"],
        "pending_send_snapshot_ignored"
    );

    let conversation = Url::parse("https://chatgpt.com/c/first").unwrap();
    let conversation_key = semantic_context::page_context_key("chatgpt", conversation.as_str());
    runtime.record_adapter_event_with_context_and_url(
        "session",
        "message_snapshot",
        json!({"type":"message_snapshot","composerReady":true,"messages":[
            {"id":"user","role":"user","state":"completed","content":[{"type":"text","text":"first"}]},
            {"id":"answer","role":"assistant","state":"completed","content":[{"type":"text","text":"answer"}]}
        ]}),
        conversation_key.as_deref(),
        Some(conversation.as_str()),
    );

    let restored = runtime.snapshot("session").unwrap();
    assert!(restored.context_ready);
    assert_eq!(restored.context_status, "bound");
    assert_eq!(
        restored.active_conversation_id.as_deref(),
        Some(conversation_id.as_str())
    );
    assert_eq!(
        runtime.cached_restorable_url("session").as_deref(),
        Some(conversation.as_str())
    );
}

#[test]
fn google_conversation_cache_exposes_only_opaque_metadata_and_restores_messages() {
    let runtime = LocalAiBrowserRuntime::default();
    runtime.ensure_session("session", "google-ai-mode", "active");
    runtime.mark_navigation(
        "session",
        &Url::parse(
            "https://www.google.com/search?q=private+prompt&udm=50&csuir=thread_cache_1234",
        )
        .unwrap(),
        true,
        None,
    );
    runtime.record_adapter_event(
        "session",
        "message_snapshot",
        json!({
            "type": "message_snapshot",
            "title": "Cached search",
            "streaming": false,
            "messages": [{"role": "assistant", "content": [{"type": "text", "text": "answer"}]}],
        }),
    );

    let snapshot = runtime.snapshot("session").unwrap();
    assert_eq!(snapshot.local_conversations.len(), 1);
    assert!(snapshot.local_conversations[0].active);
    let encoded = serde_json::to_string(&snapshot).unwrap();
    assert!(!encoded.contains("private+prompt"));

    let id = snapshot.local_conversations[0].id.clone();
    assert!(runtime
        .activate_cached_conversation("session", &id)
        .is_some());
    let restored = runtime.snapshot("session").unwrap();
    assert_eq!(restored.semantic_cache_status, "cached");
    assert_eq!(restored.semantic_event.unwrap()["title"], "Cached search");
}

#[test]
fn google_cached_conversation_rejects_a_partial_live_history_overwrite() {
    let runtime = LocalAiBrowserRuntime::default();
    runtime.ensure_session("session", "google-ai-mode", "active");
    let url = Url::parse(
        "https://www.google.com/search?q=first&udm=50&csuir=thread_history_1234",
    )
    .unwrap();
    runtime.mark_navigation("session", &url, true, None);
    let context_key = semantic_context::page_context_key("google-ai-mode", url.as_str());
    runtime.record_adapter_event_with_context(
        "session",
        "message_snapshot",
        json!({"type":"message_snapshot","url":url.as_str(),"messages":[
            {"role":"user","state":"completed","content":[{"type":"text","text":"first"}]},
            {"role":"assistant","state":"completed","content":[{"type":"text","text":"first answer"}]},
            {"role":"user","state":"completed","content":[{"type":"text","text":"second"}]},
            {"role":"assistant","state":"completed","content":[{"type":"text","text":"second answer"}]}
        ]}),
        context_key.as_deref(),
    );
    let id = runtime.snapshot("session").unwrap().local_conversations[0]
        .id
        .clone();

    assert!(runtime
        .activate_cached_conversation("session", &id)
        .is_some());
    runtime.record_adapter_event_with_context(
        "session",
        "message_snapshot",
        json!({"type":"message_snapshot","url":url.as_str(),"messages":[
            {"role":"user","state":"completed","content":[{"type":"text","text":"first"}]},
            {"role":"assistant","state":"completed","content":[{"type":"text","text":"first answer"}]}
        ]}),
        context_key.as_deref(),
    );

    let restored = runtime.snapshot("session").unwrap();
    let messages = restored.semantic_event.unwrap()["messages"]
        .as_array()
        .unwrap()
        .clone();
    assert_eq!(messages.len(), 4);
    assert_eq!(messages[2]["content"][0]["text"], "second");
}

#[test]
fn google_followups_merge_into_one_stable_native_conversation() {
    let runtime = LocalAiBrowserRuntime::default();
    runtime.ensure_session("session", "google-ai-mode", "active");
    let first_url = Url::parse(
        "https://www.google.com/search?udm=50&q=first-private-prompt&csuir=thread_followup_1234",
    )
    .unwrap();
    runtime.mark_navigation("session", &first_url, true, None);
    runtime.record_adapter_event_with_context(
        "session",
        "message_snapshot",
        json!({
            "type": "message_snapshot",
            "title": "First question",
            "streaming": false,
            "messages": [
                {"id":"google-query-current","role":"user","state":"completed","content":[{"type":"text","text":"first"}]},
                {"id":"google-answer-current","role":"assistant","state":"completed","content":[{"type":"text","text":"first answer"}]}
            ],
        }),
        semantic_context::page_context_key("google-ai-mode", first_url.as_str()).as_deref(),
    );
    let first = runtime.snapshot("session").unwrap();
    let conversation_id = first.active_conversation_id.clone().unwrap();

    runtime.mark_command_pending_with_value(
        "session",
        "send_prompt",
        Some("mcp_followup"),
        Some("second"),
    );
    let followup_url = Url::parse(
        "https://www.google.com/search?udm=50&q=second-private-prompt&csuir=thread_followup_1234",
    )
    .unwrap();
    runtime.mark_navigation("session", &followup_url, true, None);
    runtime.record_adapter_event_with_context(
        "session",
        "message_snapshot",
        json!({
            "type": "message_snapshot",
            "title": "Second question",
            "streaming": false,
            "messages": [
                {"id":"google-query-current","role":"user","state":"completed","content":[{"type":"text","text":"second"}]},
                {"id":"google-answer-current","role":"assistant","state":"completed","content":[{"type":"text","text":"second answer"}]}
            ],
        }),
        semantic_context::page_context_key("google-ai-mode", followup_url.as_str()).as_deref(),
    );

    let followup = runtime.snapshot("session").unwrap();
    let messages = followup.semantic_event.unwrap()["messages"]
        .as_array()
        .unwrap()
        .clone();
    assert_eq!(messages.len(), 4);
    assert_eq!(messages[0]["content"][0]["text"], "first");
    assert_eq!(messages[2]["content"][0]["text"], "second");
    assert_eq!(
        followup.active_conversation_id.as_deref(),
        Some(conversation_id.as_str())
    );
    assert_eq!(followup.local_conversations.len(), 1);
}

#[test]
fn google_visible_page_followup_keeps_context_without_a_native_send_receipt() {
    let runtime = LocalAiBrowserRuntime::default();
    runtime.ensure_session("session", "google-ai-mode", "active");
    let first_url = Url::parse("https://www.google.com/search?udm=50&q=first").unwrap();
    runtime.mark_navigation("session", &first_url, true, None);
    runtime.record_adapter_event_with_context(
        "session",
        "message_snapshot",
        json!({"type":"message_snapshot","messages":[
            {"role":"user","state":"completed","content":[{"type":"text","text":"first"}]},
            {"role":"assistant","state":"completed","content":[{"type":"text","text":"first answer"}]}
        ]}),
        semantic_context::page_context_key("google-ai-mode", first_url.as_str()).as_deref(),
    );
    let first_id = runtime
        .snapshot("session")
        .unwrap()
        .active_conversation_id
        .unwrap();

    let second_url = Url::parse("https://www.google.com/search?udm=50&q=second").unwrap();
    runtime.mark_navigation("session", &second_url, true, None);
    runtime.record_adapter_event_with_context(
        "session",
        "message_snapshot",
        json!({"type":"message_snapshot","messages":[
            {"role":"user","state":"completed","content":[{"type":"text","text":"second"}]},
            {"role":"assistant","state":"completed","content":[{"type":"text","text":"second answer"}]}
        ]}),
        semantic_context::page_context_key("google-ai-mode", second_url.as_str()).as_deref(),
    );
    let second = runtime.snapshot("session").unwrap();
    assert_eq!(
        second.active_conversation_id.as_deref(),
        Some(first_id.as_str())
    );
    assert_eq!(
        second.semantic_event.unwrap()["messages"]
            .as_array()
            .unwrap()
            .len(),
        4
    );
}

#[test]
fn google_private_directory_open_rebinds_instead_of_merging_the_previous_thread() {
    let runtime = LocalAiBrowserRuntime::default();
    runtime.ensure_session("session", "google-ai-mode", "active");
    let first_url = Url::parse("https://www.google.com/search?udm=50&q=first").unwrap();
    runtime.mark_navigation("session", &first_url, true, None);
    runtime.record_adapter_event_with_context(
        "session",
        "message_snapshot",
        json!({"type":"message_snapshot","messages":[
            {"role":"user","state":"completed","content":[{"type":"text","text":"first"}]},
            {"role":"assistant","state":"completed","content":[{"type":"text","text":"first answer"}]}
        ]}),
        semantic_context::page_context_key("google-ai-mode", first_url.as_str()).as_deref(),
    );
    let first_id = runtime
        .snapshot("session")
        .unwrap()
        .active_conversation_id
        .unwrap();

    runtime.mark_command_pending_with_value(
        "session",
        "open_conversation",
        Some("mcp_google_open"),
        Some("/c/thread_1234567890"),
    );
    let second_url =
        Url::parse("https://www.google.com/search?udm=50&q=second&csuir=thread_1234567890")
            .unwrap();
    let second_key = semantic_context::page_context_key("google-ai-mode", second_url.as_str());
    runtime.mark_navigation("session", &second_url, true, None);
    runtime.record_adapter_event_with_context(
        "session",
        "message_snapshot",
        json!({"type":"message_snapshot","messages":[
            {"role":"user","state":"completed","content":[{"type":"text","text":"second"}]},
            {"role":"assistant","state":"completed","content":[{"type":"text","text":"second answer"}]}
        ]}),
        second_key.as_deref(),
    );

    let opened = runtime.snapshot("session").unwrap();
    assert_ne!(
        opened.active_conversation_id.as_deref(),
        Some(first_id.as_str())
    );
    assert_eq!(
        opened.active_conversation_id.as_deref(),
        second_key.as_deref()
    );
    let messages = opened.semantic_event.unwrap()["messages"]
        .as_array()
        .unwrap()
        .clone();
    assert_eq!(messages.len(), 2);
    assert_eq!(messages[0]["content"][0]["text"], "second");
}

#[test]
fn late_snapshot_from_previous_page_cannot_replace_opened_conversation() {
    let runtime = LocalAiBrowserRuntime::default();
    runtime.ensure_session("session", "chatgpt", "active");
    let first_url = Url::parse("https://chatgpt.com/c/first").unwrap();
    runtime.mark_navigation("session", &first_url, true, None);
    runtime.record_adapter_event_with_context(
        "session",
        "message_snapshot",
        json!({"type":"message_snapshot","url":first_url.as_str(),"messages":[
            {"id":"first","role":"assistant","state":"completed","content":[{"type":"text","text":"first answer"}]}
        ]}),
        semantic_context::page_context_key("chatgpt", first_url.as_str()).as_deref(),
    );

    runtime.mark_command_pending_with_value(
        "session",
        "open_conversation",
        Some("mcp_open_second"),
        Some("/c/second"),
    );
    let second_url = Url::parse("https://chatgpt.com/c/second").unwrap();
    runtime.mark_navigation("session", &second_url, true, None);
    runtime.record_adapter_event_with_context(
        "session",
        "message_snapshot",
        json!({"type":"message_snapshot","url":first_url.as_str(),"messages":[
            {"id":"late","role":"assistant","state":"completed","content":[{"type":"text","text":"late first answer"}]}
        ]}),
        semantic_context::page_context_key("chatgpt", first_url.as_str()).as_deref(),
    );
    assert_eq!(
        runtime.snapshot("session").unwrap().semantic_event.unwrap()["messages"][0]["id"],
        "first",
    );

    runtime.mark_page_finished("session", &second_url);
    runtime.record_adapter_event_with_context(
        "session",
        "message_snapshot",
        json!({"type":"message_snapshot","url":second_url.as_str(),"messages":[
            {"id":"second","role":"assistant","state":"completed","content":[{"type":"text","text":"second answer"}]}
        ]}),
        semantic_context::page_context_key("chatgpt", second_url.as_str()).as_deref(),
    );
    let opened = runtime.snapshot("session").unwrap();
    assert_eq!(
        opened.semantic_event.unwrap()["messages"][0]["id"],
        "second"
    );
    assert!(opened.context_ready);
}

#[test]
fn navigation_keeps_the_snapshot_visible_but_marks_it_cached() {
    let runtime = LocalAiBrowserRuntime::default();
    runtime.ensure_session("session", "chatgpt", "reserved");
    runtime.record_adapter_event(
        "session",
        "message_snapshot",
        json!({"type": "message_snapshot", "messages": []}),
    );
    assert_eq!(runtime.snapshot("session").unwrap().cache_status, "live");

    runtime.mark_navigation(
        "session",
        &Url::parse("https://chatgpt.com/c/example").unwrap(),
        true,
        None,
    );
    let cached = runtime.snapshot("session").unwrap();
    assert_eq!(cached.cache_status, "cached");
    assert!(cached.semantic_event.is_some());

    runtime.clear_snapshots("session");
    let cleared = runtime.snapshot("session").unwrap();
    assert_eq!(cleared.cache_status, "empty");
    assert!(cleared.semantic_event.is_none());
}
