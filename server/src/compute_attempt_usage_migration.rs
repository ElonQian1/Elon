use anyhow::Result;
use rusqlite::Connection;

pub(crate) fn migration_v188(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS compute_attempt_usage_declarations (
           snapshot_id                       TEXT PRIMARY KEY,
           lease_id                          TEXT NOT NULL,
           provider_id                       TEXT NOT NULL,
           consumer_account_id               TEXT NOT NULL,
           sequence_no                       INTEGER NOT NULL CHECK(sequence_no > 0),
           source_lease_revision             INTEGER NOT NULL CHECK(source_lease_revision > 0),
           source_lease_digest               TEXT NOT NULL CHECK(length(source_lease_digest) = 64),
           source_lease_status               TEXT NOT NULL CHECK(source_lease_status = 'running'),
           fencing_generation                INTEGER NOT NULL CHECK(fencing_generation > 0),
           job_id                            TEXT NOT NULL,
           job_revision                      INTEGER NOT NULL CHECK(job_revision > 0),
           job_digest                        TEXT NOT NULL CHECK(length(job_digest) = 64),
           reservation_id                    TEXT NOT NULL,
           reservation_revision              INTEGER NOT NULL CHECK(reservation_revision > 0),
           reservation_digest                TEXT NOT NULL CHECK(length(reservation_digest) = 64),
           capacity_claim_id                 TEXT NOT NULL,
           capacity_claim_revision           INTEGER NOT NULL CHECK(capacity_claim_revision > 0),
           capacity_claim_digest             TEXT NOT NULL CHECK(length(capacity_claim_digest) = 64),
           executor_usage_ref                TEXT NOT NULL CHECK(length(trim(executor_usage_ref)) > 0),
           cumulative_usage_json             TEXT NOT NULL CHECK(length(trim(cumulative_usage_json)) > 0),
           cumulative_usage_digest           TEXT NOT NULL CHECK(length(cumulative_usage_digest) = 64),
           reserved_contract_json            TEXT NOT NULL CHECK(length(trim(reserved_contract_json)) > 0),
           reserved_contract_digest          TEXT NOT NULL CHECK(length(reserved_contract_digest) = 64),
           overage_meters_json               TEXT NOT NULL CHECK(length(trim(overage_meters_json)) > 0),
           request_digest                    TEXT NOT NULL CHECK(length(request_digest) = 64),
           event_digest                      TEXT NOT NULL CHECK(length(event_digest) = 64),
           idempotency_scope                 TEXT NOT NULL CHECK(length(trim(idempotency_scope)) > 0),
           idempotency_key                   TEXT NOT NULL CHECK(length(trim(idempotency_key)) > 0),
           declared_by_user_id               TEXT NOT NULL CHECK(length(trim(declared_by_user_id)) > 0),
           declared_at                       TEXT NOT NULL,
           created_at                        TEXT NOT NULL,
           UNIQUE(lease_id, sequence_no),
           UNIQUE(idempotency_scope, idempotency_key),
           FOREIGN KEY(lease_id) REFERENCES compute_attempt_activations(lease_id) ON DELETE RESTRICT,
           FOREIGN KEY(provider_id) REFERENCES compute_providers(provider_id) ON DELETE RESTRICT,
           FOREIGN KEY(job_id) REFERENCES compute_jobs(job_id) ON DELETE RESTRICT,
           FOREIGN KEY(reservation_id) REFERENCES compute_reservations(reservation_id) ON DELETE RESTRICT,
           FOREIGN KEY(capacity_claim_id) REFERENCES compute_capacity_claims(claim_id) ON DELETE RESTRICT
         );
         CREATE INDEX IF NOT EXISTS idx_compute_attempt_usage_declarations_lease
           ON compute_attempt_usage_declarations(lease_id, sequence_no DESC);
         CREATE INDEX IF NOT EXISTS idx_compute_attempt_usage_declarations_provider
           ON compute_attempt_usage_declarations(provider_id, declared_at DESC, snapshot_id);
         CREATE INDEX IF NOT EXISTS idx_compute_attempt_usage_declarations_consumer
           ON compute_attempt_usage_declarations(consumer_account_id, declared_at DESC, snapshot_id);
         CREATE TRIGGER IF NOT EXISTS trg_compute_attempt_usage_declarations_no_update
         BEFORE UPDATE ON compute_attempt_usage_declarations
         BEGIN
           SELECT RAISE(ABORT, 'compute attempt usage declarations are append-only');
         END;
         CREATE TRIGGER IF NOT EXISTS trg_compute_attempt_usage_declarations_no_delete
         BEFORE DELETE ON compute_attempt_usage_declarations
         BEGIN
           SELECT RAISE(ABORT, 'compute attempt usage declarations are append-only');
         END;",
    )?;
    Ok(())
}
