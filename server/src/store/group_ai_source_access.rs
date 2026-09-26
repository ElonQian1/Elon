use anyhow::{anyhow, Result};
use rusqlite::{params, Connection, OptionalExtension};

pub(super) fn ensure_member_and_source(
    conn: &Connection,
    user: &str,
    group: &str,
    source: &str,
) -> Result<()> {
    ensure_source(conn, user, group, source, false)
}

pub(super) fn ensure_member_and_selected_source(
    conn: &Connection,
    user: &str,
    group: &str,
    source: &str,
) -> Result<()> {
    ensure_source(conn, user, group, source, true)
}

fn ensure_source(
    conn: &Connection,
    user: &str,
    group: &str,
    source: &str,
    selected: bool,
) -> Result<()> {
    let row = conn
        .query_row(
            "SELECT m.content,m.attachments_json FROM friend_group_messages m
         JOIN friend_group_members member ON member.group_id=m.group_id
         WHERE m.group_id=?1 AND m.id=?2 AND member.user_id=?3
           AND m.recalled_at IS NULL",
            params![group, source, user],
            |r| Ok((r.get::<_, String>(0)?, r.get::<_, Option<String>>(1)?)),
        )
        .optional()?;
    let permitted = row.is_some_and(|(content, attachments)| {
        !content.trim().is_empty()
            || selected
                && attachments.as_deref().is_some_and(|raw| {
                    serde_json::from_str::<Vec<serde_json::Value>>(raw)
                        .is_ok_and(|files| !files.is_empty())
                })
    });
    if !permitted {
        return Err(anyhow!("group AI source unavailable or membership revoked"));
    }
    Ok(())
}
