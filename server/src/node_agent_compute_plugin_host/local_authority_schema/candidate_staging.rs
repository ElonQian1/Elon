/// Immutable evidence that one verified candidate was extracted and sealed in managed staging.
///
/// The future staging Store transaction must advance the inventory from `verifying` to `staged`
/// before inserting this row. The insert trigger then proves that the receipt describes the exact
/// post-transaction authority and the one still-owned, previously verified candidate.
pub(super) const CANDIDATE_STAGING_SCHEMA_V3: &str = r#"
CREATE TABLE candidate_staging_receipts (
    staging_id                       TEXT PRIMARY KEY CHECK (length(staging_id) > 0),
    candidate_token                  TEXT NOT NULL UNIQUE,
    candidate_token_digest           TEXT NOT NULL UNIQUE CHECK (
        length(candidate_token_digest) = 64
        AND candidate_token_digest NOT GLOB '*[^0-9a-f]*'
    ),
    owner_plan_id                    TEXT NOT NULL,
    owner_plan_digest                TEXT NOT NULL CHECK (
        length(owner_plan_digest) = 64
        AND owner_plan_digest NOT GLOB '*[^0-9a-f]*'
    ),
    verification_id                  TEXT NOT NULL UNIQUE,
    verification_generation          INTEGER NOT NULL CHECK (verification_generation > 0),
    candidate_generation             INTEGER NOT NULL CHECK (candidate_generation > 0),
    application_inventory_revision   INTEGER NOT NULL CHECK (
        application_inventory_revision > 0
    ),
    verification_result_digest       TEXT NOT NULL CHECK (
        length(verification_result_digest) = 64
        AND verification_result_digest NOT GLOB '*[^0-9a-f]*'
    ),
    root_identity_digest             TEXT NOT NULL CHECK (
        length(root_identity_digest) = 64
        AND root_identity_digest NOT GLOB '*[^0-9a-f]*'
    ),
    staging_run_digest               TEXT NOT NULL UNIQUE CHECK (
        length(staging_run_digest) = 64
        AND staging_run_digest NOT GLOB '*[^0-9a-f]*'
    ),
    extraction_plan_json             TEXT NOT NULL CHECK (length(extraction_plan_json) > 0),
    extraction_plan_digest           TEXT NOT NULL CHECK (
        length(extraction_plan_digest) = 64
        AND extraction_plan_digest NOT GLOB '*[^0-9a-f]*'
    ),
    extraction_evidence_json         TEXT NOT NULL CHECK (length(extraction_evidence_json) > 0),
    extraction_evidence_digest       TEXT NOT NULL CHECK (
        length(extraction_evidence_digest) = 64
        AND extraction_evidence_digest NOT GLOB '*[^0-9a-f]*'
    ),
    staging_seal_json                TEXT NOT NULL CHECK (length(staging_seal_json) > 0),
    staging_seal_payload_digest      TEXT NOT NULL CHECK (
        length(staging_seal_payload_digest) = 64
        AND staging_seal_payload_digest NOT GLOB '*[^0-9a-f]*'
    ),
    staging_seal_file_digest         TEXT NOT NULL CHECK (
        length(staging_seal_file_digest) = 64
        AND staging_seal_file_digest NOT GLOB '*[^0-9a-f]*'
    ),
    staging_seal_identity_digest     TEXT NOT NULL CHECK (
        length(staging_seal_identity_digest) = 64
        AND staging_seal_identity_digest NOT GLOB '*[^0-9a-f]*'
    ),
    staging_seal_size_bytes          INTEGER NOT NULL CHECK (
        staging_seal_size_bytes > 0 AND staging_seal_size_bytes <= 1048576
    ),
    extracted_file_count             INTEGER NOT NULL CHECK (
        extracted_file_count > 0 AND extracted_file_count <= 4096
    ),
    extracted_bytes                  INTEGER NOT NULL CHECK (
        extracted_bytes > 0 AND extracted_bytes <= 68719476736
    ),
    authority_state_revision_before  INTEGER NOT NULL CHECK (
        authority_state_revision_before > 0
    ),
    authority_state_revision_after   INTEGER NOT NULL CHECK (
        authority_state_revision_after = authority_state_revision_before + 1
    ),
    inventory_revision_before        INTEGER NOT NULL CHECK (inventory_revision_before > 0),
    inventory_revision_after         INTEGER NOT NULL CHECK (
        inventory_revision_after = inventory_revision_before + 1
    ),
    inventory_digest_before          TEXT NOT NULL CHECK (
        length(inventory_digest_before) = 64
        AND inventory_digest_before NOT GLOB '*[^0-9a-f]*'
    ),
    inventory_digest_after           TEXT NOT NULL CHECK (
        length(inventory_digest_after) = 64
        AND inventory_digest_after NOT GLOB '*[^0-9a-f]*'
        AND inventory_digest_after <> inventory_digest_before
    ),
    inventory_json_after             TEXT NOT NULL CHECK (length(inventory_json_after) > 0),
    authority_epoch_before           INTEGER NOT NULL CHECK (authority_epoch_before > 0),
    authority_epoch_after            INTEGER NOT NULL CHECK (
        authority_epoch_after = authority_epoch_before + 1
    ),
    process_owner_epoch              INTEGER NOT NULL CHECK (process_owner_epoch > 0),
    staged_at_ms                     INTEGER NOT NULL CHECK (staged_at_ms > 0),
    receipt_json                     TEXT NOT NULL CHECK (length(receipt_json) > 0),
    receipt_digest                   TEXT NOT NULL UNIQUE CHECK (
        length(receipt_digest) = 64
        AND receipt_digest NOT GLOB '*[^0-9a-f]*'
    ),
    FOREIGN KEY (candidate_token, owner_plan_id, owner_plan_digest)
        REFERENCES candidate_owners(candidate_token, owner_plan_id, owner_plan_digest)
        ON DELETE RESTRICT,
    FOREIGN KEY (verification_id)
        REFERENCES candidate_verification_runs(verification_id) ON DELETE RESTRICT
);

CREATE TRIGGER candidate_staging_insert_fenced
BEFORE INSERT ON candidate_staging_receipts
WHEN NOT EXISTS (
    SELECT 1
    FROM authority_meta AS meta
    JOIN plan_applications AS application
      ON application.plan_id = NEW.owner_plan_id
     AND application.plan_digest = NEW.owner_plan_digest
    JOIN plan_application_seals AS application_seal
      ON application_seal.plan_id = application.plan_id
     AND application_seal.plan_digest = application.plan_digest
    JOIN candidate_owners AS candidate
      ON candidate.candidate_token = NEW.candidate_token
     AND candidate.owner_plan_id = application.plan_id
     AND candidate.owner_plan_digest = application.plan_digest
     AND candidate.application_inventory_revision = NEW.application_inventory_revision
    JOIN candidate_verification_runs AS verification
      ON verification.verification_id = NEW.verification_id
     AND verification.candidate_token = candidate.candidate_token
     AND verification.owner_plan_id = application.plan_id
     AND verification.owner_plan_digest = application.plan_digest
    WHERE meta.singleton = 1
      AND meta.clock_status = 'trusted'
      AND meta.sharing_enabled = 1
      AND meta.trusted_time_high_water_ms = NEW.staged_at_ms
      AND meta.updated_at_ms = NEW.staged_at_ms
      AND meta.state_revision = NEW.authority_state_revision_after
      AND meta.inventory_revision = NEW.inventory_revision_after
      AND meta.inventory_digest = NEW.inventory_digest_after
      AND meta.inventory_json = NEW.inventory_json_after
      AND meta.authority_epoch = NEW.authority_epoch_after
      AND meta.process_owner_epoch = NEW.process_owner_epoch
      AND meta.active_bundle_revision = application.keyring_bundle_revision
      AND meta.publisher_keyring_revision = application.publisher_keyring_revision
      AND meta.publisher_keyring_digest = application.publisher_keyring_digest
      AND meta.control_keyring_revision = application.control_keyring_revision
      AND meta.control_keyring_digest = application.control_keyring_digest
      AND candidate.state = 'owned'
      AND candidate.candidate_generation = NEW.candidate_generation
      AND verification.state = 'verified'
      AND verification.verification_generation = NEW.verification_generation
      AND verification.candidate_generation = NEW.candidate_generation
      AND verification.application_inventory_revision = NEW.application_inventory_revision
      AND verification.process_owner_epoch = NEW.process_owner_epoch
      AND verification.result_digest = NEW.verification_result_digest
      AND verification.resolved_at_ms IS NOT NULL
      AND verification.resolved_at_ms < NEW.staged_at_ms
      AND NEW.authority_state_revision_before = verification.authority_state_revision + 1
      AND NEW.authority_epoch_before = verification.authority_epoch + 1
      AND NEW.inventory_revision_before >= NEW.application_inventory_revision
      AND NEW.staged_at_ms >= application.applied_at_ms
      AND NEW.staged_at_ms < application.expires_at_ms
      AND NOT EXISTS (
          SELECT 1 FROM fetch_claims AS claim
          WHERE claim.state = 'prepared'
      )
      AND NOT EXISTS (
          SELECT 1 FROM candidate_verification_runs AS prepared
          WHERE prepared.state = 'prepared'
      )
)
BEGIN
    SELECT RAISE(ABORT, 'candidate staging receipt lost its authority fence');
END;

CREATE TRIGGER candidate_staging_update_forbidden
BEFORE UPDATE ON candidate_staging_receipts
BEGIN
    SELECT RAISE(ABORT, 'candidate staging receipt is immutable');
END;

CREATE TRIGGER candidate_staging_delete_forbidden
BEFORE DELETE ON candidate_staging_receipts
BEGIN
    SELECT RAISE(ABORT, 'candidate staging receipt is immutable');
END;
"#;
