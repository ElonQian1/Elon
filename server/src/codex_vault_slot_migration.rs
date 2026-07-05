use anyhow::Result;
use rusqlite::Connection;

pub(crate) fn migration_v88(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS user_codex_credential_slots (
          slot_id             TEXT PRIMARY KEY,
          user_id             TEXT NOT NULL,
          auth_mode           TEXT NOT NULL,
          account_hint_hash   TEXT,
          source_device       TEXT,
          ciphertext_b64      TEXT NOT NULL,
          nonce_b64           TEXT NOT NULL,
          credential_version  INTEGER NOT NULL DEFAULT 1,
          status              TEXT NOT NULL DEFAULT 'active',
          priority            INTEGER NOT NULL DEFAULT 100,
          failure_count       INTEGER NOT NULL DEFAULT 0,
          last_backup_at      TEXT,
          last_lease_at       TEXT,
          last_failure_at     TEXT,
          last_success_at     TEXT,
          last_error          TEXT,
          created_at          TEXT NOT NULL,
          updated_at          TEXT NOT NULL,
          FOREIGN KEY (user_id) REFERENCES users(id)
        );

        CREATE UNIQUE INDEX IF NOT EXISTS idx_user_codex_slots_user_hint
          ON user_codex_credential_slots(user_id, account_hint_hash)
          WHERE account_hint_hash IS NOT NULL AND trim(account_hint_hash) != '' AND status != 'deleted';

        CREATE INDEX IF NOT EXISTS idx_user_codex_slots_pick
          ON user_codex_credential_slots(user_id, status, failure_count, priority, updated_at DESC);

        INSERT OR IGNORE INTO user_codex_credential_slots (
          slot_id, user_id, auth_mode, account_hint_hash, source_device,
          ciphertext_b64, nonce_b64, credential_version, status, priority,
          failure_count, last_backup_at, last_lease_at, created_at, updated_at
        )
        SELECT
          'legacy-' || user_id, user_id, auth_mode, account_hint_hash, source_device,
          ciphertext_b64, nonce_b64, credential_version, 'active', 100,
          0, last_backup_at, last_lease_at, created_at, updated_at
        FROM user_codex_credentials;
        "#,
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::migration_v88;
    use rusqlite::Connection;

    #[test]
    fn migration_backfills_legacy_codex_vault_slot() {
        let conn = Connection::open_in_memory().expect("db");
        conn.execute_batch(
            r#"
            CREATE TABLE users (
              id TEXT PRIMARY KEY
            );
            CREATE TABLE user_codex_credentials (
              user_id TEXT PRIMARY KEY,
              auth_mode TEXT NOT NULL,
              account_hint_hash TEXT,
              source_device TEXT,
              ciphertext_b64 TEXT NOT NULL,
              nonce_b64 TEXT NOT NULL,
              credential_version INTEGER NOT NULL DEFAULT 1,
              last_backup_at TEXT,
              last_lease_at TEXT,
              created_at TEXT NOT NULL,
              updated_at TEXT NOT NULL
            );
            INSERT INTO users (id) VALUES ('u1'), ('u2');
            INSERT INTO user_codex_credentials
              (user_id, auth_mode, account_hint_hash, source_device, ciphertext_b64, nonce_b64,
               credential_version, last_backup_at, last_lease_at, created_at, updated_at)
            VALUES
              ('u1', 'chatgpt', 'hint-a', 'pc', 'cipher', 'nonce', 3, 'backup', NULL, 'created', 'updated'),
              ('u2', 'chatgpt', 'hint-b', 'pc', 'cipher', 'nonce', 3, 'backup', NULL, 'created', 'updated');
            "#,
        )
        .expect("seed");

        migration_v88(&conn).expect("migration should apply");
        migration_v88(&conn).expect("migration should be idempotent");

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM user_codex_credential_slots",
                [],
                |row| row.get(0),
            )
            .expect("count");
        let hint: String = conn
            .query_row(
                "SELECT account_hint_hash FROM user_codex_credential_slots WHERE slot_id = 'legacy-u1'",
                [],
                |row| row.get(0),
            )
            .expect("hint");
        assert_eq!(count, 2);
        assert_eq!(hint, "hint-a");
    }
}
