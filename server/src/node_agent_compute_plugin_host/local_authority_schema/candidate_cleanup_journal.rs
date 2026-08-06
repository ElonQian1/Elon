/// Append-only physical cleanup journal anchored to one sealed expected-object topology.
///
/// Four events per object deliberately separate the durable intent, exact handle disposition,
/// parent-relative absence observation and namespace durability boundary. Completion may reference
/// only the terminal durability event after every object has advanced through the same chain.
pub(super) const CANDIDATE_CLEANUP_JOURNAL_SCHEMA_V3: &str = r#"
CREATE TABLE candidate_cleanup_step_events (
    cleanup_id                  TEXT NOT NULL,
    plan_digest                TEXT NOT NULL,
    event_sequence             INTEGER NOT NULL CHECK (
        event_sequence > 0 AND event_sequence <= 131072
    ),
    step_ordinal               INTEGER NOT NULL CHECK (
        step_ordinal >= 0 AND step_ordinal < 32768
    ),
    event_kind                 TEXT NOT NULL CHECK (
        event_kind IN (
            'delete_intent',
            'exact_handle_disposition_set',
            'absence_recovered_after_intent',
            'parent_namespace_absence_observed',
            'namespace_durable'
        )
    ),
    object_digest              TEXT NOT NULL CHECK (
        length(object_digest) = 64
        AND object_digest NOT GLOB '*[^0-9a-f]*'
    ),
    observed_identity_digest   TEXT CHECK (
        observed_identity_digest IS NULL OR (
            length(observed_identity_digest) = 64
            AND observed_identity_digest NOT GLOB '*[^0-9a-f]*'
        )
    ),
    observed_parent_identity_digest TEXT NOT NULL CHECK (
        length(observed_parent_identity_digest) = 64
        AND observed_parent_identity_digest NOT GLOB '*[^0-9a-f]*'
    ),
    namespace_durability_kind TEXT CHECK (
        namespace_durability_kind IS NULL OR (
            length(namespace_durability_kind) > 0
            AND length(namespace_durability_kind) <= 128
        )
    ),
    namespace_durability_evidence_digest TEXT CHECK (
        namespace_durability_evidence_digest IS NULL OR (
            length(namespace_durability_evidence_digest) = 64
            AND namespace_durability_evidence_digest NOT GLOB '*[^0-9a-f]*'
        )
    ),
    previous_event_digest      TEXT NOT NULL CHECK (
        length(previous_event_digest) = 64
        AND previous_event_digest NOT GLOB '*[^0-9a-f]*'
    ),
    process_owner_epoch        INTEGER NOT NULL CHECK (process_owner_epoch > 0),
    recorded_at_ms             INTEGER NOT NULL CHECK (recorded_at_ms >= 0),
    event_json                 TEXT NOT NULL CHECK (
        length(event_json) > 0 AND length(event_json) <= 131072
    ),
    event_digest               TEXT NOT NULL UNIQUE CHECK (
        length(event_digest) = 64
        AND event_digest NOT GLOB '*[^0-9a-f]*'
    ),
    CHECK (
        (
            event_kind = 'namespace_durable'
            AND namespace_durability_kind IS NOT NULL
            AND namespace_durability_evidence_digest IS NOT NULL
        )
        OR
        (
            event_kind <> 'namespace_durable'
            AND namespace_durability_kind IS NULL
            AND namespace_durability_evidence_digest IS NULL
        )
    ),
    PRIMARY KEY (cleanup_id, event_sequence),
    FOREIGN KEY (cleanup_id, plan_digest)
        REFERENCES candidate_cleanup_execution_plan_seals(cleanup_id, plan_digest)
        ON DELETE RESTRICT,
    FOREIGN KEY (cleanup_id, step_ordinal, object_digest)
        REFERENCES candidate_cleanup_expected_objects(
            cleanup_id, step_ordinal, object_digest
        ) ON DELETE RESTRICT
);

CREATE TRIGGER candidate_cleanup_step_event_insert_fenced
BEFORE INSERT ON candidate_cleanup_step_events
WHEN NOT EXISTS (
    SELECT 1
    FROM candidate_cleanup_execution_plan_seals AS seal
    JOIN candidate_cleanup_execution_plans AS plan
      ON plan.cleanup_id = seal.cleanup_id
     AND plan.plan_digest = seal.plan_digest
    JOIN candidate_cleanup_expected_objects AS object
      ON object.cleanup_id = plan.cleanup_id
     AND object.step_ordinal = NEW.step_ordinal
     AND object.object_digest = NEW.object_digest
    JOIN candidate_cleanup_authorizations AS authorization
      ON authorization.cleanup_id = plan.cleanup_id
     AND authorization.candidate_token = plan.candidate_token
     AND authorization.receipt_digest = plan.authorization_receipt_digest
    JOIN candidate_staging_receipts AS staging
      ON staging.staging_id = authorization.staging_id
     AND staging.candidate_token = authorization.candidate_token
    JOIN candidate_owners AS candidate
      ON candidate.candidate_token = plan.candidate_token
    JOIN authority_meta AS meta ON meta.singleton = 1
    WHERE seal.cleanup_id = NEW.cleanup_id
      AND seal.plan_digest = NEW.plan_digest
      AND candidate.state = 'cleanup_pending'
      AND meta.clock_status = 'trusted'
      AND meta.installation_id_digest = plan.installation_id_digest
      AND staging.root_identity_digest = plan.root_identity_digest
      AND meta.process_owner_epoch = NEW.process_owner_epoch
      AND meta.trusted_time_high_water_ms = NEW.recorded_at_ms
      AND meta.updated_at_ms = NEW.recorded_at_ms
      AND NEW.recorded_at_ms >= plan.planned_at_ms
      AND NEW.event_sequence <= plan.object_count * 4
      AND NEW.observed_parent_identity_digest = object.expected_parent_identity_digest
      AND (
          (
              NEW.event_kind = 'delete_intent'
              AND NEW.event_sequence = NEW.step_ordinal * 4 + 1
              AND NEW.observed_identity_digest = object.expected_identity_digest
          )
          OR
          (
              NEW.event_kind IN (
                  'exact_handle_disposition_set', 'absence_recovered_after_intent'
              )
              AND NEW.event_sequence = NEW.step_ordinal * 4 + 2
              AND (
                  (NEW.event_kind = 'exact_handle_disposition_set'
                      AND NEW.observed_identity_digest = object.expected_identity_digest)
                  OR
                  (NEW.event_kind = 'absence_recovered_after_intent'
                      AND NEW.observed_identity_digest IS NULL)
              )
              AND EXISTS (
                  SELECT 1 FROM candidate_cleanup_step_events AS intent
                  WHERE intent.cleanup_id = NEW.cleanup_id
                    AND intent.event_sequence = NEW.event_sequence - 1
                    AND intent.step_ordinal = NEW.step_ordinal
                    AND intent.event_kind = 'delete_intent'
                    AND intent.object_digest = NEW.object_digest
              )
          )
          OR
          (
              NEW.event_kind = 'parent_namespace_absence_observed'
              AND NEW.event_sequence = NEW.step_ordinal * 4 + 3
              AND NEW.observed_identity_digest IS NULL
              AND EXISTS (
                  SELECT 1 FROM candidate_cleanup_step_events AS disposition
                  WHERE disposition.cleanup_id = NEW.cleanup_id
                    AND disposition.event_sequence = NEW.event_sequence - 1
                    AND disposition.step_ordinal = NEW.step_ordinal
                    AND disposition.event_kind IN (
                        'exact_handle_disposition_set', 'absence_recovered_after_intent'
                    )
                    AND disposition.object_digest = NEW.object_digest
              )
          )
          OR
          (
              NEW.event_kind = 'namespace_durable'
              AND NEW.event_sequence = NEW.step_ordinal * 4 + 4
              AND NEW.observed_identity_digest IS NULL
              AND EXISTS (
                  SELECT 1 FROM candidate_cleanup_step_events AS absence
                  WHERE absence.cleanup_id = NEW.cleanup_id
                    AND absence.event_sequence = NEW.event_sequence - 1
                    AND absence.step_ordinal = NEW.step_ordinal
                    AND absence.event_kind = 'parent_namespace_absence_observed'
                    AND absence.object_digest = NEW.object_digest
              )
          )
      )
      AND (
          (
              NEW.event_sequence = 1
              AND NEW.previous_event_digest = plan.plan_digest
          )
          OR EXISTS (
              SELECT 1 FROM candidate_cleanup_step_events AS previous
              WHERE previous.cleanup_id = NEW.cleanup_id
                AND previous.event_sequence = NEW.event_sequence - 1
                AND previous.event_digest = NEW.previous_event_digest
                AND previous.recorded_at_ms <= NEW.recorded_at_ms
                AND (
                    NEW.event_kind <> 'delete_intent'
                    OR previous.event_kind = 'namespace_durable'
                )
          )
      )
      AND NOT EXISTS (
          SELECT 1 FROM candidate_cleanup_completions AS completion
          WHERE completion.cleanup_id = NEW.cleanup_id
      )
)
BEGIN
    SELECT RAISE(ABORT, 'candidate cleanup step event lost its sealed monotonic fence');
END;

CREATE TRIGGER candidate_cleanup_step_event_update_forbidden
BEFORE UPDATE ON candidate_cleanup_step_events
BEGIN
    SELECT RAISE(ABORT, 'candidate cleanup step event is append-only');
END;

CREATE TRIGGER candidate_cleanup_step_event_delete_forbidden
BEFORE DELETE ON candidate_cleanup_step_events
BEGIN
    SELECT RAISE(ABORT, 'candidate cleanup step event is append-only');
END;

CREATE TRIGGER candidate_cleanup_completion_requires_execution_journal
BEFORE INSERT ON candidate_cleanup_completions
WHEN NOT EXISTS (
    SELECT 1
    FROM candidate_cleanup_execution_plan_seals AS seal
    JOIN candidate_cleanup_execution_plans AS plan
      ON plan.cleanup_id = seal.cleanup_id
     AND plan.plan_digest = seal.plan_digest
    WHERE plan.cleanup_id = NEW.cleanup_id
      AND plan.candidate_token = NEW.candidate_token
      AND plan.authorization_receipt_digest = NEW.authorization_receipt_digest
      AND plan.plan_digest = NEW.execution_plan_digest
      AND (SELECT COUNT(*) FROM candidate_cleanup_step_events AS event
           WHERE event.cleanup_id = NEW.cleanup_id
             AND event.event_kind = 'namespace_durable') = plan.object_count
      AND EXISTS (
          SELECT 1 FROM candidate_cleanup_step_events AS terminal
          WHERE terminal.cleanup_id = NEW.cleanup_id
            AND terminal.event_sequence = plan.object_count * 4
            AND terminal.event_digest = NEW.terminal_journal_digest
            AND terminal.event_kind = 'namespace_durable'
      )
)
BEGIN
    SELECT RAISE(ABORT, 'candidate cleanup completion requires terminal execution journal');
END;
"#;
