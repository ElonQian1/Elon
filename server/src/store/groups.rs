use anyhow::{anyhow, Result};
use rusqlite::{params, Connection, OptionalExtension};
use std::collections::BTreeSet;

use crate::project_ws_protocol::ProjectAttachmentRef;

use super::friend_messages::{attachments_to_json, parse_attachments};
use super::{new_id, now, FriendGroupMemberPreview, FriendGroupMessage, FriendGroupProfile, Store};

impl Store {
    pub fn list_friend_groups(&self, user_id: &str) -> Result<Vec<FriendGroupProfile>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            "SELECT g.id, g.name, g.created_at,
                    (
                        SELECT COUNT(*)
                        FROM friend_group_members count_member
                        WHERE count_member.group_id = g.id
                    ) AS member_count,
                    latest.content,
                    latest.created_at,
                    (
                        SELECT COUNT(*)
                        FROM friend_group_messages unread
                        WHERE unread.group_id = g.id
                          AND unread.sender_user_id != ?1
                          AND (
                              gm.last_read_at IS NULL
                              OR unread.created_at > gm.last_read_at
                          )
                    ) AS unread_count
             FROM friend_group_members gm
             JOIN friend_groups g ON g.id = gm.group_id
             LEFT JOIN friend_group_messages latest
               ON latest.id = (
                   SELECT newest.id
                   FROM friend_group_messages newest
                   WHERE newest.group_id = g.id
                   ORDER BY newest.created_at DESC
                   LIMIT 1
               )
             WHERE gm.user_id = ?1
             ORDER BY COALESCE(latest.created_at, g.updated_at) DESC",
        )?;
        let rows = stmt
            .query_map(params![user_id], |row| {
                Ok(FriendGroupProfile {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    created_at: row.get(2)?,
                    member_count: row.get(3)?,
                    members: Vec::new(),
                    last_message: row.get(4)?,
                    last_message_at: row.get(5)?,
                    unread_count: row.get(6)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        let groups = rows
            .into_iter()
            .map(|mut group| {
                group.members = list_group_member_previews(&conn, &group.id)?;
                Ok(group)
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(groups)
    }

    pub fn create_friend_group(
        &self,
        owner_user_id: &str,
        name: Option<&str>,
        member_ids: &[String],
    ) -> Result<FriendGroupProfile> {
        let mut members = BTreeSet::new();
        members.insert(owner_user_id.to_string());
        for id in member_ids {
            let id = id.trim();
            if !id.is_empty() {
                members.insert(id.to_string());
            }
        }
        if members.len() < 2 {
            return Err(anyhow!("发起群聊至少需要选择 1 位好友"));
        }
        for member_id in members.iter().filter(|id| id.as_str() != owner_user_id) {
            self.ensure_group_candidate_friend(owner_user_id, member_id)?;
        }

        let group_id = new_id("grp");
        let created_at = now();
        let group_name = normalize_group_name(name, &members)?;
        let conn = self.conn()?;
        let tx = conn.unchecked_transaction()?;
        tx.execute(
            "INSERT INTO friend_groups (id, name, owner_user_id, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?4)",
            params![group_id, group_name, owner_user_id, created_at],
        )?;
        for member_id in &members {
            let last_read_at = if member_id == owner_user_id {
                Some(created_at.as_str())
            } else {
                None
            };
            tx.execute(
                "INSERT INTO friend_group_members (group_id, user_id, created_at, last_read_at)
                 VALUES (?1, ?2, ?3, ?4)",
                params![group_id, member_id, created_at, last_read_at],
            )?;
        }
        tx.commit()?;

        let members_preview = list_group_member_previews(&conn, &group_id)?;
        Ok(FriendGroupProfile {
            id: group_id,
            name: group_name,
            member_count: members.len() as i64,
            members: members_preview,
            created_at,
            last_message: None,
            last_message_at: None,
            unread_count: 0,
        })
    }

    pub fn list_friend_group_messages(
        &self,
        user_id: &str,
        group_id: &str,
        after: Option<&str>,
        limit: i64,
    ) -> Result<Vec<FriendGroupMessage>> {
        self.ensure_group_member(user_id, group_id)?;
        let limit = limit.clamp(1, 200);
        let after = after.map(str::trim).filter(|value| !value.is_empty());
        let conn = self.conn()?;
        let sql = if after.is_some() {
            "SELECT m.id, m.group_id, m.sender_user_id,
                    COALESCE(u.nickname, u.email, u.phone, m.sender_user_id) AS sender_name,
                    m.content, m.attachments_json, m.created_at
             FROM friend_group_messages m
             JOIN users u ON u.id = m.sender_user_id
             WHERE m.group_id = ?1 AND m.created_at > ?2
             ORDER BY m.created_at ASC
             LIMIT ?3"
        } else {
            "SELECT id, group_id, sender_user_id, sender_name, content, attachments_json, created_at
             FROM (
                 SELECT m.id, m.group_id, m.sender_user_id,
                        COALESCE(u.nickname, u.email, u.phone, m.sender_user_id) AS sender_name,
                        m.content, m.attachments_json, m.created_at
                 FROM friend_group_messages m
                 JOIN users u ON u.id = m.sender_user_id
                 WHERE m.group_id = ?1
                 ORDER BY m.created_at DESC
                 LIMIT ?3
             )
             ORDER BY created_at ASC"
        };
        let mut stmt = conn.prepare(sql)?;
        let messages = if let Some(after) = after {
            stmt.query_map(params![group_id, after, limit], |row| {
                row_to_group_message(row, user_id)
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?
        } else {
            stmt.query_map(params![group_id, "", limit], |row| {
                row_to_group_message(row, user_id)
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?
        };
        drop(stmt);
        mark_group_messages_read(&conn, user_id, group_id)?;
        Ok(messages)
    }

    pub fn send_friend_group_message(
        &self,
        user_id: &str,
        group_id: &str,
        content: &str,
        attachments: Option<&[ProjectAttachmentRef]>,
    ) -> Result<FriendGroupMessage> {
        self.ensure_group_member(user_id, group_id)?;
        let content = content.trim();
        let attachments_json = attachments_to_json(attachments)?;
        if content.is_empty() && attachments_json.is_none() {
            return Err(anyhow!("消息不能为空"));
        }
        if content.chars().count() > 4000 {
            return Err(anyhow!("消息过长"));
        }

        let id = new_id("gmsg");
        let created_at = now();
        let conn = self.conn()?;
        let sender_name = conn.query_row(
            "SELECT COALESCE(nickname, email, phone, id)
             FROM users
             WHERE id = ?1",
            params![user_id],
            |row| row.get::<_, String>(0),
        )?;
        conn.execute(
            "INSERT INTO friend_group_messages (
                id, group_id, sender_user_id, content, attachments_json, created_at
             )
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![id, group_id, user_id, content, attachments_json, created_at],
        )?;
        conn.execute(
            "UPDATE friend_groups SET updated_at = ?1 WHERE id = ?2",
            params![created_at, group_id],
        )?;
        mark_group_messages_read(&conn, user_id, group_id)?;

        Ok(FriendGroupMessage {
            id,
            group_id: group_id.to_string(),
            sender_user_id: user_id.to_string(),
            sender_name,
            content: content.to_string(),
            attachments: attachments.unwrap_or(&[]).to_vec(),
            created_at,
            outgoing: true,
        })
    }

    pub fn delete_friend_group_message(
        &self,
        user_id: &str,
        group_id: &str,
        message_id: &str,
    ) -> Result<()> {
        self.ensure_group_member(user_id, group_id)?;
        let conn = self.conn()?;
        let changed = conn.execute(
            "DELETE FROM friend_group_messages
             WHERE id = ?1
               AND group_id = ?2
               AND sender_user_id = ?3",
            params![message_id, group_id, user_id],
        )?;
        if changed == 0 {
            return Err(anyhow!("只能撤销自己发送的消息"));
        }
        conn.execute(
            "UPDATE friend_groups SET updated_at = ?1 WHERE id = ?2",
            params![now(), group_id],
        )?;
        Ok(())
    }

    pub fn friend_group_member_ids(&self, user_id: &str, group_id: &str) -> Result<Vec<String>> {
        self.ensure_group_member(user_id, group_id)?;
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            "SELECT user_id
             FROM friend_group_members
             WHERE group_id = ?1",
        )?;
        let members = stmt
            .query_map(params![group_id], |row| row.get::<_, String>(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(members)
    }

    fn ensure_group_candidate_friend(&self, user_id: &str, friend_id: &str) -> Result<()> {
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
            Err(anyhow!("只能选择已添加的好友"))
        }
    }

    fn ensure_group_member(&self, user_id: &str, group_id: &str) -> Result<()> {
        let exists = self
            .conn()?
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
}

fn list_group_member_previews(
    conn: &Connection,
    group_id: &str,
) -> Result<Vec<FriendGroupMemberPreview>> {
    let mut stmt = conn.prepare(
        "SELECT u.id,
                COALESCE(u.nickname, u.email, u.phone, u.id) AS display_name,
                u.avatar_data_url
         FROM friend_group_members gm
         JOIN users u ON u.id = gm.user_id
         WHERE gm.group_id = ?1
         ORDER BY gm.created_at ASC, u.id ASC
         LIMIT 9",
    )?;
    let members = stmt
        .query_map(params![group_id], |row| {
            Ok(FriendGroupMemberPreview {
                id: row.get(0)?,
                display_name: row.get(1)?,
                avatar_data_url: row.get(2)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(members)
}

fn normalize_group_name(name: Option<&str>, members: &BTreeSet<String>) -> Result<String> {
    let name = name.map(str::trim).filter(|value| !value.is_empty());
    let name = name
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| format!("群聊 {}", members.len()));
    if name.chars().count() > 24 {
        return Err(anyhow!("群聊名称不能超过 24 个字"));
    }
    Ok(name)
}

fn mark_group_messages_read(conn: &Connection, user_id: &str, group_id: &str) -> Result<()> {
    conn.execute(
        "UPDATE friend_group_members
         SET last_read_at = ?1
         WHERE group_id = ?2 AND user_id = ?3",
        params![now(), group_id, user_id],
    )?;
    Ok(())
}

fn row_to_group_message(
    row: &rusqlite::Row<'_>,
    user_id: &str,
) -> rusqlite::Result<FriendGroupMessage> {
    let sender_user_id: String = row.get(2)?;
    Ok(FriendGroupMessage {
        id: row.get(0)?,
        group_id: row.get(1)?,
        sender_name: row.get(3)?,
        content: row.get(4)?,
        attachments: parse_attachments(row.get::<_, Option<String>>(5)?.as_deref())?,
        created_at: row.get(6)?,
        outgoing: sender_user_id == user_id,
        sender_user_id,
    })
}
