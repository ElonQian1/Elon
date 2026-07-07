use anyhow::{anyhow, Result};
use rusqlite::{params, Connection, OptionalExtension};

use crate::project_ws_protocol::ProjectAttachmentRef;

use super::friend_messages::parse_attachments;
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
                    m.content,
                    m.attachments_json
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
              AND m.recalled_at IS NULL
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
                        parse_attachments(row.get::<_, Option<String>>(3)?.as_deref())?,
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
            recalled_at: None,
            recalled_by: None,
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
                    m.content,
                    m.attachments_json
             FROM friend_group_messages m
             LEFT JOIN users u ON u.id = m.sender_user_id
             WHERE m.group_id = ?1
               AND m.recalled_at IS NULL
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
                    parse_attachments(row.get::<_, Option<String>>(3)?.as_deref())?,
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
            recalled_at: None,
            recalled_by: None,
        })
    }
}

fn history_message_from_row(
    request_user_id: &str,
    sender_user_id: &str,
    sender_name: &str,
    content: String,
    attachments: Vec<ProjectAttachmentRef>,
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
        content: crate::social_ai_attachment_context::append_to_message_content(
            &content,
            &attachments,
        ),
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
        recalled_at: None,
        recalled_by: None,
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
                m.content,
                m.attachments_json
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
           AND m.recalled_at IS NULL
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
                    parse_attachments(row.get::<_, Option<String>>(3)?.as_deref())?,
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
#[path = "social_ai_messages_tests.rs"]
mod tests;
