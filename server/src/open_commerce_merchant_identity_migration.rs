use anyhow::Result;
use rusqlite::Connection;

pub(crate) fn migration_v143(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS open_commerce_merchant_identity_keys (
           id                       TEXT PRIMARY KEY,
           project_id               TEXT NOT NULL,
           merchant_id              TEXT NOT NULL,
           key_id                   TEXT NOT NULL,
           algorithm                TEXT NOT NULL,
           public_key_pem           TEXT NOT NULL,
           proof_signature_base64   TEXT NOT NULL,
           status                   TEXT NOT NULL,
           proof_verified_at        TEXT NOT NULL,
           created_by_user_id       TEXT NOT NULL,
           created_at               TEXT NOT NULL,
           revoked_at               TEXT,
           FOREIGN KEY(project_id) REFERENCES projects(id) ON DELETE CASCADE,
           FOREIGN KEY(merchant_id) REFERENCES open_commerce_merchants(id) ON DELETE CASCADE,
           FOREIGN KEY(created_by_user_id) REFERENCES users(id) ON DELETE CASCADE,
           UNIQUE(merchant_id, key_id)
         );
         CREATE INDEX IF NOT EXISTS idx_open_commerce_merchant_identity_keys_active
           ON open_commerce_merchant_identity_keys(merchant_id, status, created_at DESC);
         CREATE INDEX IF NOT EXISTS idx_open_commerce_merchant_identity_keys_key_id
           ON open_commerce_merchant_identity_keys(key_id, status);",
    )?;
    Ok(())
}
