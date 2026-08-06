use anyhow::Result;
use rusqlite::Connection;

pub(crate) fn migration_v206(conn: &Connection) -> Result<()> {
    crate::store_migrations::add_column_if_missing(
        conn,
        "users",
        "password_login_enabled",
        "password_login_enabled INTEGER NOT NULL DEFAULT 1",
    )?;
    conn.execute_batch(
        "UPDATE users SET password_login_enabled = 0 WHERE password_hash = 'device-user';

         CREATE TABLE IF NOT EXISTS user_identities (
           id             TEXT PRIMARY KEY,
           user_id        TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
           provider       TEXT NOT NULL,
           issuer         TEXT NOT NULL,
           subject        TEXT NOT NULL,
           email          TEXT,
           display_name   TEXT,
           avatar_url     TEXT,
           created_at     TEXT NOT NULL,
           last_login_at  TEXT,
           UNIQUE(provider, issuer, subject)
         );
         CREATE INDEX IF NOT EXISTS idx_user_identities_user
           ON user_identities(user_id, created_at);

         CREATE TABLE IF NOT EXISTS auth_identity_challenges (
           id          TEXT PRIMARY KEY,
           provider    TEXT NOT NULL,
           mode        TEXT NOT NULL,
           user_id     TEXT,
           nonce_hash  TEXT NOT NULL,
           platform    TEXT NOT NULL,
           expires_at  TEXT NOT NULL,
           consumed_at TEXT,
           created_at  TEXT NOT NULL
         );
         CREATE INDEX IF NOT EXISTS idx_auth_identity_challenges_expiry
           ON auth_identity_challenges(expires_at, consumed_at);

         CREATE TABLE IF NOT EXISTS auth_identity_audit (
           id          TEXT PRIMARY KEY,
           user_id     TEXT,
           provider    TEXT NOT NULL,
           action      TEXT NOT NULL,
           outcome     TEXT NOT NULL,
           detail      TEXT,
           created_at  TEXT NOT NULL
         );
         CREATE INDEX IF NOT EXISTS idx_auth_identity_audit_user
           ON auth_identity_audit(user_id, created_at);",
    )?;
    Ok(())
}
