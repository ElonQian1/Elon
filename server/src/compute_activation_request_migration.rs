use anyhow::Result;
use rusqlite::Connection;

use crate::store_migrations::add_column_if_missing;

pub(crate) fn migration_v177(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS compute_activation_evidence_requests (
           request_id                       TEXT PRIMARY KEY,
           provider_id                      TEXT NOT NULL,
           pool_id                          TEXT NOT NULL,
           owner_user_id                    TEXT NOT NULL,
           expected_provider_policy_revision INTEGER NOT NULL CHECK(expected_provider_policy_revision > 0),
           expected_provider_digest         TEXT NOT NULL CHECK(length(expected_provider_digest) = 64),
           expected_capacity_epoch          INTEGER NOT NULL CHECK(expected_capacity_epoch > 0),
           expected_pool_revision           INTEGER NOT NULL CHECK(expected_pool_revision > 0),
           expected_pool_digest             TEXT NOT NULL CHECK(length(expected_pool_digest) = 64),
           node_binding_ref                 TEXT NOT NULL CHECK(length(trim(node_binding_ref)) > 0),
           ready_capability_digest          TEXT NOT NULL CHECK(length(ready_capability_digest) = 64),
           route_proof_digest               TEXT NOT NULL CHECK(length(route_proof_digest) = 64),
           hardware_observation_digest      TEXT NOT NULL CHECK(length(hardware_observation_digest) = 64),
           ledger_audit_digest              TEXT NOT NULL CHECK(length(ledger_audit_digest) = 64),
           status                           TEXT NOT NULL CHECK(status IN (
                                                'submitted', 'changes_requested', 'approved',
                                                'rejected', 'canceled', 'activated', 'superseded'
                                            )),
           idempotency_scope                TEXT NOT NULL CHECK(length(trim(idempotency_scope)) > 0),
           idempotency_key                  TEXT NOT NULL CHECK(length(trim(idempotency_key)) > 0),
           request_digest                   TEXT NOT NULL CHECK(length(request_digest) = 64),
           requested_at                     TEXT NOT NULL,
           reviewed_at                      TEXT,
           reviewed_by_user_id              TEXT,
           review_note                      TEXT,
           canceled_at                      TEXT,
           created_at                       TEXT NOT NULL,
           updated_at                       TEXT NOT NULL,
           UNIQUE(idempotency_scope, idempotency_key),
           FOREIGN KEY(provider_id) REFERENCES compute_providers(provider_id) ON DELETE RESTRICT,
           FOREIGN KEY(pool_id) REFERENCES compute_capacity_pools(pool_id) ON DELETE RESTRICT
         );
         CREATE UNIQUE INDEX IF NOT EXISTS idx_compute_activation_request_active_pool
           ON compute_activation_evidence_requests(provider_id, pool_id)
           WHERE status IN ('submitted', 'approved');
         CREATE INDEX IF NOT EXISTS idx_compute_activation_request_owner
           ON compute_activation_evidence_requests(
                owner_user_id, provider_id, pool_id, requested_at DESC, request_id
           );
         CREATE INDEX IF NOT EXISTS idx_compute_activation_request_review
           ON compute_activation_evidence_requests(status, requested_at ASC, request_id);",
    )?;
    Ok(())
}

pub(crate) fn migration_v178(conn: &Connection) -> Result<()> {
    add_column_if_missing(
        conn,
        "compute_activation_evidence_requests",
        "superseded_at",
        "superseded_at TEXT",
    )?;
    add_column_if_missing(
        conn,
        "compute_activation_evidence_requests",
        "superseded_by_user_id",
        "superseded_by_user_id TEXT",
    )?;
    add_column_if_missing(
        conn,
        "compute_activation_evidence_requests",
        "supersede_reason",
        "supersede_reason TEXT",
    )?;
    Ok(())
}
