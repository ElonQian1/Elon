use anyhow::Result;
use rusqlite::Connection;

pub(crate) fn migration_v200(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS compute_settlement_withdrawal_requests (
           withdrawal_id                   TEXT PRIMARY KEY,
           provider_id                    TEXT NOT NULL CHECK(length(trim(provider_id)) > 0),
           provider_policy_revision       INTEGER NOT NULL CHECK(provider_policy_revision > 0),
           provider_digest                TEXT NOT NULL CHECK(length(provider_digest) = 64),
           provider_account_id            TEXT NOT NULL CHECK(length(trim(provider_account_id)) > 0),
           owner_user_id                  TEXT NOT NULL CHECK(length(trim(owner_user_id)) > 0),
           currency                       TEXT NOT NULL CHECK(currency = 'CNY'),
           amount_micros                  INTEGER NOT NULL CHECK(amount_micros > 0),
           destination_kind               TEXT NOT NULL CHECK(destination_kind IN (
             'bank_account_vault_ref', 'digital_wallet_vault_ref',
             'sui_address_ref', 'other_vault_ref'
           )),
           destination_ref                TEXT NOT NULL CHECK(length(trim(destination_ref)) > 0),
           request_posting_id             TEXT NOT NULL UNIQUE,
           request_posting_digest         TEXT NOT NULL CHECK(length(request_posting_digest) = 64),
           request_json                   TEXT NOT NULL CHECK(length(trim(request_json)) > 0),
           request_digest                 TEXT NOT NULL CHECK(length(request_digest) = 64),
           receipt_json                   TEXT NOT NULL CHECK(length(trim(receipt_json)) > 0),
           event_digest                   TEXT NOT NULL CHECK(length(event_digest) = 64),
           idempotency_scope              TEXT NOT NULL CHECK(length(trim(idempotency_scope)) > 0),
           idempotency_key                TEXT NOT NULL CHECK(length(trim(idempotency_key)) > 0),
           requested_at                   TEXT NOT NULL,
           created_at                     TEXT NOT NULL,
           UNIQUE(idempotency_scope, idempotency_key),
           FOREIGN KEY(request_posting_id)
             REFERENCES compute_settlement_withdrawal_request_postings(posting_id)
             ON DELETE RESTRICT DEFERRABLE INITIALLY DEFERRED
         );
         CREATE INDEX IF NOT EXISTS idx_compute_settlement_withdrawals_provider_time
           ON compute_settlement_withdrawal_requests(
             provider_id, requested_at DESC, withdrawal_id DESC
           );
         CREATE INDEX IF NOT EXISTS idx_compute_settlement_withdrawals_account_time
           ON compute_settlement_withdrawal_requests(
             provider_account_id, requested_at DESC, withdrawal_id DESC
           );

         CREATE TABLE IF NOT EXISTS compute_settlement_withdrawal_request_postings (
           posting_id                TEXT PRIMARY KEY,
           withdrawal_id             TEXT NOT NULL UNIQUE,
           provider_account_id       TEXT NOT NULL CHECK(length(trim(provider_account_id)) > 0),
           currency                  TEXT NOT NULL CHECK(currency = 'CNY'),
           amount_micros             INTEGER NOT NULL CHECK(amount_micros > 0),
           posting_digest            TEXT NOT NULL CHECK(length(posting_digest) = 64),
           posted_at                 TEXT NOT NULL,
           FOREIGN KEY(withdrawal_id)
             REFERENCES compute_settlement_withdrawal_requests(withdrawal_id)
             ON DELETE RESTRICT DEFERRABLE INITIALLY DEFERRED
         );

         CREATE TABLE IF NOT EXISTS compute_settlement_withdrawal_request_ledger_legs (
           posting_id                    TEXT NOT NULL,
           line_no                       INTEGER NOT NULL CHECK(line_no > 0),
           account_kind                  TEXT NOT NULL CHECK(account_kind = 'provider'),
           leg_kind                      TEXT NOT NULL CHECK(leg_kind IN (
             'provider_available_withdrawal_reserve',
             'provider_withdrawn_reserve_credit'
           )),
           account_id                    TEXT NOT NULL CHECK(length(trim(account_id)) > 0),
           currency                      TEXT NOT NULL CHECK(currency = 'CNY'),
           direction                     TEXT NOT NULL CHECK(direction IN ('debit','credit')),
           amount_micros                 INTEGER NOT NULL CHECK(amount_micros > 0),
           balance_state                 TEXT NOT NULL CHECK(balance_state IN ('available','withdrawn')),
           balance_after_micros          INTEGER NOT NULL CHECK(balance_after_micros >= 0),
           account_revision_after        INTEGER NOT NULL CHECK(account_revision_after > 0),
           PRIMARY KEY(posting_id, line_no),
           FOREIGN KEY(posting_id)
             REFERENCES compute_settlement_withdrawal_request_postings(posting_id)
             ON DELETE RESTRICT
         );

         CREATE TRIGGER IF NOT EXISTS trg_compute_settlement_withdrawal_requests_no_update
         BEFORE UPDATE ON compute_settlement_withdrawal_requests
         BEGIN
           SELECT RAISE(ABORT, 'compute settlement withdrawal requests are append-only');
         END;
         CREATE TRIGGER IF NOT EXISTS trg_compute_settlement_withdrawal_requests_no_delete
         BEFORE DELETE ON compute_settlement_withdrawal_requests
         BEGIN
           SELECT RAISE(ABORT, 'compute settlement withdrawal requests are append-only');
         END;
         CREATE TRIGGER IF NOT EXISTS trg_compute_settlement_withdrawal_request_postings_no_update
         BEFORE UPDATE ON compute_settlement_withdrawal_request_postings
         BEGIN
           SELECT RAISE(ABORT, 'compute settlement withdrawal request postings are append-only');
         END;
         CREATE TRIGGER IF NOT EXISTS trg_compute_settlement_withdrawal_request_postings_no_delete
         BEFORE DELETE ON compute_settlement_withdrawal_request_postings
         BEGIN
           SELECT RAISE(ABORT, 'compute settlement withdrawal request postings are append-only');
         END;
         CREATE TRIGGER IF NOT EXISTS trg_compute_settlement_withdrawal_request_legs_no_update
         BEFORE UPDATE ON compute_settlement_withdrawal_request_ledger_legs
         BEGIN
           SELECT RAISE(ABORT, 'compute settlement withdrawal request legs are append-only');
         END;
         CREATE TRIGGER IF NOT EXISTS trg_compute_settlement_withdrawal_request_legs_no_delete
         BEFORE DELETE ON compute_settlement_withdrawal_request_ledger_legs
         BEGIN
           SELECT RAISE(ABORT, 'compute settlement withdrawal request legs are append-only');
         END;",
    )?;
    Ok(())
}
