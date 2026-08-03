use anyhow::Result;
use rusqlite::Connection;

pub(crate) fn migration_v162(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS open_commerce_consumer_data_vault_items (
           id                    TEXT PRIMARY KEY,
           consumer_project_id   TEXT NOT NULL,
           consumer_user_id      TEXT NOT NULL,
           label                 TEXT NOT NULL,
           item_kind             TEXT NOT NULL,
           envelope_json         TEXT NOT NULL,
           ciphertext_sha256     TEXT NOT NULL,
           ciphertext_bytes      INTEGER NOT NULL,
           revision              INTEGER NOT NULL,
           created_at            TEXT NOT NULL,
           updated_at            TEXT NOT NULL,
           FOREIGN KEY(consumer_project_id) REFERENCES projects(id) ON DELETE CASCADE,
           FOREIGN KEY(consumer_user_id) REFERENCES users(id) ON DELETE CASCADE
         );
         CREATE INDEX IF NOT EXISTS idx_open_commerce_consumer_vault_owner
           ON open_commerce_consumer_data_vault_items(
             consumer_project_id, consumer_user_id, updated_at DESC
           );",
    )?;
    Ok(())
}
