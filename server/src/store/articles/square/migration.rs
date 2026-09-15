use anyhow::Result;
use rusqlite::Connection;
pub(crate) fn migrate(conn: &Connection) -> Result<()> {
    conn.execute_batch("CREATE TABLE IF NOT EXISTS article_square_accounts (
      owner_id TEXT PRIMARY KEY REFERENCES users(id), label TEXT NOT NULL, encrypted_key TEXT NOT NULL,
      key_fingerprint TEXT NOT NULL, masked_key TEXT NOT NULL, generation INTEGER NOT NULL,
      verified_at INTEGER, updated_at INTEGER NOT NULL);
    CREATE TABLE IF NOT EXISTS article_square_jobs (
      id TEXT PRIMARY KEY, owner_id TEXT NOT NULL REFERENCES users(id), content_id TEXT NOT NULL,
      revision INTEGER NOT NULL, generation INTEGER NOT NULL, key_fingerprint TEXT NOT NULL,
      request_key TEXT NOT NULL, payload_hash TEXT NOT NULL, payload_json TEXT NOT NULL,
      status TEXT NOT NULL CHECK(status IN ('queued','preparing','submitting','published','failed','uncertain','cancelled')),
      scheduled_at INTEGER NOT NULL, created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL,
      attempts INTEGER NOT NULL DEFAULT 0, submitted_at INTEGER, post_id TEXT, message TEXT NOT NULL DEFAULT '',
      UNIQUE(owner_id,request_key), UNIQUE(owner_id,key_fingerprint,payload_hash),
      FOREIGN KEY(content_id,revision) REFERENCES social_content_revisions(content_id,revision));
    CREATE INDEX IF NOT EXISTS article_square_due ON article_square_jobs(status,scheduled_at,created_at);
    CREATE INDEX IF NOT EXISTS article_square_history ON article_square_jobs(owner_id,created_at DESC,id);
    CREATE TABLE IF NOT EXISTS article_square_requests (
      owner_id TEXT NOT NULL REFERENCES users(id), request_key TEXT NOT NULL, request_hash TEXT NOT NULL,
      job_id TEXT NOT NULL REFERENCES article_square_jobs(id), PRIMARY KEY(owner_id,request_key));
    CREATE TABLE IF NOT EXISTS article_square_uploads (
      id TEXT PRIMARY KEY, owner_id TEXT NOT NULL REFERENCES users(id), created_at INTEGER NOT NULL);
    CREATE INDEX IF NOT EXISTS article_square_upload_quota ON article_square_uploads(owner_id,created_at);
    CREATE TABLE IF NOT EXISTS article_square_submissions (
      id TEXT PRIMARY KEY, owner_id TEXT NOT NULL REFERENCES users(id), created_at INTEGER NOT NULL);
    CREATE INDEX IF NOT EXISTS article_square_submit_quota ON article_square_submissions(owner_id,created_at);
    CREATE TABLE IF NOT EXISTS article_square_videos (
      id TEXT PRIMARY KEY, owner_id TEXT NOT NULL REFERENCES users(id), bytes BLOB NOT NULL,
      mime_type TEXT NOT NULL, duration REAL NOT NULL, cover_id TEXT NOT NULL REFERENCES social_content_media(id),
      sha256 TEXT NOT NULL, created_at INTEGER NOT NULL, UNIQUE(owner_id,sha256));")?;
    Ok(())
}
