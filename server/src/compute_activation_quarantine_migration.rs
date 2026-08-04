use anyhow::Result;
use rusqlite::Connection;

pub(crate) fn migration_v181(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS compute_activation_quarantines (
           quarantine_id                         TEXT PRIMARY KEY,
           application_id                        TEXT NOT NULL UNIQUE,
           request_id                            TEXT NOT NULL UNIQUE,
           provider_id                           TEXT NOT NULL,
           pool_id                               TEXT NOT NULL,
           application_digest                    TEXT NOT NULL CHECK(length(application_digest) = 64),
           previous_provider_policy_revision     INTEGER NOT NULL CHECK(previous_provider_policy_revision > 0),
           previous_provider_digest              TEXT NOT NULL CHECK(length(previous_provider_digest) = 64),
           quarantined_provider_policy_revision  INTEGER NOT NULL CHECK(quarantined_provider_policy_revision > 1),
           quarantined_provider_digest           TEXT NOT NULL CHECK(length(quarantined_provider_digest) = 64),
           capacity_epoch                        INTEGER NOT NULL CHECK(capacity_epoch > 0),
           pool_lifecycle_event_id                TEXT NOT NULL UNIQUE,
           reason                                TEXT NOT NULL CHECK(length(trim(reason)) > 0),
           quarantine_digest                     TEXT NOT NULL CHECK(length(quarantine_digest) = 64),
           idempotency_scope                     TEXT NOT NULL CHECK(length(trim(idempotency_scope)) > 0),
           idempotency_key                       TEXT NOT NULL CHECK(length(trim(idempotency_key)) > 0),
           quarantined_by_user_id                TEXT NOT NULL CHECK(length(trim(quarantined_by_user_id)) > 0),
           quarantined_at                        TEXT NOT NULL,
           created_at                            TEXT NOT NULL,
           UNIQUE(idempotency_scope, idempotency_key),
           FOREIGN KEY(application_id) REFERENCES compute_activation_applications(application_id) ON DELETE RESTRICT,
           FOREIGN KEY(request_id) REFERENCES compute_activation_evidence_requests(request_id) ON DELETE RESTRICT,
           FOREIGN KEY(provider_id) REFERENCES compute_providers(provider_id) ON DELETE RESTRICT,
           FOREIGN KEY(pool_id) REFERENCES compute_capacity_pools(pool_id) ON DELETE RESTRICT,
           FOREIGN KEY(pool_lifecycle_event_id)
             REFERENCES compute_capacity_pool_lifecycle_events(event_id) ON DELETE RESTRICT
         );
         CREATE INDEX IF NOT EXISTS idx_compute_activation_quarantines_provider
           ON compute_activation_quarantines(provider_id, quarantined_at DESC, quarantine_id);
         CREATE TRIGGER IF NOT EXISTS trg_compute_activation_quarantines_no_update
         BEFORE UPDATE ON compute_activation_quarantines
         BEGIN
           SELECT RAISE(ABORT, 'compute activation quarantines are append-only');
         END;
         CREATE TRIGGER IF NOT EXISTS trg_compute_activation_quarantines_no_delete
         BEFORE DELETE ON compute_activation_quarantines
         BEGIN
           SELECT RAISE(ABORT, 'compute activation quarantines are append-only');
         END;",
    )?;
    Ok(())
}
