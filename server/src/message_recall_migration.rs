use anyhow::Result;
use rusqlite::Connection;

use crate::store_migrations::add_column_if_missing;

pub(crate) fn migration_v95(conn: &Connection) -> Result<()> {
    for table in [
        "friend_messages",
        "friend_group_messages",
        "messages",
        "project_channel_messages",
        "project_member_conversation_discussion_messages",
    ] {
        add_column_if_missing(conn, table, "recalled_at", "recalled_at TEXT")?;
        add_column_if_missing(conn, table, "recalled_by", "recalled_by TEXT")?;
    }

    Ok(())
}
