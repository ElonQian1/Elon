/// Storage-level invariants for begin, redirect, commit, abort and revocation.
///
/// The write order is intentional: begin advances trusted time, inserts a prepared claim, then
/// advances the download cursor; commit advances trusted time and the download before terminalizing
/// that same claim. A failed statement rolls the surrounding transaction back.
pub(super) const FETCH_CLAIM_SCHEMA_V3: &str = r#"
CREATE TRIGGER fetch_claim_initial_state
BEFORE INSERT ON fetch_claims
WHEN NEW.state <> 'prepared'
  OR NEW.redirect_generation <> 0
  OR NEW.resolved_at_ms IS NOT NULL
  OR NEW.resolution_reason IS NOT NULL
  OR EXISTS (SELECT 1 FROM fetch_claims WHERE claim_id = NEW.claim_id)
  OR EXISTS (
      SELECT 1 FROM fetch_claims
      WHERE plan_id = NEW.plan_id
        AND ordinal = NEW.ordinal
        AND cursor_generation = NEW.cursor_generation
  )
  OR EXISTS (
      SELECT 1 FROM fetch_claims
      WHERE plan_id = NEW.plan_id
        AND ordinal = NEW.ordinal
        AND state = 'prepared'
  )
BEGIN
    SELECT RAISE(ABORT, 'fetch claim must start prepared and unresolved');
END;

CREATE TRIGGER fetch_claim_begin_fenced
BEFORE INSERT ON fetch_claims
WHEN NOT EXISTS (
    SELECT 1
    FROM authority_meta AS meta
    JOIN plan_applications AS application
      ON application.plan_id = NEW.plan_id
     AND application.plan_digest = NEW.plan_digest
    JOIN plan_application_seals AS seal
      ON seal.plan_id = application.plan_id
     AND seal.plan_digest = application.plan_digest
    JOIN candidate_owners AS candidate
      ON candidate.candidate_token = NEW.candidate_token
     AND candidate.owner_plan_id = application.plan_id
     AND candidate.owner_plan_digest = application.plan_digest
     AND candidate.application_inventory_revision = application.application_inventory_revision
    JOIN planned_downloads AS download
      ON download.plan_id = application.plan_id
     AND download.plan_digest = application.plan_digest
     AND download.ordinal = NEW.ordinal
     AND download.candidate_token = candidate.candidate_token
    WHERE meta.singleton = 1
      AND meta.clock_status = 'trusted'
      AND meta.trusted_time_high_water_ms IS NOT NULL
      AND meta.sharing_enabled = 1
      AND meta.inventory_revision >= application.application_inventory_revision
      AND meta.active_bundle_revision = application.keyring_bundle_revision
      AND meta.publisher_keyring_revision = application.publisher_keyring_revision
      AND meta.publisher_keyring_digest = application.publisher_keyring_digest
      AND meta.control_keyring_revision = application.control_keyring_revision
      AND meta.control_keyring_digest = application.control_keyring_digest
      AND meta.authority_epoch = NEW.authority_epoch
      AND meta.authority_epoch > 0
      AND meta.process_owner_epoch = NEW.process_owner_epoch
      AND meta.process_owner_epoch > 0
      AND NEW.prepared_at_ms = meta.trusted_time_high_water_ms
      AND NEW.prepared_at_ms >= application.applied_at_ms
      AND NEW.prepared_at_ms < application.expires_at_ms
      AND candidate.state = 'owned'
      AND download.state IN ('pending', 'downloading', 'failed')
      AND NEW.prepared_at_ms >= download.updated_at_ms
      AND download.committed_offset = NEW.offset_bytes
      AND NEW.end_offset_bytes <= download.size_bytes
      AND NEW.cursor_generation = download.cursor_generation + 1
)
BEGIN
    SELECT RAISE(ABORT, 'fetch claim does not match current durable authority');
END;

CREATE TRIGGER fetch_claim_identity_immutable
BEFORE UPDATE OF
    claim_id, plan_id, plan_digest, ordinal, candidate_token,
    authority_epoch, process_owner_epoch, cursor_generation,
    offset_bytes, length_bytes, end_offset_bytes, prepared_at_ms
ON fetch_claims
BEGIN
    SELECT RAISE(ABORT, 'fetch claim identity is immutable');
END;

CREATE TRIGGER fetch_claim_transition
BEFORE UPDATE OF redirect_generation, state, resolved_at_ms, resolution_reason
ON fetch_claims
WHEN NOT (
    OLD.state = 'prepared'
    AND NEW.state = 'prepared'
    AND NEW.redirect_generation = OLD.redirect_generation + 1
    AND NEW.resolved_at_ms IS NULL
    AND NEW.resolution_reason IS NULL
    AND EXISTS (
        SELECT 1
        FROM authority_meta AS meta
        JOIN plan_applications AS application
          ON application.plan_id = OLD.plan_id
         AND application.plan_digest = OLD.plan_digest
        JOIN plan_application_seals AS seal
          ON seal.plan_id = application.plan_id
         AND seal.plan_digest = application.plan_digest
        JOIN candidate_owners AS candidate
          ON candidate.candidate_token = OLD.candidate_token
         AND candidate.owner_plan_id = application.plan_id
         AND candidate.owner_plan_digest = application.plan_digest
         AND candidate.application_inventory_revision = application.application_inventory_revision
        JOIN planned_downloads AS download
          ON download.plan_id = application.plan_id
         AND download.plan_digest = application.plan_digest
         AND download.ordinal = OLD.ordinal
         AND download.candidate_token = candidate.candidate_token
        WHERE meta.singleton = 1
          AND meta.clock_status = 'trusted'
          AND meta.trusted_time_high_water_ms IS NOT NULL
          AND meta.sharing_enabled = 1
          AND meta.trusted_time_high_water_ms >= application.applied_at_ms
          AND meta.trusted_time_high_water_ms < application.expires_at_ms
          AND meta.inventory_revision >= application.application_inventory_revision
          AND meta.active_bundle_revision = application.keyring_bundle_revision
          AND meta.publisher_keyring_revision = application.publisher_keyring_revision
          AND meta.publisher_keyring_digest = application.publisher_keyring_digest
          AND meta.control_keyring_revision = application.control_keyring_revision
          AND meta.control_keyring_digest = application.control_keyring_digest
          AND meta.authority_epoch = OLD.authority_epoch
          AND meta.process_owner_epoch = OLD.process_owner_epoch
          AND candidate.state = 'owned'
          AND download.state = 'downloading'
          AND download.cursor_generation = OLD.cursor_generation
          AND download.committed_offset = OLD.offset_bytes
    )
)
AND NOT (
    OLD.state = 'prepared'
    AND NEW.state = 'committed'
    AND NEW.redirect_generation = OLD.redirect_generation
    AND NEW.resolved_at_ms IS NOT NULL
    AND NEW.resolution_reason = 'segment_committed'
    AND EXISTS (
        SELECT 1
        FROM authority_meta AS meta
        JOIN plan_applications AS application
          ON application.plan_id = OLD.plan_id
         AND application.plan_digest = OLD.plan_digest
        JOIN plan_application_seals AS seal
          ON seal.plan_id = application.plan_id
         AND seal.plan_digest = application.plan_digest
        JOIN candidate_owners AS candidate
          ON candidate.candidate_token = OLD.candidate_token
         AND candidate.owner_plan_id = application.plan_id
         AND candidate.owner_plan_digest = application.plan_digest
         AND candidate.application_inventory_revision = application.application_inventory_revision
        JOIN planned_downloads AS download
          ON download.plan_id = application.plan_id
         AND download.plan_digest = application.plan_digest
         AND download.ordinal = OLD.ordinal
         AND download.candidate_token = candidate.candidate_token
        WHERE meta.singleton = 1
          AND meta.clock_status = 'trusted'
          AND meta.sharing_enabled = 1
          AND meta.trusted_time_high_water_ms = NEW.resolved_at_ms
          AND meta.trusted_time_high_water_ms >= application.applied_at_ms
          AND meta.trusted_time_high_water_ms < application.expires_at_ms
          AND meta.inventory_revision >= application.application_inventory_revision
          AND meta.active_bundle_revision = application.keyring_bundle_revision
          AND meta.publisher_keyring_revision = application.publisher_keyring_revision
          AND meta.publisher_keyring_digest = application.publisher_keyring_digest
          AND meta.control_keyring_revision = application.control_keyring_revision
          AND meta.control_keyring_digest = application.control_keyring_digest
          AND meta.authority_epoch = OLD.authority_epoch
          AND meta.process_owner_epoch = OLD.process_owner_epoch
          AND candidate.state = 'owned'
          AND download.cursor_generation = OLD.cursor_generation
          AND download.committed_offset = OLD.end_offset_bytes
          AND download.updated_at_ms = NEW.resolved_at_ms
          AND (
              (download.committed_offset = download.size_bytes AND download.state = 'complete')
              OR
              (download.committed_offset < download.size_bytes AND download.state = 'downloading')
          )
    )
)
AND NOT (
    OLD.state = 'prepared'
    AND NEW.state = 'aborted'
    AND NEW.redirect_generation = OLD.redirect_generation
    AND NEW.resolved_at_ms IS NOT NULL
    AND NEW.resolution_reason IS NOT NULL
    AND NEW.resolution_reason <> ''
    AND EXISTS (
        SELECT 1
        FROM authority_meta AS meta
        JOIN planned_downloads AS download
          ON download.plan_id = OLD.plan_id
         AND download.plan_digest = OLD.plan_digest
         AND download.ordinal = OLD.ordinal
         AND download.candidate_token = OLD.candidate_token
        WHERE meta.singleton = 1
          AND meta.authority_epoch = OLD.authority_epoch
          AND meta.process_owner_epoch = OLD.process_owner_epoch
          AND NEW.resolved_at_ms >= COALESCE(
              meta.trusted_time_high_water_ms, OLD.prepared_at_ms
          )
          AND download.cursor_generation = OLD.cursor_generation
          AND download.committed_offset = OLD.offset_bytes
    )
)
AND NOT (
    OLD.state = 'prepared'
    AND NEW.state = 'revoked'
    AND NEW.redirect_generation = OLD.redirect_generation
    AND NEW.resolved_at_ms IS NOT NULL
    AND NEW.resolution_reason IN (
        'authority_epoch_advanced_by_keyring',
        'authority_epoch_advanced_by_plan',
        'authority_epoch_advanced_by_verification',
        'process_owner_epoch_advanced',
        'candidate_released_by_plan'
    )
    AND EXISTS (
        SELECT 1 FROM authority_meta AS meta
        WHERE meta.singleton = 1
          AND OLD.authority_epoch <= meta.authority_epoch
          AND OLD.process_owner_epoch <= meta.process_owner_epoch
          AND NEW.resolved_at_ms >= COALESCE(
              meta.trusted_time_high_water_ms, OLD.prepared_at_ms
          )
    )
)
BEGIN
    SELECT RAISE(ABORT, 'fetch claim transition is not fenced');
END;

CREATE TRIGGER fetch_claim_delete_forbidden
BEFORE DELETE ON fetch_claims
BEGIN
    SELECT RAISE(ABORT, 'fetch claim history is immutable');
END;

CREATE TRIGGER planned_download_fetch_progress_fenced
BEFORE UPDATE OF committed_offset, cursor_generation, state ON planned_downloads
WHEN (
    NEW.committed_offset IS NOT OLD.committed_offset
    OR NEW.cursor_generation IS NOT OLD.cursor_generation
    OR (OLD.state <> 'downloading' AND NEW.state = 'downloading')
)
AND NOT (
    NEW.committed_offset = OLD.committed_offset
    AND NEW.cursor_generation = OLD.cursor_generation + 1
    AND NEW.state = 'downloading'
    AND EXISTS (
        SELECT 1 FROM fetch_claims AS claim
        WHERE claim.plan_id = OLD.plan_id
          AND claim.plan_digest = OLD.plan_digest
          AND claim.ordinal = OLD.ordinal
          AND claim.candidate_token = OLD.candidate_token
          AND claim.state = 'prepared'
          AND claim.cursor_generation = NEW.cursor_generation
          AND claim.offset_bytes = OLD.committed_offset
          AND claim.prepared_at_ms = NEW.updated_at_ms
    )
)
AND NOT (
    NEW.cursor_generation = OLD.cursor_generation
    AND NEW.committed_offset > OLD.committed_offset
    AND EXISTS (
        SELECT 1 FROM fetch_claims AS claim
        WHERE claim.plan_id = OLD.plan_id
          AND claim.plan_digest = OLD.plan_digest
          AND claim.ordinal = OLD.ordinal
          AND claim.candidate_token = OLD.candidate_token
          AND claim.state = 'prepared'
          AND claim.cursor_generation = OLD.cursor_generation
          AND claim.offset_bytes = OLD.committed_offset
          AND claim.end_offset_bytes = NEW.committed_offset
          AND NEW.updated_at_ms >= claim.prepared_at_ms
          AND (
              (NEW.committed_offset = NEW.size_bytes AND NEW.state = 'complete')
              OR
              (NEW.committed_offset < NEW.size_bytes AND NEW.state = 'downloading')
          )
    )
)
BEGIN
    SELECT RAISE(ABORT, 'planned download progress lacks an exact prepared claim');
END;
"#;
