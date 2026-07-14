use super::*;

#[test]
fn selected_message_topic_hint_removes_mentions() {
    let selected = SocialAiHistoryMessage {
        speaker: "用户".into(),
        content: " @EL 帮我看今天这张票 ".into(),
        from_request_user: true,
    };

    assert_eq!(
        selected_message_topic_hint(&selected).as_deref(),
        Some("帮我看今天这张票")
    );
}

#[test]
fn selected_message_source_uses_stable_shape() {
    let source = selected_message_citation_source("gmsg-1");

    assert_eq!(source["kind"], "selected_message");
    assert_eq!(source["id"], "gmsg-1");
    assert_eq!(source["label"], "被长按的群聊消息");
}

#[test]
fn selected_message_source_is_appended_when_model_omits_it() {
    let reply =
        ensure_selected_message_source("这句说法风险较高。\n来源：match_id EXT-1", "gmsg-1");

    assert!(reply.contains("来源：match_id EXT-1"));
    assert!(reply.contains("selected_message_id gmsg-1"));
}

#[test]
fn selected_message_source_is_not_duplicated() {
    let reply = ensure_selected_message_source(
        "这句说法风险较高。\n来源：selected_message_id gmsg-1",
        "gmsg-1",
    );

    assert_eq!(reply.matches("gmsg-1").count(), 1);
}

#[test]
fn selected_message_reply_replaces_stale_context_audit_id() {
    let context = json!({"context_audit_id": "current-audit-2"});
    let reply = ensure_current_context_audit_source(
        "这句说法风险较高。\n来源：match_id EXT-1，context_audit_id old-audit-1",
        Some(&context),
    );

    assert!(reply.contains("context_audit_id current-audit-2"));
    assert!(!reply.contains("old-audit-1"));
}

#[test]
fn selected_message_reply_appends_current_context_audit_id() {
    let context = json!({"context_audit_id": "current-audit-2"});
    let reply = ensure_current_context_audit_source(
        "这句说法风险较高。\n来源：match_id EXT-1",
        Some(&context),
    );

    assert!(reply.contains("来源：match_id EXT-1"));
    assert!(reply.contains("context_audit_id current-audit-2"));
}

#[test]
fn selected_message_reply_shape_adds_review_policy() {
    let context = json!({
        "app_id": "fb2",
        "answer_policy": {"schema": "fb2.answer_policy.v1"},
        "context_audit_id": "audit-selected-1"
    });
    let reply = ensure_selected_message_reply_shape(
        "这句说法需要谨慎。",
        "gmsg-selected-1",
        "西班牙让两球肯定赢盘、可以重注。",
        Some(&context),
    );

    assert!(reply.contains("数据事实："));
    assert!(reply.contains("AI推断："));
    assert!(reply.contains("风险边界："));
    assert!(reply.contains("selected_message_id gmsg-selected-1"));
    assert!(reply.contains("context_audit_id audit-selected-1"));
    assert!(reply.contains("肯定赢盘"));
    assert!(reply.contains("重注"));
    assert!(reply.contains("过于绝对") || reply.contains("不合理"));
}

#[test]
fn selected_message_error_fallback_is_policy_shaped() {
    let selected = SocialAiHistoryMessage {
        speaker: "用户A".into(),
        content: "西班牙让两球肯定赢盘、可以重注。".into(),
        from_request_user: false,
    };
    let context = json!({
        "app_id": "fb2",
        "answer_policy": {"schema": "fb2.answer_policy.v1"},
        "context_audit_id": "audit-selected-2"
    });

    let reply = selected_message_generation_fallback(&selected, "gmsg-selected-2", Some(&context));

    assert!(reply.contains("数据事实："));
    assert!(reply.contains("AI推断："));
    assert!(reply.contains("风险边界："));
    assert!(reply.contains("selected_message_id gmsg-selected-2"));
    assert!(reply.contains("context_audit_id audit-selected-2"));
    assert!(reply.contains("肯定赢盘"));
    assert!(reply.contains("重注"));
    assert!(reply.contains("不合理"));
}
