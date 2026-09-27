use anyhow::Result;
use rusqlite::Connection;

pub(crate) fn migrate(conn: &Connection) -> Result<()> {
    conn.execute_batch("CREATE TABLE IF NOT EXISTS social_chat_records (
        id TEXT PRIMARY KEY, owner_id TEXT NOT NULL REFERENCES users(id),
        group_id TEXT NOT NULL REFERENCES friend_groups(id) ON DELETE CASCADE,
        message_id TEXT NOT NULL UNIQUE,
        operation TEXT NOT NULL, request_hash TEXT NOT NULL, document_json TEXT NOT NULL,
        created_at TEXT NOT NULL, revoked INTEGER NOT NULL DEFAULT 0,
        UNIQUE(owner_id,group_id,operation));
    CREATE TABLE IF NOT EXISTS social_chat_record_assets (
        id TEXT PRIMARY KEY, owner_id TEXT NOT NULL REFERENCES users(id),
        group_id TEXT NOT NULL REFERENCES friend_groups(id) ON DELETE CASCADE,
        sha256 TEXT NOT NULL, mime_type TEXT NOT NULL, bytes BLOB NOT NULL,
        created_at INTEGER NOT NULL, UNIQUE(owner_id,group_id,sha256));
    CREATE TABLE IF NOT EXISTS social_chat_record_asset_refs (
        record_id TEXT NOT NULL REFERENCES social_chat_records(id) ON DELETE CASCADE,
        asset_id TEXT NOT NULL REFERENCES social_chat_record_assets(id),
        PRIMARY KEY(record_id,asset_id));
    CREATE TRIGGER IF NOT EXISTS social_chat_record_immutable
        BEFORE UPDATE OF owner_id,group_id,message_id,operation,request_hash,document_json ON social_chat_records
        BEGIN SELECT RAISE(ABORT,'chat record is immutable'); END;
    CREATE TRIGGER IF NOT EXISTS social_chat_record_revoke_final
        BEFORE UPDATE OF revoked ON social_chat_records WHEN OLD.revoked=1 AND NEW.revoked!=1
        BEGIN SELECT RAISE(ABORT,'chat record revocation is final'); END;")?;
    Ok(())
}
