//! Offline-only Sui adapter identities and append-only preflight reports.

use anyhow::Result;
use rusqlite::Connection;

pub(crate) fn migration_v158(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS task_sui_preflight_adapters (
           id                         TEXT PRIMARY KEY,
           project_id                 TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
           display_name               TEXT NOT NULL,
           status                     TEXT NOT NULL CHECK(status IN ('active', 'disabled')),
           allowed_networks_json      TEXT NOT NULL,
           allowed_package_kinds_json TEXT NOT NULL,
           token_hash                 TEXT NOT NULL UNIQUE,
           token_hint                 TEXT NOT NULL,
           credential_version         INTEGER NOT NULL DEFAULT 1 CHECK(credential_version > 0),
           created_by_user_id         TEXT NOT NULL REFERENCES users(id),
           last_used_at               TEXT,
           expires_at                 TEXT NOT NULL,
           disabled_at                TEXT,
           created_at                 TEXT NOT NULL,
           updated_at                 TEXT NOT NULL
         );
         CREATE INDEX IF NOT EXISTS idx_task_sui_preflight_adapters_project
           ON task_sui_preflight_adapters(project_id, status, updated_at DESC);

         CREATE TABLE IF NOT EXISTS task_sui_preflight_reports (
           id                    TEXT PRIMARY KEY,
           project_id            TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
           adapter_id            TEXT NOT NULL REFERENCES task_sui_preflight_adapters(id),
           credential_version    INTEGER NOT NULL CHECK(credential_version > 0),
           package_kind          TEXT NOT NULL CHECK(package_kind IN ('standard', 'correction')),
           projection_package_id TEXT NOT NULL,
           target_network        TEXT NOT NULL CHECK(target_network IN ('devnet', 'testnet', 'mainnet')),
           handoff_digest        TEXT NOT NULL,
           projection_digest     TEXT NOT NULL,
           outcome               TEXT NOT NULL CHECK(outcome IN ('passed', 'rejected')),
           summary               TEXT NOT NULL,
           tool_version          TEXT NOT NULL,
           idempotency_key       TEXT NOT NULL,
           report_digest         TEXT NOT NULL,
           created_at            TEXT NOT NULL,
           UNIQUE(adapter_id, idempotency_key)
         );
         CREATE INDEX IF NOT EXISTS idx_task_sui_preflight_reports_project
           ON task_sui_preflight_reports(project_id, created_at DESC);
         CREATE INDEX IF NOT EXISTS idx_task_sui_preflight_reports_package
           ON task_sui_preflight_reports(
             project_id, package_kind, projection_package_id, created_at DESC
           );",
    )?;
    Ok(())
}
