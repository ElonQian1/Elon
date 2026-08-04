use anyhow::Result;
use rusqlite::Connection;

pub(crate) fn migration_v187(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS compute_attempt_aborts (
           abort_id                         TEXT PRIMARY KEY,
           lease_id                         TEXT NOT NULL UNIQUE,
           provider_id                      TEXT NOT NULL,
           consumer_account_id              TEXT NOT NULL,
           executor_abort_ref                TEXT NOT NULL CHECK(length(trim(executor_abort_ref)) > 0),
           reason_code                       TEXT NOT NULL CHECK(length(trim(reason_code)) > 0),
           fencing_generation                INTEGER NOT NULL CHECK(fencing_generation > 0),
           source_lease_revision             INTEGER NOT NULL CHECK(source_lease_revision > 0),
           source_lease_digest               TEXT NOT NULL CHECK(length(source_lease_digest) = 64),
           terminal_lease_revision           INTEGER NOT NULL CHECK(terminal_lease_revision = source_lease_revision + 1),
           terminal_lease_digest             TEXT NOT NULL CHECK(length(terminal_lease_digest) = 64),
           terminal_lease_json               TEXT NOT NULL CHECK(length(trim(terminal_lease_json)) > 0),
           job_id                            TEXT NOT NULL,
           source_job_revision               INTEGER NOT NULL CHECK(source_job_revision > 0),
           source_job_digest                 TEXT NOT NULL CHECK(length(source_job_digest) = 64),
           terminal_job_revision             INTEGER NOT NULL CHECK(terminal_job_revision = source_job_revision + 1),
           terminal_job_digest               TEXT NOT NULL CHECK(length(terminal_job_digest) = 64),
           reservation_id                    TEXT NOT NULL,
           source_reservation_revision       INTEGER NOT NULL CHECK(source_reservation_revision > 0),
           source_reservation_digest         TEXT NOT NULL CHECK(length(source_reservation_digest) = 64),
           terminal_reservation_revision     INTEGER NOT NULL CHECK(terminal_reservation_revision = source_reservation_revision + 1),
           terminal_reservation_digest       TEXT NOT NULL CHECK(length(terminal_reservation_digest) = 64),
           capacity_claim_id                 TEXT NOT NULL,
           source_claim_revision             INTEGER NOT NULL CHECK(source_claim_revision > 0),
           source_claim_digest               TEXT NOT NULL CHECK(length(source_claim_digest) = 64),
           terminal_claim_revision           INTEGER NOT NULL CHECK(terminal_claim_revision = source_claim_revision + 1),
           terminal_claim_digest             TEXT NOT NULL CHECK(length(terminal_claim_digest) = 64),
           budget_reservation_id             TEXT NOT NULL,
           budget_refunded_fen               INTEGER NOT NULL CHECK(budget_refunded_fen >= 0),
           budget_terminal_status            TEXT NOT NULL CHECK(budget_terminal_status = 'released_no_usage'),
           capacity_transaction_id           TEXT NOT NULL UNIQUE,
           capacity_transaction_digest       TEXT NOT NULL CHECK(length(capacity_transaction_digest) = 64),
           activation_request_digest         TEXT NOT NULL CHECK(length(activation_request_digest) = 64),
           request_digest                    TEXT NOT NULL CHECK(length(request_digest) = 64),
           event_digest                      TEXT NOT NULL CHECK(length(event_digest) = 64),
           idempotency_scope                 TEXT NOT NULL CHECK(length(trim(idempotency_scope)) > 0),
           idempotency_key                   TEXT NOT NULL CHECK(length(trim(idempotency_key)) > 0),
           aborted_by_user_id                TEXT NOT NULL CHECK(length(trim(aborted_by_user_id)) > 0),
           aborted_at                        TEXT NOT NULL,
           created_at                        TEXT NOT NULL,
           UNIQUE(idempotency_scope, idempotency_key),
           FOREIGN KEY(lease_id) REFERENCES compute_attempt_activations(lease_id) ON DELETE RESTRICT,
           FOREIGN KEY(provider_id) REFERENCES compute_providers(provider_id) ON DELETE RESTRICT,
           FOREIGN KEY(job_id) REFERENCES compute_jobs(job_id) ON DELETE RESTRICT,
           FOREIGN KEY(reservation_id) REFERENCES compute_reservations(reservation_id) ON DELETE RESTRICT,
           FOREIGN KEY(capacity_claim_id) REFERENCES compute_capacity_claims(claim_id) ON DELETE RESTRICT,
           FOREIGN KEY(capacity_transaction_id)
             REFERENCES compute_capacity_ledger_transactions(transaction_id) ON DELETE RESTRICT,
           FOREIGN KEY(budget_reservation_id) REFERENCES billing_reservations(id) ON DELETE RESTRICT
         );
         CREATE INDEX IF NOT EXISTS idx_compute_attempt_aborts_provider
           ON compute_attempt_aborts(provider_id, aborted_at DESC, abort_id);
         CREATE INDEX IF NOT EXISTS idx_compute_attempt_aborts_consumer
           ON compute_attempt_aborts(consumer_account_id, aborted_at DESC, abort_id);
         CREATE TRIGGER IF NOT EXISTS trg_compute_attempt_aborts_no_update
         BEFORE UPDATE ON compute_attempt_aborts
         BEGIN
           SELECT RAISE(ABORT, 'compute attempt aborts are append-only');
         END;
         CREATE TRIGGER IF NOT EXISTS trg_compute_attempt_aborts_no_delete
         BEFORE DELETE ON compute_attempt_aborts
         BEGIN
           SELECT RAISE(ABORT, 'compute attempt aborts are append-only');
         END;",
    )?;
    Ok(())
}
