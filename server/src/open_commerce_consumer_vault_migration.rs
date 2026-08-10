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

pub(crate) fn migration_v220(conn: &Connection) -> Result<()> {
    if has_owner_scoped_primary_key(conn)? {
        conn.execute_batch(
            "CREATE INDEX IF NOT EXISTS idx_open_commerce_consumer_vault_owner
               ON open_commerce_consumer_data_vault_items(
                 consumer_project_id, consumer_user_id, updated_at DESC
               );",
        )?;
        return Ok(());
    }

    conn.execute_batch(
        "BEGIN IMMEDIATE;
         CREATE TABLE open_commerce_consumer_data_vault_items_v220 (
           id                    TEXT NOT NULL,
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
           PRIMARY KEY(consumer_project_id, consumer_user_id, id),
           FOREIGN KEY(consumer_project_id) REFERENCES projects(id) ON DELETE CASCADE,
           FOREIGN KEY(consumer_user_id) REFERENCES users(id) ON DELETE CASCADE
         );
         INSERT INTO open_commerce_consumer_data_vault_items_v220 (
           id, consumer_project_id, consumer_user_id, label, item_kind,
           envelope_json, ciphertext_sha256, ciphertext_bytes, revision,
           created_at, updated_at
         )
         SELECT id, consumer_project_id, consumer_user_id, label, item_kind,
                envelope_json, ciphertext_sha256, ciphertext_bytes, revision,
                created_at, updated_at
           FROM open_commerce_consumer_data_vault_items;
         DROP TABLE open_commerce_consumer_data_vault_items;
         ALTER TABLE open_commerce_consumer_data_vault_items_v220
           RENAME TO open_commerce_consumer_data_vault_items;
         CREATE INDEX idx_open_commerce_consumer_vault_owner
           ON open_commerce_consumer_data_vault_items(
             consumer_project_id, consumer_user_id, updated_at DESC
           );
         COMMIT;",
    )?;
    Ok(())
}

fn has_owner_scoped_primary_key(conn: &Connection) -> Result<bool> {
    let mut stmt = conn.prepare("PRAGMA table_info(open_commerce_consumer_data_vault_items)")?;
    let mut primary_key = stmt
        .query_map([], |row| {
            Ok((row.get::<_, String>(1)?, row.get::<_, i64>(5)?))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    primary_key.retain(|(_, position)| *position > 0);
    primary_key.sort_by_key(|(_, position)| *position);
    Ok(primary_key
        == [
            ("consumer_project_id".to_string(), 1),
            ("consumer_user_id".to_string(), 2),
            ("id".to_string(), 3),
        ])
}

#[cfg(test)]
#[path = "open_commerce_consumer_vault_migration_tests.rs"]
mod tests;
