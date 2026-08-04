use anyhow::Result;
use rusqlite::Connection;

pub(crate) fn migration_v194(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS compute_attempt_finalizations (
           finalization_id          TEXT PRIMARY KEY,
           lease_id                TEXT NOT NULL UNIQUE,
           execution_receipt_id    TEXT NOT NULL UNIQUE,
           execution_receipt_digest TEXT NOT NULL CHECK(length(execution_receipt_digest) = 64),
           request_json            TEXT NOT NULL CHECK(length(trim(request_json)) > 0),
           request_digest          TEXT NOT NULL CHECK(length(request_digest) = 64),
           receipt_json            TEXT NOT NULL CHECK(length(trim(receipt_json)) > 0),
           event_digest            TEXT NOT NULL CHECK(length(event_digest) = 64),
           idempotency_scope       TEXT NOT NULL CHECK(length(trim(idempotency_scope)) > 0),
           idempotency_key         TEXT NOT NULL CHECK(length(trim(idempotency_key)) > 0),
           finalized_by_user_id    TEXT NOT NULL CHECK(length(trim(finalized_by_user_id)) > 0),
           effective_at            TEXT NOT NULL,
           finalized_at            TEXT NOT NULL,
           created_at              TEXT NOT NULL,
           UNIQUE(idempotency_scope, idempotency_key),
           FOREIGN KEY(lease_id)
             REFERENCES compute_attempt_activations(lease_id) ON DELETE RESTRICT,
           FOREIGN KEY(execution_receipt_id)
             REFERENCES compute_attempt_execution_receipts(execution_receipt_id) ON DELETE RESTRICT
         );
         CREATE INDEX IF NOT EXISTS idx_compute_attempt_finalizations_issued
           ON compute_attempt_finalizations(finalized_at DESC, finalization_id);
         CREATE TRIGGER IF NOT EXISTS trg_compute_attempt_finalizations_no_update
         BEFORE UPDATE ON compute_attempt_finalizations
         BEGIN
           SELECT RAISE(ABORT, 'compute attempt finalizations are append-only');
         END;
         CREATE TRIGGER IF NOT EXISTS trg_compute_attempt_finalizations_no_delete
         BEFORE DELETE ON compute_attempt_finalizations
         BEGIN
           SELECT RAISE(ABORT, 'compute attempt finalizations are append-only');
         END;",
    )?;
    Ok(())
}
