use anyhow::Result;
use rusqlite::Connection;

pub(crate) fn migration_v192(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS compute_attempt_verification_decisions (
           verification_decision_id               TEXT PRIMARY KEY,
           terminal_candidate_id                  TEXT NOT NULL UNIQUE,
           terminal_candidate_event_digest        TEXT NOT NULL CHECK(length(terminal_candidate_event_digest) = 64),
           consumer_review_id                     TEXT NOT NULL UNIQUE,
           consumer_review_event_digest           TEXT NOT NULL CHECK(length(consumer_review_event_digest) = 64),
           platform_observation_id                TEXT NOT NULL UNIQUE,
           platform_observation_event_digest      TEXT NOT NULL CHECK(length(platform_observation_event_digest) = 64),
           lease_id                               TEXT NOT NULL UNIQUE,
           policy_id                              TEXT NOT NULL CHECK(length(trim(policy_id)) > 0),
           policy_version                         INTEGER NOT NULL CHECK(policy_version > 0),
           decision                               TEXT NOT NULL CHECK(decision IN ('accepted','rejected','disputed')),
           reason_codes_json                      TEXT NOT NULL CHECK(length(trim(reason_codes_json)) > 0),
           reason_codes_digest                    TEXT NOT NULL CHECK(length(reason_codes_digest) = 64),
           decision_ref                           TEXT NOT NULL CHECK(length(trim(decision_ref)) > 0),
           verified_usage_json                    TEXT NOT NULL CHECK(length(trim(verified_usage_json)) > 0),
           verified_usage_digest                  TEXT NOT NULL CHECK(length(verified_usage_digest) = 64),
           compensable_usage_json                 TEXT NOT NULL CHECK(length(trim(compensable_usage_json)) > 0),
           compensable_usage_digest               TEXT NOT NULL CHECK(length(compensable_usage_digest) = 64),
           request_digest                         TEXT NOT NULL CHECK(length(request_digest) = 64),
           event_digest                           TEXT NOT NULL CHECK(length(event_digest) = 64),
           idempotency_scope                      TEXT NOT NULL CHECK(length(trim(idempotency_scope)) > 0),
           idempotency_key                        TEXT NOT NULL CHECK(length(trim(idempotency_key)) > 0),
           decided_by_user_id                     TEXT NOT NULL CHECK(length(trim(decided_by_user_id)) > 0),
           decided_at                             TEXT NOT NULL,
           created_at                             TEXT NOT NULL,
           UNIQUE(idempotency_scope, idempotency_key),
           FOREIGN KEY(terminal_candidate_id) REFERENCES compute_attempt_terminal_candidates(terminal_candidate_id) ON DELETE RESTRICT,
           FOREIGN KEY(consumer_review_id) REFERENCES compute_attempt_consumer_reviews(consumer_review_id) ON DELETE RESTRICT,
           FOREIGN KEY(platform_observation_id) REFERENCES compute_attempt_platform_observations(platform_observation_id) ON DELETE RESTRICT,
           FOREIGN KEY(lease_id) REFERENCES compute_attempt_activations(lease_id) ON DELETE RESTRICT
         );
         CREATE INDEX IF NOT EXISTS idx_compute_attempt_verification_decisions_decided
           ON compute_attempt_verification_decisions(decision, decided_at DESC, verification_decision_id);
         CREATE TRIGGER IF NOT EXISTS trg_compute_attempt_verification_decisions_no_update
         BEFORE UPDATE ON compute_attempt_verification_decisions
         BEGIN
           SELECT RAISE(ABORT, 'compute attempt verification decisions are append-only');
         END;
         CREATE TRIGGER IF NOT EXISTS trg_compute_attempt_verification_decisions_no_delete
         BEFORE DELETE ON compute_attempt_verification_decisions
         BEGIN
           SELECT RAISE(ABORT, 'compute attempt verification decisions are append-only');
         END;",
    )?;
    Ok(())
}
