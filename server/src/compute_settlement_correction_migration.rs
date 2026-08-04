use anyhow::Result;
use rusqlite::Connection;

pub(crate) fn migration_v199(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS compute_settlement_corrections (
           correction_id                       TEXT PRIMARY KEY,
           challenge_id                        TEXT NOT NULL UNIQUE,
           challenge_event_digest              TEXT NOT NULL CHECK(length(challenge_event_digest) = 64),
           resolution_id                       TEXT NOT NULL UNIQUE,
           resolution_event_digest             TEXT NOT NULL CHECK(length(resolution_event_digest) = 64),
           settlement_receipt_id                TEXT NOT NULL UNIQUE,
           settlement_event_digest              TEXT NOT NULL CHECK(length(settlement_event_digest) = 64),
           lease_id                             TEXT NOT NULL UNIQUE,
           consumer_account_id                  TEXT NOT NULL CHECK(length(trim(consumer_account_id)) > 0),
           provider_account_id                  TEXT NOT NULL CHECK(length(trim(provider_account_id)) > 0),
           original_consumer_charge_micros      INTEGER NOT NULL CHECK(original_consumer_charge_micros >= 0),
           corrected_consumer_charge_micros     INTEGER NOT NULL CHECK(corrected_consumer_charge_micros >= 0),
           consumer_refund_micros               INTEGER NOT NULL CHECK(consumer_refund_micros > 0),
           original_provider_payable_micros     INTEGER NOT NULL CHECK(original_provider_payable_micros >= 0),
           corrected_provider_payable_micros    INTEGER NOT NULL CHECK(corrected_provider_payable_micros >= 0),
           provider_reversal_micros             INTEGER NOT NULL CHECK(provider_reversal_micros >= 0),
           original_platform_margin_micros      INTEGER NOT NULL CHECK(original_platform_margin_micros >= 0),
           corrected_platform_margin_micros     INTEGER NOT NULL CHECK(corrected_platform_margin_micros >= 0),
           platform_reversal_micros             INTEGER NOT NULL CHECK(platform_reversal_micros >= 0),
           statement                            TEXT NOT NULL CHECK(length(trim(statement)) > 0),
           evidence_refs_json                   TEXT NOT NULL CHECK(length(trim(evidence_refs_json)) > 0),
           evidence_refs_digest                 TEXT NOT NULL CHECK(length(evidence_refs_digest) = 64),
           policy_id                            TEXT NOT NULL CHECK(length(trim(policy_id)) > 0),
           policy_version                       INTEGER NOT NULL CHECK(policy_version > 0),
           posting_id                           TEXT NOT NULL UNIQUE,
           posting_digest                       TEXT NOT NULL CHECK(length(posting_digest) = 64),
           request_json                         TEXT NOT NULL CHECK(length(trim(request_json)) > 0),
           request_digest                       TEXT NOT NULL CHECK(length(request_digest) = 64),
           receipt_json                         TEXT NOT NULL CHECK(length(trim(receipt_json)) > 0),
           event_digest                         TEXT NOT NULL CHECK(length(event_digest) = 64),
           idempotency_scope                    TEXT NOT NULL CHECK(length(trim(idempotency_scope)) > 0),
           idempotency_key                      TEXT NOT NULL CHECK(length(trim(idempotency_key)) > 0),
           corrected_by_user_id                 TEXT NOT NULL CHECK(length(trim(corrected_by_user_id)) > 0),
           corrected_at                         TEXT NOT NULL,
           created_at                           TEXT NOT NULL,
           UNIQUE(idempotency_scope, idempotency_key),
           FOREIGN KEY(challenge_id)
             REFERENCES compute_settlement_challenges(challenge_id) ON DELETE RESTRICT,
           FOREIGN KEY(resolution_id)
             REFERENCES compute_settlement_challenge_resolutions(resolution_id) ON DELETE RESTRICT,
           FOREIGN KEY(settlement_receipt_id)
             REFERENCES compute_attempt_settlements(settlement_receipt_id) ON DELETE RESTRICT,
           FOREIGN KEY(lease_id)
             REFERENCES compute_attempt_activations(lease_id) ON DELETE RESTRICT
         );
         CREATE INDEX IF NOT EXISTS idx_compute_settlement_corrections_time
           ON compute_settlement_corrections(corrected_at DESC, correction_id);

         CREATE TABLE IF NOT EXISTS compute_settlement_correction_postings (
           posting_id                         TEXT PRIMARY KEY,
           correction_id                      TEXT NOT NULL UNIQUE,
           settlement_receipt_id              TEXT NOT NULL UNIQUE,
           currency                           TEXT NOT NULL CHECK(currency = 'CNY'),
           consumer_refund_micros              INTEGER NOT NULL CHECK(consumer_refund_micros > 0),
           provider_pending_reversal_micros    INTEGER NOT NULL CHECK(provider_pending_reversal_micros >= 0),
           platform_pending_reversal_micros    INTEGER NOT NULL CHECK(platform_pending_reversal_micros >= 0),
           posting_digest                     TEXT NOT NULL CHECK(length(posting_digest) = 64),
           posted_at                          TEXT NOT NULL,
           FOREIGN KEY(correction_id)
             REFERENCES compute_settlement_corrections(correction_id)
             ON DELETE RESTRICT DEFERRABLE INITIALLY DEFERRED,
           FOREIGN KEY(settlement_receipt_id)
             REFERENCES compute_attempt_settlements(settlement_receipt_id) ON DELETE RESTRICT
         );

         CREATE TABLE IF NOT EXISTS compute_settlement_correction_ledger_legs (
           posting_id             TEXT NOT NULL,
           line_no                INTEGER NOT NULL CHECK(line_no > 0),
           account_kind           TEXT NOT NULL CHECK(account_kind IN ('consumer','provider','platform')),
           leg_kind               TEXT NOT NULL CHECK(leg_kind IN (
             'consumer_correction_refund',
             'provider_pending_reversal',
             'platform_pending_reversal'
           )),
           account_id             TEXT NOT NULL CHECK(length(trim(account_id)) > 0),
           currency               TEXT NOT NULL CHECK(currency = 'CNY'),
           direction              TEXT NOT NULL CHECK(direction IN ('debit','credit')),
           amount_micros          INTEGER NOT NULL CHECK(amount_micros >= 0),
           balance_state          TEXT NOT NULL CHECK(balance_state IN ('consumer_balance','pending')),
           balance_after_micros   INTEGER NOT NULL CHECK(balance_after_micros >= 0),
           account_revision_after INTEGER CHECK(account_revision_after IS NULL OR account_revision_after > 0),
           PRIMARY KEY(posting_id, line_no),
           FOREIGN KEY(posting_id)
             REFERENCES compute_settlement_correction_postings(posting_id) ON DELETE RESTRICT
         );

         CREATE TRIGGER IF NOT EXISTS trg_compute_settlement_corrections_no_update
         BEFORE UPDATE ON compute_settlement_corrections
         BEGIN
           SELECT RAISE(ABORT, 'compute settlement corrections are append-only');
         END;
         CREATE TRIGGER IF NOT EXISTS trg_compute_settlement_corrections_no_delete
         BEFORE DELETE ON compute_settlement_corrections
         BEGIN
           SELECT RAISE(ABORT, 'compute settlement corrections are append-only');
         END;
         CREATE TRIGGER IF NOT EXISTS trg_compute_settlement_correction_postings_no_update
         BEFORE UPDATE ON compute_settlement_correction_postings
         BEGIN
           SELECT RAISE(ABORT, 'compute settlement correction postings are append-only');
         END;
         CREATE TRIGGER IF NOT EXISTS trg_compute_settlement_correction_postings_no_delete
         BEFORE DELETE ON compute_settlement_correction_postings
         BEGIN
           SELECT RAISE(ABORT, 'compute settlement correction postings are append-only');
         END;
         CREATE TRIGGER IF NOT EXISTS trg_compute_settlement_correction_legs_no_update
         BEFORE UPDATE ON compute_settlement_correction_ledger_legs
         BEGIN
           SELECT RAISE(ABORT, 'compute settlement correction ledger legs are append-only');
         END;
         CREATE TRIGGER IF NOT EXISTS trg_compute_settlement_correction_legs_no_delete
         BEFORE DELETE ON compute_settlement_correction_ledger_legs
         BEGIN
           SELECT RAISE(ABORT, 'compute settlement correction ledger legs are append-only');
         END;",
    )?;
    Ok(())
}
