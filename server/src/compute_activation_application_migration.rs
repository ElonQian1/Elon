use anyhow::Result;
use rusqlite::Connection;

pub(crate) fn migration_v180(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS compute_activation_applications (
           application_id                    TEXT PRIMARY KEY,
           plan_id                           TEXT NOT NULL UNIQUE,
           request_id                        TEXT NOT NULL UNIQUE,
           provider_id                       TEXT NOT NULL,
           pool_id                           TEXT NOT NULL,
           plan_digest                       TEXT NOT NULL CHECK(length(plan_digest) = 64),
           target_provider_policy_revision   INTEGER NOT NULL CHECK(target_provider_policy_revision > 1),
           target_provider_digest            TEXT NOT NULL CHECK(length(target_provider_digest) = 64),
           pool_lifecycle_event_id            TEXT NOT NULL UNIQUE,
           application_digest                TEXT NOT NULL CHECK(length(application_digest) = 64),
           idempotency_scope                 TEXT NOT NULL CHECK(length(trim(idempotency_scope)) > 0),
           idempotency_key                   TEXT NOT NULL CHECK(length(trim(idempotency_key)) > 0),
           applied_by_user_id                TEXT NOT NULL CHECK(length(trim(applied_by_user_id)) > 0),
           applied_at                        TEXT NOT NULL,
           created_at                        TEXT NOT NULL,
           UNIQUE(idempotency_scope, idempotency_key),
           FOREIGN KEY(plan_id) REFERENCES compute_activation_plans(plan_id) ON DELETE RESTRICT,
           FOREIGN KEY(request_id) REFERENCES compute_activation_evidence_requests(request_id) ON DELETE RESTRICT,
           FOREIGN KEY(provider_id) REFERENCES compute_providers(provider_id) ON DELETE RESTRICT,
           FOREIGN KEY(pool_id) REFERENCES compute_capacity_pools(pool_id) ON DELETE RESTRICT,
           FOREIGN KEY(pool_lifecycle_event_id)
             REFERENCES compute_capacity_pool_lifecycle_events(event_id) ON DELETE RESTRICT
         );
         CREATE INDEX IF NOT EXISTS idx_compute_activation_applications_provider
           ON compute_activation_applications(provider_id, applied_at DESC, application_id);
         CREATE TRIGGER IF NOT EXISTS trg_compute_activation_applications_no_update
         BEFORE UPDATE ON compute_activation_applications
         BEGIN
           SELECT RAISE(ABORT, 'compute activation applications are append-only');
         END;
         CREATE TRIGGER IF NOT EXISTS trg_compute_activation_applications_no_delete
         BEFORE DELETE ON compute_activation_applications
         BEGIN
           SELECT RAISE(ABORT, 'compute activation applications are append-only');
         END;",
    )?;
    Ok(())
}
