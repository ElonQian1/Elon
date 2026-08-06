/// Two-phase failed-candidate cleanup journal. Authorization fences the owner before any physical
/// deletion. Completion is written only after an external linear executor reports exact success.
pub(super) const CANDIDATE_CLEANUP_SCHEMA_V3: &str = r#"
CREATE TABLE candidate_cleanup_authorizations (
    cleanup_id                          TEXT PRIMARY KEY CHECK (length(cleanup_id) > 0),
    candidate_token                    TEXT NOT NULL UNIQUE,
    candidate_token_digest             TEXT NOT NULL CHECK (
        length(candidate_token_digest) = 64
        AND candidate_token_digest NOT GLOB '*[^0-9a-f]*'
    ),
    quarantine_id                      TEXT NOT NULL UNIQUE,
    quarantine_receipt_digest          TEXT NOT NULL UNIQUE CHECK (
        length(quarantine_receipt_digest) = 64
        AND quarantine_receipt_digest NOT GLOB '*[^0-9a-f]*'
    ),
    staging_id                          TEXT NOT NULL UNIQUE,
    staging_run_digest                  TEXT NOT NULL CHECK (
        length(staging_run_digest) = 64
        AND staging_run_digest NOT GLOB '*[^0-9a-f]*'
    ),
    authority_state_revision_before    INTEGER NOT NULL CHECK (
        authority_state_revision_before > 0
    ),
    authority_state_revision_after     INTEGER NOT NULL CHECK (
        authority_state_revision_after = authority_state_revision_before + 1
    ),
    inventory_revision                 INTEGER NOT NULL CHECK (inventory_revision > 0),
    inventory_digest                   TEXT NOT NULL CHECK (
        length(inventory_digest) = 64
        AND inventory_digest NOT GLOB '*[^0-9a-f]*'
    ),
    authority_epoch_before             INTEGER NOT NULL CHECK (authority_epoch_before > 0),
    authority_epoch_after              INTEGER NOT NULL CHECK (
        authority_epoch_after = authority_epoch_before + 1
    ),
    process_owner_epoch                INTEGER NOT NULL CHECK (process_owner_epoch > 0),
    trusted_time_high_water_ms_before  INTEGER NOT NULL CHECK (
        trusted_time_high_water_ms_before >= 0
    ),
    authorized_at_ms                   INTEGER NOT NULL CHECK (
        authorized_at_ms > trusted_time_high_water_ms_before
    ),
    slot_phase_before                  TEXT NOT NULL CHECK (slot_phase_before = 'failed'),
    receipt_json                       TEXT NOT NULL CHECK (
        length(receipt_json) > 0 AND length(receipt_json) <= 65536
    ),
    receipt_digest                     TEXT NOT NULL UNIQUE CHECK (
        length(receipt_digest) = 64
        AND receipt_digest NOT GLOB '*[^0-9a-f]*'
    ),
    UNIQUE (cleanup_id, candidate_token, receipt_digest),
    FOREIGN KEY (candidate_token)
        REFERENCES candidate_owners(candidate_token) ON DELETE RESTRICT,
    FOREIGN KEY (quarantine_id)
        REFERENCES candidate_health_quarantine_receipts(quarantine_id) ON DELETE RESTRICT,
    FOREIGN KEY (staging_id)
        REFERENCES candidate_staging_receipts(staging_id) ON DELETE RESTRICT
);

CREATE TABLE candidate_cleanup_completions (
    completion_id                      TEXT PRIMARY KEY CHECK (length(completion_id) > 0),
    cleanup_id                         TEXT NOT NULL UNIQUE,
    candidate_token                    TEXT NOT NULL UNIQUE,
    authorization_receipt_digest       TEXT NOT NULL UNIQUE CHECK (
        length(authorization_receipt_digest) = 64
        AND authorization_receipt_digest NOT GLOB '*[^0-9a-f]*'
    ),
    execution_evidence_digest          TEXT NOT NULL UNIQUE CHECK (
        length(execution_evidence_digest) = 64
        AND execution_evidence_digest NOT GLOB '*[^0-9a-f]*'
    ),
    authority_state_revision_before    INTEGER NOT NULL CHECK (
        authority_state_revision_before > 0
    ),
    authority_state_revision_after     INTEGER NOT NULL CHECK (
        authority_state_revision_after = authority_state_revision_before + 1
    ),
    inventory_revision_before          INTEGER NOT NULL CHECK (inventory_revision_before > 0),
    inventory_revision_after           INTEGER NOT NULL CHECK (
        inventory_revision_after = inventory_revision_before + 1
    ),
    inventory_digest_before            TEXT NOT NULL CHECK (
        length(inventory_digest_before) = 64
        AND inventory_digest_before NOT GLOB '*[^0-9a-f]*'
    ),
    inventory_digest_after             TEXT NOT NULL CHECK (
        length(inventory_digest_after) = 64
        AND inventory_digest_after NOT GLOB '*[^0-9a-f]*'
        AND inventory_digest_after <> inventory_digest_before
    ),
    authority_epoch_before             INTEGER NOT NULL CHECK (authority_epoch_before > 0),
    authority_epoch_after              INTEGER NOT NULL CHECK (
        authority_epoch_after = authority_epoch_before + 1
    ),
    process_owner_epoch                INTEGER NOT NULL CHECK (process_owner_epoch > 0),
    trusted_time_high_water_ms_before  INTEGER NOT NULL CHECK (
        trusted_time_high_water_ms_before >= 0
    ),
    completed_at_ms                    INTEGER NOT NULL CHECK (
        completed_at_ms > trusted_time_high_water_ms_before
    ),
    slot_phase_before                  TEXT NOT NULL CHECK (slot_phase_before = 'failed'),
    slot_phase_after                   TEXT NOT NULL CHECK (slot_phase_after = 'removed'),
    receipt_json                       TEXT NOT NULL CHECK (
        length(receipt_json) > 0 AND length(receipt_json) <= 65536
    ),
    receipt_digest                     TEXT NOT NULL UNIQUE CHECK (
        length(receipt_digest) = 64
        AND receipt_digest NOT GLOB '*[^0-9a-f]*'
    ),
    FOREIGN KEY (cleanup_id, candidate_token, authorization_receipt_digest)
        REFERENCES candidate_cleanup_authorizations(
            cleanup_id, candidate_token, receipt_digest
        ) ON DELETE RESTRICT,
    FOREIGN KEY (candidate_token)
        REFERENCES candidate_owners(candidate_token) ON DELETE RESTRICT
);

CREATE TRIGGER candidate_cleanup_authorization_insert_fenced
BEFORE INSERT ON candidate_cleanup_authorizations
WHEN NOT EXISTS (
    SELECT 1
    FROM authority_meta AS meta
    JOIN candidate_health_quarantine_receipts AS quarantine
      ON quarantine.quarantine_id = NEW.quarantine_id
     AND quarantine.candidate_token = NEW.candidate_token
     AND quarantine.candidate_token_digest = NEW.candidate_token_digest
     AND quarantine.staging_id = NEW.staging_id
     AND quarantine.staging_run_digest = NEW.staging_run_digest
     AND quarantine.receipt_digest = NEW.quarantine_receipt_digest
    JOIN candidate_owners AS candidate
      ON candidate.candidate_token = quarantine.candidate_token
    WHERE meta.singleton = 1
      AND meta.clock_status = 'trusted'
      AND meta.trusted_time_high_water_ms = NEW.authorized_at_ms
      AND meta.updated_at_ms = NEW.authorized_at_ms
      AND meta.state_revision = NEW.authority_state_revision_after
      AND meta.inventory_revision = NEW.inventory_revision
      AND meta.inventory_digest = NEW.inventory_digest
      AND meta.authority_epoch = NEW.authority_epoch_after
      AND meta.process_owner_epoch = NEW.process_owner_epoch
      AND candidate.state = 'owned'
      AND quarantine.slot_phase_after = 'failed'
      AND quarantine.failed_at_ms < NEW.authorized_at_ms
      AND quarantine.failed_at_ms <= NEW.trusted_time_high_water_ms_before
      AND quarantine.authority_state_revision_after <= NEW.authority_state_revision_before
      AND quarantine.inventory_revision_after <= NEW.inventory_revision
      AND quarantine.authority_epoch_after <= NEW.authority_epoch_before
      AND NOT EXISTS (SELECT 1 FROM fetch_claims WHERE state = 'prepared')
      AND NOT EXISTS (SELECT 1 FROM candidate_verification_runs WHERE state = 'prepared')
)
BEGIN
    SELECT RAISE(ABORT, 'candidate cleanup authorization lost its failed authority fence');
END;

CREATE TRIGGER candidate_cleanup_authorization_update_forbidden
BEFORE UPDATE ON candidate_cleanup_authorizations
BEGIN
    SELECT RAISE(ABORT, 'candidate cleanup authorization is immutable');
END;

CREATE TRIGGER candidate_cleanup_authorization_delete_forbidden
BEFORE DELETE ON candidate_cleanup_authorizations
BEGIN
    SELECT RAISE(ABORT, 'candidate cleanup authorization is immutable');
END;

CREATE TRIGGER candidate_cleanup_completion_insert_fenced
BEFORE INSERT ON candidate_cleanup_completions
WHEN NOT EXISTS (
    SELECT 1
    FROM authority_meta AS meta
    JOIN candidate_cleanup_authorizations AS authorization
      ON authorization.cleanup_id = NEW.cleanup_id
     AND authorization.candidate_token = NEW.candidate_token
     AND authorization.receipt_digest = NEW.authorization_receipt_digest
    JOIN candidate_owners AS candidate
      ON candidate.candidate_token = authorization.candidate_token
    WHERE meta.singleton = 1
      AND meta.clock_status = 'trusted'
      AND meta.trusted_time_high_water_ms = NEW.completed_at_ms
      AND meta.updated_at_ms = NEW.completed_at_ms
      AND meta.state_revision = NEW.authority_state_revision_after
      AND meta.inventory_revision = NEW.inventory_revision_after
      AND meta.inventory_digest = NEW.inventory_digest_after
      AND meta.authority_epoch = NEW.authority_epoch_after
      AND meta.process_owner_epoch = NEW.process_owner_epoch
      AND candidate.state = 'cleanup_pending'
      AND authorization.authorized_at_ms < NEW.completed_at_ms
      AND authorization.authorized_at_ms <= NEW.trusted_time_high_water_ms_before
      AND authorization.authority_state_revision_after <= NEW.authority_state_revision_before
      AND authorization.inventory_revision <= NEW.inventory_revision_before
      AND authorization.authority_epoch_after <= NEW.authority_epoch_before
      AND NOT EXISTS (SELECT 1 FROM fetch_claims WHERE state = 'prepared')
      AND NOT EXISTS (SELECT 1 FROM candidate_verification_runs WHERE state = 'prepared')
)
BEGIN
    SELECT RAISE(ABORT, 'candidate cleanup completion lost its pending authority fence');
END;

CREATE TRIGGER candidate_cleanup_completion_update_forbidden
BEFORE UPDATE ON candidate_cleanup_completions
BEGIN
    SELECT RAISE(ABORT, 'candidate cleanup completion is immutable');
END;

CREATE TRIGGER candidate_cleanup_completion_delete_forbidden
BEFORE DELETE ON candidate_cleanup_completions
BEGIN
    SELECT RAISE(ABORT, 'candidate cleanup completion is immutable');
END;
"#;
