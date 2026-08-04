use anyhow::Result;
use rusqlite::Connection;

pub(crate) fn migration_v198(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS compute_settlement_releases (
           release_id                         TEXT PRIMARY KEY,
           settlement_receipt_id              TEXT NOT NULL UNIQUE,
           settlement_event_digest            TEXT NOT NULL CHECK(length(settlement_event_digest) = 64),
           source_posting_id                   TEXT NOT NULL UNIQUE,
           source_posting_digest               TEXT NOT NULL CHECK(length(source_posting_digest) = 64),
           lease_id                            TEXT NOT NULL UNIQUE,
           consumer_account_id                 TEXT NOT NULL CHECK(length(trim(consumer_account_id)) > 0),
           provider_account_id                 TEXT NOT NULL CHECK(length(trim(provider_account_id)) > 0),
           provider_released_micros            INTEGER NOT NULL CHECK(provider_released_micros >= 0),
           platform_released_micros            INTEGER NOT NULL CHECK(platform_released_micros >= 0),
           challenge_deadline                  TEXT NOT NULL,
           challenge_gate_json                 TEXT NOT NULL CHECK(length(trim(challenge_gate_json)) > 0),
           challenge_gate_digest               TEXT NOT NULL CHECK(length(challenge_gate_digest) = 64),
           policy_id                           TEXT NOT NULL CHECK(length(trim(policy_id)) > 0),
           policy_version                      INTEGER NOT NULL CHECK(policy_version > 0),
           release_posting_id                  TEXT NOT NULL UNIQUE,
           release_posting_digest              TEXT NOT NULL CHECK(length(release_posting_digest) = 64),
           request_json                        TEXT NOT NULL CHECK(length(trim(request_json)) > 0),
           request_digest                      TEXT NOT NULL CHECK(length(request_digest) = 64),
           receipt_json                        TEXT NOT NULL CHECK(length(trim(receipt_json)) > 0),
           event_digest                        TEXT NOT NULL CHECK(length(event_digest) = 64),
           idempotency_scope                   TEXT NOT NULL CHECK(length(trim(idempotency_scope)) > 0),
           idempotency_key                     TEXT NOT NULL CHECK(length(trim(idempotency_key)) > 0),
           released_by_user_id                 TEXT NOT NULL CHECK(length(trim(released_by_user_id)) > 0),
           released_at                         TEXT NOT NULL,
           created_at                          TEXT NOT NULL,
           UNIQUE(idempotency_scope, idempotency_key),
           FOREIGN KEY(settlement_receipt_id)
             REFERENCES compute_attempt_settlements(settlement_receipt_id) ON DELETE RESTRICT,
           FOREIGN KEY(source_posting_id)
             REFERENCES compute_settlement_postings(posting_id) ON DELETE RESTRICT,
           FOREIGN KEY(lease_id)
             REFERENCES compute_attempt_activations(lease_id) ON DELETE RESTRICT
         );
         CREATE INDEX IF NOT EXISTS idx_compute_settlement_releases_time
           ON compute_settlement_releases(released_at DESC, release_id);

         CREATE TABLE IF NOT EXISTS compute_settlement_release_postings (
           posting_id                  TEXT PRIMARY KEY,
           release_id                  TEXT NOT NULL UNIQUE,
           settlement_receipt_id       TEXT NOT NULL UNIQUE,
           currency                    TEXT NOT NULL CHECK(currency = 'CNY'),
           provider_released_micros    INTEGER NOT NULL CHECK(provider_released_micros >= 0),
           platform_released_micros    INTEGER NOT NULL CHECK(platform_released_micros >= 0),
           posting_digest              TEXT NOT NULL CHECK(length(posting_digest) = 64),
           posted_at                   TEXT NOT NULL,
           FOREIGN KEY(release_id)
             REFERENCES compute_settlement_releases(release_id)
             ON DELETE RESTRICT DEFERRABLE INITIALLY DEFERRED,
           FOREIGN KEY(settlement_receipt_id)
             REFERENCES compute_attempt_settlements(settlement_receipt_id) ON DELETE RESTRICT
         );

         CREATE TABLE IF NOT EXISTS compute_settlement_release_ledger_legs (
           posting_id             TEXT NOT NULL,
           line_no                INTEGER NOT NULL CHECK(line_no > 0),
           account_kind           TEXT NOT NULL CHECK(account_kind IN ('provider','platform')),
           leg_kind               TEXT NOT NULL CHECK(leg_kind IN (
             'provider_pending_release', 'provider_available_credit',
             'platform_pending_release', 'platform_available_credit'
           )),
           account_id             TEXT NOT NULL CHECK(length(trim(account_id)) > 0),
           currency               TEXT NOT NULL CHECK(currency = 'CNY'),
           direction              TEXT NOT NULL CHECK(direction IN ('debit','credit')),
           amount_micros          INTEGER NOT NULL CHECK(amount_micros >= 0),
           balance_state          TEXT NOT NULL CHECK(balance_state IN ('pending','available')),
           balance_after_micros   INTEGER NOT NULL CHECK(balance_after_micros >= 0),
           account_revision_after INTEGER NOT NULL CHECK(account_revision_after > 0),
           PRIMARY KEY(posting_id, line_no),
           FOREIGN KEY(posting_id)
             REFERENCES compute_settlement_release_postings(posting_id) ON DELETE RESTRICT
         );

         CREATE TRIGGER IF NOT EXISTS trg_compute_settlement_releases_no_update
         BEFORE UPDATE ON compute_settlement_releases
         BEGIN
           SELECT RAISE(ABORT, 'compute settlement releases are append-only');
         END;
         CREATE TRIGGER IF NOT EXISTS trg_compute_settlement_releases_no_delete
         BEFORE DELETE ON compute_settlement_releases
         BEGIN
           SELECT RAISE(ABORT, 'compute settlement releases are append-only');
         END;
         CREATE TRIGGER IF NOT EXISTS trg_compute_settlement_release_postings_no_update
         BEFORE UPDATE ON compute_settlement_release_postings
         BEGIN
           SELECT RAISE(ABORT, 'compute settlement release postings are append-only');
         END;
         CREATE TRIGGER IF NOT EXISTS trg_compute_settlement_release_postings_no_delete
         BEFORE DELETE ON compute_settlement_release_postings
         BEGIN
           SELECT RAISE(ABORT, 'compute settlement release postings are append-only');
         END;
         CREATE TRIGGER IF NOT EXISTS trg_compute_settlement_release_legs_no_update
         BEFORE UPDATE ON compute_settlement_release_ledger_legs
         BEGIN
           SELECT RAISE(ABORT, 'compute settlement release ledger legs are append-only');
         END;
         CREATE TRIGGER IF NOT EXISTS trg_compute_settlement_release_legs_no_delete
         BEFORE DELETE ON compute_settlement_release_ledger_legs
         BEGIN
           SELECT RAISE(ABORT, 'compute settlement release ledger legs are append-only');
         END;",
    )?;
    Ok(())
}
