use anyhow::Result;
use rusqlite::Connection;

pub(crate) fn migration_v140(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS open_commerce_consumer_portability_trust_keys (
           id                       TEXT PRIMARY KEY,
           destination_project_id   TEXT NOT NULL,
           consumer_user_id         TEXT NOT NULL,
           source_operator          TEXT NOT NULL,
           key_id                   TEXT NOT NULL,
           algorithm                TEXT NOT NULL,
           public_key_pem           TEXT NOT NULL,
           status                   TEXT NOT NULL,
           created_at               TEXT NOT NULL,
           revoked_at               TEXT,
           FOREIGN KEY(destination_project_id) REFERENCES projects(id) ON DELETE CASCADE,
           FOREIGN KEY(consumer_user_id) REFERENCES users(id) ON DELETE CASCADE,
           UNIQUE(destination_project_id, consumer_user_id, source_operator, key_id)
         );
         CREATE INDEX IF NOT EXISTS idx_open_commerce_portability_trust_keys_owner
           ON open_commerce_consumer_portability_trust_keys(
             destination_project_id, consumer_user_id, created_at DESC
           );",
    )?;
    crate::store_migrations::add_column_if_missing(
        conn,
        "open_commerce_consumer_portability_imports",
        "trust_status",
        "trust_status TEXT NOT NULL DEFAULT 'integrity_verified_source_untrusted'",
    )?;
    crate::store_migrations::add_column_if_missing(
        conn,
        "open_commerce_consumer_portability_imports",
        "signer_key_record_id",
        "signer_key_record_id TEXT",
    )?;
    crate::store_migrations::add_column_if_missing(
        conn,
        "open_commerce_consumer_portability_imports",
        "signature_algorithm",
        "signature_algorithm TEXT",
    )?;
    crate::store_migrations::add_column_if_missing(
        conn,
        "open_commerce_consumer_portability_imports",
        "signer_key_id",
        "signer_key_id TEXT",
    )?;
    crate::store_migrations::add_column_if_missing(
        conn,
        "open_commerce_consumer_portability_imports",
        "signature_base64",
        "signature_base64 TEXT",
    )?;
    crate::store_migrations::add_column_if_missing(
        conn,
        "open_commerce_consumer_portability_imports",
        "signature_verified_at",
        "signature_verified_at TEXT",
    )?;
    Ok(())
}
