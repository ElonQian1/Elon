use anyhow::Result;
use rusqlite::Connection;

pub(crate) fn migrate(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS social_snapshot_operations (
        owner_id TEXT NOT NULL REFERENCES users(id), group_id TEXT NOT NULL,
        key_hash TEXT NOT NULL, request_hash TEXT NOT NULL,
        content_id TEXT NOT NULL UNIQUE REFERENCES social_contents(id),
        message_id TEXT NOT NULL, created_at TEXT NOT NULL,
        PRIMARY KEY(owner_id,group_id,key_hash));
    CREATE TABLE IF NOT EXISTS social_snapshot_asset_grants (
        owner_id TEXT NOT NULL REFERENCES users(id),
        group_id TEXT NOT NULL REFERENCES friend_groups(id) ON DELETE CASCADE,
        media_id TEXT NOT NULL REFERENCES social_content_media(id),
        PRIMARY KEY(owner_id,group_id,media_id));
    CREATE TRIGGER IF NOT EXISTS social_snapshot_identity_immutable
        BEFORE UPDATE OF owner_id,kind,draft_json,edit_version ON social_contents
        WHEN OLD.kind='ai_snapshot'
        BEGIN SELECT RAISE(ABORT,'snapshot identity is immutable'); END;
    CREATE TRIGGER IF NOT EXISTS social_snapshot_revocation_final
        BEFORE UPDATE OF status ON social_contents
        WHEN OLD.kind='ai_snapshot' AND OLD.status='withdrawn' AND NEW.status!='withdrawn'
        BEGIN SELECT RAISE(ABORT,'snapshot revocation is final'); END;
    CREATE TRIGGER IF NOT EXISTS social_snapshot_operation_immutable
        BEFORE UPDATE ON social_snapshot_operations
        BEGIN SELECT RAISE(ABORT,'snapshot operation is immutable'); END;",
    )?;
    Ok(())
}
