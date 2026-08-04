use anyhow::Result;
use rusqlite::Connection;

pub(crate) fn migration_v201(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS compute_settlement_withdrawal_terminals (
           terminal_id                      TEXT PRIMARY KEY,
           withdrawal_id                    TEXT NOT NULL UNIQUE,
           withdrawal_event_digest          TEXT NOT NULL CHECK(length(withdrawal_event_digest) = 64),
           request_posting_id                TEXT NOT NULL UNIQUE,
           request_posting_digest            TEXT NOT NULL CHECK(length(request_posting_digest) = 64),
           provider_id                       TEXT NOT NULL CHECK(length(trim(provider_id)) > 0),
           provider_account_id               TEXT NOT NULL CHECK(length(trim(provider_account_id)) > 0),
           owner_user_id                     TEXT NOT NULL CHECK(length(trim(owner_user_id)) > 0),
           currency                          TEXT NOT NULL CHECK(currency = 'CNY'),
           amount_micros                     INTEGER NOT NULL CHECK(amount_micros > 0),
           action                            TEXT NOT NULL CHECK(action IN (
             'cancelled', 'rejected', 'external_paid_attested'
           )),
           reason_code                       TEXT NOT NULL CHECK(length(trim(reason_code)) > 0),
           reason_detail                     TEXT,
           external_evidence_kind            TEXT,
           external_evidence_ref             TEXT,
           external_evidence_digest          TEXT,
           balance_returned_micros            INTEGER NOT NULL CHECK(balance_returned_micros >= 0),
           terminal_posting_id               TEXT NOT NULL UNIQUE,
           terminal_posting_digest           TEXT NOT NULL CHECK(length(terminal_posting_digest) = 64),
           request_json                      TEXT NOT NULL CHECK(length(trim(request_json)) > 0),
           request_digest                    TEXT NOT NULL CHECK(length(request_digest) = 64),
           receipt_json                      TEXT NOT NULL CHECK(length(trim(receipt_json)) > 0),
           event_digest                      TEXT NOT NULL CHECK(length(event_digest) = 64),
           idempotency_scope                 TEXT NOT NULL CHECK(length(trim(idempotency_scope)) > 0),
           idempotency_key                   TEXT NOT NULL CHECK(length(trim(idempotency_key)) > 0),
           actor_user_id                     TEXT NOT NULL CHECK(length(trim(actor_user_id)) > 0),
           actor_role                        TEXT NOT NULL CHECK(actor_role IN ('provider_owner','platform_admin')),
           terminal_at                       TEXT NOT NULL,
           created_at                        TEXT NOT NULL,
           UNIQUE(idempotency_scope, idempotency_key),
           CHECK (
             (action='cancelled' AND actor_role='provider_owner'
              AND balance_returned_micros=amount_micros
              AND external_evidence_kind IS NULL
              AND external_evidence_ref IS NULL
              AND external_evidence_digest IS NULL)
             OR
             (action='rejected' AND actor_role='platform_admin'
              AND balance_returned_micros=amount_micros
              AND external_evidence_kind IS NULL
              AND external_evidence_ref IS NULL
              AND external_evidence_digest IS NULL)
             OR
             (action='external_paid_attested' AND actor_role='platform_admin'
              AND balance_returned_micros=0
              AND external_evidence_kind IS NOT NULL
              AND external_evidence_ref IS NOT NULL
              AND external_evidence_digest IS NOT NULL
              AND length(external_evidence_digest)=64)
           ),
           FOREIGN KEY(withdrawal_id)
             REFERENCES compute_settlement_withdrawal_requests(withdrawal_id) ON DELETE RESTRICT,
           FOREIGN KEY(request_posting_id)
             REFERENCES compute_settlement_withdrawal_request_postings(posting_id) ON DELETE RESTRICT,
           FOREIGN KEY(terminal_posting_id)
             REFERENCES compute_settlement_withdrawal_terminal_postings(posting_id)
             ON DELETE RESTRICT DEFERRABLE INITIALLY DEFERRED
         );
         CREATE INDEX IF NOT EXISTS idx_compute_settlement_withdrawal_terminals_time
           ON compute_settlement_withdrawal_terminals(terminal_at DESC, terminal_id DESC);

         CREATE TABLE IF NOT EXISTS compute_settlement_withdrawal_terminal_postings (
           posting_id                   TEXT PRIMARY KEY,
           terminal_id                  TEXT NOT NULL UNIQUE,
           withdrawal_id                TEXT NOT NULL UNIQUE,
           provider_account_id          TEXT NOT NULL CHECK(length(trim(provider_account_id)) > 0),
           currency                     TEXT NOT NULL CHECK(currency = 'CNY'),
           action                       TEXT NOT NULL CHECK(action IN (
             'cancelled', 'rejected', 'external_paid_attested'
           )),
           amount_micros                INTEGER NOT NULL CHECK(amount_micros > 0),
           balance_returned_micros       INTEGER NOT NULL CHECK(balance_returned_micros >= 0),
           posting_digest               TEXT NOT NULL CHECK(length(posting_digest) = 64),
           posted_at                    TEXT NOT NULL,
           FOREIGN KEY(terminal_id)
             REFERENCES compute_settlement_withdrawal_terminals(terminal_id)
             ON DELETE RESTRICT DEFERRABLE INITIALLY DEFERRED,
           FOREIGN KEY(withdrawal_id)
             REFERENCES compute_settlement_withdrawal_requests(withdrawal_id) ON DELETE RESTRICT
         );

         CREATE TABLE IF NOT EXISTS compute_settlement_withdrawal_terminal_ledger_legs (
           posting_id                    TEXT NOT NULL,
           line_no                       INTEGER NOT NULL CHECK(line_no > 0),
           account_kind                  TEXT NOT NULL CHECK(account_kind = 'provider'),
           leg_kind                      TEXT NOT NULL CHECK(leg_kind IN (
             'provider_withdrawn_terminal_release',
             'provider_available_terminal_return'
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
             REFERENCES compute_settlement_withdrawal_terminal_postings(posting_id) ON DELETE RESTRICT
         );

         CREATE TRIGGER IF NOT EXISTS trg_compute_settlement_withdrawal_terminals_no_update
         BEFORE UPDATE ON compute_settlement_withdrawal_terminals
         BEGIN
           SELECT RAISE(ABORT, 'compute settlement withdrawal terminals are append-only');
         END;
         CREATE TRIGGER IF NOT EXISTS trg_compute_settlement_withdrawal_terminals_no_delete
         BEFORE DELETE ON compute_settlement_withdrawal_terminals
         BEGIN
           SELECT RAISE(ABORT, 'compute settlement withdrawal terminals are append-only');
         END;
         CREATE TRIGGER IF NOT EXISTS trg_compute_settlement_withdrawal_terminal_postings_no_update
         BEFORE UPDATE ON compute_settlement_withdrawal_terminal_postings
         BEGIN
           SELECT RAISE(ABORT, 'compute settlement withdrawal terminal postings are append-only');
         END;
         CREATE TRIGGER IF NOT EXISTS trg_compute_settlement_withdrawal_terminal_postings_no_delete
         BEFORE DELETE ON compute_settlement_withdrawal_terminal_postings
         BEGIN
           SELECT RAISE(ABORT, 'compute settlement withdrawal terminal postings are append-only');
         END;
         CREATE TRIGGER IF NOT EXISTS trg_compute_settlement_withdrawal_terminal_legs_no_update
         BEFORE UPDATE ON compute_settlement_withdrawal_terminal_ledger_legs
         BEGIN
           SELECT RAISE(ABORT, 'compute settlement withdrawal terminal legs are append-only');
         END;
         CREATE TRIGGER IF NOT EXISTS trg_compute_settlement_withdrawal_terminal_legs_no_delete
         BEFORE DELETE ON compute_settlement_withdrawal_terminal_ledger_legs
         BEGIN
           SELECT RAISE(ABORT, 'compute settlement withdrawal terminal legs are append-only');
         END;",
    )?;
    Ok(())
}
