use anyhow::Result;
use rusqlite::Connection;

mod attempt_dispatch;

pub(crate) fn migration_v185(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS compute_attempt_activations (
           lease_id                         TEXT PRIMARY KEY,
           reservation_id                   TEXT NOT NULL,
           job_id                           TEXT NOT NULL,
           provider_id                      TEXT NOT NULL,
           consumer_account_id              TEXT NOT NULL,
           executor_id                      TEXT NOT NULL,
           attempt_no                       INTEGER NOT NULL CHECK(attempt_no > 0),
           fencing_generation               INTEGER NOT NULL CHECK(fencing_generation > 0),
           executor_acceptance_ref           TEXT NOT NULL,
           budget_reservation_id             TEXT NOT NULL,
           budget_reserved_fen               INTEGER NOT NULL CHECK(budget_reserved_fen >= 0),
           source_job_revision               INTEGER NOT NULL CHECK(source_job_revision > 0),
           source_job_digest                 TEXT NOT NULL CHECK(length(source_job_digest) = 64),
           running_job_revision              INTEGER NOT NULL CHECK(running_job_revision > source_job_revision),
           running_job_digest                TEXT NOT NULL CHECK(length(running_job_digest) = 64),
           source_reservation_revision       INTEGER NOT NULL CHECK(source_reservation_revision > 0),
           source_reservation_digest         TEXT NOT NULL CHECK(length(source_reservation_digest) = 64),
           active_reservation_revision       INTEGER NOT NULL CHECK(active_reservation_revision > source_reservation_revision),
           active_reservation_digest         TEXT NOT NULL CHECK(length(active_reservation_digest) = 64),
           source_claim_revision             INTEGER NOT NULL CHECK(source_claim_revision > 0),
           capacity_claim_id                 TEXT NOT NULL,
           source_claim_digest               TEXT NOT NULL CHECK(length(source_claim_digest) = 64),
           active_claim_revision             INTEGER NOT NULL CHECK(active_claim_revision > source_claim_revision),
           active_claim_digest               TEXT NOT NULL CHECK(length(active_claim_digest) = 64),
           capacity_transaction_id           TEXT NOT NULL UNIQUE,
           capacity_transaction_digest       TEXT NOT NULL CHECK(length(capacity_transaction_digest) = 64),
           request_digest                    TEXT NOT NULL CHECK(length(request_digest) = 64),
           lease_digest                      TEXT NOT NULL CHECK(length(lease_digest) = 64),
           lease_json                        TEXT NOT NULL CHECK(length(trim(lease_json)) > 0),
           idempotency_scope                 TEXT NOT NULL CHECK(length(trim(idempotency_scope)) > 0),
           idempotency_key                   TEXT NOT NULL CHECK(length(trim(idempotency_key)) > 0),
           activated_by_user_id              TEXT NOT NULL CHECK(length(trim(activated_by_user_id)) > 0),
           activated_at                      TEXT NOT NULL,
           created_at                        TEXT NOT NULL,
           UNIQUE(idempotency_scope, idempotency_key),
           UNIQUE(job_id, attempt_no),
           UNIQUE(reservation_id, fencing_generation),
           FOREIGN KEY(reservation_id) REFERENCES compute_reservations(reservation_id) ON DELETE RESTRICT,
           FOREIGN KEY(job_id) REFERENCES compute_jobs(job_id) ON DELETE RESTRICT,
           FOREIGN KEY(provider_id) REFERENCES compute_providers(provider_id) ON DELETE RESTRICT,
           FOREIGN KEY(capacity_claim_id) REFERENCES compute_capacity_claims(claim_id) ON DELETE RESTRICT,
           FOREIGN KEY(capacity_transaction_id)
             REFERENCES compute_capacity_ledger_transactions(transaction_id) ON DELETE RESTRICT
         );
         CREATE INDEX IF NOT EXISTS idx_compute_attempt_activations_provider
           ON compute_attempt_activations(provider_id, activated_at DESC, lease_id);
         CREATE INDEX IF NOT EXISTS idx_compute_attempt_activations_consumer
           ON compute_attempt_activations(job_id, activated_at DESC, lease_id);
         CREATE TRIGGER IF NOT EXISTS trg_compute_attempt_activations_no_update
         BEFORE UPDATE ON compute_attempt_activations
         BEGIN
           SELECT RAISE(ABORT, 'compute attempt activations are append-only');
         END;
         CREATE TRIGGER IF NOT EXISTS trg_compute_attempt_activations_no_delete
         BEFORE DELETE ON compute_attempt_activations
         BEGIN
           SELECT RAISE(ABORT, 'compute attempt activations are append-only');
         END;",
    )?;
    Ok(())
}

pub(crate) fn migration_v211(conn: &Connection) -> Result<()> {
    attempt_dispatch::migration_v211(conn)
}
