use super::super::selection::GroupAiSelection;
use super::*;
use crate::store::articles::chat_records::{Document, SCHEMA};
use serde_json::json;

fn setup(f: &Fixture, asset: bool) -> (String, String) {
    let asset_id = if asset {
        Some(
            f.store
                .upload_chat_record_asset(&f.other, &f.group, b"%PDF-1.4 synthetic test")
                .unwrap()
                .asset_id,
        )
    } else {
        None
    };
    let document: Document = serde_json::from_value(json!({"schema":SCHEMA,"source":"wechat","title":"Synthetic export",
        "raw_text":"export original","messages":[
            {"id":"parent","sender":"Alice","time":"yesterday","kind":"forward","text":"Nested discussion"},
            {"id":"child","parent_id":"parent","sender":"Bob","time":"10:00","kind":"text","text":"FULL_CONTEXT_BEYOND_PREVIEW\n```rust\nlet x = 1;\n```"},
            {"id":"file","sender":"Bob","time":"10:01","kind":"file","text":"report","filename":"report.pdf","asset_id":asset_id},
            {"id":"video","sender":"Alice","time":"10:02","kind":"video","text":"clip","filename":"video.mp4"}
        ]})).unwrap();
    let record = f
        .store
        .create_chat_record(&f.other, &f.group, "record-operation", document)
        .unwrap();
    (record.message.id, record.card.record_id)
}
fn input(id: &str) -> GroupAiSelection {
    GroupAiSelection {
        message_ids: vec![id.into()],
        message_revisions: [(id.into(), 1)].into(),
        question: "Summarize this record".into(),
        allow_continue: false,
        attachment_transport_version: 1,
    }
}
#[test]
fn nested_record_uses_full_context_and_exact_protected_manifest() {
    let f = Fixture::new();
    let (id, record) = setup(&f, true);
    let operation = Uuid::new_v4().to_string();
    let r = f
        .store
        .prepare_group_ai_selection(&f.user, &f.group, &id, &operation, &input(&id))
        .unwrap();
    assert!(r.prompt.contains("FULL_CONTEXT_BEYOND_PREVIEW"));
    assert!(r.prompt.contains("```rust"));
    assert!(r.prompt.contains("parent_id"));
    assert!(r.prompt.contains("not_exported"));
    assert!(!r.prompt.contains("@EL explain"));
    assert!(!r.prompt.contains("/api/"));
    assert_eq!(r.attachments.len(), 1);
    assert_eq!(r.attachments[0].message_id, id);
    assert!(r.attachments[0]
        .download_path
        .contains(&format!("/chat-records/{record}/assets/")));
    assert!(r.attachments[0].sha256.is_some());
    assert!(r.prompt.contains(&r.attachments[0].name));
    assert_eq!(
        r.id,
        f.store
            .prepare_group_ai_selection(&f.user, &f.group, &id, &operation, &input(&id))
            .unwrap()
            .id
    );
    assert!(
        f.store
            .group_web_ai_action(&f.user, &f.group, &r.id, &operation, "dispatch")
            .unwrap()
            .dispatch_permit
    );
    let result = f
        .store
        .complete_group_ai_reply(&f.user, &r.id, "Synthetic record analysis")
        .unwrap();
    assert_eq!(
        result.id,
        f.store
            .complete_group_ai_reply(&f.user, &r.id, "Synthetic record analysis")
            .unwrap()
            .id
    );
    assert_eq!(f.count(), 1);
    let draft = f
        .store
        .group_ai_context_share_draft(&f.user, &f.group, &result.id)
        .unwrap();
    let context = draft["document"]["messages"][0]["content"]
        .as_str()
        .unwrap();
    assert!(context.contains("FULL_CONTEXT_BEYOND_PREVIEW"));
    assert!(context.contains("本次继续讨论不自动上传附件原文件"));
}
#[test]
fn revoked_record_blocks_dispatch_without_replaying_or_using_cached_content() {
    let f = Fixture::new();
    let (id, record) = setup(&f, true);
    let operation = Uuid::new_v4().to_string();
    let r = f
        .store
        .prepare_group_ai_selection(&f.user, &f.group, &id, &operation, &input(&id))
        .unwrap();
    f.store
        .revoke_chat_record(&f.other, &f.group, &record)
        .unwrap();
    assert!(f
        .store
        .group_web_ai_action(&f.user, &f.group, &r.id, &operation, "dispatch")
        .is_err());
    assert!(f
        .store
        .prepare_group_ai_selection(
            &f.user,
            &f.group,
            &id,
            &Uuid::new_v4().to_string(),
            &input(&id)
        )
        .is_err());
    assert_eq!(f.count(), 0);
}
#[test]
fn old_client_cannot_silently_omit_record_files_and_membership_is_required() {
    let f = Fixture::new();
    let (id, _) = setup(&f, true);
    let mut selection = input(&id);
    selection.attachment_transport_version = 0;
    assert!(f
        .store
        .prepare_group_ai_selection(
            &f.user,
            &f.group,
            &id,
            &Uuid::new_v4().to_string(),
            &selection
        )
        .is_err());
    selection.attachment_transport_version = 1;
    f.store
        .conn()
        .unwrap()
        .execute(
            "DELETE FROM friend_group_members WHERE user_id=?1 AND group_id=?2",
            params![f.user, f.group],
        )
        .unwrap();
    assert!(f
        .store
        .prepare_group_ai_selection(
            &f.user,
            &f.group,
            &id,
            &Uuid::new_v4().to_string(),
            &selection
        )
        .is_err());
}
#[test]
fn forged_card_is_not_accepted_as_plain_text_or_an_arbitrary_record_lookup() {
    let f = Fixture::new();
    let (_, record) = setup(&f, false);
    f.store
        .conn()
        .unwrap()
        .execute(
            "UPDATE friend_group_messages SET content=?1 WHERE id=?2",
            params![
                format!("【一龙聊天记录】\n{{\"record_id\":\"{record}\"}}"),
                f.trigger
            ],
        )
        .unwrap();
    assert!(f
        .store
        .prepare_group_ai_selection(
            &f.user,
            &f.group,
            &f.trigger,
            &Uuid::new_v4().to_string(),
            &input(&f.trigger)
        )
        .is_err());
}
#[test]
fn long_records_fail_explicitly_without_silent_truncation() {
    let f = Fixture::new();
    let document: Document=serde_json::from_value(json!({"schema":SCHEMA,"source":"wechat","title":"Long export",
        "raw_text":"original","messages":[{"id":"m","sender":"A","time":"now","kind":"text","text":"long".repeat(6000)}]})).unwrap();
    let id = f
        .store
        .create_chat_record(&f.user, &f.group, "long-record-operation", document)
        .unwrap()
        .message
        .id;
    let error = f
        .store
        .prepare_group_ai_selection(
            &f.user,
            &f.group,
            &id,
            &Uuid::new_v4().to_string(),
            &input(&id),
        )
        .unwrap_err();
    assert!(error.to_string().contains("没有截断"));
    assert_eq!(f.count(), 0);
}
