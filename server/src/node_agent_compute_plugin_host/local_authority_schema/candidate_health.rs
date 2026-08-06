/// Immutable healthy observations for candidates that remain in the staged slot.
///
/// The insert does not promote the candidate or mutate inventory. It only succeeds while the
/// exact staged candidate, authority fences, process owner and trusted-time high-water still match
/// the validated observation. Promotion must later consume one exact, unexpired receipt.
pub(super) const CANDIDATE_HEALTH_SCHEMA_V3: &str = r#"
CREATE TABLE candidate_health_receipts (
    health_id                   TEXT PRIMARY KEY CHECK (length(health_id) > 0),
    evaluation_id               TEXT NOT NULL UNIQUE CHECK (length(evaluation_id) > 0),
    candidate_token             TEXT NOT NULL,
    candidate_token_digest      TEXT NOT NULL CHECK (
        length(candidate_token_digest) = 64
        AND candidate_token_digest NOT GLOB '*[^0-9a-f]*'
    ),
    staging_id                  TEXT NOT NULL,
    staging_receipt_digest      TEXT NOT NULL CHECK (
        length(staging_receipt_digest) = 64
        AND staging_receipt_digest NOT GLOB '*[^0-9a-f]*'
    ),
    staging_run_digest          TEXT NOT NULL CHECK (
        length(staging_run_digest) = 64
        AND staging_run_digest NOT GLOB '*[^0-9a-f]*'
    ),
    health_observation_json     TEXT NOT NULL CHECK (
        length(health_observation_json) > 0
        AND length(health_observation_json) <= 131072
    ),
    health_observation_digest   TEXT NOT NULL UNIQUE CHECK (
        length(health_observation_digest) = 64
        AND health_observation_digest NOT GLOB '*[^0-9a-f]*'
    ),
    authority_state_revision    INTEGER NOT NULL CHECK (authority_state_revision > 0),
    inventory_revision          INTEGER NOT NULL CHECK (inventory_revision > 0),
    inventory_digest            TEXT NOT NULL CHECK (
        length(inventory_digest) = 64
        AND inventory_digest NOT GLOB '*[^0-9a-f]*'
    ),
    authority_epoch             INTEGER NOT NULL CHECK (authority_epoch > 0),
    process_owner_epoch         INTEGER NOT NULL CHECK (process_owner_epoch > 0),
    recorded_at_ms              INTEGER NOT NULL CHECK (recorded_at_ms > 0),
    expires_at_ms               INTEGER NOT NULL CHECK (expires_at_ms > recorded_at_ms),
    receipt_json                TEXT NOT NULL CHECK (
        length(receipt_json) > 0 AND length(receipt_json) <= 65536
    ),
    receipt_digest              TEXT NOT NULL UNIQUE CHECK (
        length(receipt_digest) = 64
        AND receipt_digest NOT GLOB '*[^0-9a-f]*'
    ),
    FOREIGN KEY (candidate_token)
        REFERENCES candidate_owners(candidate_token) ON DELETE RESTRICT,
    FOREIGN KEY (staging_id)
        REFERENCES candidate_staging_receipts(staging_id) ON DELETE RESTRICT
);

CREATE TRIGGER candidate_health_insert_fenced
BEFORE INSERT ON candidate_health_receipts
WHEN NOT EXISTS (
    SELECT 1
    FROM authority_meta AS meta
    JOIN candidate_staging_receipts AS staging
      ON staging.staging_id = NEW.staging_id
     AND staging.candidate_token = NEW.candidate_token
     AND staging.candidate_token_digest = NEW.candidate_token_digest
     AND staging.staging_run_digest = NEW.staging_run_digest
     AND staging.receipt_digest = NEW.staging_receipt_digest
    JOIN candidate_owners AS candidate
      ON candidate.candidate_token = staging.candidate_token
    WHERE meta.singleton = 1
      AND meta.clock_status = 'trusted'
      AND meta.sharing_enabled = 1
      AND meta.trusted_time_high_water_ms = NEW.recorded_at_ms
      AND meta.updated_at_ms = NEW.recorded_at_ms
      AND meta.state_revision = NEW.authority_state_revision
      AND meta.inventory_revision = NEW.inventory_revision
      AND meta.inventory_digest = NEW.inventory_digest
      AND meta.authority_epoch = NEW.authority_epoch
      AND meta.process_owner_epoch = NEW.process_owner_epoch
      AND candidate.state = 'owned'
      AND staging.authority_state_revision_after <= NEW.authority_state_revision
      AND staging.inventory_revision_after <= NEW.inventory_revision
      AND staging.authority_epoch_after <= NEW.authority_epoch
      AND staging.process_owner_epoch = NEW.process_owner_epoch
      AND staging.staged_at_ms < NEW.recorded_at_ms
      AND NEW.recorded_at_ms < NEW.expires_at_ms
      AND NOT EXISTS (
          SELECT 1 FROM candidate_health_receipts AS prior
          WHERE prior.candidate_token = NEW.candidate_token
            AND prior.expires_at_ms > NEW.recorded_at_ms
      )
)
BEGIN
    SELECT RAISE(ABORT, 'candidate health receipt lost its staged authority fence');
END;

CREATE TRIGGER candidate_health_update_forbidden
BEFORE UPDATE ON candidate_health_receipts
BEGIN
    SELECT RAISE(ABORT, 'candidate health receipt is immutable');
END;

CREATE TRIGGER candidate_health_delete_forbidden
BEFORE DELETE ON candidate_health_receipts
BEGIN
    SELECT RAISE(ABORT, 'candidate health receipt is immutable');
END;
"#;
