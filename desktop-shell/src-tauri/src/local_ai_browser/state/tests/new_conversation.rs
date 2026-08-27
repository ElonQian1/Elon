use super::*;

#[test]
fn same_home_url_keeps_distinct_context_and_rejects_late_private_turns() {
    let runtime = LocalAiBrowserRuntime::default();
    runtime.ensure_session("session", "chatgpt", "active");
    let first_url = Url::parse("https://chatgpt.com/c/first").unwrap();
    let first_key = semantic_context::page_context_key("chatgpt", first_url.as_str());
    runtime.mark_navigation("session", &first_url, true, None);
    runtime.record_adapter_event_with_context(
        "session",
        "message_snapshot",
        json!({"type":"message_snapshot","messages":[
            {"id":"first-user","role":"user","state":"completed","content":[{"type":"text","text":"first"}]},
            {"id":"first-answer","role":"assistant","state":"completed","content":[{"type":"text","text":"first answer"}]}
        ]}),
        first_key.as_deref(),
    );
    let first_id = runtime
        .snapshot("session")
        .unwrap()
        .active_conversation_id
        .unwrap();

    runtime.mark_command_pending_with_value("session", "new_conversation", Some("mcp_new"), None);
    runtime.record_adapter_event_with_context(
        "session",
        "message_snapshot",
        json!({"type":"message_snapshot","messages":[
            {"id":"late-private-answer","role":"assistant","state":"streaming","content":[{"type":"text","text":"late private answer"}]}
        ]}),
        first_key.as_deref(),
    );
    let assistant_only_stale = runtime.snapshot("session").unwrap();
    assert!(!assistant_only_stale.context_ready);
    assert_eq!(
        assistant_only_stale.diagnostics["lastEventKind"],
        "stale_new_conversation_snapshot_ignored"
    );

    runtime.record_adapter_event_with_context(
        "session",
        "message_snapshot",
        json!({"type":"message_snapshot","messages":[
            {"id":"late-user","role":"user","state":"completed","content":[{"type":"text","text":"first"}]},
            {"id":"late-answer","role":"assistant","state":"completed","content":[{"type":"text","text":"late first answer"}]}
        ]}),
        first_key.as_deref(),
    );
    assert!(!runtime.snapshot("session").unwrap().context_ready);

    let home = Url::parse("https://chatgpt.com/").unwrap();
    let home_key = semantic_context::page_context_key("chatgpt", home.as_str());
    runtime.mark_navigation("session", &home, true, None);
    runtime.record_adapter_event_with_context(
        "session",
        "message_snapshot",
        json!({"type":"message_snapshot","messages":[]}),
        home_key.as_deref(),
    );
    runtime.mark_page_finished("session", &home);
    let blank = runtime.snapshot("session").unwrap();
    assert!(blank.context_ready);
    assert!(blank.semantic_event.unwrap()["messages"]
        .as_array()
        .unwrap()
        .is_empty());

    // DOM observers and private streams can both finish after the empty home snapshot.
    // Neither delayed copy may acquire the new local generation.
    for messages in [
        json!([
            {"id":"late-after-home-user","role":"user","state":"completed","content":[{"type":"text","text":"first"}]},
            {"id":"late-after-home-answer","role":"assistant","state":"completed","content":[{"type":"text","text":"late first answer"}]}
        ]),
        json!([
            {"id":"late-after-home-private-answer","role":"assistant","state":"streaming","content":[{"type":"text","text":"late private answer"}]}
        ]),
    ] {
        runtime.record_adapter_event_with_context(
            "session",
            "message_snapshot",
            json!({"type":"message_snapshot","messages":messages}),
            home_key.as_deref(),
        );
        let still_blank = runtime.snapshot("session").unwrap();
        assert!(still_blank.semantic_event.unwrap()["messages"]
            .as_array()
            .unwrap()
            .is_empty());
        assert_eq!(
            still_blank.diagnostics["lastEventKind"],
            "stale_new_conversation_snapshot_ignored"
        );
    }

    runtime.mark_command_pending_with_value(
        "session",
        "send_prompt",
        Some("mcp_second"),
        Some("second"),
    );
    runtime.record_adapter_event_with_context(
        "session",
        "message_snapshot",
        json!({"type":"message_snapshot","messages":[
            {"id":"second-user","role":"user","state":"completed","content":[{"type":"text","text":"second"}]},
            {"id":"second-answer","role":"assistant","state":"completed","content":[{"type":"text","text":"second answer"}]}
        ]}),
        home_key.as_deref(),
    );
    let second_url = Url::parse("https://chatgpt.com/c/second").unwrap();
    runtime.mark_navigation("session", &second_url, true, None);
    runtime.record_adapter_event_with_context(
        "session",
        "message_snapshot",
        json!({"type":"message_snapshot","messages":[
            {"id":"second-user","role":"user","state":"completed","content":[{"type":"text","text":"second"}]},
            {"id":"second-answer","role":"assistant","state":"completed","content":[{"type":"text","text":"second answer"}]}
        ]}),
        semantic_context::page_context_key("chatgpt", second_url.as_str()).as_deref(),
    );
    let second = runtime.snapshot("session").unwrap();
    let second_id = second.active_conversation_id.unwrap();
    assert_ne!(first_id, second_id);
    assert_eq!(second.local_conversations.len(), 2);
    assert_eq!(
        second.semantic_event.unwrap()["messages"][0]["id"],
        "second-user"
    );
}

#[test]
fn timed_out_new_conversation_never_rebinds_or_reveals_the_previous_snapshot() {
    let runtime = LocalAiBrowserRuntime::default();
    runtime.ensure_session("session", "chatgpt", "active");
    let previous_url = Url::parse("https://chatgpt.com/c/previous").unwrap();
    let previous_key = semantic_context::page_context_key("chatgpt", previous_url.as_str());
    runtime.mark_navigation("session", &previous_url, true, None);
    runtime.mark_page_finished("session", &previous_url);
    runtime.record_adapter_event_with_context(
        "session",
        "message_snapshot",
        json!({"type":"message_snapshot","messages":[
            {"id":"previous-user","role":"user","state":"completed","content":[{"type":"text","text":"previous"}]},
            {"id":"previous-answer","role":"assistant","state":"completed","content":[{"type":"text","text":"previous answer"}]}
        ]}),
        previous_key.as_deref(),
    );

    runtime.mark_command_pending_with_value("session", "new_conversation", Some("mcp_new"), None);
    runtime.backdate_pending_context_for_test("session", 9_500);
    let timed_out = runtime.snapshot("session").unwrap();
    assert!(!timed_out.context_ready);
    assert_eq!(timed_out.context_status, "unbound");
    assert!(!timed_out.semantic_conversation_aligned);
    assert_eq!(
        timed_out.diagnostics["lastEventKind"],
        "new_conversation_transition_timed_out"
    );

    runtime.record_adapter_event_with_context(
        "session",
        "message_snapshot",
        json!({"type":"message_snapshot","messages":[
            {"id":"late-private-answer","role":"assistant","state":"streaming","content":[{"type":"text","text":"late previous answer"}]}
        ]}),
        previous_key.as_deref(),
    );
    let guarded = runtime.snapshot("session").unwrap();
    assert!(!guarded.semantic_conversation_aligned);
    assert_eq!(
        guarded.semantic_event.unwrap()["messages"][0]["id"],
        "previous-user"
    );
    assert_eq!(
        guarded.diagnostics["lastEventKind"],
        "stale_new_conversation_snapshot_ignored"
    );
}

#[test]
fn failed_page_action_can_reestablish_a_safe_home_boundary() {
    let runtime = LocalAiBrowserRuntime::default();
    runtime.ensure_session("session", "chatgpt", "active");
    let previous_url = Url::parse("https://chatgpt.com/").unwrap();
    let previous_key = semantic_context::page_context_key("chatgpt", previous_url.as_str());
    runtime.mark_navigation("session", &previous_url, true, None);
    runtime.mark_page_finished("session", &previous_url);
    runtime.record_adapter_event_with_context(
        "session",
        "message_snapshot",
        json!({"type":"message_snapshot","messages":[
            {"id":"previous-user","role":"user","state":"completed","content":[{"type":"text","text":"previous"}]},
            {"id":"previous-answer","role":"assistant","state":"completed","content":[{"type":"text","text":"previous answer"}]}
        ]}),
        previous_key.as_deref(),
    );
    let previous_id = runtime
        .snapshot("session")
        .unwrap()
        .active_conversation_id
        .unwrap();

    runtime.mark_command_pending_with_value("session", "new_conversation", Some("page"), None);
    runtime.record_adapter_event(
        "session",
        "command_result",
        json!({
            "type":"command_result",
            "action":"new_conversation",
            "ok":false,
            "detail":"stale root",
            "requestId":"page"
        }),
    );
    assert_eq!(
        runtime
            .snapshot("session")
            .unwrap()
            .active_conversation_id
            .as_deref(),
        Some(previous_id.as_str())
    );

    // `new_conversation_home` performs this host transition before navigating.
    runtime.mark_command_pending("session", "new_conversation", None);
    let recovering = runtime.snapshot("session").unwrap();
    assert_ne!(recovering.active_conversation_id.as_deref(), Some(previous_id.as_str()));
    assert!(!recovering.context_ready);

    runtime.record_adapter_event_with_context(
        "session",
        "message_snapshot",
        json!({"type":"message_snapshot","messages":[
            {"id":"late-user","role":"user","state":"completed","content":[{"type":"text","text":"previous"}]},
            {"id":"late-answer","role":"assistant","state":"completed","content":[{"type":"text","text":"late previous answer"}]}
        ]}),
        previous_key.as_deref(),
    );
    assert_eq!(
        runtime.snapshot("session").unwrap().diagnostics["lastEventKind"],
        "stale_new_conversation_snapshot_ignored"
    );

    runtime.record_adapter_event_with_context(
        "session",
        "message_snapshot",
        json!({"type":"message_snapshot","messages":[],"composerReady":true}),
        previous_key.as_deref(),
    );
    assert!(runtime.snapshot("session").unwrap().context_ready);
}

#[test]
fn stale_new_conversation_receipt_cannot_rollback_the_current_request_generation() {
    let runtime = LocalAiBrowserRuntime::default();
    runtime.ensure_session("session", "chatgpt", "active");
    let previous = Url::parse("https://chatgpt.com/c/previous").unwrap();
    let previous_key = semantic_context::page_context_key("chatgpt", previous.as_str());
    runtime.mark_navigation("session", &previous, true, None);
    runtime.mark_page_finished("session", &previous);
    runtime.record_adapter_event_with_context(
        "session",
        "message_snapshot",
        json!({"type":"message_snapshot","messages":[
            {"id":"previous-user","role":"user","state":"completed","content":[{"type":"text","text":"previous"}]},
            {"id":"previous-answer","role":"assistant","state":"completed","content":[{"type":"text","text":"answer"}]}
        ]}),
        previous_key.as_deref(),
    );
    let previous_id = runtime.snapshot("session").unwrap().active_conversation_id.unwrap();

    runtime.mark_command_pending_with_value(
        "session",
        "new_conversation",
        Some("mcp_current"),
        None,
    );
    let current_id = runtime.snapshot("session").unwrap().active_conversation_id.unwrap();
    assert_ne!(current_id, previous_id);
    runtime.record_adapter_event(
        "session",
        "command_result",
        json!({
            "type":"command_result",
            "action":"new_conversation",
            "requestId":"mcp_stale",
            "ok":false
        }),
    );

    let guarded = runtime.snapshot("session").unwrap();
    assert_eq!(guarded.active_conversation_id.as_deref(), Some(current_id.as_str()));
    assert!(!guarded.context_ready);
    assert_eq!(
        guarded.diagnostics["lastEventKind"],
        "stale_context_command_result_ignored"
    );
}

#[test]
fn verified_chatgpt_new_conversation_receipt_establishes_an_empty_sendable_boundary() {
    let runtime = LocalAiBrowserRuntime::default();
    runtime.ensure_session("session", "chatgpt", "active");
    let previous = Url::parse("https://chatgpt.com/c/previous").unwrap();
    let previous_key = semantic_context::page_context_key("chatgpt", previous.as_str());
    runtime.mark_navigation("session", &previous, true, None);
    runtime.mark_page_finished("session", &previous);
    runtime.record_adapter_event_with_context(
        "session",
        "message_snapshot",
        json!({
            "type":"message_snapshot",
            "authenticated":false,
            "composerReady":true,
            "currentModel":"auto",
            "capabilities":["send_prompt"],
            "messages":[
                {"id":"previous-user","role":"user","state":"completed","content":[{"type":"text","text":"previous"}]},
                {"id":"previous-answer","role":"assistant","state":"completed","content":[{"type":"text","text":"answer"}]}
            ]
        }),
        previous_key.as_deref(),
    );
    let previous_id = runtime
        .snapshot("session")
        .unwrap()
        .active_conversation_id
        .unwrap();

    runtime.mark_command_pending_with_value(
        "session",
        "new_conversation",
        Some("mcp_current"),
        None,
    );
    let home = Url::parse("https://chatgpt.com/").unwrap();
    runtime.mark_navigation("session", &home, true, None);
    runtime.mark_page_finished("session", &home);
    runtime.record_adapter_event(
        "session",
        "command_result",
        json!({
            "type":"command_result",
            "action":"new_conversation",
            "requestId":"mcp_current",
            "ok":true
        }),
    );

    let blank = runtime.snapshot("session").unwrap();
    assert!(blank.context_ready);
    assert!(blank.semantic_conversation_aligned);
    assert_ne!(blank.active_conversation_id.as_deref(), Some(previous_id.as_str()));
    assert_eq!(blank.semantic_event.as_ref().unwrap()["messages"], json!([]));
    assert_eq!(blank.semantic_event.as_ref().unwrap()["composerReady"], true);
    assert_eq!(blank.semantic_event.as_ref().unwrap()["currentModel"], "auto");
    assert_eq!(
        blank.diagnostics["lastEventKind"],
        "verified_empty_new_conversation"
    );

    runtime.record_adapter_event_with_context(
        "session",
        "message_snapshot",
        json!({"type":"message_snapshot","messages":[
            {"id":"late-user","role":"user","state":"completed","content":[{"type":"text","text":"previous"}]},
            {"id":"late-answer","role":"assistant","state":"completed","content":[{"type":"text","text":"late answer"}]}
        ]}),
        previous_key.as_deref(),
    );
    let still_blank = runtime.snapshot("session").unwrap();
    assert!(still_blank.semantic_event.unwrap()["messages"]
        .as_array()
        .unwrap()
        .is_empty());
    assert_eq!(
        still_blank.diagnostics["lastEventKind"],
        "stale_new_conversation_snapshot_ignored"
    );
}

#[test]
fn stale_send_failure_cannot_cancel_the_current_prompt_generation() {
    let runtime = LocalAiBrowserRuntime::default();
    runtime.ensure_session("session", "chatgpt", "active");
    let url = Url::parse("https://chatgpt.com/c/current").unwrap();
    let key = semantic_context::page_context_key("chatgpt", url.as_str());
    runtime.mark_navigation("session", &url, true, None);
    runtime.mark_page_finished("session", &url);
    runtime.record_adapter_event_with_context(
        "session",
        "message_snapshot",
        json!({"type":"message_snapshot","messages":[
            {"id":"user","role":"user","state":"completed","content":[{"type":"text","text":"first"}]},
            {"id":"answer","role":"assistant","state":"completed","content":[{"type":"text","text":"answer"}]}
        ]}),
        key.as_deref(),
    );
    runtime.mark_command_pending_with_value(
        "session",
        "send_prompt",
        Some("mcp_current"),
        Some("second"),
    );
    for request_id in ["mcp_stale", "mcp_current"] {
        runtime.record_adapter_event(
            "session",
            "command_result",
            json!({
                "type":"command_result",
                "action":"send_prompt",
                "requestId":request_id,
                "ok":false
            }),
        );
        let state = runtime.snapshot("session").unwrap();
        if request_id == "mcp_stale" {
            assert!(!state.context_ready);
            assert_eq!(
                state.diagnostics["lastEventKind"],
                "stale_context_command_result_ignored"
            );
        } else {
            assert!(state.context_ready);
        }
    }
}
