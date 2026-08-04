use anyhow::Result;
use rusqlite::Connection;

pub(crate) fn migration_v195(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS compute_attempt_settlements (
           settlement_receipt_id    TEXT PRIMARY KEY,
           lease_id                 TEXT NOT NULL UNIQUE,
           finalization_id          TEXT NOT NULL UNIQUE,
           finalization_event_digest TEXT NOT NULL CHECK(length(finalization_event_digest) = 64),
           execution_receipt_id     TEXT NOT NULL UNIQUE,
           execution_receipt_digest TEXT NOT NULL CHECK(length(execution_receipt_digest) = 64),
           budget_reservation_id    TEXT NOT NULL UNIQUE,
           price_snapshot_id        TEXT NOT NULL,
           price_snapshot_digest    TEXT NOT NULL CHECK(length(price_snapshot_digest) = 64),
           job_id                   TEXT NOT NULL,
           source_job_revision      INTEGER NOT NULL CHECK(source_job_revision > 0),
           source_job_digest        TEXT NOT NULL CHECK(length(source_job_digest) = 64),
           terminal_job_revision    INTEGER NOT NULL CHECK(terminal_job_revision > source_job_revision),
           terminal_job_digest      TEXT NOT NULL CHECK(length(terminal_job_digest) = 64),
           request_json             TEXT NOT NULL CHECK(length(trim(request_json)) > 0),
           request_digest           TEXT NOT NULL CHECK(length(request_digest) = 64),
           receipt_json             TEXT NOT NULL CHECK(length(trim(receipt_json)) > 0),
           event_digest             TEXT NOT NULL CHECK(length(event_digest) = 64),
           idempotency_scope        TEXT NOT NULL CHECK(length(trim(idempotency_scope)) > 0),
           idempotency_key          TEXT NOT NULL CHECK(length(trim(idempotency_key)) > 0),
           settled_by_user_id       TEXT NOT NULL CHECK(length(trim(settled_by_user_id)) > 0),
           settled_at               TEXT NOT NULL,
           created_at               TEXT NOT NULL,
           UNIQUE(idempotency_scope, idempotency_key),
           FOREIGN KEY(lease_id)
             REFERENCES compute_attempt_activations(lease_id) ON DELETE RESTRICT,
           FOREIGN KEY(finalization_id)
             REFERENCES compute_attempt_finalizations(finalization_id) ON DELETE RESTRICT,
           FOREIGN KEY(execution_receipt_id)
             REFERENCES compute_attempt_execution_receipts(execution_receipt_id) ON DELETE RESTRICT,
           FOREIGN KEY(budget_reservation_id)
             REFERENCES billing_reservations(id) ON DELETE RESTRICT,
           FOREIGN KEY(price_snapshot_id)
             REFERENCES compute_price_snapshots(snapshot_id) ON DELETE RESTRICT,
           FOREIGN KEY(job_id, source_job_revision)
             REFERENCES compute_job_versions(job_id, revision) ON DELETE RESTRICT,
           FOREIGN KEY(job_id, terminal_job_revision)
             REFERENCES compute_job_versions(job_id, revision) ON DELETE RESTRICT
         );
         CREATE INDEX IF NOT EXISTS idx_compute_attempt_settlements_issued
           ON compute_attempt_settlements(settled_at DESC, settlement_receipt_id);

         CREATE TABLE IF NOT EXISTS compute_settlement_postings (
           posting_id                TEXT PRIMARY KEY,
           settlement_receipt_id     TEXT NOT NULL UNIQUE,
           currency                  TEXT NOT NULL CHECK(currency = 'CNY'),
           consumer_charge_micros    INTEGER NOT NULL CHECK(consumer_charge_micros >= 0),
           consumer_refund_micros    INTEGER NOT NULL CHECK(consumer_refund_micros >= 0),
           provider_pending_micros   INTEGER NOT NULL CHECK(provider_pending_micros >= 0),
           platform_pending_micros   INTEGER NOT NULL CHECK(platform_pending_micros >= 0),
           posting_digest            TEXT NOT NULL CHECK(length(posting_digest) = 64),
           posted_at                 TEXT NOT NULL,
           FOREIGN KEY(settlement_receipt_id)
             REFERENCES compute_attempt_settlements(settlement_receipt_id)
             ON DELETE RESTRICT DEFERRABLE INITIALLY DEFERRED
         );

         CREATE TABLE IF NOT EXISTS compute_settlement_ledger_legs (
           posting_id             TEXT NOT NULL,
           line_no                INTEGER NOT NULL CHECK(line_no > 0),
           leg_kind               TEXT NOT NULL CHECK(leg_kind IN (
             'consumer_capture', 'consumer_refund',
             'provider_pending', 'platform_pending'
           )),
           account_id             TEXT NOT NULL CHECK(length(trim(account_id)) > 0),
           currency               TEXT NOT NULL CHECK(currency = 'CNY'),
           direction              TEXT NOT NULL CHECK(direction IN ('debit', 'credit', 'release')),
           amount_micros          INTEGER NOT NULL CHECK(amount_micros >= 0),
           balance_state          TEXT NOT NULL CHECK(balance_state IN ('preauthorization', 'pending')),
           balance_after_micros   INTEGER,
           PRIMARY KEY(posting_id, line_no),
           FOREIGN KEY(posting_id)
             REFERENCES compute_settlement_postings(posting_id) ON DELETE RESTRICT
         );

         CREATE TABLE IF NOT EXISTS compute_settlement_account_balances (
           account_kind       TEXT NOT NULL CHECK(account_kind IN ('provider', 'platform')),
           account_id         TEXT NOT NULL CHECK(length(trim(account_id)) > 0),
           currency           TEXT NOT NULL CHECK(currency = 'CNY'),
           pending_micros     INTEGER NOT NULL DEFAULT 0 CHECK(pending_micros >= 0),
           available_micros   INTEGER NOT NULL DEFAULT 0 CHECK(available_micros >= 0),
           disputed_micros    INTEGER NOT NULL DEFAULT 0 CHECK(disputed_micros >= 0),
           withdrawn_micros   INTEGER NOT NULL DEFAULT 0 CHECK(withdrawn_micros >= 0),
           revision           INTEGER NOT NULL DEFAULT 1 CHECK(revision > 0),
           updated_at         TEXT NOT NULL,
           PRIMARY KEY(account_kind, account_id, currency)
         );

         CREATE TRIGGER IF NOT EXISTS trg_compute_attempt_settlements_no_update
         BEFORE UPDATE ON compute_attempt_settlements
         BEGIN
           SELECT RAISE(ABORT, 'compute attempt settlements are append-only');
         END;
         CREATE TRIGGER IF NOT EXISTS trg_compute_attempt_settlements_no_delete
         BEFORE DELETE ON compute_attempt_settlements
         BEGIN
           SELECT RAISE(ABORT, 'compute attempt settlements are append-only');
         END;
         CREATE TRIGGER IF NOT EXISTS trg_compute_settlement_postings_no_update
         BEFORE UPDATE ON compute_settlement_postings
         BEGIN
           SELECT RAISE(ABORT, 'compute settlement postings are append-only');
         END;
         CREATE TRIGGER IF NOT EXISTS trg_compute_settlement_postings_no_delete
         BEFORE DELETE ON compute_settlement_postings
         BEGIN
           SELECT RAISE(ABORT, 'compute settlement postings are append-only');
         END;
         CREATE TRIGGER IF NOT EXISTS trg_compute_settlement_legs_no_update
         BEFORE UPDATE ON compute_settlement_ledger_legs
         BEGIN
           SELECT RAISE(ABORT, 'compute settlement ledger legs are append-only');
         END;
         CREATE TRIGGER IF NOT EXISTS trg_compute_settlement_legs_no_delete
         BEFORE DELETE ON compute_settlement_ledger_legs
         BEGIN
           SELECT RAISE(ABORT, 'compute settlement ledger legs are append-only');
         END;",
    )?;
    Ok(())
}
