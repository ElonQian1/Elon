use anyhow::Result;
use rusqlite::Connection;

pub(crate) fn migrate(conn: &Connection) -> Result<()> {
    conn.execute_batch("CREATE TABLE IF NOT EXISTS social_contents (
        id TEXT PRIMARY KEY, owner_id TEXT NOT NULL REFERENCES users(id),
        kind TEXT NOT NULL CHECK(length(kind)>0), draft_json TEXT NOT NULL,
        edit_version INTEGER NOT NULL CHECK(edit_version>0),
        status TEXT NOT NULL CHECK(status IN ('draft','published','withdrawn')),
        created_at TEXT NOT NULL, updated_at TEXT NOT NULL);
    CREATE INDEX IF NOT EXISTS social_contents_owner ON social_contents(owner_id,updated_at DESC,id);
    CREATE TABLE IF NOT EXISTS social_content_revisions (
        content_id TEXT NOT NULL REFERENCES social_contents(id), revision INTEGER NOT NULL,
        document_json TEXT NOT NULL, published_at TEXT NOT NULL, PRIMARY KEY(content_id,revision));
    CREATE TABLE IF NOT EXISTS social_content_distributions (
        content_id TEXT NOT NULL, revision INTEGER NOT NULL,
        target_kind TEXT NOT NULL CHECK(length(target_kind)>0), target_id TEXT NOT NULL,
        message_id TEXT UNIQUE REFERENCES friend_group_messages(id) ON DELETE CASCADE, created_at TEXT NOT NULL,
        CHECK(target_kind!='group' OR message_id IS NOT NULL),
        PRIMARY KEY(content_id,revision,target_kind,target_id),
        FOREIGN KEY(content_id,revision) REFERENCES social_content_revisions(content_id,revision));
    CREATE INDEX IF NOT EXISTS social_content_group_feed ON social_content_distributions(target_kind,target_id,created_at DESC);
    CREATE TRIGGER IF NOT EXISTS social_content_group_target BEFORE INSERT ON social_content_distributions
        WHEN NEW.target_kind='group' AND NOT EXISTS(SELECT 1 FROM friend_groups WHERE id=NEW.target_id)
        BEGIN SELECT RAISE(ABORT,'article group target does not exist'); END;
    CREATE TRIGGER IF NOT EXISTS social_content_group_cleanup AFTER DELETE ON friend_groups
        BEGIN DELETE FROM social_content_distributions WHERE target_kind='group' AND target_id=OLD.id; END;
    CREATE TABLE IF NOT EXISTS social_content_media (
        id TEXT PRIMARY KEY, owner_id TEXT NOT NULL REFERENCES users(id), sha256 TEXT NOT NULL,
        mime_type TEXT NOT NULL, bytes BLOB NOT NULL, thumbnail TEXT NOT NULL, created_at TEXT NOT NULL,
        UNIQUE(owner_id,sha256));
    CREATE TRIGGER IF NOT EXISTS social_content_revision_no_update BEFORE UPDATE ON social_content_revisions BEGIN SELECT RAISE(ABORT,'article revision is immutable'); END;
    CREATE TRIGGER IF NOT EXISTS social_content_revision_no_delete BEFORE DELETE ON social_content_revisions BEGIN SELECT RAISE(ABORT,'article revision is immutable'); END;
    CREATE TRIGGER IF NOT EXISTS social_content_media_no_update BEFORE UPDATE ON social_content_media BEGIN SELECT RAISE(ABORT,'article media is immutable'); END;")?;
    Ok(())
}
