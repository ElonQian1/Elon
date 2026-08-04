/// Candidate-level raw artifact verification journal for authority schema v3.
///
/// A `complete` planned download only proves its byte cursor reached the signed size. One prepared
/// run binds the complete candidate closure to an exact set of pinned local files; only a future
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
    authority_epoch                INTEGER NOT NULL CHECK (authority_epoch > 0),
    process_owner_epoch            INTEGER NOT NULL CHECK (process_owner_epoch > 0),
    artifact_count                 INTEGER NOT NULL CHECK (
        artifact_count > 0 AND artifact_count <= 4096
    ),
    artifact_bytes                 INTEGER NOT NULL CHECK (artifact_bytes > 0),
    expected_artifact_set_digest   TEXT NOT NULL,
    file_set_binding_digest        TEXT NOT NULL,
    state                          TEXT NOT NULL CHECK (
        state IN ('prepared', 'verified', 'rejected', 'aborted', 'revoked')
    ),
    prepared_at_ms                 INTEGER NOT NULL CHECK (prepared_at_ms >= 0),
    resolved_at_ms                 INTEGER,
    resolution_reason              TEXT,
    result_json                    TEXT,
    result_digest                  TEXT,
    mismatch_ordinal               INTEGER CHECK (mismatch_ordinal >= 0),
    observed_digest                TEXT,
    UNIQUE (candidate_token, verification_generation),
    FOREIGN KEY (candidate_token, owner_plan_id, owner_plan_digest)
        REFERENCES candidate_owners(candidate_token, owner_plan_id, owner_plan_digest)
        ON DELETE RESTRICT,
    CHECK (
        (state = 'prepared'
         AND resolved_at_ms IS NULL
         AND resolution_reason IS NULL
         AND result_json IS NULL
         AND result_digest IS NULL
         AND mismatch_ordinal IS NULL
         AND observed_digest IS NULL)
        OR
        (state = 'verified'
         AND resolved_at_ms IS NOT NULL
         AND resolved_at_ms >= prepared_at_ms
         AND resolution_reason IS NOT NULL
         AND resolution_reason = 'artifact_set_verified'
         AND result_json IS NOT NULL
         AND result_digest IS NOT NULL
         AND mismatch_ordinal IS NULL
         AND observed_digest IS NULL)
        OR
        (state = 'rejected'
         AND resolved_at_ms IS NOT NULL
         AND resolved_at_ms >= prepared_at_ms
         AND resolution_reason IS NOT NULL
         AND resolution_reason = 'artifact_digest_mismatch'
         AND result_json IS NOT NULL
         AND result_digest IS NOT NULL
         AND mismatch_ordinal IS NOT NULL
         AND observed_digest IS NOT NULL)
        OR
        (state = 'aborted'
         AND resolved_at_ms IS NOT NULL
         AND resolved_at_ms >= prepared_at_ms
         AND resolution_reason IS NOT NULL
         AND resolution_reason IN ('verification_aborted', 'authority_recovery')
         AND result_json IS NOT NULL
         AND result_digest IS NOT NULL
         AND mismatch_ordinal IS NULL
         AND observed_digest IS NULL)
        OR
        (state = 'revoked'
         AND resolved_at_ms IS NOT NULL
         AND resolved_at_ms >= prepared_at_ms
         AND resolution_reason IS NOT NULL
         AND resolution_reason IN (
             'authority_epoch_advanced_by_keyring',
             'authority_epoch_advanced_by_plan',
             'process_owner_epoch_advanced',
             'candidate_released_by_plan'
         )
         AND result_json IS NOT NULL
         AND result_digest IS NOT NULL
         AND mismatch_ordinal IS NULL
         AND observed_digest IS NULL)
    )
);

CREATE UNIQUE INDEX one_prepared_verification_per_candidate
    ON candidate_verification_runs(candidate_token) WHERE state = 'prepared';

CREATE UNIQUE INDEX one_verified_artifact_set_per_candidate
    ON candidate_verification_runs(candidate_token) WHERE state = 'verified';

CREATE TRIGGER candidate_verification_initial_state
BEFORE INSERT ON candidate_verification_runs
WHEN NEW.state <> 'prepared'
  OR NEW.resolved_at_ms IS NOT NULL
  OR NEW.resolution_reason IS NOT NULL
  OR NEW.result_json IS NOT NULL
  OR NEW.result_digest IS NOT NULL
  OR NEW.mismatch_ordinal IS NOT NULL
  OR NEW.observed_digest IS NOT NULL
  OR NEW.verification_generation <> COALESCE((
      SELECT MAX(verification_generation) + 1
      FROM candidate_verification_runs
      WHERE candidate_token = NEW.candidate_token
  ), 1)
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
    authority_epoch, process_owner_epoch, artifact_count, artifact_bytes,
    expected_artifact_set_digest, file_set_binding_digest, prepared_at_ms
ON candidate_verification_runs
BEGIN
    SELECT RAISE(ABORT, 'candidate verification identity is immutable');
END;

CREATE TRIGGER candidate_verification_transition
BEFORE UPDATE OF
    state, resolved_at_ms, resolution_reason, result_json, result_digest,
    mismatch_ordinal, observed_digest
ON candidate_verification_runs
WHEN OLD.state <> 'prepared' OR NEW.state <> 'revoked'
BEGIN
    SELECT RAISE(ABORT, 'candidate verification resolution is unavailable until the hash binder lands');
END;

CREATE TRIGGER candidate_verification_delete_forbidden
BEFORE DELETE ON candidate_verification_runs
BEGIN
    SELECT RAISE(ABORT, 'candidate verification history is immutable');
END;
"#;
