use anyhow::Result;
use rusqlite::Connection;

pub(crate) fn migration_v141(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS open_commerce_consumer_portability_adoptions (
           id                       TEXT PRIMARY KEY,
           import_id                TEXT NOT NULL,
           destination_project_id   TEXT NOT NULL,
           consumer_user_id         TEXT NOT NULL,
           adoption_kind            TEXT NOT NULL,
           before_preferences_json  TEXT,
           before_revision          INTEGER,
           applied_preferences_json TEXT NOT NULL,
           resulting_revision       INTEGER NOT NULL,
           status                   TEXT NOT NULL,
           applied_at               TEXT NOT NULL,
           rolled_back_at           TEXT,
           rollback_revision        INTEGER,
           FOREIGN KEY(import_id) REFERENCES open_commerce_consumer_portability_imports(id) ON DELETE CASCADE,
           FOREIGN KEY(destination_project_id) REFERENCES projects(id) ON DELETE CASCADE,
           FOREIGN KEY(consumer_user_id) REFERENCES users(id) ON DELETE CASCADE
         );
         CREATE UNIQUE INDEX IF NOT EXISTS idx_open_commerce_portability_active_adoption
           ON open_commerce_consumer_portability_adoptions(
             import_id, destination_project_id, consumer_user_id, adoption_kind
           ) WHERE status='applied';
         CREATE INDEX IF NOT EXISTS idx_open_commerce_portability_adoptions_owner
           ON open_commerce_consumer_portability_adoptions(
             destination_project_id, consumer_user_id, applied_at DESC
           );",
    )?;
    Ok(())
}
