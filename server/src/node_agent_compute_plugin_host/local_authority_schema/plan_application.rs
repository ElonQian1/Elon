/// Cross-table fences for the immutable plan-application journal.
///
/// Kept separate from the base table declarations so the schema entrypoint remains assembly-only.
pub(super) const PLAN_APPLICATION_SCHEMA_V3: &str = r#"
CREATE TRIGGER authority_inventory_change_fenced
BEFORE UPDATE OF inventory_revision, inventory_digest, inventory_json ON authority_meta
WHEN (
    NEW.inventory_revision IS NOT OLD.inventory_revision
    OR NEW.inventory_digest IS NOT OLD.inventory_digest
    OR NEW.inventory_json IS NOT OLD.inventory_json
) AND (
    NEW.inventory_revision <> OLD.inventory_revision + 1
    OR NEW.inventory_digest IS OLD.inventory_digest
    OR NEW.inventory_json IS OLD.inventory_json
    OR NEW.state_revision <> OLD.state_revision + 1
    OR NEW.authority_epoch <> OLD.authority_epoch + 1
) BEGIN
    SELECT RAISE(ABORT, 'inventory change must advance inventory, state and authority fences');
END;

CREATE TRIGGER plan_application_matches_authority
BEFORE INSERT ON plan_applications
WHEN NOT EXISTS (
    SELECT 1 FROM authority_meta AS meta
    WHERE meta.singleton = 1
      AND meta.clock_status = 'trusted'
      AND meta.trusted_time_high_water_ms IS NOT NULL
      AND NEW.expected_inventory_revision = meta.inventory_revision
      AND NEW.expected_inventory_digest = meta.inventory_digest
      AND NEW.application_inventory_revision = meta.inventory_revision + 1
      AND NEW.application_state_revision = meta.state_revision + 1
      AND NEW.authority_epoch_at_apply = meta.authority_epoch + 1
      AND NEW.keyring_bundle_revision = meta.active_bundle_revision
      AND NEW.publisher_keyring_revision = meta.publisher_keyring_revision
      AND NEW.publisher_keyring_digest = meta.publisher_keyring_digest
      AND NEW.control_keyring_revision = meta.control_keyring_revision
      AND NEW.control_keyring_digest = meta.control_keyring_digest
      AND NEW.applied_at_ms >= meta.trusted_time_high_water_ms
) BEGIN
    SELECT RAISE(ABORT, 'plan application does not match current authority');
END;

CREATE TRIGGER plan_event_contiguous
BEFORE INSERT ON plan_events
WHEN NEW.event_index <> COALESCE((
    SELECT MAX(event_index) + 1 FROM plan_events WHERE plan_id = NEW.plan_id
), 0)
OR (NEW.event_index = 0 AND NEW.event_type <> 'applied')
BEGIN
    SELECT RAISE(ABORT, 'plan events must start applied and remain contiguous');
END;

CREATE TRIGGER plan_application_seal_complete
BEFORE INSERT ON plan_application_seals
WHEN NOT EXISTS (
    SELECT 1 FROM plan_applications AS application
    WHERE application.plan_id = NEW.plan_id
      AND application.plan_digest = NEW.plan_digest
      AND application.application_request_digest = NEW.application_request_digest
      AND application.receipt_digest = NEW.receipt_digest
      AND NEW.sealed_at_ms = application.applied_at_ms
)
OR (SELECT COUNT(*) FROM candidate_owners
    WHERE owner_plan_id = NEW.plan_id
      AND owner_plan_digest = NEW.plan_digest)
   <> (SELECT new_candidate_count FROM plan_applications WHERE plan_id = NEW.plan_id)
OR (SELECT COUNT(*) FROM candidate_owners
    WHERE owner_plan_id = NEW.plan_id
      AND owner_plan_digest = NEW.plan_digest
      AND state = 'owned')
   <> (SELECT new_candidate_count FROM plan_applications WHERE plan_id = NEW.plan_id)
OR (SELECT COUNT(*) FROM candidate_owners
    WHERE state = 'released'
      AND closed_by_plan_id = NEW.plan_id
      AND closed_by_plan_digest = NEW.plan_digest)
   <> (SELECT closed_candidate_count FROM plan_applications WHERE plan_id = NEW.plan_id)
OR (SELECT COUNT(*) FROM planned_downloads
    WHERE plan_id = NEW.plan_id AND plan_digest = NEW.plan_digest)
   <> (SELECT download_count FROM plan_applications WHERE plan_id = NEW.plan_id)
OR COALESCE((SELECT SUM(size_bytes) FROM planned_downloads
    WHERE plan_id = NEW.plan_id AND plan_digest = NEW.plan_digest), 0)
   <> (SELECT download_bytes FROM plan_applications WHERE plan_id = NEW.plan_id)
OR NOT EXISTS (
    SELECT 1 FROM plan_events
    WHERE plan_id = NEW.plan_id
      AND plan_digest = NEW.plan_digest
      AND event_index = 0
      AND event_type = 'applied'
)
OR EXISTS (
    SELECT 1 FROM plan_events AS event
    JOIN plan_applications AS application
      ON application.plan_id = event.plan_id
     AND application.plan_digest = event.plan_digest
    WHERE event.plan_id = NEW.plan_id
      AND event.plan_digest = NEW.plan_digest
      AND event.event_index = 0
      AND event.recorded_at_ms <> application.applied_at_ms
)
OR EXISTS (
    SELECT 1 FROM candidate_owners AS candidate
    JOIN plan_applications AS application
      ON application.plan_id = candidate.owner_plan_id
     AND application.plan_digest = candidate.owner_plan_digest
    WHERE candidate.owner_plan_id = NEW.plan_id
      AND candidate.owner_plan_digest = NEW.plan_digest
      AND candidate.created_at_ms <> application.applied_at_ms
)
OR EXISTS (
    SELECT 1 FROM candidate_owners AS candidate
    JOIN plan_applications AS application
      ON application.plan_id = candidate.closed_by_plan_id
     AND application.plan_digest = candidate.closed_by_plan_digest
    WHERE candidate.closed_by_plan_id = NEW.plan_id
      AND candidate.closed_by_plan_digest = NEW.plan_digest
      AND (candidate.closed_at_ms <> application.applied_at_ms
           OR candidate.close_reason <> 'cancel_candidate')
)
OR EXISTS (
    SELECT 1 FROM planned_downloads AS download
    JOIN plan_applications AS application
      ON application.plan_id = download.plan_id
     AND application.plan_digest = download.plan_digest
    WHERE download.plan_id = NEW.plan_id
      AND download.plan_digest = NEW.plan_digest
      AND download.created_at_ms <> application.applied_at_ms
)
OR EXISTS (SELECT 1 FROM fetch_claims WHERE state = 'prepared')
OR EXISTS (SELECT 1 FROM candidate_verification_runs WHERE state = 'prepared')
BEGIN
    SELECT RAISE(ABORT, 'plan application cannot seal with incomplete child rows');
END;

CREATE TRIGGER sealed_plan_candidate_insert
BEFORE INSERT ON candidate_owners
WHEN EXISTS (
    SELECT 1 FROM plan_application_seals
    WHERE plan_id = NEW.owner_plan_id AND plan_digest = NEW.owner_plan_digest
) BEGIN
    SELECT RAISE(ABORT, 'sealed plan cannot accept candidate owners');
END;

CREATE TRIGGER sealed_plan_download_insert
BEFORE INSERT ON planned_downloads
WHEN EXISTS (
    SELECT 1 FROM plan_application_seals
    WHERE plan_id = NEW.plan_id AND plan_digest = NEW.plan_digest
) BEGIN
    SELECT RAISE(ABORT, 'sealed plan cannot accept planned downloads');
END;

CREATE TRIGGER candidate_initial_state
BEFORE INSERT ON candidate_owners
WHEN NEW.state <> 'owned'
  OR NEW.closed_at_ms IS NOT NULL
  OR NEW.closed_by_plan_id IS NOT NULL
  OR NEW.closed_by_plan_digest IS NOT NULL
  OR NEW.close_reason IS NOT NULL
BEGIN
    SELECT RAISE(ABORT, 'candidate owner must start open and owned');
END;

CREATE TRIGGER candidate_identity_immutable
BEFORE UPDATE OF
    candidate_token, plugin_id, slot_ref, candidate_generation,
    release_json, permission_grant_digest, owner_plan_id, owner_plan_digest,
    application_inventory_revision, created_at_ms
ON candidate_owners BEGIN
    SELECT RAISE(ABORT, 'candidate ownership identity is immutable');
END;

CREATE TRIGGER candidate_state_transition
BEFORE UPDATE OF
    state, closed_at_ms, closed_by_plan_id, closed_by_plan_digest, close_reason
ON candidate_owners
WHEN NOT (
    (OLD.state = 'owned' AND NEW.state IN ('released', 'promoted', 'cleanup_pending'))
    OR (OLD.state = 'cleanup_pending' AND NEW.state = 'cleaned')
)
BEGIN
    SELECT RAISE(ABORT, 'candidate owner transition is not allowed');
END;

CREATE TRIGGER candidate_cleanup_pending_requires_authorization
BEFORE UPDATE OF
    state, closed_at_ms, closed_by_plan_id, closed_by_plan_digest, close_reason
ON candidate_owners
WHEN NEW.state = 'cleanup_pending' AND NOT EXISTS (
    SELECT 1
    FROM candidate_cleanup_authorizations AS authorization
    JOIN authority_meta AS meta ON meta.singleton = 1
    WHERE authorization.candidate_token = OLD.candidate_token
      AND meta.state_revision = authorization.authority_state_revision_after
      AND meta.inventory_revision = authorization.inventory_revision
      AND meta.inventory_digest = authorization.inventory_digest
      AND meta.authority_epoch = authorization.authority_epoch_after
      AND meta.process_owner_epoch = authorization.process_owner_epoch
      AND meta.trusted_time_high_water_ms = authorization.authorized_at_ms
      AND meta.updated_at_ms = authorization.authorized_at_ms
)
BEGIN
    SELECT RAISE(ABORT, 'candidate cleanup requires durable authorization');
END;

CREATE TRIGGER candidate_cleaned_requires_completion
BEFORE UPDATE OF
    state, closed_at_ms, closed_by_plan_id, closed_by_plan_digest, close_reason
ON candidate_owners
WHEN NEW.state = 'cleaned' AND NOT EXISTS (
    SELECT 1
    FROM candidate_cleanup_completions AS completion
    JOIN authority_meta AS meta ON meta.singleton = 1
    WHERE completion.candidate_token = OLD.candidate_token
      AND completion.completed_at_ms = NEW.closed_at_ms
      AND meta.state_revision = completion.authority_state_revision_after
      AND meta.inventory_revision = completion.inventory_revision_after
      AND meta.inventory_digest = completion.inventory_digest_after
      AND meta.authority_epoch = completion.authority_epoch_after
      AND meta.process_owner_epoch = completion.process_owner_epoch
      AND meta.trusted_time_high_water_ms = completion.completed_at_ms
      AND meta.updated_at_ms = completion.completed_at_ms
)
BEGIN
    SELECT RAISE(ABORT, 'candidate cleanup requires durable completion');
END;

CREATE TRIGGER candidate_promotion_requires_receipt
BEFORE UPDATE OF
    state, closed_at_ms, closed_by_plan_id, closed_by_plan_digest, close_reason
ON candidate_owners
WHEN NEW.state = 'promoted'
BEGIN
    SELECT RAISE(ABORT, 'candidate promotion is unavailable without content and health receipts');
END;

CREATE TRIGGER candidate_close_plan_open
BEFORE UPDATE OF
    state, closed_at_ms, closed_by_plan_id, closed_by_plan_digest, close_reason
ON candidate_owners
WHEN NEW.state = 'released' AND EXISTS (
    SELECT 1 FROM plan_application_seals
    WHERE plan_id = NEW.closed_by_plan_id AND plan_digest = NEW.closed_by_plan_digest
)
BEGIN
    SELECT RAISE(ABORT, 'sealed plan cannot acquire candidate closures');
END;

CREATE TRIGGER candidate_release_verification_fenced
BEFORE UPDATE OF
    state, closed_at_ms, closed_by_plan_id, closed_by_plan_digest, close_reason
ON candidate_owners
WHEN NEW.state = 'released' AND EXISTS (
    SELECT 1 FROM candidate_verification_runs
    WHERE candidate_token = OLD.candidate_token AND state = 'prepared'
)
BEGIN
    SELECT RAISE(ABORT, 'candidate release must revoke its prepared verification');
END;

CREATE TRIGGER candidate_owner_delete_forbidden
BEFORE DELETE ON candidate_owners BEGIN
    SELECT RAISE(ABORT, 'candidate ownership history is immutable');
END;

CREATE TRIGGER planned_download_initial_state
BEFORE INSERT ON planned_downloads
WHEN NEW.state <> 'pending'
  OR NEW.committed_offset <> 0
  OR NEW.cursor_generation <> 0
  OR NEW.updated_at_ms <> NEW.created_at_ms
BEGIN
    SELECT RAISE(ABORT, 'planned download must start at an empty pending cursor');
END;

CREATE TRIGGER planned_download_identity_immutable
BEFORE UPDATE OF
    plan_id, plan_digest, ordinal, item_index, candidate_token,
    artifact_kind, artifact_id, artifact_digest, source_ref, cache_class,
    part_relative_path, size_bytes, created_at_ms
ON planned_downloads BEGIN
    SELECT RAISE(ABORT, 'planned download identity is immutable');
END;

CREATE TRIGGER planned_download_state_transition
BEFORE UPDATE OF committed_offset, cursor_generation, state, updated_at_ms
ON planned_downloads
WHEN NEW.committed_offset < OLD.committed_offset
  OR NEW.cursor_generation < OLD.cursor_generation
  OR NEW.updated_at_ms < OLD.updated_at_ms
  OR (OLD.state = 'pending' AND NEW.state NOT IN ('pending', 'downloading', 'canceled', 'failed'))
  OR (OLD.state = 'downloading' AND NEW.state NOT IN ('downloading', 'complete', 'canceled', 'failed'))
  OR (OLD.state = 'failed' AND NEW.state NOT IN ('failed', 'downloading', 'canceled'))
  OR (OLD.state IN ('complete', 'canceled') AND (
      NEW.state <> OLD.state
      OR NEW.committed_offset <> OLD.committed_offset
      OR NEW.cursor_generation <> OLD.cursor_generation
  ))
BEGIN
    SELECT RAISE(ABORT, 'planned download cursor or state transition is invalid');
END;

CREATE TRIGGER planned_download_delete_forbidden
BEFORE DELETE ON planned_downloads BEGIN
    SELECT RAISE(ABORT, 'planned download history is immutable');
END;
"#;
