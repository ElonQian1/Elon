use super::*;
use crate::project_ws_protocol::{ProjectAttachmentAnnotation, ProjectAttachmentRef};
use crate::store::Store;
use uuid::Uuid;

fn temp_store() -> Store {
    let path = std::env::temp_dir().join(format!(
        "elon_social_ai_store_test_{}.db",
        Uuid::new_v4().simple()
    ));
    Store::open(&path).expect("store should open")
}

fn image_attachment(name: &str) -> ProjectAttachmentRef {
    ProjectAttachmentRef {
        attachment_id: Some(format!("att_{}", name)),
        kind: Some("image".to_string()),
        display_name: Some(name.to_string()),
        file_name: Some(name.to_string()),
        mime_type: Some("image/jpeg".to_string()),
        path: Some(format!("/tmp/{}", name)),
        url: Some(format!("http://example.test/{}", name)),
        sha256: None,
        size_bytes: Some(2048),
        image_width: Some(1080),
        image_height: Some(720),
        duration_seconds: None,
        transcription: None,
        annotations: Vec::new(),
    }
}

fn voice_attachment(seconds: u32) -> ProjectAttachmentRef {
    ProjectAttachmentRef {
        attachment_id: Some("att_voice".to_string()),
        kind: Some("voice".to_string()),
        display_name: Some("voice.m4a".to_string()),
        file_name: Some("voice.m4a".to_string()),
        mime_type: Some("audio/mp4".to_string()),
        path: Some("/tmp/voice.m4a".to_string()),
        url: Some("http://example.test/voice.m4a".to_string()),
        sha256: None,
        size_bytes: Some(4096),
        image_width: None,
        image_height: None,
        duration_seconds: Some(seconds),
        transcription: None,
        annotations: Vec::new(),
    }
}

#[test]
fn social_sidebar_peek_preserves_unread_and_tracks_latest_received_message() {
    let store = temp_store();
    let alice = store
        .create_user(
            "sidebar-received-alice@example.com",
            "secret1",
            Some("Alice"),
            None,
        )
        .expect("alice should be created");
    let bob = store
        .create_user(
            "sidebar-received-bob@example.com",
            "secret1",
            Some("Bob"),
            None,
        )
        .expect("bob should be created");
    store
        .add_friend(&alice.id, Some("email"), "sidebar-received-bob@example.com")
        .expect("alice can add bob");

    store
        .send_friend_message(&alice.id, &bob.id, "incoming friend", None)
        .expect("incoming friend message should be stored");
    store
        .send_friend_message(&bob.id, &alice.id, "newer outgoing friend", None)
        .expect("outgoing friend message should be stored");
    let bob_friend_after_outgoing = store
        .list_friends(&bob.id)
        .expect("friends should load")
        .into_iter()
        .find(|friend| friend.id == alice.id)
        .expect("alice should be listed");
    assert_eq!(
        bob_friend_after_outgoing.last_received_message.as_deref(),
        Some("incoming friend")
    );
    assert!(bob_friend_after_outgoing.last_received_at.is_some());
    store
        .send_friend_message(&alice.id, &bob.id, "current incoming friend", None)
        .expect("current incoming friend message should be stored");
    let bob_friend = store
        .list_friends(&bob.id)
        .expect("friends should load with current incoming")
        .into_iter()
        .find(|friend| friend.id == alice.id)
        .expect("alice should be listed with current incoming");
    assert_eq!(
        bob_friend.last_received_message.as_deref(),
        Some("current incoming friend")
    );
    let friend_unread_before_peek = bob_friend.unread_count;
    assert!(friend_unread_before_peek > 0);

    store
        .peek_friend_messages(&bob.id, &alice.id, None, 20)
        .expect("sidebar can peek friend messages");
    let unread_after_peek = store
        .list_friends(&bob.id)
        .expect("friends should load after peek")
        .into_iter()
        .find(|friend| friend.id == alice.id)
        .expect("alice should still be listed")
        .unread_count;
    assert_eq!(unread_after_peek, friend_unread_before_peek);

    let group = store
        .create_friend_group(&alice.id, Some("Sidebar group"), &[bob.id.clone()])
        .expect("group should be created");
    store
        .send_friend_group_message(&alice.id, &group.id, "incoming group", None)
        .expect("incoming group message should be stored");
    store
        .send_friend_group_message(&bob.id, &group.id, "newer outgoing group", None)
        .expect("outgoing group message should be stored");
    let bob_group_after_outgoing = store
        .list_friend_groups(&bob.id)
        .expect("groups should load")
        .into_iter()
        .find(|candidate| candidate.id == group.id)
        .expect("group should be listed");
    assert_eq!(
        bob_group_after_outgoing.last_received_message.as_deref(),
        Some("incoming group")
    );
    assert!(bob_group_after_outgoing.last_received_at.is_some());
    store
        .send_friend_group_message(&alice.id, &group.id, "current incoming group", None)
        .expect("current incoming group message should be stored");
    let bob_group = store
        .list_friend_groups(&bob.id)
        .expect("groups should load with current incoming")
        .into_iter()
        .find(|candidate| candidate.id == group.id)
        .expect("group should be listed with current incoming");
    assert_eq!(
        bob_group.last_received_message.as_deref(),
        Some("current incoming group")
    );
    let group_unread_before_peek = bob_group.unread_count;
    assert!(group_unread_before_peek > 0);

    store
        .peek_friend_group_messages(&bob.id, &group.id, None, 20)
        .expect("sidebar can peek group messages");
    let group_unread_after_peek = store
        .list_friend_groups(&bob.id)
        .expect("groups should load after peek")
        .into_iter()
        .find(|candidate| candidate.id == group.id)
        .expect("group should still be listed")
        .unread_count;
    assert_eq!(group_unread_after_peek, group_unread_before_peek);
}

#[test]
fn friend_image_annotations_are_visible_to_recipient() {
    let store = temp_store();
    let alice = store
        .create_user(
            "alice-annotation@example.com",
            "secret1",
            Some("Alice"),
            None,
        )
        .expect("alice should be created");
    let bob = store
        .create_user("bob-annotation@example.com", "secret1", Some("Bob"), None)
        .expect("bob should be created");
    store
        .add_friend(&alice.id, Some("email"), "bob-annotation@example.com")
        .expect("alice can add bob");

    let attachments = vec![ProjectAttachmentRef {
        attachment_id: Some("att_annotated".to_string()),
        kind: Some("image".to_string()),
        display_name: Some("marked.jpg".to_string()),
        file_name: Some("marked.jpg".to_string()),
        mime_type: Some("image/jpeg".to_string()),
        path: Some("/tmp/marked.jpg".to_string()),
        url: Some("http://example.test/marked.jpg".to_string()),
        sha256: None,
        size_bytes: Some(2048),
        image_width: Some(1080),
        image_height: Some(720),
        duration_seconds: None,
        transcription: None,
        annotations: vec![ProjectAttachmentAnnotation {
            x: 0.1,
            y: 0.2,
            width: 0.3,
            height: 0.4,
            note: "tap this note".to_string(),
            icon_x: Some(0.41),
            icon_y: Some(0.58),
            icon_width: Some(0.06),
            icon_height: Some(0.08),
        }],
    }];

    let sent = store
        .send_friend_message(&alice.id, &bob.id, "", Some(&attachments))
        .expect("message with annotated image should be stored");
    assert_eq!(sent.attachments[0].annotations[0].note, "tap this note");

    let bob_messages = store
        .list_friend_messages(&bob.id, &alice.id, None, 20)
        .expect("recipient can list messages");
    let received = bob_messages
        .last()
        .expect("recipient should see annotated image message");
    let annotation = received.attachments[0]
        .annotations
        .first()
        .expect("annotation should be preserved for recipient");
    assert_eq!(annotation.note, "tap this note");
    assert_eq!(annotation.icon_x, Some(0.41));
    assert!(!received.outgoing);

    let bob_friends = store.list_friends(&bob.id).expect("friends should load");
    let alice_profile = bob_friends
        .iter()
        .find(|friend| friend.id == alice.id)
        .expect("alice should be listed");
    assert_eq!(alice_profile.last_message.as_deref(), Some("【图片】"));
}

#[test]
fn annotated_images_are_included_in_social_ai_history() {
    let store = temp_store();
    let alice = store
        .create_user(
            "social-ai-annotated-alice@example.com",
            "secret1",
            Some("Alice"),
            None,
        )
        .expect("alice should be created");

    let mut attachment = image_attachment("marked.jpg");
    attachment.annotations = vec![ProjectAttachmentAnnotation {
        x: 0.12,
        y: 0.24,
        width: 0.36,
        height: 0.48,
        note: "read this marked area".to_string(),
        icon_x: Some(0.55),
        icon_y: Some(0.66),
        icon_width: Some(0.07),
        icon_height: Some(0.08),
    }];

    store
        .send_friend_message(&alice.id, SOCIAL_AI_USER_ID, "", Some(&[attachment]))
        .expect("direct AI image-only message should be stored");

    let history = store
        .list_recent_friend_messages_for_social_ai(&alice.id, SOCIAL_AI_USER_ID, 10)
        .expect("social AI history should load");
    let content = &history
        .last()
        .expect("history should contain the image message")
        .content;

    assert!(content.contains("Attached media context for AI"));
    assert!(content.contains("image_annotation #1"));
    assert!(content.contains("read this marked area"));
    assert!(content.contains("region x=0.120"));
}

#[test]
fn media_messages_have_chat_list_previews() {
    let store = temp_store();
    let alice = store
        .create_user(
            "alice-media-preview@example.com",
            "secret1",
            Some("Alice"),
            None,
        )
        .expect("alice should be created");
    let bob = store
        .create_user(
            "bob-media-preview@example.com",
            "secret1",
            Some("Bob"),
            None,
        )
        .expect("bob should be created");
    store
        .add_friend(&alice.id, Some("email"), "bob-media-preview@example.com")
        .expect("alice can add bob");

    let voice = vec![voice_attachment(7)];
    store
        .send_friend_message(&alice.id, &bob.id, "", Some(&voice))
        .expect("voice message should be stored");
    let bob_friends = store.list_friends(&bob.id).expect("friends should load");
    let alice_profile = bob_friends
        .iter()
        .find(|friend| friend.id == alice.id)
        .expect("alice should be listed");
    assert_eq!(alice_profile.last_message.as_deref(), Some("【语音】7秒"));

    let group = store
        .create_friend_group(&alice.id, Some("Media preview"), &[bob.id.clone()])
        .expect("group should be created");
    let group_image = vec![image_attachment("group.jpg")];
    store
        .send_friend_group_message(&alice.id, &group.id, "", Some(&group_image))
        .expect("group image message should be stored");
    let bob_groups = store
        .list_friend_groups(&bob.id)
        .expect("groups should load");
    let listed_group = bob_groups
        .iter()
        .find(|candidate| candidate.id == group.id)
        .expect("group should be listed");
    assert_eq!(listed_group.last_message.as_deref(), Some("【图片】"));
}

#[test]
fn friend_ai_reply_is_visible_to_both_sides_in_context() {
    let store = temp_store();
    let alice = store
        .create_user("alice@example.com", "secret1", Some("Alice"), None)
        .expect("alice should be created");
    let bob = store
        .create_user("bob@example.com", "secret1", Some("Bob"), None)
        .expect("bob should be created");
    store
        .add_friend(&alice.id, Some("email"), "bob@example.com")
        .expect("alice can add bob");

    store
        .send_friend_message(&alice.id, &bob.id, "@EL 这句话是什么意思？", None)
        .expect("trigger message should be stored");
    let inserted = store
        .insert_friend_social_ai_reply(&alice.id, &bob.id, "这是一个解释。")
        .expect("ai reply should be inserted");
    assert_eq!(inserted.len(), 2);

    let alice_messages = store
        .list_friend_messages(&alice.id, &bob.id, None, 20)
        .expect("alice can list messages");
    let alice_ai = alice_messages.last().expect("alice sees ai reply");
    assert_eq!(alice_ai.sender_user_id, SOCIAL_AI_USER_ID);
    assert_eq!(alice_ai.context_user_id.as_deref(), Some(bob.id.as_str()));
    assert_eq!(
        alice_ai.sender_name.as_deref(),
        Some(SOCIAL_AI_DISPLAY_NAME)
    );

    let bob_messages = store
        .list_friend_messages(&bob.id, &alice.id, None, 20)
        .expect("bob can list messages");
    let bob_ai = bob_messages.last().expect("bob sees ai reply");
    assert_eq!(bob_ai.sender_user_id, SOCIAL_AI_USER_ID);
    assert_eq!(bob_ai.context_user_id.as_deref(), Some(alice.id.as_str()));
}

#[test]
fn direct_social_ai_friend_is_listed_and_keeps_private_history() {
    let store = temp_store();
    let alice = store
        .create_user(
            "direct-social-ai-alice@example.com",
            "secret1",
            Some("Alice"),
            None,
        )
        .expect("alice should be created");

    let friends = store.list_friends(&alice.id).expect("friends should load");
    let ai_friend = friends
        .iter()
        .find(|friend| friend.id == SOCIAL_AI_USER_ID)
        .expect("direct social AI friend should be listed");
    assert_eq!(ai_friend.nickname.as_deref(), Some(SOCIAL_AI_FRIEND_NAME));
    assert_eq!(
        ai_friend.last_message.as_deref(),
        Some(SOCIAL_AI_FRIEND_PREVIEW)
    );

    store
        .send_friend_message(&alice.id, SOCIAL_AI_USER_ID, "我有一个隐私问题", None)
        .expect("user can send direct message to social AI");
    store
        .insert_direct_social_ai_reply(&alice.id, "可以单独说。")
        .expect("direct social AI reply should be stored");

    let bob = store
        .create_user(
            "direct-social-ai-bob@example.com",
            "secret1",
            Some("Bob"),
            None,
        )
        .expect("bob should be created");
    store
        .add_friend(&alice.id, Some("email"), "direct-social-ai-bob@example.com")
        .expect("alice can add bob");
    store
        .send_friend_message(&alice.id, &bob.id, "@EL 这句普通好友消息怎么理解？", None)
        .expect("context trigger should be stored");
    store
        .insert_friend_social_ai_reply(&alice.id, &bob.id, "普通好友上下文回复")
        .expect("context AI reply should be stored");

    let direct_messages = store
        .list_friend_messages(&alice.id, SOCIAL_AI_USER_ID, None, 20)
        .expect("direct social AI messages should load");
    let direct_contents = direct_messages
        .iter()
        .map(|message| message.content.as_str())
        .collect::<Vec<_>>();
    assert_eq!(direct_messages.len(), 2);
    assert!(direct_contents.contains(&"我有一个隐私问题"));
    assert!(direct_contents.contains(&"可以单独说。"));
    assert!(!direct_contents.contains(&"普通好友上下文回复"));
    assert!(direct_messages
        .iter()
        .all(|message| message.context_user_id.is_none()));
}

#[test]
fn group_ai_reply_uses_el_sender() {
    let store = temp_store();
    let alice = store
        .create_user("group-alice@example.com", "secret1", Some("Alice"), None)
        .expect("alice should be created");
    let bob = store
        .create_user("group-bob@example.com", "secret1", Some("Bob"), None)
        .expect("bob should be created");
    store
        .add_friend(&alice.id, Some("email"), "group-bob@example.com")
        .expect("alice can add bob");
    let group = store
        .create_friend_group(&alice.id, Some("测试群"), &[bob.id.clone()])
        .expect("group should be created");

    store
        .insert_group_social_ai_reply(&group.id, "群聊里的 EL 回复。")
        .expect("ai group reply should be inserted");
    let messages = store
        .list_friend_group_messages(&alice.id, &group.id, None, 20)
        .expect("group messages should load");
    let ai = messages.last().expect("ai reply should be visible");
    assert_eq!(ai.sender_user_id, SOCIAL_AI_USER_ID);
    assert_eq!(ai.sender_name, SOCIAL_AI_DISPLAY_NAME);
    assert!(!ai.outgoing);
}
