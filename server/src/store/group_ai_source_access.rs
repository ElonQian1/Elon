use anyhow::{anyhow, Result};
use rusqlite::{params, Connection, OptionalExtension};

pub(super) fn ensure_member_and_source(
    conn: &Connection,
    user: &str,
    group: &str,
    source: &str,
) -> Result<()> {
    let permitted = conn
        .query_row(
            "SELECT 1 FROM friend_group_messages m
         JOIN friend_group_members member ON member.group_id=m.group_id
         WHERE m.group_id=?1 AND m.id=?2 AND member.user_id=?3
           AND m.recalled_at IS NULL AND LENGTH(TRIM(m.content)) > 0",
            params![group, source, user],
            |_| Ok(()),
        )
        .optional()?
        .is_some();
    if !permitted {
        return Err(anyhow!("group AI source unavailable or membership revoked"));
    }
    Ok(())
}
