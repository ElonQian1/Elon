//! V1.1 lifecycle metadata for ERP catalog evolution and auditable upgrades.

use anyhow::Result;
use rusqlite::{params, Connection};
use serde_json::json;

pub(crate) fn migration_v115(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "ALTER TABLE erp_blueprints
           ADD COLUMN definition_revision INTEGER NOT NULL DEFAULT 1;
         ALTER TABLE erp_instances
           ADD COLUMN configuration_revision INTEGER NOT NULL DEFAULT 1;
         ALTER TABLE erp_instances
           ADD COLUMN bootstrap_matter_id TEXT;
         ALTER TABLE erp_upgrade_campaigns
           ADD COLUMN instance_revision INTEGER NOT NULL DEFAULT 1;
         ALTER TABLE erp_upgrade_campaigns
           ADD COLUMN adopted_instance_revision INTEGER;
         ALTER TABLE erp_upgrade_campaigns
           ADD COLUMN from_configuration_json TEXT NOT NULL
             DEFAULT '{\"theme_key\":\"\",\"enabled_modules\":[],\"plugins\":[]}';
         ALTER TABLE erp_upgrade_campaigns
           ADD COLUMN target_configuration_json TEXT NOT NULL
             DEFAULT '{\"theme_key\":\"\",\"enabled_modules\":[],\"plugins\":[]}';
         ALTER TABLE erp_upgrade_campaigns
           ADD COLUMN adoption_evidence_json TEXT;",
    )?;

    let mut stmt = conn.prepare(
        "SELECT c.id, c.status, i.theme_key, i.enabled_modules_json, i.plugins_json,
                i.configuration_revision
           FROM erp_upgrade_campaigns c
           JOIN erp_instances i ON i.id=c.instance_id",
    )?;
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, i64>(5)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    drop(stmt);
    for (id, status, theme_key, modules, plugins, revision) in rows {
        let snapshot = json!({
            "theme_key": theme_key,
            "enabled_modules": serde_json::from_str::<serde_json::Value>(&modules)?,
            "plugins": serde_json::from_str::<serde_json::Value>(&plugins)?,
        });
        conn.execute(
            "UPDATE erp_upgrade_campaigns
                SET instance_revision=?1, adopted_instance_revision=?2,
                    from_configuration_json=?3, target_configuration_json=?3
              WHERE id=?4",
            params![
                revision,
                matches!(status.as_str(), "adopted" | "rolled_back").then_some(revision),
                serde_json::to_string(&snapshot)?,
                id,
            ],
        )?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migration_adds_revision_and_upgrade_snapshot_columns() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys=ON; CREATE TABLE projects(id TEXT PRIMARY KEY);")
            .unwrap();
        crate::erp_blueprint_migration::migration_v114(&conn).unwrap();
        migration_v115(&conn).unwrap();
        let instance_columns: Vec<String> = conn
            .prepare("PRAGMA table_info(erp_instances)")
            .unwrap()
            .query_map([], |row| row.get(1))
            .unwrap()
            .collect::<rusqlite::Result<_>>()
            .unwrap();
        let upgrade_columns: Vec<String> = conn
            .prepare("PRAGMA table_info(erp_upgrade_campaigns)")
            .unwrap()
            .query_map([], |row| row.get(1))
            .unwrap()
            .collect::<rusqlite::Result<_>>()
            .unwrap();
        assert!(instance_columns.contains(&"configuration_revision".into()));
        assert!(instance_columns.contains(&"bootstrap_matter_id".into()));
        assert!(upgrade_columns.contains(&"adoption_evidence_json".into()));
    }
}
