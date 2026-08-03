use anyhow::Result;
use rusqlite::Connection;

pub(crate) fn migration_v139(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS open_commerce_consumer_portability_imports (
           id                       TEXT PRIMARY KEY,
           destination_project_id   TEXT NOT NULL,
           consumer_user_id         TEXT NOT NULL,
           source_operator          TEXT NOT NULL,
           source_project_id        TEXT NOT NULL,
           source_package_id        TEXT NOT NULL,
           source_package_schema    TEXT NOT NULL,
           envelope_sha256          TEXT NOT NULL,
           payload_sha256           TEXT NOT NULL,
           package_json             TEXT NOT NULL,
           imported_at              TEXT NOT NULL,
           FOREIGN KEY(destination_project_id) REFERENCES projects(id) ON DELETE CASCADE,
           FOREIGN KEY(consumer_user_id) REFERENCES users(id) ON DELETE CASCADE,
           UNIQUE(destination_project_id, consumer_user_id, envelope_sha256)
         );
         CREATE INDEX IF NOT EXISTS idx_open_commerce_portability_imports_owner
           ON open_commerce_consumer_portability_imports(
             destination_project_id, consumer_user_id, imported_at DESC
           );",
    )?;
    Ok(())
}
