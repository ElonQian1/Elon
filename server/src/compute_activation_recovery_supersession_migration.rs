use anyhow::Result;
use rusqlite::Connection;

pub(crate) fn migration_v205(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS compute_activation_recovery_plan_supersessions (
           recovery_supersession_id TEXT PRIMARY KEY,
           recovery_plan_id         TEXT NOT NULL UNIQUE,
           quarantine_id            TEXT NOT NULL,
           request_id               TEXT NOT NULL,
           provider_id              TEXT NOT NULL,
           pool_id                  TEXT NOT NULL,
           plan_digest              TEXT NOT NULL CHECK(length(plan_digest) = 64),
           reason                   TEXT NOT NULL CHECK(length(trim(reason)) > 0),
           request_digest           TEXT NOT NULL CHECK(length(request_digest) = 64),
           supersession_digest      TEXT NOT NULL CHECK(length(supersession_digest) = 64),
           idempotency_scope        TEXT NOT NULL CHECK(length(trim(idempotency_scope)) > 0),
           idempotency_key          TEXT NOT NULL CHECK(length(trim(idempotency_key)) > 0),
           superseded_by_user_id    TEXT NOT NULL CHECK(length(trim(superseded_by_user_id)) > 0),
           superseded_at            TEXT NOT NULL,
           created_at               TEXT NOT NULL,
           UNIQUE(idempotency_scope, idempotency_key),
           FOREIGN KEY(recovery_plan_id) REFERENCES compute_activation_recovery_plans(recovery_plan_id) ON DELETE RESTRICT,
           FOREIGN KEY(quarantine_id) REFERENCES compute_activation_quarantines(quarantine_id) ON DELETE RESTRICT,
           FOREIGN KEY(request_id) REFERENCES compute_activation_evidence_requests(request_id) ON DELETE RESTRICT,
           FOREIGN KEY(provider_id) REFERENCES compute_providers(provider_id) ON DELETE RESTRICT,
           FOREIGN KEY(pool_id) REFERENCES compute_capacity_pools(pool_id) ON DELETE RESTRICT
         );
         CREATE INDEX IF NOT EXISTS idx_compute_activation_recovery_supersessions_request
           ON compute_activation_recovery_plan_supersessions(request_id, superseded_at DESC, recovery_supersession_id);
         CREATE TRIGGER IF NOT EXISTS trg_compute_activation_recovery_supersessions_no_update
         BEFORE UPDATE ON compute_activation_recovery_plan_supersessions BEGIN
           SELECT RAISE(ABORT, 'compute activation recovery supersessions are append-only');
         END;
         CREATE TRIGGER IF NOT EXISTS trg_compute_activation_recovery_supersessions_no_delete
         BEFORE DELETE ON compute_activation_recovery_plan_supersessions BEGIN
           SELECT RAISE(ABORT, 'compute activation recovery supersessions are append-only');
         END;",
    )?;
    Ok(())
}
