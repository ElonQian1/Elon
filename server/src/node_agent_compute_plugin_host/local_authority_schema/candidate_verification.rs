/// Candidate-level raw artifact verification journal for authority schema v3.
///
/// A `complete` planned download only proves its byte cursor reached the signed size. One prepared
/// run binds the complete candidate closure to an exact set of pinned local files; only the
/// purpose-specific resolver may terminalize it and advance the inventory slot phase.
pub(super) const CANDIDATE_VERIFICATION_SCHEMA_V3: &str = r#"
CREATE TABLE candidate_verification_runs (
    verification_id                TEXT PRIMARY KEY,
    candidate_token                TEXT NOT NULL,
    owner_plan_id                  TEXT NOT NULL,
    owner_plan_digest              TEXT NOT NULL,
    verification_generation        INTEGER NOT NULL CHECK (verification_generation > 0),
    candidate_generation           INTEGER NOT NULL CHECK (candidate_generation > 0),
    application_inventory_revision INTEGER NOT NULL CHECK (
        application_inventory_revision > 0
    ),
    authority_state_revision       INTEGER NOT NULL CHECK (authority_state_revision > 0),
    authority_epoch                INTEGER NOT NULL CHECK (authority_epoch > 0),
    process_owner_epoch            INTEGER NOT NULL CHECK (process_owner_epoch > 0),
    artifact_count                 INTEGER NOT NULL CHECK (
        artifact_count > 0 AND artifact_count <= 4096
    ),
    artifact_bytes                 INTEGER NOT NULL CHECK (artifact_bytes > 0),
    expected_artifact_set_digest   TEXT NOT NULL CHECK (
        length(expected_artifact_set_digest) = 64
        AND expected_artifact_set_digest NOT GLOB '*[^0-9a-f]*'
    ),
    file_set_binding_digest        TEXT NOT NULL CHECK (
        length(file_set_binding_digest) = 64
        AND file_set_binding_digest NOT GLOB '*[^0-9a-f]*'
    ),
    state                          TEXT NOT NULL CHECK (
        state IN ('prepared', 'verified', 'rejected', 'aborted', 'revoked')
    ),
    prepared_at_ms                 INTEGER NOT NULL CHECK (prepared_at_ms >= 0),
    resolved_at_ms                 INTEGER,
    resolution_reason              TEXT,
    result_json                    TEXT,
    result_digest                  TEXT,
    observed_artifact_set_digest   TEXT,
    mismatch_ordinal               INTEGER CHECK (mismatch_ordinal >= 0),
    mismatch_observed_digest       TEXT,
    UNIQUE (candidate_token, verification_generation),
    FOREIGN KEY (candidate_token, owner_plan_id, owner_plan_digest)
        REFERENCES candidate_owners(candidate_token, owner_plan_id, owner_plan_digest)
        ON DELETE RESTRICT,
    CHECK (CASE
        WHEN state = 'prepared'
         AND resolved_at_ms IS NULL
         AND resolution_reason IS NULL
         AND result_json IS NULL
         AND result_digest IS NULL
         AND observed_artifact_set_digest IS NULL
         AND mismatch_ordinal IS NULL
         AND mismatch_observed_digest IS NULL THEN 1
        WHEN state = 'verified'
         AND resolved_at_ms IS NOT NULL
         AND resolved_at_ms >= prepared_at_ms
         AND resolution_reason = 'artifact_set_verified'
         AND length(result_json) > 0
         AND length(result_digest) = 64
         AND result_digest NOT GLOB '*[^0-9a-f]*'
         AND length(observed_artifact_set_digest) = 64
         AND observed_artifact_set_digest NOT GLOB '*[^0-9a-f]*'
         AND mismatch_ordinal IS NULL
         AND mismatch_observed_digest IS NULL THEN 1
        WHEN state = 'rejected'
         AND resolved_at_ms IS NOT NULL
         AND resolved_at_ms >= prepared_at_ms
         AND resolution_reason = 'artifact_digest_mismatch'
         AND length(result_json) > 0
         AND length(result_digest) = 64
         AND result_digest NOT GLOB '*[^0-9a-f]*'
         AND mismatch_ordinal IS NOT NULL
         AND length(observed_artifact_set_digest) = 64
         AND observed_artifact_set_digest NOT GLOB '*[^0-9a-f]*'
         AND length(mismatch_observed_digest) = 64
         AND mismatch_observed_digest NOT GLOB '*[^0-9a-f]*' THEN 1
        WHEN state = 'aborted'
         AND resolved_at_ms IS NOT NULL
         AND resolved_at_ms >= prepared_at_ms
         AND resolution_reason IN ('verification_aborted', 'authority_recovery')
         AND length(result_json) > 0
         AND length(result_digest) = 64
         AND result_digest NOT GLOB '*[^0-9a-f]*'
         AND observed_artifact_set_digest IS NULL
         AND mismatch_ordinal IS NULL
         AND mismatch_observed_digest IS NULL THEN 1
        WHEN state = 'revoked'
         AND resolved_at_ms IS NOT NULL
         AND resolved_at_ms >= prepared_at_ms
         AND resolution_reason IN (
             'authority_epoch_advanced_by_keyring',
             'authority_epoch_advanced_by_plan',
             'authority_epoch_advanced_by_verification',
             'process_owner_epoch_advanced',
             'candidate_released_by_plan'
         )
         AND length(result_json) > 0
         AND length(result_digest) = 64
         AND result_digest NOT GLOB '*[^0-9a-f]*'
         AND observed_artifact_set_digest IS NULL
         AND mismatch_ordinal IS NULL
         AND mismatch_observed_digest IS NULL THEN 1
        ELSE 0 END = 1
    )
);

CREATE UNIQUE INDEX one_prepared_verification_per_candidate
    ON candidate_verification_runs(candidate_token) WHERE state = 'prepared';

CREATE UNIQUE INDEX one_verified_artifact_set_per_candidate
    ON candidate_verification_runs(candidate_token) WHERE state = 'verified';

CREATE TRIGGER candidate_verification_initial_state
BEFORE INSERT ON candidate_verification_runs
WHEN CASE WHEN NEW.state = 'prepared'
  AND NEW.resolved_at_ms IS NULL
  AND NEW.resolution_reason IS NULL
  AND NEW.result_json IS NULL
  AND NEW.result_digest IS NULL
  AND NEW.observed_artifact_set_digest IS NULL
  AND NEW.mismatch_ordinal IS NULL
  AND NEW.mismatch_observed_digest IS NULL
  AND NEW.verification_generation = COALESCE((
      SELECT MAX(verification_generation) + 1
      FROM candidate_verification_runs
      WHERE candidate_token = NEW.candidate_token
  ), 1) THEN 0 ELSE 1 END
BEGIN
    SELECT RAISE(ABORT, 'candidate verification must start prepared at the next generation');
END;

CREATE TRIGGER candidate_verification_begin_fenced
BEFORE INSERT ON candidate_verification_runs
WHEN NOT EXISTS (
    SELECT 1
    FROM authority_meta AS meta
    JOIN plan_applications AS application
      ON application.plan_id = NEW.owner_plan_id
     AND application.plan_digest = NEW.owner_plan_digest
    JOIN plan_application_seals AS seal
      ON seal.plan_id = application.plan_id
     AND seal.plan_digest = application.plan_digest
    JOIN candidate_owners AS candidate
      ON candidate.candidate_token = NEW.candidate_token
     AND candidate.owner_plan_id = application.plan_id
     AND candidate.owner_plan_digest = application.plan_digest
     AND candidate.application_inventory_revision = application.application_inventory_revision
    WHERE meta.singleton = 1
      AND meta.clock_status = 'trusted'
      AND meta.trusted_time_high_water_ms = NEW.prepared_at_ms
      AND meta.sharing_enabled = 1
      AND meta.inventory_revision >= application.application_inventory_revision
      AND meta.active_bundle_revision = application.keyring_bundle_revision
      AND meta.publisher_keyring_revision = application.publisher_keyring_revision
      AND meta.publisher_keyring_digest = application.publisher_keyring_digest
      AND meta.control_keyring_revision = application.control_keyring_revision
      AND meta.control_keyring_digest = application.control_keyring_digest
      AND meta.state_revision = NEW.authority_state_revision
      AND meta.authority_epoch = NEW.authority_epoch
      AND meta.process_owner_epoch = NEW.process_owner_epoch
      AND NEW.prepared_at_ms >= application.applied_at_ms
      AND NEW.prepared_at_ms < application.expires_at_ms
      AND candidate.state = 'owned'
      AND candidate.candidate_generation = NEW.candidate_generation
      AND candidate.application_inventory_revision = NEW.application_inventory_revision
      AND (SELECT COUNT(*) FROM planned_downloads AS download
           WHERE download.candidate_token = candidate.candidate_token)
          = NEW.artifact_count
      AND (SELECT COALESCE(SUM(size_bytes), 0) FROM planned_downloads AS download
           WHERE download.candidate_token = candidate.candidate_token)
          = NEW.artifact_bytes
      AND NOT EXISTS (
          SELECT 1 FROM planned_downloads AS download
          WHERE download.candidate_token = candidate.candidate_token
            AND (download.state <> 'complete'
                 OR download.committed_offset <> download.size_bytes)
      )
      AND NOT EXISTS (
          SELECT 1 FROM fetch_claims AS claim
          WHERE claim.candidate_token = candidate.candidate_token
            AND claim.state = 'prepared'
      )
      AND NOT EXISTS (
          SELECT 1 FROM candidate_verification_runs AS existing
          WHERE existing.candidate_token = candidate.candidate_token
            AND existing.state IN ('prepared', 'verified')
      )
)
BEGIN
    SELECT RAISE(ABORT, 'candidate verification does not match current complete authority');
END;

CREATE TRIGGER candidate_verification_identity_immutable
BEFORE UPDATE OF
    verification_id, candidate_token, owner_plan_id, owner_plan_digest,
    verification_generation, candidate_generation, application_inventory_revision,
    authority_state_revision, authority_epoch, process_owner_epoch,
    artifact_count, artifact_bytes,
    expected_artifact_set_digest, file_set_binding_digest, prepared_at_ms
ON candidate_verification_runs
BEGIN
    SELECT RAISE(ABORT, 'candidate verification identity is immutable');
END;

CREATE TRIGGER candidate_verification_transition
BEFORE UPDATE OF
    state, resolved_at_ms, resolution_reason, result_json, result_digest,
    observed_artifact_set_digest, mismatch_ordinal, mismatch_observed_digest
ON candidate_verification_runs
WHEN CASE WHEN OLD.state = 'prepared'
 AND NEW.state IN ('verified', 'rejected', 'aborted', 'revoked') THEN 0 ELSE 1 END
BEGIN
    SELECT RAISE(ABORT, 'candidate verification transition is not available');
END;

CREATE TRIGGER candidate_verification_resolution_fenced
BEFORE UPDATE OF
    state, resolved_at_ms, resolution_reason, result_json, result_digest,
    observed_artifact_set_digest, mismatch_ordinal, mismatch_observed_digest
ON candidate_verification_runs
WHEN NEW.state IN ('verified', 'rejected') AND CASE WHEN
    OLD.state = 'prepared'
    AND OLD.resolved_at_ms IS NULL
    AND OLD.resolution_reason IS NULL
    AND OLD.result_json IS NULL
    AND OLD.result_digest IS NULL
    AND OLD.observed_artifact_set_digest IS NULL
    AND OLD.mismatch_ordinal IS NULL
    AND OLD.mismatch_observed_digest IS NULL
    AND NEW.resolved_at_ms IS NOT NULL
    AND NEW.resolved_at_ms > OLD.prepared_at_ms
    AND length(NEW.result_json) > 0
    AND length(NEW.result_digest) = 64
    AND NEW.result_digest NOT GLOB '*[^0-9a-f]*'
    AND length(NEW.observed_artifact_set_digest) = 64
    AND NEW.observed_artifact_set_digest NOT GLOB '*[^0-9a-f]*'
    AND (
        (NEW.state = 'verified'
         AND NEW.resolution_reason = 'artifact_set_verified'
         AND NEW.mismatch_ordinal IS NULL
         AND NEW.mismatch_observed_digest IS NULL)
        OR
        (NEW.state = 'rejected'
         AND NEW.resolution_reason = 'artifact_digest_mismatch'
         AND NEW.mismatch_ordinal IS NOT NULL
         AND NEW.mismatch_ordinal >= 0
         AND length(NEW.mismatch_observed_digest) = 64
         AND NEW.mismatch_observed_digest NOT GLOB '*[^0-9a-f]*'
         AND EXISTS (
             SELECT 1 FROM planned_downloads AS mismatch
             WHERE mismatch.candidate_token = OLD.candidate_token
               AND mismatch.ordinal = NEW.mismatch_ordinal
               AND mismatch.artifact_digest <> NEW.mismatch_observed_digest
         ))
    )
    AND EXISTS (
        SELECT 1
        FROM authority_meta AS meta
        JOIN plan_applications AS application
          ON application.plan_id = OLD.owner_plan_id
         AND application.plan_digest = OLD.owner_plan_digest
        JOIN plan_application_seals AS seal
          ON seal.plan_id = application.plan_id
         AND seal.plan_digest = application.plan_digest
        JOIN candidate_owners AS candidate
          ON candidate.candidate_token = OLD.candidate_token
         AND candidate.owner_plan_id = application.plan_id
         AND candidate.owner_plan_digest = application.plan_digest
         AND candidate.application_inventory_revision = OLD.application_inventory_revision
        WHERE meta.singleton = 1
          AND meta.clock_status = 'trusted'
          AND meta.trusted_time_high_water_ms = NEW.resolved_at_ms
          AND meta.sharing_enabled = 1
          AND meta.inventory_revision >= OLD.application_inventory_revision
          AND meta.active_bundle_revision = application.keyring_bundle_revision
          AND meta.publisher_keyring_revision = application.publisher_keyring_revision
          AND meta.publisher_keyring_digest = application.publisher_keyring_digest
          AND meta.control_keyring_revision = application.control_keyring_revision
          AND meta.control_keyring_digest = application.control_keyring_digest
          AND meta.state_revision = OLD.authority_state_revision
          AND meta.authority_epoch = OLD.authority_epoch
          AND meta.process_owner_epoch = OLD.process_owner_epoch
          AND NEW.resolved_at_ms >= application.applied_at_ms
          AND NEW.resolved_at_ms < application.expires_at_ms
          AND candidate.state = 'owned'
          AND candidate.candidate_generation = OLD.candidate_generation
          AND (SELECT COUNT(*) FROM planned_downloads AS download
               WHERE download.candidate_token = candidate.candidate_token)
              = OLD.artifact_count
          AND (SELECT COALESCE(SUM(size_bytes), 0) FROM planned_downloads AS download
               WHERE download.candidate_token = candidate.candidate_token)
              = OLD.artifact_bytes
          AND NOT EXISTS (
              SELECT 1 FROM planned_downloads AS download
              WHERE download.candidate_token = candidate.candidate_token
                AND (download.state <> 'complete'
                     OR download.committed_offset <> download.size_bytes)
          )
          AND NOT EXISTS (
              SELECT 1 FROM fetch_claims AS claim
              WHERE claim.candidate_token = candidate.candidate_token
                AND claim.state = 'prepared'
          )
    )
THEN 0 ELSE 1 END
BEGIN
    SELECT RAISE(ABORT, 'candidate verification resolution lost its authority fence');
END;

CREATE TRIGGER candidate_verification_abort_fenced
BEFORE UPDATE OF
    state, resolved_at_ms, resolution_reason, result_json, result_digest,
    observed_artifact_set_digest, mismatch_ordinal, mismatch_observed_digest
ON candidate_verification_runs
WHEN NEW.state = 'aborted' AND CASE WHEN
    OLD.state = 'prepared'
    AND OLD.resolved_at_ms IS NULL
    AND OLD.resolution_reason IS NULL
    AND OLD.result_json IS NULL
    AND OLD.result_digest IS NULL
    AND OLD.observed_artifact_set_digest IS NULL
    AND OLD.mismatch_ordinal IS NULL
    AND OLD.mismatch_observed_digest IS NULL
    AND NEW.resolved_at_ms IS NOT NULL
    AND NEW.resolved_at_ms >= OLD.prepared_at_ms
    AND NEW.resolution_reason IN ('verification_aborted', 'authority_recovery')
    AND length(NEW.result_json) > 0
    AND length(NEW.result_digest) = 64
    AND NEW.result_digest NOT GLOB '*[^0-9a-f]*'
    AND NEW.observed_artifact_set_digest IS NULL
    AND NEW.mismatch_ordinal IS NULL
    AND NEW.mismatch_observed_digest IS NULL
    AND EXISTS (
        SELECT 1 FROM authority_meta AS meta
        WHERE meta.singleton = 1
          AND meta.clock_status = 'trusted'
          AND meta.trusted_time_high_water_ms = NEW.resolved_at_ms
          AND meta.state_revision = OLD.authority_state_revision
          AND meta.authority_epoch = OLD.authority_epoch
          AND meta.process_owner_epoch = OLD.process_owner_epoch
    )
THEN 0 ELSE 1 END
BEGIN
    SELECT RAISE(ABORT, 'candidate verification abort lost its authority fence');
END;

CREATE TRIGGER candidate_verification_delete_forbidden
BEFORE DELETE ON candidate_verification_runs
BEGIN
    SELECT RAISE(ABORT, 'candidate verification history is immutable');
END;
"#;
