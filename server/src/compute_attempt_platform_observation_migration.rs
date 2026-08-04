use anyhow::Result;
use rusqlite::Connection;

pub(crate) fn migration_v191(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS compute_attempt_platform_observations (
           platform_observation_id             TEXT PRIMARY KEY,
           terminal_candidate_id              TEXT NOT NULL UNIQUE,
           terminal_candidate_event_digest    TEXT NOT NULL CHECK(length(terminal_candidate_event_digest) = 64),
           lease_id                           TEXT NOT NULL UNIQUE,
           provider_id                        TEXT NOT NULL,
           consumer_account_id                TEXT NOT NULL,
           source_lease_revision              INTEGER NOT NULL CHECK(source_lease_revision > 0),
           source_lease_digest                TEXT NOT NULL CHECK(length(source_lease_digest) = 64),
           fencing_generation                 INTEGER NOT NULL CHECK(fencing_generation > 0),
           job_id                             TEXT NOT NULL,
           job_revision                       INTEGER NOT NULL CHECK(job_revision > 0),
           job_digest                         TEXT NOT NULL CHECK(length(job_digest) = 64),
           reservation_id                     TEXT NOT NULL,
           reservation_revision               INTEGER NOT NULL CHECK(reservation_revision > 0),
           reservation_digest                 TEXT NOT NULL CHECK(length(reservation_digest) = 64),
           capacity_claim_id                  TEXT NOT NULL,
           capacity_claim_revision            INTEGER NOT NULL CHECK(capacity_claim_revision > 0),
           capacity_claim_digest              TEXT NOT NULL CHECK(length(capacity_claim_digest) = 64),
           final_usage_snapshot_id            TEXT NOT NULL,
           final_usage_sequence_no            INTEGER NOT NULL CHECK(final_usage_sequence_no > 0),
           final_provider_usage_digest        TEXT NOT NULL CHECK(length(final_provider_usage_digest) = 64),
           candidate_outcome                  TEXT NOT NULL CHECK(candidate_outcome IN ('succeeded','failed','canceled')),
           observation_source                 TEXT NOT NULL CHECK(observation_source IN ('control_plane','transport_gateway','server_metering')),
           observer_ref                       TEXT NOT NULL CHECK(length(trim(observer_ref)) > 0),
           observed_outcome                   TEXT NOT NULL CHECK(observed_outcome IN ('succeeded','failed','canceled','indeterminate')),
           cumulative_observed_usage_json     TEXT NOT NULL CHECK(length(trim(cumulative_observed_usage_json)) > 0),
           cumulative_observed_usage_digest   TEXT NOT NULL CHECK(length(cumulative_observed_usage_digest) = 64),
           variance_meters_json               TEXT NOT NULL CHECK(length(trim(variance_meters_json)) > 0),
           variance_meters_digest             TEXT NOT NULL CHECK(length(variance_meters_digest) = 64),
           evidence_refs_json                 TEXT NOT NULL CHECK(length(trim(evidence_refs_json)) > 0),
           evidence_refs_digest               TEXT NOT NULL CHECK(length(evidence_refs_digest) = 64),
           request_digest                     TEXT NOT NULL CHECK(length(request_digest) = 64),
           event_digest                       TEXT NOT NULL CHECK(length(event_digest) = 64),
           idempotency_scope                  TEXT NOT NULL CHECK(length(trim(idempotency_scope)) > 0),
           idempotency_key                    TEXT NOT NULL CHECK(length(trim(idempotency_key)) > 0),
           observed_by_user_id                TEXT NOT NULL CHECK(length(trim(observed_by_user_id)) > 0),
           observed_at                        TEXT NOT NULL,
           created_at                         TEXT NOT NULL,
           UNIQUE(idempotency_scope, idempotency_key),
           FOREIGN KEY(terminal_candidate_id) REFERENCES compute_attempt_terminal_candidates(terminal_candidate_id) ON DELETE RESTRICT,
           FOREIGN KEY(lease_id) REFERENCES compute_attempt_activations(lease_id) ON DELETE RESTRICT,
           FOREIGN KEY(provider_id) REFERENCES compute_providers(provider_id) ON DELETE RESTRICT,
           FOREIGN KEY(job_id) REFERENCES compute_jobs(job_id) ON DELETE RESTRICT,
           FOREIGN KEY(reservation_id) REFERENCES compute_reservations(reservation_id) ON DELETE RESTRICT,
           FOREIGN KEY(capacity_claim_id) REFERENCES compute_capacity_claims(claim_id) ON DELETE RESTRICT,
           FOREIGN KEY(final_usage_snapshot_id) REFERENCES compute_attempt_usage_declarations(snapshot_id) ON DELETE RESTRICT
         );
         CREATE INDEX IF NOT EXISTS idx_compute_attempt_platform_observations_consumer
           ON compute_attempt_platform_observations(consumer_account_id, observed_at DESC, platform_observation_id);
         CREATE INDEX IF NOT EXISTS idx_compute_attempt_platform_observations_provider
           ON compute_attempt_platform_observations(provider_id, observed_at DESC, platform_observation_id);
         CREATE TRIGGER IF NOT EXISTS trg_compute_attempt_platform_observations_no_update
         BEFORE UPDATE ON compute_attempt_platform_observations
         BEGIN
           SELECT RAISE(ABORT, 'compute attempt platform observations are append-only');
         END;
         CREATE TRIGGER IF NOT EXISTS trg_compute_attempt_platform_observations_no_delete
         BEFORE DELETE ON compute_attempt_platform_observations
         BEGIN
           SELECT RAISE(ABORT, 'compute attempt platform observations are append-only');
         END;",
    )?;
    Ok(())
}
