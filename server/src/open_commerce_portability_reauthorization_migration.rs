use anyhow::Result;
use rusqlite::Connection;

pub(crate) fn migration_v142(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS open_commerce_portability_relationship_mappings (
           id                         TEXT PRIMARY KEY,
           destination_project_id     TEXT NOT NULL,
           consumer_user_id           TEXT NOT NULL,
           import_id                  TEXT NOT NULL,
           source_relationship_id     TEXT NOT NULL,
           source_merchant_id         TEXT NOT NULL,
           target_merchant_id         TEXT NOT NULL,
           target_merchant_project_id TEXT NOT NULL,
           status                     TEXT NOT NULL,
           created_at                 TEXT NOT NULL,
           revoked_at                 TEXT,
           FOREIGN KEY(destination_project_id) REFERENCES projects(id) ON DELETE CASCADE,
           FOREIGN KEY(consumer_user_id) REFERENCES users(id) ON DELETE CASCADE,
           FOREIGN KEY(import_id) REFERENCES open_commerce_consumer_portability_imports(id) ON DELETE CASCADE,
           FOREIGN KEY(target_merchant_id) REFERENCES open_commerce_merchants(id) ON DELETE CASCADE,
           FOREIGN KEY(target_merchant_project_id) REFERENCES projects(id) ON DELETE CASCADE
         );
         CREATE UNIQUE INDEX IF NOT EXISTS idx_open_commerce_active_relationship_mapping
           ON open_commerce_portability_relationship_mappings(
             destination_project_id, consumer_user_id, import_id, source_relationship_id
           ) WHERE status='active';
         CREATE INDEX IF NOT EXISTS idx_open_commerce_relationship_mapping_owner
           ON open_commerce_portability_relationship_mappings(
             destination_project_id, consumer_user_id, created_at DESC
           );",
    )?;
    Ok(())
}
