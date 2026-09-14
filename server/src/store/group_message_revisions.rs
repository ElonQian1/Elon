//! Current message text and append-only revisions are committed in one transaction.
use anyhow::Result;
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use serde::Serialize;

use super::{now, Store};

#[cfg(test)]
#[path = "group_message_revisions/tests.rs"]
mod tests;

#[derive(Debug, PartialEq)]
pub(crate) enum RevisionError {
    Forbidden,
    NotFound,
    Conflict,
    Recalled,
    Invalid,
    RateLimited,
}

impl std::fmt::Display for RevisionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Forbidden => "只有群成员能查看记录，且只能编辑本人发送的消息",
            Self::NotFound => "消息不存在",
            Self::Conflict => "消息已在其他设备修改，请查看最新版本后再保存；本次草稿未提交",
            Self::Recalled => "消息已撤回，无法编辑或查看正文历史",
            Self::Invalid => "请输入 1 至 4000 字的文字；项目卡片不能编辑",
            Self::RateLimited => "这条消息修改过于频繁，请稍后再试",
        })
    }
}

impl std::error::Error for RevisionError {}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct MessageRevision {
    pub revision: i64,
    pub content: String,
    pub edited_by: String,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct MessageHistory {
    pub message_id: String,
    pub current_revision: i64,
    pub revisions: Vec<MessageRevision>,
    pub next_before_revision: Option<i64>,
}

#[derive(Debug, Serialize)]
pub(crate) struct MessageEdit {
    pub id: String,
    pub group_id: String,
    pub content: String,
    pub revision: i64,
    pub edited_at: Option<String>,
    pub changed: bool,
}

struct CurrentMessage {
    sender: String,
    content: String,
    created_at: String,
    recalled_at: Option<String>,
    revision: i64,
    edited_at: Option<String>,
}

impl Store {
    pub(crate) fn edit_group_message(
        &self,
        actor: &str,
        group: &str,
        message: &str,
        expected: i64,
        content: &str,
    ) -> Result<MessageEdit> {
        let mut conn = self.conn()?;
        edit(&mut conn, actor, group, message, expected, content)
    }

    pub(crate) fn group_message_history(
        &self,
        actor: &str,
        group: &str,
        message: &str,
        before: Option<i64>,
        limit: i64,
    ) -> Result<MessageHistory> {
        let conn = self.conn()?;
        history(&conn, actor, group, message, before, limit)
    }
}

fn current(conn: &Connection, actor: &str, group: &str, message: &str) -> Result<CurrentMessage> {
    let member: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM friend_group_members WHERE group_id = ?1 AND user_id = ?2)",
        params![group, actor],
        |r| r.get(0),
    )?;
    if !member {
        return Err(RevisionError::Forbidden.into());
    }
    let found = conn
        .query_row(
            "SELECT sender_user_id, content, created_at, recalled_at, revision, edited_at
         FROM friend_group_messages WHERE id = ?1 AND group_id = ?2",
            params![message, group],
            |r| {
                Ok(CurrentMessage {
                    sender: r.get(0)?,
                    content: r.get(1)?,
                    created_at: r.get(2)?,
                    recalled_at: r.get(3)?,
                    revision: r.get(4)?,
                    edited_at: r.get(5)?,
                })
            },
        )
        .optional()?
        .ok_or(RevisionError::NotFound)?;
    if found.recalled_at.is_some() {
        return Err(RevisionError::Recalled.into());
    }
    Ok(found)
}

fn edit(
    conn: &mut Connection,
    actor: &str,
    group: &str,
    message: &str,
    expected: i64,
    content: &str,
) -> Result<MessageEdit> {
    let content = content.trim();
    if content.is_empty() || content.chars().count() > 4000 || expected < 1 {
        return Err(RevisionError::Invalid.into());
    }
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let original = current(&tx, actor, group, message)?;
    if original.sender != actor {
        return Err(RevisionError::Forbidden.into());
    }
    if original.content.starts_with("【一龙项目卡片】") || content.starts_with("【一龙项目卡片】")
    {
        return Err(RevisionError::Invalid.into());
    }
    // Retrying an acknowledged-but-lost response must not create another revision.
    if original.content == content {
        return Ok(MessageEdit {
            id: message.into(),
            group_id: group.into(),
            content: content.into(),
            revision: original.revision,
            edited_at: original.edited_at,
            changed: false,
        });
    }
    if original.revision != expected {
        return Err(RevisionError::Conflict.into());
    }
    let since = (chrono::Utc::now() - chrono::Duration::minutes(1)).to_rfc3339();
    let recent: i64 = tx.query_row(
        "SELECT COUNT(*) FROM friend_group_message_revisions WHERE message_id = ?1 AND revision > 1 AND created_at >= ?2",
        params![message, since], |r| r.get(0),
    )?;
    if recent >= 30 {
        return Err(RevisionError::RateLimited.into());
    }
    let edited_at = now();
    if original.revision == 1 {
        tx.execute(
            "INSERT INTO friend_group_message_revisions(message_id, revision, content, edited_by, created_at)
             VALUES (?1, 1, ?2, ?3, ?4)",
            params![message, original.content, original.sender, original.created_at],
        )?;
    }
    let revision = original.revision + 1;
    tx.execute(
        "INSERT INTO friend_group_message_revisions(message_id, revision, content, edited_by, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![message, revision, content, actor, edited_at],
    )?;
    tx.execute(
        "UPDATE friend_group_messages SET content = ?2, revision = ?3, edited_at = ?4 WHERE id = ?1",
        params![message, content, revision, edited_at],
    )?;
    tx.commit()?;
    Ok(MessageEdit {
        id: message.into(),
        group_id: group.into(),
        content: content.into(),
        revision,
        edited_at: Some(edited_at),
        changed: true,
    })
}

fn history(
    conn: &Connection,
    actor: &str,
    group: &str,
    message: &str,
    before: Option<i64>,
    limit: i64,
) -> Result<MessageHistory> {
    let tx = conn.unchecked_transaction()?;
    let original = current(&tx, actor, group, message)?;
    let limit = limit.clamp(1, 50);
    let mut revisions = if original.revision == 1 {
        if before.is_some_and(|v| v <= 1) {
            Vec::new()
        } else {
            vec![MessageRevision {
                revision: 1,
                content: original.content,
                edited_by: original.sender,
                created_at: original.created_at,
            }]
        }
    } else {
        let mut stmt = tx.prepare(
            "SELECT revision, content, edited_by, created_at FROM friend_group_message_revisions
             WHERE message_id = ?1 AND revision < ?2 ORDER BY revision DESC LIMIT ?3",
        )?;
        let rows = stmt
            .query_map(
                params![message, before.unwrap_or(i64::MAX), limit + 1],
                |r| {
                    Ok(MessageRevision {
                        revision: r.get(0)?,
                        content: r.get(1)?,
                        edited_by: r.get(2)?,
                        created_at: r.get(3)?,
                    })
                },
            )?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        rows
    };
    let more = revisions.len() > limit as usize;
    revisions.truncate(limit as usize);
    let next_before_revision = if more {
        revisions.last().map(|v| v.revision)
    } else {
        None
    };
    tx.commit()?;
    Ok(MessageHistory {
        message_id: message.into(),
        current_revision: original.revision,
        revisions,
        next_before_revision,
    })
}
