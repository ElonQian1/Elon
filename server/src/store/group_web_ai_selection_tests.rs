use super::super::selection::GroupAiSelection;
use super::*;

fn selection(ids: &[String]) -> GroupAiSelection {
    GroupAiSelection {
        message_ids: ids.to_vec(),
        message_revisions: ids.iter().map(|id| (id.clone(), 1)).collect(),
        question: "compare".into(),
        allow_continue: false,
        attachment_transport_version: 1,
    }
}

#[test]
fn selected_original_files_require_transport_support_and_keep_rich_text() {
    let f = Fixture::new();
    let files: Vec<crate::project_ws_protocol::ProjectAttachmentRef> = serde_json::from_value(serde_json::json!([{
        "attachment_id":"synthetic", "display_name":"chart.png", "mime_type":"image/png", "size_bytes":123,
        "url":"https://platform.example/api/user/fixture/chat-attachments/group/chart.png"
    }])).unwrap();
    let content = "> quoted text\n\n```rust\nlet n = 1;\n```\n\n| A | B |\n|---|---|\n| 1 | 2 |";
    let message = f
        .store
        .send_friend_group_message(&f.user, &f.group, content, Some(&files))
        .unwrap();
    let mut input = selection(&[message.id.clone()]);
    input.attachment_transport_version = 0;
    let operation = Uuid::new_v4().to_string();
    assert!(f
        .store
        .prepare_group_ai_selection(&f.user, &f.group, &message.id, &operation, &input)
        .is_err());
    input.attachment_transport_version = 1;
    let request = f
        .store
        .prepare_group_ai_selection(&f.user, &f.group, &message.id, &operation, &input)
        .unwrap();
    assert_eq!(request.attachments.len(), 1);
    assert_eq!(request.attachments[0].message_id, message.id);
    assert!(request.prompt.contains("group_01_chart.png"));
    assert!(request.prompt.contains("```rust"));
    assert!(request.prompt.contains("| 1 | 2 |"));
    assert!(!request.prompt.contains("platform.example"));
}

#[test]
fn selection_is_ordered_exclusive_and_idempotent() {
    let f = Fixture::new();
    f.store
        .send_friend_group_message(&f.user, &f.group, "EXCLUDED", None)
        .unwrap();
    let last = f
        .store
        .send_friend_group_message(&f.other, &f.group, "SELECTED_LAST", None)
        .unwrap()
        .id;
    let op = Uuid::new_v4().to_string();
    let mut input = selection(&[last.clone(), f.trigger.clone()]);
    let a = f
        .store
        .prepare_group_ai_selection(&f.user, &f.group, &last, &op, &input)
        .unwrap();
    assert!(!a.prompt.contains("EXCLUDED"));
    assert!(a.prompt.find("@EL explain").unwrap() < a.prompt.find("SELECTED_LAST").unwrap());
    input.message_ids.reverse();
    let again = f
        .store
        .prepare_group_ai_selection(&f.user, &f.group, &last, &op, &input)
        .unwrap();
    assert_eq!(a.id, again.id);
    input.question = "different question".into();
    assert!(f
        .store
        .prepare_group_ai_selection(&f.user, &f.group, &last, &op, &input)
        .is_err());
    let other = f
        .store
        .prepare_group_ai_selection(
            &f.other,
            &f.group,
            &last,
            &Uuid::new_v4().to_string(),
            &input,
        )
        .unwrap();
    assert_ne!(a.id, other.id);
    assert!(
        f.store
            .group_web_ai_action(&f.user, &f.group, &a.id, &op, "dispatch")
            .unwrap()
            .dispatch_permit
    );
    assert!(
        !f.store
            .group_web_ai_action(&f.user, &f.group, &a.id, &op, "dispatch")
            .unwrap()
            .dispatch_permit
    );
    let result = f
        .store
        .complete_group_ai_reply(&f.user, &a.id, "answer")
        .unwrap();
    assert_eq!(
        result.id,
        f.store
            .complete_group_ai_reply(&f.user, &a.id, "answer")
            .unwrap()
            .id
    );
    assert_eq!(f.count(), 1);
}

#[test]
fn selection_rejects_invalid_sources_and_never_silently_truncates() {
    let f = Fixture::new();
    let op = Uuid::new_v4().to_string();
    let prepare = |input: &GroupAiSelection| {
        f.store
            .prepare_group_ai_selection(&f.user, &f.group, &f.trigger, &op, input)
    };
    assert!(prepare(&selection(&[])).is_err());
    assert!(prepare(&selection(&[f.trigger.clone(), f.trigger.clone()])).is_err());
    assert!(prepare(&selection(&[f.trigger.clone(), "not-in-group".into()])).is_err());
    let conn = f.store.conn().unwrap();
    conn.execute(
        "UPDATE friend_group_messages SET content=?1 WHERE id=?2",
        params!["x".repeat(21_000), f.trigger],
    )
    .unwrap();
    assert!(prepare(&selection(&[f.trigger.clone()])).is_err());
    assert_eq!(
        conn.query_row("SELECT count(*) FROM group_ai_reply_requests", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        0
    );
}

#[test]
fn selection_rechecks_every_revision_before_dispatch_and_publish() {
    let f = Fixture::new();
    let second = f
        .store
        .send_friend_group_message(&f.other, &f.group, "other", None)
        .unwrap()
        .id;
    let op = Uuid::new_v4().to_string();
    let req = f
        .store
        .prepare_group_ai_selection(
            &f.user,
            &f.group,
            &f.trigger,
            &op,
            &selection(&[f.trigger.clone(), second.clone()]),
        )
        .unwrap();
    assert!(f
        .store
        .group_web_ai_action(&f.user, &f.group, &req.id, &op, "fallback")
        .is_err());
    f.store
        .group_web_ai_action(&f.user, &f.group, &req.id, &op, "dispatch")
        .unwrap();
    let conn = f.store.conn().unwrap();
    conn.execute(
        "UPDATE friend_group_messages SET revision=revision+1 WHERE id=?1",
        [&second],
    )
    .unwrap();
    assert!(f
        .store
        .complete_group_ai_reply(&f.user, &req.id, "must not publish")
        .is_err());
    assert!(f
        .store
        .group_web_ai_action(&f.user, &f.group, &req.id, &op, "status")
        .is_err());
    assert_eq!(f.count(), 0);
}

#[test]
fn selection_does_not_consume_legacy_recent_owner() {
    let f = Fixture::new();
    f.store
        .prepare_group_ai_selection(
            &f.user,
            &f.group,
            &f.trigger,
            &Uuid::new_v4().to_string(),
            &selection(&[f.trigger.clone()]),
        )
        .unwrap();
    assert!(f
        .store
        .claim_group_ai_reply(&f.user, &f.group, &f.trigger)
        .unwrap()
        .is_some());
    assert!(f
        .store
        .claim_group_ai_reply(&f.user, &f.group, &f.trigger)
        .unwrap()
        .is_none());
}
