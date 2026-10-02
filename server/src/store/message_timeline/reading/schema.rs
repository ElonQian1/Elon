use anyhow::Result;
use rusqlite::Connection;

pub(crate) fn migrate(conn: &Connection) -> Result<()> {
    conn.execute_batch("CREATE TABLE IF NOT EXISTS reading_bookmarks (
        owner TEXT NOT NULL, id TEXT NOT NULL, kind TEXT NOT NULL, project TEXT NOT NULL,
        scope TEXT NOT NULL, title TEXT NOT NULL, note TEXT NOT NULL,
        anchor TEXT NOT NULL, revision INTEGER NOT NULL DEFAULT 1,
        deleted INTEGER NOT NULL DEFAULT 0, created_at TEXT NOT NULL, updated_at TEXT NOT NULL,
        PRIMARY KEY(owner,id));
      CREATE INDEX IF NOT EXISTS reading_bookmark_scope ON reading_bookmarks(owner,kind,project,scope,deleted,id);
      CREATE TABLE IF NOT EXISTS reading_progress (
        owner TEXT NOT NULL, target TEXT NOT NULL, position TEXT NOT NULL, furthest TEXT NOT NULL,
        revision INTEGER NOT NULL, device TEXT NOT NULL, device_seq INTEGER NOT NULL,
        updated_at TEXT NOT NULL, PRIMARY KEY(owner,target));
      CREATE TABLE IF NOT EXISTS reading_candidates (
        owner TEXT NOT NULL, target TEXT NOT NULL, device TEXT NOT NULL, device_seq INTEGER NOT NULL,
        position TEXT NOT NULL, updated_at TEXT NOT NULL, resolved INTEGER NOT NULL DEFAULT 0, PRIMARY KEY(owner,target,device));
      CREATE TABLE IF NOT EXISTS reading_operations (
        owner TEXT NOT NULL, operation TEXT NOT NULL, target TEXT NOT NULL,
        digest TEXT NOT NULL, result TEXT NOT NULL, created_at TEXT NOT NULL,
        PRIMARY KEY(owner,operation));
      CREATE INDEX IF NOT EXISTS reading_operation_retention ON reading_operations(created_at);")?;
    Ok(())
}
