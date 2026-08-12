//! Adds the canonical open-commerce identity to an ERP instance configuration.

use anyhow::Result;
use rusqlite::Connection;

pub(crate) fn migration_v248(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "ALTER TABLE erp_instances
           ADD COLUMN open_commerce_merchant_id TEXT;
         CREATE UNIQUE INDEX IF NOT EXISTS idx_erp_instances_open_commerce_merchant
           ON erp_instances(open_commerce_merchant_id)
           WHERE open_commerce_merchant_id IS NOT NULL;",
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migration_adds_nullable_unique_merchant_identity() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys=ON; CREATE TABLE projects(id TEXT PRIMARY KEY);")
            .unwrap();
        crate::erp_blueprint_migration::migration_v114(&conn).unwrap();
        crate::erp_blueprint_evolution_migration::migration_v115(&conn).unwrap();
        crate::erp_instance_onboarding_migration::migration_v120(&conn).unwrap();
        migration_v248(&conn).unwrap();

        let columns: Vec<String> = conn
            .prepare("PRAGMA table_info(erp_instances)")
            .unwrap()
            .query_map([], |row| row.get(1))
            .unwrap()
            .collect::<rusqlite::Result<_>>()
            .unwrap();
        assert!(columns.contains(&"open_commerce_merchant_id".into()));
    }
}
