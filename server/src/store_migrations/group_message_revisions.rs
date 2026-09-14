use anyhow::Result;
use rusqlite::Connection;

pub(crate) fn migrate(conn: &Connection) -> Result<()> {
    super::add_column_if_missing(
        conn,
        "friend_group_messages",
        "revision",
        "revision INTEGER NOT NULL DEFAULT 1",
    )?;
    super::add_column_if_missing(conn, "friend_group_messages", "edited_at", "edited_at TEXT")?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS friend_group_message_revisions (
            message_id TEXT NOT NULL,
            revision INTEGER NOT NULL CHECK (revision >= 1),
            content TEXT NOT NULL,
            edited_by TEXT NOT NULL,
            created_at TEXT NOT NULL,
            PRIMARY KEY (message_id, revision)
         );
         CREATE TRIGGER IF NOT EXISTS group_message_revision_no_update
         BEFORE UPDATE ON friend_group_message_revisions BEGIN
            SELECT RAISE(ABORT, 'Message revision history is immutable');
         END;
         CREATE TRIGGER IF NOT EXISTS group_message_revision_no_delete
         BEFORE DELETE ON friend_group_message_revisions BEGIN
            SELECT RAISE(ABORT, 'Message revision history is immutable');
         END;",
    )?;
    Ok(())
}
