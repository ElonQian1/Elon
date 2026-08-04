use anyhow::Result;
use rusqlite::Connection;

pub(crate) fn migration_v193(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS compute_attempt_execution_receipts (
           execution_receipt_id       TEXT PRIMARY KEY,
           verification_decision_id   TEXT NOT NULL UNIQUE,
           verification_event_digest  TEXT NOT NULL CHECK(length(verification_event_digest) = 64),
           lease_id                   TEXT NOT NULL UNIQUE,
           receipt_digest             TEXT NOT NULL CHECK(length(receipt_digest) = 64),
           receipt_json               TEXT NOT NULL CHECK(length(trim(receipt_json)) > 0),
           request_digest             TEXT NOT NULL CHECK(length(request_digest) = 64),
           idempotency_scope          TEXT NOT NULL CHECK(length(trim(idempotency_scope)) > 0),
           idempotency_key            TEXT NOT NULL CHECK(length(trim(idempotency_key)) > 0),
           issued_by_user_id          TEXT NOT NULL CHECK(length(trim(issued_by_user_id)) > 0),
           issued_at                  TEXT NOT NULL,
           created_at                 TEXT NOT NULL,
           UNIQUE(idempotency_scope, idempotency_key),
           FOREIGN KEY(verification_decision_id) REFERENCES compute_attempt_verification_decisions(verification_decision_id) ON DELETE RESTRICT,
           FOREIGN KEY(lease_id) REFERENCES compute_attempt_activations(lease_id) ON DELETE RESTRICT
         );
         CREATE INDEX IF NOT EXISTS idx_compute_attempt_execution_receipts_issued
           ON compute_attempt_execution_receipts(issued_at DESC, execution_receipt_id);
         CREATE TRIGGER IF NOT EXISTS trg_compute_attempt_execution_receipts_no_update
         BEFORE UPDATE ON compute_attempt_execution_receipts
         BEGIN
           SELECT RAISE(ABORT, 'compute attempt execution receipts are append-only');
         END;
         CREATE TRIGGER IF NOT EXISTS trg_compute_attempt_execution_receipts_no_delete
         BEFORE DELETE ON compute_attempt_execution_receipts
         BEGIN
           SELECT RAISE(ABORT, 'compute attempt execution receipts are append-only');
         END;",
    )?;
    Ok(())
}
