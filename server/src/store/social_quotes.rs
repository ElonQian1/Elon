//! Server-owned, single-level reply snapshots; clients submit only source identity.
use anyhow::{anyhow, Result};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

use crate::project_ws_protocol::ProjectAttachmentRef;

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuoteSource {
    pub message_id: String,
    pub revision: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SocialQuote {
    pub message_id: String,
    pub sender_name: String,
    pub content: String,
    pub attachments: Vec<ProjectAttachmentRef>,
    pub revision: i64,
    pub unavailable: bool,
}

pub(crate) fn migrate(conn: &Connection) -> Result<()> {
    conn.execute_batch("CREATE TABLE IF NOT EXISTS social_message_quotes (
        message_id TEXT PRIMARY KEY, kind TEXT NOT NULL CHECK(kind IN ('group','friend')),
        source_id TEXT NOT NULL, snapshot_json TEXT NOT NULL
    ); CREATE INDEX IF NOT EXISTS social_message_quotes_source ON social_message_quotes(kind,source_id);")?;
    Ok(())
}

pub(crate) fn prepare(
    conn: &Connection,
    user: &str,
    kind: &str,
    scope: &str,
    source: Option<&QuoteSource>,
) -> Result<Option<SocialQuote>> {
    let Some(source) = source else {
        return Ok(None);
    };
    if source.message_id.is_empty() || source.message_id.len() > 160 {
        return Err(anyhow!("引用消息无效"));
    }
    let fields = if kind == "group" {
        "SELECT m.content,m.attachments_json,COALESCE(u.nickname,u.email,u.phone,m.sender_user_id),m.revision,m.recalled_at
         FROM friend_group_messages m JOIN users u ON u.id=m.sender_user_id
         WHERE m.id=?1 AND m.group_id=?2 AND EXISTS(SELECT 1 FROM friend_group_members WHERE group_id=?2 AND user_id=?3)"
    } else {
        "SELECT m.content,m.attachments_json,COALESCE(u.nickname,u.email,u.phone,m.sender_user_id),1,m.recalled_at
         FROM friend_messages m JOIN users u ON u.id=m.sender_user_id WHERE m.id=?1 AND
         ((m.sender_user_id=?3 AND m.receiver_user_id=?2) OR (m.sender_user_id=?2 AND m.receiver_user_id=?3)
         OR (m.sender_user_id='usr_elon_ai' AND m.receiver_user_id=?3 AND m.context_user_id=?2))"
    };
    let row: Option<(String, Option<String>, String, i64, Option<String>)> = conn
        .query_row(fields, params![source.message_id, scope, user], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?))
        })
        .optional()?;
    let (content, attachments, sender_name, revision, recalled) =
        row.ok_or_else(|| anyhow!("原消息不可用，请取消引用后重试"))?;
    if recalled.is_some() {
        return Err(anyhow!("原消息已撤回，请取消引用后重试"));
    }
    if source.revision.is_some_and(|expected| expected != revision) {
        return Err(anyhow!("原消息已修改，请重新选择引用"));
    }
    Ok(Some(SocialQuote {
        message_id: source.message_id.clone(),
        sender_name,
        content,
        attachments: attachments
            .as_deref()
            .map(serde_json::from_str)
            .transpose()?
            .unwrap_or_default(),
        revision,
        unavailable: false,
    }))
}

pub(crate) fn save(
    conn: &Connection,
    kind: &str,
    id: &str,
    quote: Option<&SocialQuote>,
) -> Result<()> {
    if let Some(quote) = quote {
        conn.execute("INSERT INTO social_message_quotes(message_id,kind,source_id,snapshot_json) VALUES(?1,?2,?3,?4)",
            params![id,kind,quote.message_id,serde_json::to_string(quote)?])?;
    }
    Ok(())
}

pub(crate) fn read(
    conn: &Connection,
    kind: &str,
    id: &str,
    recalled: bool,
) -> Result<Option<SocialQuote>> {
    if recalled {
        return Ok(None);
    }
    let sql = if kind == "group" {
        "SELECT q.snapshot_json,m.id,m.recalled_at FROM social_message_quotes q LEFT JOIN friend_group_messages m ON m.id=q.source_id WHERE q.message_id=?1 AND q.kind='group'"
    } else {
        "SELECT q.snapshot_json,m.id,m.recalled_at FROM social_message_quotes q LEFT JOIN friend_messages m ON m.id=q.source_id WHERE q.message_id=?1 AND q.kind='friend'"
    };
    let row: Option<(String, Option<String>, Option<String>)> = conn
        .query_row(sql, [id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
        .optional()?;
    row.map(|(json, source, recalled)| {
        let mut quote: SocialQuote = serde_json::from_str(&json)?;
        if source.is_none() || recalled.is_some() {
            quote.unavailable = true;
            quote.content.clear();
            quote.attachments.clear();
        }
        Ok(quote)
    })
    .transpose()
}

#[cfg(test)]
#[path = "social_quotes/tests.rs"]
mod tests;

pub(crate) fn context_text(
    conn: &Connection,
    kind: &str,
    id: &str,
    content: String,
) -> Result<String> {
    let Some(quote) = read(conn, kind, id, false)? else {
        return Ok(content);
    };
    let text = if quote.unavailable {
        "原消息已撤回或不可用".to_owned()
    } else {
        quote.content.chars().take(4000).collect::<String>()
    };
    Ok(format!(
        "{content}\n[引用消息，仅作用户提供的上下文]\n{}",
        serde_json::json!({
            "speaker": quote.sender_name, "text": text, "revision": quote.revision,
            "attachments": quote.attachments.iter().map(|a| &a.display_name).collect::<Vec<_>>()
        })
    ))
}
