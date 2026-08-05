use anyhow::Result;
use rusqlite::Connection;

pub(crate) fn migration_v204(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS compute_activation_recovery_plans (
           recovery_plan_id                   TEXT PRIMARY KEY,
           quarantine_id                      TEXT NOT NULL,
           application_id                     TEXT NOT NULL,
           request_id                         TEXT NOT NULL,
           provider_id                        TEXT NOT NULL,
           pool_id                            TEXT NOT NULL,
           expected_quarantine_digest         TEXT NOT NULL CHECK(length(expected_quarantine_digest) = 64),
           expected_provider_policy_revision  INTEGER NOT NULL CHECK(expected_provider_policy_revision > 1),
           expected_provider_digest           TEXT NOT NULL CHECK(length(expected_provider_digest) = 64),
           expected_capacity_epoch            INTEGER NOT NULL CHECK(expected_capacity_epoch > 0),
           expected_pool_revision             INTEGER NOT NULL CHECK(expected_pool_revision > 0),
           expected_pool_digest               TEXT NOT NULL CHECK(length(expected_pool_digest) = 64),
           target_provider_policy_revision    INTEGER NOT NULL CHECK(target_provider_policy_revision > 2),
           target_provider_digest             TEXT NOT NULL CHECK(length(target_provider_digest) = 64),
           target_provider_json               TEXT NOT NULL CHECK(length(trim(target_provider_json)) > 0),
           routing_digest                     TEXT NOT NULL CHECK(length(routing_digest) = 64),
           remediation_summary                TEXT NOT NULL CHECK(length(trim(remediation_summary)) > 0),
           evidence_refs_json                 TEXT NOT NULL CHECK(length(trim(evidence_refs_json)) > 0),
           evidence_refs_digest               TEXT NOT NULL CHECK(length(evidence_refs_digest) = 64),
           status                             TEXT NOT NULL CHECK(status IN ('prepared','applied','superseded')),
           plan_digest                        TEXT NOT NULL CHECK(length(plan_digest) = 64),
           idempotency_scope                  TEXT NOT NULL CHECK(length(trim(idempotency_scope)) > 0),
           idempotency_key                    TEXT NOT NULL CHECK(length(trim(idempotency_key)) > 0),
           prepared_by_user_id                TEXT NOT NULL CHECK(length(trim(prepared_by_user_id)) > 0),
           prepared_at                        TEXT NOT NULL,
           applied_at                         TEXT,
           superseded_at                      TEXT,
           created_at                         TEXT NOT NULL,
           updated_at                         TEXT NOT NULL,
           UNIQUE(idempotency_scope, idempotency_key),
           FOREIGN KEY(quarantine_id) REFERENCES compute_activation_quarantines(quarantine_id) ON DELETE RESTRICT,
           FOREIGN KEY(application_id) REFERENCES compute_activation_applications(application_id) ON DELETE RESTRICT,
           FOREIGN KEY(request_id) REFERENCES compute_activation_evidence_requests(request_id) ON DELETE RESTRICT,
           FOREIGN KEY(provider_id) REFERENCES compute_providers(provider_id) ON DELETE RESTRICT,
           FOREIGN KEY(pool_id) REFERENCES compute_capacity_pools(pool_id) ON DELETE RESTRICT
         );
         CREATE UNIQUE INDEX IF NOT EXISTS idx_compute_activation_recovery_plans_open
           ON compute_activation_recovery_plans(quarantine_id) WHERE status='prepared';
         CREATE INDEX IF NOT EXISTS idx_compute_activation_recovery_plans_request
           ON compute_activation_recovery_plans(request_id, prepared_at DESC, recovery_plan_id);

         CREATE TABLE IF NOT EXISTS compute_activation_recovery_reviews (
           recovery_review_id                 TEXT PRIMARY KEY,
           recovery_plan_id                   TEXT NOT NULL UNIQUE,
           request_id                         TEXT NOT NULL,
           plan_digest                        TEXT NOT NULL CHECK(length(plan_digest) = 64),
           prepared_by_user_id                TEXT NOT NULL CHECK(length(trim(prepared_by_user_id)) > 0),
           reviewed_by_user_id                TEXT NOT NULL CHECK(length(trim(reviewed_by_user_id)) > 0),
           review_note                        TEXT,
           request_digest                     TEXT NOT NULL CHECK(length(request_digest) = 64),
           review_digest                      TEXT NOT NULL CHECK(length(review_digest) = 64),
           idempotency_scope                  TEXT NOT NULL CHECK(length(trim(idempotency_scope)) > 0),
           idempotency_key                    TEXT NOT NULL CHECK(length(trim(idempotency_key)) > 0),
           reviewed_at                        TEXT NOT NULL,
           created_at                         TEXT NOT NULL,
           CHECK(prepared_by_user_id <> reviewed_by_user_id),
           UNIQUE(idempotency_scope, idempotency_key),
           FOREIGN KEY(recovery_plan_id) REFERENCES compute_activation_recovery_plans(recovery_plan_id) ON DELETE RESTRICT,
           FOREIGN KEY(request_id) REFERENCES compute_activation_evidence_requests(request_id) ON DELETE RESTRICT
         );
         CREATE TRIGGER IF NOT EXISTS trg_compute_activation_recovery_reviews_no_update
         BEFORE UPDATE ON compute_activation_recovery_reviews BEGIN
           SELECT RAISE(ABORT, 'compute activation recovery reviews are append-only');
         END;
         CREATE TRIGGER IF NOT EXISTS trg_compute_activation_recovery_reviews_no_delete
         BEFORE DELETE ON compute_activation_recovery_reviews BEGIN
           SELECT RAISE(ABORT, 'compute activation recovery reviews are append-only');
         END;

         CREATE TABLE IF NOT EXISTS compute_activation_recovery_applications (
           recovery_application_id            TEXT PRIMARY KEY,
           recovery_plan_id                   TEXT NOT NULL UNIQUE,
           recovery_review_id                 TEXT NOT NULL UNIQUE,
           quarantine_id                      TEXT NOT NULL UNIQUE,
           request_id                         TEXT NOT NULL UNIQUE,
           provider_id                        TEXT NOT NULL,
           pool_id                            TEXT NOT NULL,
           plan_digest                        TEXT NOT NULL CHECK(length(plan_digest) = 64),
           review_digest                      TEXT NOT NULL CHECK(length(review_digest) = 64),
           recovered_provider_policy_revision INTEGER NOT NULL CHECK(recovered_provider_policy_revision > 2),
           recovered_provider_digest          TEXT NOT NULL CHECK(length(recovered_provider_digest) = 64),
           capacity_epoch                     INTEGER NOT NULL CHECK(capacity_epoch > 0),
           pool_lifecycle_event_id             TEXT NOT NULL UNIQUE,
           application_digest                 TEXT NOT NULL CHECK(length(application_digest) = 64),
           idempotency_scope                  TEXT NOT NULL CHECK(length(trim(idempotency_scope)) > 0),
           idempotency_key                    TEXT NOT NULL CHECK(length(trim(idempotency_key)) > 0),
           applied_by_user_id                 TEXT NOT NULL CHECK(length(trim(applied_by_user_id)) > 0),
           applied_at                         TEXT NOT NULL,
           created_at                         TEXT NOT NULL,
           UNIQUE(idempotency_scope, idempotency_key),
           FOREIGN KEY(recovery_plan_id) REFERENCES compute_activation_recovery_plans(recovery_plan_id) ON DELETE RESTRICT,
           FOREIGN KEY(recovery_review_id) REFERENCES compute_activation_recovery_reviews(recovery_review_id) ON DELETE RESTRICT,
           FOREIGN KEY(quarantine_id) REFERENCES compute_activation_quarantines(quarantine_id) ON DELETE RESTRICT,
           FOREIGN KEY(request_id) REFERENCES compute_activation_evidence_requests(request_id) ON DELETE RESTRICT,
           FOREIGN KEY(provider_id) REFERENCES compute_providers(provider_id) ON DELETE RESTRICT,
           FOREIGN KEY(pool_id) REFERENCES compute_capacity_pools(pool_id) ON DELETE RESTRICT,
           FOREIGN KEY(pool_lifecycle_event_id) REFERENCES compute_capacity_pool_lifecycle_events(event_id) ON DELETE RESTRICT
         );
         CREATE TRIGGER IF NOT EXISTS trg_compute_activation_recovery_applications_no_update
         BEFORE UPDATE ON compute_activation_recovery_applications BEGIN
           SELECT RAISE(ABORT, 'compute activation recovery applications are append-only');
         END;
         CREATE TRIGGER IF NOT EXISTS trg_compute_activation_recovery_applications_no_delete
         BEFORE DELETE ON compute_activation_recovery_applications BEGIN
           SELECT RAISE(ABORT, 'compute activation recovery applications are append-only');
         END;",
    )?;
    Ok(())
}
