use anyhow::Result;
use rusqlite::Connection;

pub(crate) fn migration_v197(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS compute_settlement_challenge_resolutions (
           resolution_id                  TEXT PRIMARY KEY,
           challenge_id                   TEXT NOT NULL UNIQUE,
           challenge_event_digest         TEXT NOT NULL CHECK(length(challenge_event_digest) = 64),
           settlement_receipt_id           TEXT NOT NULL UNIQUE,
           settlement_event_digest         TEXT NOT NULL CHECK(length(settlement_event_digest) = 64),
           lease_id                        TEXT NOT NULL UNIQUE,
           consumer_account_id             TEXT NOT NULL CHECK(length(trim(consumer_account_id)) > 0),
           provider_account_id             TEXT NOT NULL CHECK(length(trim(provider_account_id)) > 0),
           action                          TEXT NOT NULL CHECK(action IN ('withdrawn','accepted','rejected')),
           statement                       TEXT NOT NULL CHECK(length(trim(statement)) > 0),
           actor_user_id                   TEXT NOT NULL CHECK(length(trim(actor_user_id)) > 0),
           actor_role                      TEXT NOT NULL CHECK(actor_role IN ('consumer','platform_admin')),
           request_json                    TEXT NOT NULL CHECK(length(trim(request_json)) > 0),
           request_digest                  TEXT NOT NULL CHECK(length(request_digest) = 64),
           receipt_json                    TEXT NOT NULL CHECK(length(trim(receipt_json)) > 0),
           event_digest                    TEXT NOT NULL CHECK(length(event_digest) = 64),
           idempotency_scope               TEXT NOT NULL CHECK(length(trim(idempotency_scope)) > 0),
           idempotency_key                 TEXT NOT NULL CHECK(length(trim(idempotency_key)) > 0),
           resolved_at                     TEXT NOT NULL,
           created_at                      TEXT NOT NULL,
           UNIQUE(idempotency_scope, idempotency_key),
           FOREIGN KEY(challenge_id)
             REFERENCES compute_settlement_challenges(challenge_id) ON DELETE RESTRICT,
           FOREIGN KEY(settlement_receipt_id)
             REFERENCES compute_attempt_settlements(settlement_receipt_id) ON DELETE RESTRICT,
           FOREIGN KEY(lease_id)
             REFERENCES compute_attempt_activations(lease_id) ON DELETE RESTRICT
         );
         CREATE INDEX IF NOT EXISTS idx_compute_settlement_challenge_resolutions_time
           ON compute_settlement_challenge_resolutions(resolved_at DESC, resolution_id);
         CREATE TRIGGER IF NOT EXISTS trg_compute_settlement_challenge_resolutions_no_update
         BEFORE UPDATE ON compute_settlement_challenge_resolutions
         BEGIN
           SELECT RAISE(ABORT, 'compute settlement challenge resolutions are append-only');
         END;
         CREATE TRIGGER IF NOT EXISTS trg_compute_settlement_challenge_resolutions_no_delete
         BEFORE DELETE ON compute_settlement_challenge_resolutions
         BEGIN
           SELECT RAISE(ABORT, 'compute settlement challenge resolutions are append-only');
         END;",
    )?;
    Ok(())
}
