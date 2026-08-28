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
    assert_ne!(
        recovering.active_conversation_id.as_deref(),
        Some(previous_id.as_str())
    );
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
    let current_id = runtime
        .snapshot("session")
        .unwrap()
        .active_conversation_id
        .unwrap();
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
    assert_eq!(
        guarded.active_conversation_id.as_deref(),
        Some(current_id.as_str())
    );
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
            "privateStreamObserved":true,
            "privateStreamRevision":17,
            "privateStreamState":"completed",
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
    assert_ne!(
        blank.active_conversation_id.as_deref(),
        Some(previous_id.as_str())
    );
    assert_eq!(
        blank.semantic_event.as_ref().unwrap()["messages"],
        json!([])
    );
    assert_eq!(
        blank.semantic_event.as_ref().unwrap()["composerReady"],
        true
    );
    assert_eq!(
        blank.semantic_event.as_ref().unwrap()["currentModel"],
        "auto"
    );
    assert_eq!(
        blank.semantic_event.as_ref().unwrap()["privateStreamRevision"],
        17
    );
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

#[test]
fn new_conversation_keeps_the_private_revision_watermark_for_the_next_send() {
    let runtime = LocalAiBrowserRuntime::default();
    runtime.ensure_session("session", "chatgpt", "active");
    let home = Url::parse("https://chatgpt.com/").unwrap();
    let key = semantic_context::page_context_key("chatgpt", home.as_str());
    runtime.mark_navigation("session", &home, true, None);
    runtime.mark_page_finished("session", &home);
    runtime.record_adapter_event_with_context(
        "session",
        "message_snapshot",
        json!({
            "type":"message_snapshot",
            "composerReady":true,
            "privateStreamObserved":true,
            "privateStreamRevision":31,
            "privateStreamState":"completed",
            "messages":[]
        }),
        key.as_deref(),
    );
    runtime.mark_command_pending_with_value(
        "session",
        "new_conversation",
        Some("mcp-new-watermark"),
        None,
    );
    runtime.record_adapter_event(
        "session",
        "command_result",
        json!({
            "type":"command_result",
            "action":"new_conversation",
            "requestId":"mcp-new-watermark",
            "ok":true
        }),
    );
    runtime.mark_command_pending_with_value(
        "session",
        "send_prompt",
        Some("mcp-after-new"),
        Some("新会话问题"),
    );
    runtime.record_adapter_event_with_context(
        "session",
        "message_snapshot",
        json!({
            "type":"message_snapshot",
            "privateStreamObserved":true,
            "privateStreamRevision":31,
            "privateStreamState":"streaming",
            "messages":[{
                "id":"private-stream:late-old-turn",
                "role":"assistant",
                "state":"streaming",
                "content":[{"type":"markdown","text":"上一会话迟到内容"}]
            }]
        }),
        key.as_deref(),
    );

    let state = runtime.snapshot("session").unwrap();
    assert!(!state.context_ready);
    assert_eq!(
        state.diagnostics["lastEventKind"],
        "pending_send_snapshot_ignored"
    );
    assert!(state.semantic_event.unwrap()["messages"]
        .as_array()
        .unwrap()
        .is_empty());
}

#[test]
fn private_stream_binds_a_new_prompt_before_the_official_dom_mounts_its_user_turn() {
    let runtime = LocalAiBrowserRuntime::default();
    runtime.ensure_session("session", "chatgpt", "active");
    let home = Url::parse("https://chatgpt.com/").unwrap();
    let home_key = semantic_context::page_context_key("chatgpt", home.as_str());
    runtime.mark_navigation("session", &home, true, None);
    runtime.mark_page_finished("session", &home);
    runtime.record_adapter_event_with_context(
        "session",
        "message_snapshot",
        json!({
            "type":"message_snapshot",
            "privateStreamObserved":true,
            "privateStreamRevision":7,
            "privateStreamState":"idle",
            "messages":[]
        }),
        home_key.as_deref(),
    );
    runtime.mark_command_pending_with_value(
        "session",
        "send_prompt",
        Some("mcp-first-private"),
        Some("比特币走势图现在怎么样"),
    );
    runtime.record_adapter_event_with_context(
        "session",
        "message_snapshot",
        json!({
            "type":"message_snapshot",
            "privateStreamObserved":true,
            "privateStreamRevision":9,
            "privateStreamState":"streaming",
            "streaming":true,
            "messages":[{
                "id":"private-stream:reply-current",
                "role":"assistant",
                "state":"streaming",
                "content":[{"type":"markdown","text":"这是本次私有流回答"}]
            }]
        }),
        home_key.as_deref(),
    );

    let state = runtime.snapshot("session").unwrap();
    assert!(state.context_ready);
    assert_eq!(
        state.diagnostics["lastEventKind"],
        "private_stream_pending_send_bound"
    );
    let semantic_event = state.semantic_event.unwrap();
    assert_eq!(semantic_event["streaming"], true);
    let messages = semantic_event["messages"].as_array().unwrap();
    assert_eq!(messages.len(), 2);
    assert_eq!(messages[0]["content"][0]["text"], "比特币走势图现在怎么样");
    assert_eq!(messages[1]["content"][0]["text"], "这是本次私有流回答");
    assert!(messages[0]["id"]
        .as_str()
        .unwrap()
        .starts_with("private-stream-bound:"));
    assert_eq!(messages[1]["id"], "private-stream:reply-current");
}

#[test]
fn private_stream_revision_must_advance_before_it_can_bind_a_missing_prompt() {
    let runtime = LocalAiBrowserRuntime::default();
    runtime.ensure_session("session", "chatgpt", "active");
    let url = Url::parse("https://chatgpt.com/c/current").unwrap();
    let key = semantic_context::page_context_key("chatgpt", url.as_str());
    runtime.mark_navigation("session", &url, true, None);
    runtime.mark_page_finished("session", &url);
    runtime.record_adapter_event_with_context(
        "session",
        "message_snapshot",
        json!({
            "type":"message_snapshot",
            "privateStreamObserved":true,
            "privateStreamRevision":11,
            "privateStreamState":"completed",
            "messages":[
                {"id":"old-user","role":"user","state":"completed","content":[{"type":"text","text":"旧问题"}]},
                {"id":"old-answer","role":"assistant","state":"completed","content":[{"type":"text","text":"旧回答"}]}
            ]
        }),
        key.as_deref(),
    );
    runtime.mark_command_pending_with_value(
        "session",
        "send_prompt",
        Some("mcp-current-private"),
        Some("新问题"),
    );
    runtime.record_adapter_event_with_context(
        "session",
        "message_snapshot",
        json!({
            "type":"message_snapshot",
            "privateStreamObserved":true,
            "privateStreamRevision":11,
            "privateStreamState":"streaming",
            "messages":[
                {"id":"old-user","role":"user","state":"completed","content":[{"type":"text","text":"旧问题"}]},
                {"id":"private-stream:old-revision","role":"assistant","state":"streaming","content":[{"type":"text","text":"不能绑定的旧修订"}]}
            ]
        }),
        key.as_deref(),
    );

    let state = runtime.snapshot("session").unwrap();
    assert!(!state.context_ready);
    assert_eq!(
        state.diagnostics["lastEventKind"],
        "pending_send_snapshot_ignored"
    );
    assert_eq!(
        state.semantic_event.unwrap()["messages"][1]["content"][0]["text"],
        "旧回答"
    );
}

#[test]
fn private_stream_binding_appends_after_old_dom_turn_instead_of_overwriting_it() {
    let runtime = LocalAiBrowserRuntime::default();
    runtime.ensure_session("session", "chatgpt", "active");
    let url = Url::parse("https://chatgpt.com/c/current").unwrap();
    let key = semantic_context::page_context_key("chatgpt", url.as_str());
    runtime.mark_navigation("session", &url, true, None);
    runtime.mark_page_finished("session", &url);
    runtime.record_adapter_event_with_context(
        "session",
        "message_snapshot",
        json!({
            "type":"message_snapshot",
            "privateStreamObserved":true,
            "privateStreamRevision":4,
            "privateStreamState":"completed",
            "observedMessageCount":2,
            "messageWindowStart":0,
            "messages":[
                {"id":"old-user","role":"user","state":"completed","content":[{"type":"text","text":"旧问题"}]},
                {"id":"old-answer","role":"assistant","state":"completed","content":[{"type":"text","text":"旧回答"}]}
            ]
        }),
        key.as_deref(),
    );
    runtime.mark_command_pending_with_value(
        "session",
        "send_prompt",
        Some("mcp-followup-private"),
        Some("新问题"),
    );
    runtime.record_adapter_event_with_context(
        "session",
        "message_snapshot",
        json!({
            "type":"message_snapshot",
            "privateStreamObserved":true,
            "privateStreamRevision":6,
            "privateStreamState":"streaming",
            "messages":[
                {"id":"old-user","role":"user","state":"completed","content":[{"type":"text","text":"旧问题"}]},
                {"id":"private-stream:followup","role":"assistant","state":"streaming","content":[{"type":"markdown","text":"新私有流回答"}]}
            ]
        }),
        key.as_deref(),
    );

    let state = runtime.snapshot("session").unwrap();
    assert!(state.context_ready);
    let semantic_event = state.semantic_event.unwrap();
    let messages = semantic_event["messages"].as_array().unwrap();
    assert_eq!(messages.len(), 4);
    assert_eq!(messages[1]["content"][0]["text"], "旧回答");
    assert_eq!(messages[2]["content"][0]["text"], "新问题");
    assert_eq!(messages[3]["content"][0]["text"], "新私有流回答");
}

#[test]
fn later_private_stream_frames_update_the_bound_answer_without_duplicating_the_turn() {
    let runtime = LocalAiBrowserRuntime::default();
    runtime.ensure_session("session", "chatgpt", "active");
    let url = Url::parse("https://chatgpt.com/").unwrap();
    let key = semantic_context::page_context_key("chatgpt", url.as_str());
    runtime.mark_navigation("session", &url, true, None);
    runtime.mark_page_finished("session", &url);
    runtime.record_adapter_event_with_context(
        "session",
        "message_snapshot",
        json!({
            "type":"message_snapshot",
            "privateStreamObserved":true,
            "privateStreamRevision":20,
            "privateStreamState":"idle",
            "messages":[]
        }),
        key.as_deref(),
    );
    runtime.mark_command_pending_with_value(
        "session",
        "send_prompt",
        Some("mcp-multiframe-private"),
        Some("连续流问题"),
    );
    for (revision, stream_state, text) in [
        (21, "streaming", "第一帧"),
        (22, "streaming", "第一帧和第二帧"),
        (23, "completed", "完整私有流回答"),
    ] {
        runtime.record_adapter_event_with_context(
            "session",
            "message_snapshot",
            json!({
                "type":"message_snapshot",
                "privateStreamObserved":true,
                "privateStreamRevision":revision,
                "privateStreamState":stream_state,
                "streaming":stream_state == "streaming",
                "messages":[{
                    "id":"private-stream:reply-multiframe",
                    "role":"assistant",
                    "state":stream_state,
                    "content":[{"type":"markdown","text":text}]
                }]
            }),
            key.as_deref(),
        );
    }

    let state = runtime.snapshot("session").unwrap();
    let semantic_event = state.semantic_event.unwrap();
    let messages = semantic_event["messages"].as_array().unwrap();
    assert_eq!(messages.len(), 2);
    assert_eq!(messages[0]["content"][0]["text"], "连续流问题");
    assert_eq!(messages[1]["id"], "private-stream:reply-multiframe");
    assert_eq!(messages[1]["state"], "completed");
    assert_eq!(messages[1]["content"][0]["text"], "完整私有流回答");
}
