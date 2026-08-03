//! Explicit offline preflight jobs and bounded machine leases.

use anyhow::Result;
use rusqlite::Connection;

pub(crate) fn migration_v159(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS task_sui_preflight_jobs (
           id                    TEXT PRIMARY KEY,
           project_id            TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
           package_kind          TEXT NOT NULL CHECK(package_kind IN ('standard', 'correction')),
           projection_package_id TEXT NOT NULL,
           target_network        TEXT NOT NULL CHECK(target_network IN ('devnet', 'testnet', 'mainnet')),
           handoff_digest        TEXT NOT NULL,
           projection_digest     TEXT NOT NULL,
           status                TEXT NOT NULL CHECK(status IN (
                                   'pending', 'leased', 'completed', 'canceled', 'blocked'
                                 )),
           adapter_id            TEXT REFERENCES task_sui_preflight_adapters(id),
           credential_version    INTEGER,
           attempt_no            INTEGER NOT NULL DEFAULT 0 CHECK(attempt_no >= 0),
           lease_token_hash      TEXT UNIQUE,
           lease_token_hint      TEXT,
           lease_started_at      TEXT,
           lease_expires_at      TEXT,
           lease_deadline_at     TEXT,
           report_id             TEXT UNIQUE REFERENCES task_sui_preflight_reports(id),
           last_error            TEXT,
           created_by_user_id    TEXT NOT NULL REFERENCES users(id),
           completed_at          TEXT,
           canceled_at           TEXT,
           created_at            TEXT NOT NULL,
           updated_at            TEXT NOT NULL
         );
         CREATE UNIQUE INDEX IF NOT EXISTS idx_task_sui_preflight_job_active_package
           ON task_sui_preflight_jobs(project_id, package_kind, projection_package_id)
          WHERE status IN ('pending', 'leased');
         CREATE INDEX IF NOT EXISTS idx_task_sui_preflight_job_claim
           ON task_sui_preflight_jobs(project_id, status, created_at);
         CREATE INDEX IF NOT EXISTS idx_task_sui_preflight_job_adapter
           ON task_sui_preflight_jobs(adapter_id, status, lease_expires_at);",
    )?;
    Ok(())
}
