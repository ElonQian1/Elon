use anyhow::Result;
use rusqlite::Connection;

pub(crate) fn migration_v164(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS open_commerce_capability_source_links (
           id                  TEXT PRIMARY KEY,
           project_id          TEXT NOT NULL,
           merchant_id         TEXT NOT NULL,
           capability_id       TEXT NOT NULL,
           capability_version  INTEGER NOT NULL CHECK(capability_version > 0),
           integration_id      TEXT NOT NULL,
           sync_receipt_id     TEXT NOT NULL,
           data_domain         TEXT NOT NULL,
           revision            INTEGER NOT NULL DEFAULT 1 CHECK(revision > 0),
           linked_by_user_id   TEXT NOT NULL,
           created_at          TEXT NOT NULL,
           updated_at          TEXT NOT NULL,
           UNIQUE(capability_id),
           FOREIGN KEY(project_id) REFERENCES projects(id) ON DELETE CASCADE,
           FOREIGN KEY(merchant_id) REFERENCES open_commerce_merchants(id) ON DELETE CASCADE,
           FOREIGN KEY(capability_id)
             REFERENCES open_commerce_capabilities(id) ON DELETE CASCADE,
           FOREIGN KEY(integration_id)
             REFERENCES open_commerce_integrations(id) ON DELETE CASCADE,
           FOREIGN KEY(sync_receipt_id)
             REFERENCES open_commerce_sync_receipts(id) ON DELETE CASCADE
         );
         CREATE INDEX IF NOT EXISTS idx_open_commerce_capability_source_project
           ON open_commerce_capability_source_links(project_id, merchant_id, updated_at DESC);
         CREATE INDEX IF NOT EXISTS idx_open_commerce_capability_source_receipt
           ON open_commerce_capability_source_links(sync_receipt_id);",
    )?;
    Ok(())
}
