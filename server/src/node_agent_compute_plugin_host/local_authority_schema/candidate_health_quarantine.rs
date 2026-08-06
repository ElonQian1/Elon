/// Immutable terminal-health failure receipts. The insert atomically observes the authority after
/// its inventory slot has moved from `staged` to `failed`; it does not release candidate ownership
/// or authorize deletion, download, installation, promotion, execution or settlement.
pub(super) const CANDIDATE_HEALTH_QUARANTINE_SCHEMA_V3: &str = r#"
CREATE TABLE candidate_health_quarantine_receipts (
    quarantine_id                      TEXT PRIMARY KEY CHECK (length(quarantine_id) > 0),
    evaluation_id                      TEXT NOT NULL UNIQUE CHECK (length(evaluation_id) > 0),
    candidate_token                    TEXT NOT NULL UNIQUE,
    candidate_token_digest             TEXT NOT NULL CHECK (
        length(candidate_token_digest) = 64
        AND candidate_token_digest NOT GLOB '*[^0-9a-f]*'
    ),
    staging_id                          TEXT NOT NULL UNIQUE,
    staging_receipt_digest              TEXT NOT NULL CHECK (
        length(staging_receipt_digest) = 64
        AND staging_receipt_digest NOT GLOB '*[^0-9a-f]*'
    ),
    staging_run_digest                  TEXT NOT NULL CHECK (
        length(staging_run_digest) = 64
        AND staging_run_digest NOT GLOB '*[^0-9a-f]*'
    ),
    failure_observation_json            TEXT NOT NULL CHECK (
        length(failure_observation_json) > 0
        AND length(failure_observation_json) <= 131072
    ),
    failure_observation_digest          TEXT NOT NULL UNIQUE CHECK (
        length(failure_observation_digest) = 64
        AND failure_observation_digest NOT GLOB '*[^0-9a-f]*'
    ),
    authority_state_revision_before     INTEGER NOT NULL CHECK (
        authority_state_revision_before > 0
    ),
    authority_state_revision_after      INTEGER NOT NULL CHECK (
        authority_state_revision_after = authority_state_revision_before + 1
    ),
    inventory_revision_before           INTEGER NOT NULL CHECK (inventory_revision_before > 0),
    inventory_revision_after            INTEGER NOT NULL CHECK (
        inventory_revision_after = inventory_revision_before + 1
    ),
    inventory_digest_before             TEXT NOT NULL CHECK (
        length(inventory_digest_before) = 64
        AND inventory_digest_before NOT GLOB '*[^0-9a-f]*'
    ),
    inventory_digest_after              TEXT NOT NULL CHECK (
        length(inventory_digest_after) = 64
        AND inventory_digest_after NOT GLOB '*[^0-9a-f]*'
        AND inventory_digest_after <> inventory_digest_before
    ),
    authority_epoch_before              INTEGER NOT NULL CHECK (authority_epoch_before > 0),
    authority_epoch_after               INTEGER NOT NULL CHECK (
        authority_epoch_after = authority_epoch_before + 1
    ),
    process_owner_epoch                 INTEGER NOT NULL CHECK (process_owner_epoch > 0),
    trusted_time_high_water_ms_before   INTEGER NOT NULL CHECK (
        trusted_time_high_water_ms_before >= 0
    ),
    failed_at_ms                        INTEGER NOT NULL CHECK (
        failed_at_ms > trusted_time_high_water_ms_before
    ),
    slot_phase_after                    TEXT NOT NULL CHECK (slot_phase_after = 'failed'),
    receipt_json                        TEXT NOT NULL CHECK (
        length(receipt_json) > 0 AND length(receipt_json) <= 65536
    ),
    receipt_digest                      TEXT NOT NULL UNIQUE CHECK (
        length(receipt_digest) = 64
        AND receipt_digest NOT GLOB '*[^0-9a-f]*'
    ),
    FOREIGN KEY (candidate_token)
        REFERENCES candidate_owners(candidate_token) ON DELETE RESTRICT,
    FOREIGN KEY (staging_id)
        REFERENCES candidate_staging_receipts(staging_id) ON DELETE RESTRICT
);

CREATE TRIGGER candidate_health_quarantine_insert_fenced
BEFORE INSERT ON candidate_health_quarantine_receipts
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
      AND meta.trusted_time_high_water_ms = NEW.failed_at_ms
      AND meta.updated_at_ms = NEW.failed_at_ms
      AND meta.state_revision = NEW.authority_state_revision_after
      AND meta.inventory_revision = NEW.inventory_revision_after
      AND meta.inventory_digest = NEW.inventory_digest_after
      AND meta.authority_epoch = NEW.authority_epoch_after
      AND meta.process_owner_epoch = NEW.process_owner_epoch
      AND candidate.state = 'owned'
      AND staging.authority_state_revision_after <= NEW.authority_state_revision_before
      AND staging.inventory_revision_after <= NEW.inventory_revision_before
      AND staging.authority_epoch_after <= NEW.authority_epoch_before
      AND staging.process_owner_epoch = NEW.process_owner_epoch
      AND staging.staged_at_ms < NEW.failed_at_ms
      AND NOT EXISTS (
          SELECT 1 FROM candidate_health_receipts AS healthy
          WHERE healthy.candidate_token = NEW.candidate_token
            AND healthy.expires_at_ms > NEW.failed_at_ms
      )
)
BEGIN
    SELECT RAISE(ABORT, 'candidate quarantine receipt lost its failed authority fence');
END;

CREATE TRIGGER candidate_health_quarantine_update_forbidden
BEFORE UPDATE ON candidate_health_quarantine_receipts
BEGIN
    SELECT RAISE(ABORT, 'candidate quarantine receipt is immutable');
END;

CREATE TRIGGER candidate_health_quarantine_delete_forbidden
BEFORE DELETE ON candidate_health_quarantine_receipts
BEGIN
    SELECT RAISE(ABORT, 'candidate quarantine receipt is immutable');
END;
"#;
