use anyhow::{anyhow, Result};
use rusqlite::{params, Connection, OptionalExtension};

use crate::project_ws_protocol::ProjectAttachmentRef;

use super::message_recall::{
    ensure_message_recall_allowed, recall_preview_for_viewer, recalled_content,
};
use super::social_ai_messages::ensure_social_ai_user;
use super::{new_id, now, FriendChatMessage, Store, SOCIAL_AI_DISPLAY_NAME, SOCIAL_AI_USER_ID};

impl Store {
    pub fn list_friend_messages(
        &self,
        user_id: &str,
        friend_id: &str,
        after: Option<&str>,
        limit: i64,
    ) -> Result<Vec<FriendChatMessage>> {
        self.ensure_friend_pair(user_id, friend_id)?;
        let limit = limit.clamp(1, 200);
        let after = after.map(str::trim).filter(|value| !value.is_empty());
        let conn = self.conn()?;
        let messages = if friend_id == SOCIAL_AI_USER_ID {
            list_direct_social_ai_messages(&conn, user_id, after, limit)?
        } else {
            list_regular_friend_messages(&conn, user_id, friend_id, after, limit)?
        };
        mark_friend_messages_read(&conn, user_id, friend_id)?;
        Ok(messages)
    }

    pub fn send_friend_message(
        &self,
        user_id: &str,
        friend_id: &str,
        content: &str,
        attachments: Option<&[ProjectAttachmentRef]>,
    ) -> Result<FriendChatMessage> {
        self.ensure_friend_pair(user_id, friend_id)?;
        let content = content.trim();
        let attachments_json = attachments_to_json(attachments)?;
        if content.is_empty() && attachments_json.is_none() {
            return Err(anyhow!("消息不能为空"));
        }
        if content.chars().count() > 4000 {
            return Err(anyhow!("消息过长"));
        }

        let id = new_id("fmsg");
        let created_at = now();
        let conn = self.conn()?;
        conn.execute(
            "INSERT INTO friend_messages (
                id, sender_user_id, receiver_user_id, content, attachments_json, created_at
             )
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                id,
                user_id,
                friend_id,
                content,
                attachments_json,
                created_at
            ],
        )?;

        Ok(FriendChatMessage {
            id,
            sender_user_id: user_id.to_string(),
            receiver_user_id: friend_id.to_string(),
            sender_name: None,
            content: content.to_string(),
            attachments: attachments.unwrap_or(&[]).to_vec(),
            created_at,
            context_user_id: None,
            outgoing: true,
            recalled_at: None,
            recalled_by: None,
        })
    }

    pub fn delete_friend_message(
        &self,
        user_id: &str,
        friend_id: &str,
        message_id: &str,
    ) -> Result<()> {
        self.ensure_friend_pair(user_id, friend_id)?;
        let conn = self.conn()?;
        let message = conn
            .query_row(
                "SELECT created_at, recalled_at
                 FROM friend_messages
                 WHERE id = ?1
                   AND sender_user_id = ?2
                   AND receiver_user_id = ?3",
                params![message_id, user_id, friend_id],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?)),
            )
            .optional()?;
        let Some((created_at, recalled_at)) = message else {
            return Err(anyhow!("只能撤回自己发送的消息"));
        };
        if recalled_at.is_some() {
            return Ok(());
        }
        ensure_message_recall_allowed(&created_at)?;
        conn.execute(
            "UPDATE friend_messages
                SET recalled_at = ?4,
                    recalled_by = ?2
              WHERE id = ?1
                AND sender_user_id = ?2
                AND receiver_user_id = ?3
                AND recalled_at IS NULL",
            params![message_id, user_id, friend_id, now()],
        )?;
        Ok(())
    }

    pub fn delete_friend_project_share_messages(
        &self,
        user_id: &str,
        project_id: &str,
    ) -> Result<usize> {
        let project_id = project_id.trim();
        if project_id.is_empty() {
            return Ok(0);
        }
        let id_marker = format!(r#""id":"{}""#, project_id);
        let changed = self.conn()?.execute(
            "DELETE FROM friend_messages
             WHERE sender_user_id = ?1
               AND content LIKE '【一龙项目卡片】%'
               AND instr(content, ?2) > 0",
            params![user_id, id_marker],
        )?;
        Ok(changed)
    }

    fn ensure_friend_pair(&self, user_id: &str, friend_id: &str) -> Result<()> {
        if user_id == friend_id {
            return Err(anyhow!("不能和自己聊天"));
        }
        if friend_id == SOCIAL_AI_USER_ID {
            let conn = self.conn()?;
            ensure_social_ai_user(&conn)?;
            return Ok(());
        }
        let exists = self
            .conn()?
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
}

fn list_regular_friend_messages(
    conn: &Connection,
    user_id: &str,
    friend_id: &str,
    after: Option<&str>,
    limit: i64,
) -> Result<Vec<FriendChatMessage>> {
    let sql = if after.is_some() {
        "SELECT m.id, m.sender_user_id, m.receiver_user_id,
                COALESCE(u.nickname, u.email, u.phone, m.sender_user_id) AS sender_name,
                m.content, m.attachments_json, m.created_at, m.context_user_id,
                m.recalled_at, m.recalled_by
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
           AND m.created_at > ?4
         ORDER BY m.created_at ASC
         LIMIT ?5"
    } else {
        "SELECT id, sender_user_id, receiver_user_id, sender_name,
                content, attachments_json, created_at, context_user_id, recalled_at, recalled_by
         FROM (
             SELECT m.id, m.sender_user_id, m.receiver_user_id,
                    COALESCE(u.nickname, u.email, u.phone, m.sender_user_id) AS sender_name,
                    m.content, m.attachments_json, m.created_at, m.context_user_id,
                    m.recalled_at, m.recalled_by
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
             LIMIT ?5
         )
         ORDER BY created_at ASC"
    };
    let mut stmt = conn.prepare(sql)?;
    if let Some(after) = after {
        Ok(stmt
            .query_map(
                params![user_id, friend_id, SOCIAL_AI_USER_ID, after, limit],
                |row| row_to_friend_message(row, user_id),
            )?
            .collect::<rusqlite::Result<Vec<_>>>()?)
    } else {
        Ok(stmt
            .query_map(
                params![user_id, friend_id, SOCIAL_AI_USER_ID, "", limit],
                |row| row_to_friend_message(row, user_id),
            )?
            .collect::<rusqlite::Result<Vec<_>>>()?)
    }
}

fn list_direct_social_ai_messages(
    conn: &Connection,
    user_id: &str,
    after: Option<&str>,
    limit: i64,
) -> Result<Vec<FriendChatMessage>> {
    let sql = if after.is_some() {
        "SELECT m.id, m.sender_user_id, m.receiver_user_id,
                COALESCE(u.nickname, u.email, u.phone, m.sender_user_id) AS sender_name,
                m.content, m.attachments_json, m.created_at, m.context_user_id,
                m.recalled_at, m.recalled_by
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
           AND m.created_at > ?3
         ORDER BY m.created_at ASC
         LIMIT ?4"
    } else {
        "SELECT id, sender_user_id, receiver_user_id, sender_name,
                content, attachments_json, created_at, context_user_id, recalled_at, recalled_by
         FROM (
             SELECT m.id, m.sender_user_id, m.receiver_user_id,
                    COALESCE(u.nickname, u.email, u.phone, m.sender_user_id) AS sender_name,
                    m.content, m.attachments_json, m.created_at, m.context_user_id,
                    m.recalled_at, m.recalled_by
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
             LIMIT ?4
         )
         ORDER BY created_at ASC"
    };
    let mut stmt = conn.prepare(sql)?;
    if let Some(after) = after {
        Ok(stmt
            .query_map(params![user_id, SOCIAL_AI_USER_ID, after, limit], |row| {
                row_to_friend_message(row, user_id)
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?)
    } else {
        Ok(stmt
            .query_map(params![user_id, SOCIAL_AI_USER_ID, "", limit], |row| {
                row_to_friend_message(row, user_id)
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?)
    }
}

fn mark_friend_messages_read(conn: &Connection, user_id: &str, friend_id: &str) -> Result<()> {
    let now = now();
    conn.execute(
        "INSERT INTO friend_read_states (user_id, friend_user_id, last_read_at)
         VALUES (?1, ?2, ?3)
         ON CONFLICT(user_id, friend_user_id)
         DO UPDATE SET last_read_at = excluded.last_read_at",
        params![user_id, friend_id, now],
    )?;
    // 实时通知原发送方（friend_id）：user_id 已读至 now
    crate::read_receipt_events::publish(user_id.to_string(), friend_id.to_string(), now);
    Ok(())
}

fn row_to_friend_message(
    row: &rusqlite::Row<'_>,
    user_id: &str,
) -> rusqlite::Result<FriendChatMessage> {
    let sender_user_id: String = row.get(1)?;
    let sender_name: String = row.get(3)?;
    let recalled_at: Option<String> = row.get(8)?;
    let recalled_by: Option<String> = row.get(9)?;
    Ok(FriendChatMessage {
        id: row.get(0)?,
        receiver_user_id: row.get(2)?,
        sender_name: Some(if sender_user_id == SOCIAL_AI_USER_ID {
            SOCIAL_AI_DISPLAY_NAME.to_string()
        } else {
            sender_name
        }),
        content: recalled_content(row.get(4)?, recalled_at.as_deref()),
        attachments: if recalled_at.is_some() {
            Vec::new()
        } else {
            parse_attachments(row.get::<_, Option<String>>(5)?.as_deref())?
        },
        created_at: row.get(6)?,
        context_user_id: row.get(7)?,
        outgoing: sender_user_id == user_id,
        recalled_at,
        recalled_by,
        sender_user_id,
    })
}

pub(super) fn attachments_to_json(
    attachments: Option<&[ProjectAttachmentRef]>,
) -> Result<Option<String>> {
    let Some(attachments) = attachments.filter(|items| !items.is_empty()) else {
        return Ok(None);
    };
    Ok(Some(serde_json::to_string(attachments)?))
}

pub(super) fn parse_attachments(
    value: Option<&str>,
) -> rusqlite::Result<Vec<ProjectAttachmentRef>> {
    let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(Vec::new());
    };
    serde_json::from_str(value).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(error))
    })
}

pub(super) fn message_preview_from_parts(
    content: Option<&str>,
    attachments_json: Option<&str>,
) -> rusqlite::Result<Option<String>> {
    if let Some(content) = content.map(str::trim).filter(|value| !value.is_empty()) {
        return Ok(Some(content.to_string()));
    }

    let attachments = parse_attachments(attachments_json)?;
    Ok(message_preview_from_attachments(&attachments))
}

pub(super) fn message_preview_for_viewer(
    content: Option<&str>,
    attachments_json: Option<&str>,
    recalled_at: Option<&str>,
    recalled_by: Option<&str>,
    viewer_user_id: &str,
) -> rusqlite::Result<Option<String>> {
    if let Some(preview) = recall_preview_for_viewer(recalled_at, recalled_by, viewer_user_id) {
        return Ok(Some(preview));
    }
    message_preview_from_parts(content, attachments_json)
}

fn message_preview_from_attachments(attachments: &[ProjectAttachmentRef]) -> Option<String> {
    for attachment in attachments {
        if is_voice_attachment(attachment) {
            return Some(match attachment.duration_seconds {
                Some(seconds) => format!("【语音】{}秒", seconds),
                None => "【语音】".to_string(),
            });
        }
        if is_image_attachment(attachment) {
            return Some("【图片】".to_string());
        }
    }

    if attachments.is_empty() {
        None
    } else {
        Some("【附件】".to_string())
    }
}

fn is_voice_attachment(attachment: &ProjectAttachmentRef) -> bool {
    attachment_field_matches(&attachment.kind, &["voice", "audio"])
        || attachment_mime_starts_with(&attachment.mime_type, "audio/")
        || attachment_has_extension(
            attachment,
            &["m4a", "mp3", "wav", "aac", "ogg", "opus", "amr"],
        )
}

fn is_image_attachment(attachment: &ProjectAttachmentRef) -> bool {
    attachment_field_matches(&attachment.kind, &["image", "photo"])
        || attachment_mime_starts_with(&attachment.mime_type, "image/")
        || attachment_has_extension(
            attachment,
            &["jpg", "jpeg", "png", "gif", "webp", "bmp", "heic", "heif"],
        )
}

fn attachment_field_matches(value: &Option<String>, choices: &[&str]) -> bool {
    value.as_deref().map(str::trim).is_some_and(|value| {
        choices
            .iter()
            .any(|choice| value.eq_ignore_ascii_case(choice))
    })
}

fn attachment_mime_starts_with(value: &Option<String>, prefix: &str) -> bool {
    value
        .as_deref()
        .map(str::trim)
        .is_some_and(|value| value.to_ascii_lowercase().starts_with(prefix))
}

fn attachment_has_extension(attachment: &ProjectAttachmentRef, extensions: &[&str]) -> bool {
    [
        &attachment.file_name,
        &attachment.display_name,
        &attachment.path,
    ]
    .into_iter()
    .filter_map(|value| value.as_deref())
    .filter_map(|value| value.rsplit(['/', '\\']).next())
    .filter_map(|value| value.rsplit_once('.').map(|(_, extension)| extension))
    .any(|extension| {
        extensions
            .iter()
            .any(|choice| extension.eq_ignore_ascii_case(choice))
    })
}
