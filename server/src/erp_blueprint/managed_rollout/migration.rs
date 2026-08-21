use anyhow::Result;
use rusqlite::Connection;

pub(crate) fn migration_v280(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE erp_managed_rollout_plans (
           id                       TEXT PRIMARY KEY,
           project_id               TEXT NOT NULL,
           instance_id              TEXT NOT NULL,
           merchant_id              TEXT NOT NULL,
           source_configuration_revision INTEGER NOT NULL CHECK(source_configuration_revision > 0),
           source_version_id        TEXT NOT NULL,
           plan_sha256              TEXT NOT NULL CHECK(length(plan_sha256) = 64),
           payload_json             TEXT NOT NULL,
           status                   TEXT NOT NULL DEFAULT 'planned' CHECK(status = 'planned'),
           created_by_user_id       TEXT NOT NULL,
           created_at               TEXT NOT NULL,
           UNIQUE(instance_id, plan_sha256),
           FOREIGN KEY(project_id) REFERENCES projects(id) ON DELETE CASCADE,
           FOREIGN KEY(instance_id) REFERENCES erp_instances(id) ON DELETE CASCADE,
           FOREIGN KEY(merchant_id) REFERENCES open_commerce_merchants(id) ON DELETE CASCADE,
           FOREIGN KEY(source_version_id) REFERENCES erp_blueprint_versions(id)
         );
         CREATE INDEX idx_erp_managed_rollout_project_instance
           ON erp_managed_rollout_plans(project_id, instance_id, created_at DESC, id DESC);",
    )?;
    Ok(())
}
