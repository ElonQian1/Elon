use anyhow::{anyhow, Result};
use rusqlite::{params, Connection, OptionalExtension};

use super::{new_id, now, FriendChatMessage, FriendGroupMessage, Store};

pub(crate) const SOCIAL_AI_USER_ID: &str = "usr_elon_ai";
pub(crate) const SOCIAL_AI_DISPLAY_NAME: &str = "EL";
pub(crate) const SOCIAL_AI_FRIEND_NAME: &str = "一龙AI";
pub(crate) const SOCIAL_AI_FRIEND_ACCOUNT: &str = "ai-agent";
pub(crate) const SOCIAL_AI_FRIEND_PREVIEW: &str = "单独问一龙AI，适合隐私问题和日常解答";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SocialAiHistoryMessage {
    pub speaker: String,
    pub content: String,
    pub from_request_user: bool,
}

impl Store {
    pub(crate) fn list_recent_friend_messages_for_social_ai(
        &self,
        user_id: &str,
        friend_id: &str,
        limit: i64,
    ) -> Result<Vec<SocialAiHistoryMessage>> {
        let conn = self.conn()?;
        if friend_id == SOCIAL_AI_USER_ID {
            ensure_direct_social_ai_pair(&conn, user_id)?;
            return list_recent_direct_social_ai_messages(&conn, user_id, limit);
        }
        ensure_friend_pair_for_social_ai(&conn, user_id, friend_id)?;
        let mut stmt = conn.prepare(
            "SELECT m.sender_user_id,
                    COALESCE(u.nickname, u.email, u.phone, m.sender_user_id) AS sender_name,
                    m.content
             FROM friend_messages m
             LEFT JOIN users u ON u.id = m.sender_user_id
             WHERE (
                 (m.sender_user_id = ?1 AND m.receiver_user_id = ?2)
                 OR (m.sender_user_id = ?2 AND m.receiver_user_id = ?1)
                 OR (
                     m.sender_user_id = ?3
                     AND m.receiver_user_id = ?1
                     AND m.context_user_id = ?2
                 )
             )
             ORDER BY m.created_at DESC
             LIMIT ?4",
        )?;
        let mut messages = stmt
            .query_map(
                params![user_id, friend_id, SOCIAL_AI_USER_ID, limit.clamp(1, 30)],
                |row| {
                    let sender_user_id: String = row.get(0)?;
                    let sender_name: String = row.get(1)?;
                    Ok(history_message_from_row(
                        user_id,
                        &sender_user_id,
                        &sender_name,
                        row.get::<_, String>(2)?,
                    ))
                },
            )?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        messages.reverse();
        Ok(messages)
    }

    pub fn insert_direct_social_ai_reply(
        &self,
        user_id: &str,
        content: &str,
    ) -> Result<FriendChatMessage> {
        let content = normalize_reply_content(content)?;
        let conn = self.conn()?;
        ensure_direct_social_ai_pair(&conn, user_id)?;

        let id = new_id("fai");
        let created_at = now();
        conn.execute(
            "INSERT INTO friend_messages (
                id, sender_user_id, receiver_user_id, context_user_id, content, created_at
             )
             VALUES (?1, ?2, ?3, NULL, ?4, ?5)",
            params![id, SOCIAL_AI_USER_ID, user_id, content, created_at],
        )?;

        Ok(FriendChatMessage {
            id,
            sender_user_id: SOCIAL_AI_USER_ID.to_string(),
            receiver_user_id: user_id.to_string(),
            sender_name: Some(SOCIAL_AI_DISPLAY_NAME.to_string()),
            content,
            attachments: Vec::new(),
            created_at,
            context_user_id: None,
            outgoing: false,
        })
    }

    pub(crate) fn list_recent_group_messages_for_social_ai(
        &self,
        user_id: &str,
        group_id: &str,
        limit: i64,
    ) -> Result<Vec<SocialAiHistoryMessage>> {
        let conn = self.conn()?;
        ensure_group_member_for_social_ai(&conn, user_id, group_id)?;
        let mut stmt = conn.prepare(
            "SELECT m.sender_user_id,
                    COALESCE(u.nickname, u.email, u.phone, m.sender_user_id) AS sender_name,
                    m.content
             FROM friend_group_messages m
             LEFT JOIN users u ON u.id = m.sender_user_id
             WHERE m.group_id = ?1
             ORDER BY m.created_at DESC
             LIMIT ?2",
        )?;
        let mut messages = stmt
            .query_map(params![group_id, limit.clamp(1, 60)], |row| {
                let sender_user_id: String = row.get(0)?;
                let sender_name: String = row.get(1)?;
                Ok(history_message_from_row(
                    user_id,
                    &sender_user_id,
                    &sender_name,
                    row.get::<_, String>(2)?,
                ))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        messages.reverse();
        Ok(messages)
    }

    pub fn insert_friend_social_ai_reply(
        &self,
        user_id: &str,
        friend_id: &str,
        content: &str,
    ) -> Result<Vec<FriendChatMessage>> {
        let content = normalize_reply_content(content)?;
        let conn = self.conn()?;
        ensure_friend_pair_for_social_ai(&conn, user_id, friend_id)?;
        ensure_social_ai_user(&conn)?;

        let created_at = now();
        let user_message_id = new_id("fai");
        let friend_message_id = new_id("fai");
        let tx = conn.unchecked_transaction()?;
        tx.execute(
            "INSERT INTO friend_messages (
                id, sender_user_id, receiver_user_id, context_user_id, content, created_at
             )
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                user_message_id,
                SOCIAL_AI_USER_ID,
                user_id,
                friend_id,
                content,
                created_at
            ],
        )?;
        tx.execute(
            "INSERT INTO friend_messages (
                id, sender_user_id, receiver_user_id, context_user_id, content, created_at
             )
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                friend_message_id,
                SOCIAL_AI_USER_ID,
                friend_id,
                user_id,
                content,
                created_at
            ],
        )?;
        tx.commit()?;

        Ok(vec![
            social_ai_friend_message(user_message_id, user_id, friend_id, &content, &created_at),
            social_ai_friend_message(friend_message_id, friend_id, user_id, &content, &created_at),
        ])
    }

    pub fn insert_group_social_ai_reply(
        &self,
        group_id: &str,
        content: &str,
    ) -> Result<FriendGroupMessage> {
        let content = normalize_reply_content(content)?;
        let conn = self.conn()?;
        ensure_group_exists(&conn, group_id)?;
        ensure_social_ai_user(&conn)?;

        let id = new_id("gai");
        let created_at = now();
        conn.execute(
            "INSERT INTO friend_group_messages (
                id, group_id, sender_user_id, content, created_at
             )
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![id, group_id, SOCIAL_AI_USER_ID, content, created_at],
        )?;
        conn.execute(
            "UPDATE friend_groups SET updated_at = ?1 WHERE id = ?2",
            params![created_at, group_id],
        )?;

        Ok(FriendGroupMessage {
            id,
            group_id: group_id.to_string(),
            sender_user_id: SOCIAL_AI_USER_ID.to_string(),
            sender_name: SOCIAL_AI_DISPLAY_NAME.to_string(),
            content,
            attachments: Vec::new(),
            created_at,
            outgoing: false,
        })
    }
}

fn history_message_from_row(
    request_user_id: &str,
    sender_user_id: &str,
    sender_name: &str,
    content: String,
) -> SocialAiHistoryMessage {
    let from_request_user = sender_user_id == request_user_id;
    let speaker = if sender_user_id == SOCIAL_AI_USER_ID {
        SOCIAL_AI_DISPLAY_NAME.to_string()
    } else if from_request_user {
        "我".to_string()
    } else {
        sender_name.to_string()
    };
    SocialAiHistoryMessage {
        speaker,
        content,
        from_request_user,
    }
}

fn social_ai_friend_message(
    id: String,
    receiver_user_id: &str,
    context_user_id: &str,
    content: &str,
    created_at: &str,
) -> FriendChatMessage {
    FriendChatMessage {
        id,
        sender_user_id: SOCIAL_AI_USER_ID.to_string(),
        receiver_user_id: receiver_user_id.to_string(),
        sender_name: Some(SOCIAL_AI_DISPLAY_NAME.to_string()),
        content: content.to_string(),
        attachments: Vec::new(),
        created_at: created_at.to_string(),
        context_user_id: Some(context_user_id.to_string()),
        outgoing: false,
    }
}

pub(super) fn ensure_social_ai_user(conn: &Connection) -> Result<()> {
    let now = now();
    conn.execute(
        "INSERT OR IGNORE INTO users (
            id, email, password_hash, nickname, role, status, created_at, updated_at
         )
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)",
        params![
            SOCIAL_AI_USER_ID,
            "el@system.local",
            "system-ai-user",
            SOCIAL_AI_DISPLAY_NAME,
            "assistant",
            "active",
            now
        ],
    )?;
    Ok(())
}

fn ensure_direct_social_ai_pair(conn: &Connection, user_id: &str) -> Result<()> {
    if user_id == SOCIAL_AI_USER_ID {
        return Err(anyhow!("不能和自己聊天"));
    }
    ensure_social_ai_user(conn)
}

fn ensure_friend_pair_for_social_ai(
    conn: &Connection,
    user_id: &str,
    friend_id: &str,
) -> Result<()> {
    if user_id == friend_id {
        return Err(anyhow!("不能和自己聊天"));
    }
    if friend_id == SOCIAL_AI_USER_ID {
        return ensure_direct_social_ai_pair(conn, user_id);
    }
    let exists = conn
        .query_row(
            "SELECT 1
             FROM user_friends
             WHERE user_id = ?1 AND friend_user_id = ?2
             LIMIT 1",
            params![user_id, friend_id],
            |_| Ok(()),
        )
        .optional()?
        .is_some();
    if exists {
        Ok(())
    } else {
        Err(anyhow!("只能和已添加的好友聊天"))
    }
}

fn list_recent_direct_social_ai_messages(
    conn: &Connection,
    user_id: &str,
    limit: i64,
) -> Result<Vec<SocialAiHistoryMessage>> {
    let mut stmt = conn.prepare(
        "SELECT m.sender_user_id,
                COALESCE(u.nickname, u.email, u.phone, m.sender_user_id) AS sender_name,
                m.content
         FROM friend_messages m
         LEFT JOIN users u ON u.id = m.sender_user_id
         WHERE (
             (
                 m.sender_user_id = ?1
                 AND m.receiver_user_id = ?2
                 AND m.context_user_id IS NULL
             )
             OR (
                 m.sender_user_id = ?2
                 AND m.receiver_user_id = ?1
                 AND m.context_user_id IS NULL
             )
         )
         ORDER BY m.created_at DESC
         LIMIT ?3",
    )?;
    let mut messages = stmt
        .query_map(
            params![user_id, SOCIAL_AI_USER_ID, limit.clamp(1, 30)],
            |row| {
                let sender_user_id: String = row.get(0)?;
                let sender_name: String = row.get(1)?;
                Ok(history_message_from_row(
                    user_id,
                    &sender_user_id,
                    &sender_name,
                    row.get::<_, String>(2)?,
                ))
            },
        )?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    messages.reverse();
    Ok(messages)
}

fn ensure_group_member_for_social_ai(
    conn: &Connection,
    user_id: &str,
    group_id: &str,
) -> Result<()> {
    let exists = conn
        .query_row(
            "SELECT 1
             FROM friend_group_members
             WHERE group_id = ?1 AND user_id = ?2
             LIMIT 1",
            params![group_id, user_id],
            |_| Ok(()),
        )
        .optional()?
        .is_some();
    if exists {
        Ok(())
    } else {
        Err(anyhow!("你不在这个群聊中"))
    }
}

fn ensure_group_exists(conn: &Connection, group_id: &str) -> Result<()> {
    let exists = conn
        .query_row(
            "SELECT 1 FROM friend_groups WHERE id = ?1 LIMIT 1",
            params![group_id],
            |_| Ok(()),
        )
        .optional()?
        .is_some();
    if exists {
        Ok(())
    } else {
        Err(anyhow!("群聊不存在"))
    }
}

fn normalize_reply_content(content: &str) -> Result<String> {
    let content = content.trim();
    if content.is_empty() {
        return Err(anyhow!("AI 回复不能为空"));
    }
    Ok(content.chars().take(4000).collect())
}

#[cfg(test)]
mod tests {
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
}
