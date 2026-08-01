//! V1.3 records whether an ERP instance started from a new or existing project.

use anyhow::Result;
use rusqlite::Connection;

pub(crate) fn migration_v120(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "ALTER TABLE erp_instances
           ADD COLUMN onboarding_mode TEXT NOT NULL DEFAULT 'new_project'
             CHECK(onboarding_mode IN ('new_project', 'existing_project'));",
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migration_records_existing_project_onboarding_mode() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys=ON; CREATE TABLE projects(id TEXT PRIMARY KEY);")
            .unwrap();
        crate::erp_blueprint_migration::migration_v114(&conn).unwrap();
        crate::erp_blueprint_evolution_migration::migration_v115(&conn).unwrap();
        migration_v120(&conn).unwrap();

        let columns: Vec<String> = conn
            .prepare("PRAGMA table_info(erp_instances)")
            .unwrap()
            .query_map([], |row| row.get(1))
            .unwrap()
            .collect::<rusqlite::Result<_>>()
            .unwrap();
        assert!(columns.contains(&"onboarding_mode".into()));
    }
}
