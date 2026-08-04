use anyhow::Result;
use rusqlite::Connection;

pub(crate) fn migration_v179(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS compute_activation_plans (
           plan_id                            TEXT PRIMARY KEY,
           request_id                         TEXT NOT NULL UNIQUE,
           provider_id                        TEXT NOT NULL,
           pool_id                            TEXT NOT NULL,
           expected_request_digest            TEXT NOT NULL CHECK(length(expected_request_digest) = 64),
           expected_provider_policy_revision  INTEGER NOT NULL CHECK(expected_provider_policy_revision > 0),
           expected_provider_digest           TEXT NOT NULL CHECK(length(expected_provider_digest) = 64),
           expected_capacity_epoch            INTEGER NOT NULL CHECK(expected_capacity_epoch > 0),
           expected_pool_revision             INTEGER NOT NULL CHECK(expected_pool_revision > 0),
           expected_pool_digest               TEXT NOT NULL CHECK(length(expected_pool_digest) = 64),
           target_provider_policy_revision    INTEGER NOT NULL CHECK(target_provider_policy_revision > 1),
           target_provider_digest             TEXT NOT NULL CHECK(length(target_provider_digest) = 64),
           target_provider_json               TEXT NOT NULL CHECK(length(trim(target_provider_json)) > 0),
           endpoint_digest                    TEXT NOT NULL CHECK(length(endpoint_digest) = 64),
           status                             TEXT NOT NULL CHECK(status IN ('prepared', 'applied', 'superseded')),
           idempotency_scope                  TEXT NOT NULL CHECK(length(trim(idempotency_scope)) > 0),
           idempotency_key                    TEXT NOT NULL CHECK(length(trim(idempotency_key)) > 0),
           plan_digest                        TEXT NOT NULL CHECK(length(plan_digest) = 64),
           prepared_by_user_id                TEXT NOT NULL CHECK(length(trim(prepared_by_user_id)) > 0),
           prepared_at                        TEXT NOT NULL,
           applied_at                         TEXT,
           superseded_at                      TEXT,
           created_at                         TEXT NOT NULL,
           updated_at                         TEXT NOT NULL,
           UNIQUE(idempotency_scope, idempotency_key),
           FOREIGN KEY(request_id) REFERENCES compute_activation_evidence_requests(request_id) ON DELETE RESTRICT,
           FOREIGN KEY(provider_id) REFERENCES compute_providers(provider_id) ON DELETE RESTRICT,
           FOREIGN KEY(pool_id) REFERENCES compute_capacity_pools(pool_id) ON DELETE RESTRICT
         );
         CREATE INDEX IF NOT EXISTS idx_compute_activation_plans_status
           ON compute_activation_plans(status, prepared_at ASC, plan_id);",
    )?;
    Ok(())
}
