use anyhow::{anyhow, Result};
use rusqlite::{params, Connection, OptionalExtension};

use super::{new_id, now, FriendChatMessage, Store};

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
        let sql = if after.is_some() {
            "SELECT id, sender_user_id, receiver_user_id, content, created_at
             FROM friend_messages
             WHERE (
                 (sender_user_id = ?1 AND receiver_user_id = ?2)
                 OR (sender_user_id = ?2 AND receiver_user_id = ?1)
             )
               AND created_at > ?3
             ORDER BY created_at ASC
             LIMIT ?4"
        } else {
            "SELECT id, sender_user_id, receiver_user_id, content, created_at
             FROM (
                 SELECT id, sender_user_id, receiver_user_id, content, created_at
                 FROM friend_messages
                 WHERE (
                     (sender_user_id = ?1 AND receiver_user_id = ?2)
                     OR (sender_user_id = ?2 AND receiver_user_id = ?1)
                 )
                 ORDER BY created_at DESC
                 LIMIT ?4
             )
             ORDER BY created_at ASC"
        };
        let mut stmt = conn.prepare(sql)?;
        let messages = if let Some(after) = after {
            stmt.query_map(params![user_id, friend_id, after, limit], |row| {
                row_to_friend_message(row, user_id)
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?
        } else {
            stmt.query_map(params![user_id, friend_id, "", limit], |row| {
                row_to_friend_message(row, user_id)
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?
        };
        drop(stmt);
        mark_friend_messages_read(&conn, user_id, friend_id)?;
        Ok(messages)
    }

    pub fn send_friend_message(
        &self,
        user_id: &str,
        friend_id: &str,
        content: &str,
    ) -> Result<FriendChatMessage> {
        self.ensure_friend_pair(user_id, friend_id)?;
        let content = content.trim();
        if content.is_empty() {
            return Err(anyhow!("消息不能为空"));
        }
        if content.chars().count() > 4000 {
            return Err(anyhow!("消息过长"));
        }

        let id = new_id("fmsg");
        let created_at = now();
        let conn = self.conn()?;
        conn.execute(
            "INSERT INTO friend_messages (id, sender_user_id, receiver_user_id, content, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![id, user_id, friend_id, content, created_at],
        )?;

        Ok(FriendChatMessage {
            id,
            sender_user_id: user_id.to_string(),
            receiver_user_id: friend_id.to_string(),
            content: content.to_string(),
            created_at,
            outgoing: true,
        })
    }

    fn ensure_friend_pair(&self, user_id: &str, friend_id: &str) -> Result<()> {
        if user_id == friend_id {
            return Err(anyhow!("不能和自己聊天"));
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

fn mark_friend_messages_read(conn: &Connection, user_id: &str, friend_id: &str) -> Result<()> {
    let now = now();
    conn.execute(
        "INSERT INTO friend_read_states (user_id, friend_user_id, last_read_at)
         VALUES (?1, ?2, ?3)
         ON CONFLICT(user_id, friend_user_id)
         DO UPDATE SET last_read_at = excluded.last_read_at",
        params![user_id, friend_id, now],
    )?;
    Ok(())
}

fn row_to_friend_message(
    row: &rusqlite::Row<'_>,
    user_id: &str,
) -> rusqlite::Result<FriendChatMessage> {
    let sender_user_id: String = row.get(1)?;
    Ok(FriendChatMessage {
        id: row.get(0)?,
        receiver_user_id: row.get(2)?,
        content: row.get(3)?,
        created_at: row.get(4)?,
        outgoing: sender_user_id == user_id,
        sender_user_id,
    })
}
