use anyhow::Result;
use rusqlite::Connection;

pub(crate) fn migration_v207(conn: &Connection) -> Result<()> {
    for (table, column, definition) in [
        ("users", "password_changed_at", "password_changed_at TEXT"),
        ("sessions", "last_seen_at", "last_seen_at TEXT"),
        ("sessions", "revoked_at", "revoked_at TEXT"),
        ("sessions", "revocation_reason", "revocation_reason TEXT"),
        ("auth_identity_challenges", "request_id", "request_id TEXT"),
        (
            "auth_identity_challenges",
            "client_key_hash",
            "client_key_hash TEXT",
        ),
        ("auth_identity_audit", "request_id", "request_id TEXT"),
        ("auth_identity_audit", "reason_code", "reason_code TEXT"),
    ] {
        crate::store_migrations::add_column_if_missing(conn, table, column, definition)?;
    }

    conn.execute_batch(
        "UPDATE sessions SET last_seen_at = created_at WHERE last_seen_at IS NULL;

         CREATE TABLE IF NOT EXISTS account_recovery_codes (
           id          TEXT PRIMARY KEY,
           user_id     TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
           batch_id    TEXT NOT NULL,
           code_hash   TEXT NOT NULL UNIQUE,
           last_four   TEXT NOT NULL,
           created_at  TEXT NOT NULL,
           used_at     TEXT,
           revoked_at  TEXT
         );
         CREATE INDEX IF NOT EXISTS idx_account_recovery_codes_user
           ON account_recovery_codes(user_id, created_at DESC);

         CREATE TABLE IF NOT EXISTS account_security_requests (
           id          TEXT PRIMARY KEY,
           user_id     TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
           action      TEXT NOT NULL,
           request_id  TEXT NOT NULL,
           outcome     TEXT NOT NULL,
           created_at  TEXT NOT NULL,
           UNIQUE(user_id, action, request_id)
         );

         CREATE TABLE IF NOT EXISTS auth_security_audit (
           id          TEXT PRIMARY KEY,
           user_id     TEXT,
           action      TEXT NOT NULL,
           outcome     TEXT NOT NULL,
           session_id  TEXT,
           request_id  TEXT,
           reason_code TEXT,
           created_at  TEXT NOT NULL
         );
         CREATE INDEX IF NOT EXISTS idx_auth_security_audit_user
           ON auth_security_audit(user_id, created_at DESC);
         CREATE TRIGGER IF NOT EXISTS trg_auth_security_audit_no_update
           BEFORE UPDATE ON auth_security_audit BEGIN
             SELECT RAISE(ABORT, 'auth security audit is append-only');
           END;
         CREATE TRIGGER IF NOT EXISTS trg_auth_security_audit_no_delete
           BEFORE DELETE ON auth_security_audit BEGIN
             SELECT RAISE(ABORT, 'auth security audit is append-only');
           END;",
    )?;
    Ok(())
}
