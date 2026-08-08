/// V5 turns policy replacement into one durable authority transition. The companion receipt is
/// inserted first and has a deferred foreign key to the immutable V4 binding receipt. Inserting
/// that binding receipt terminalizes the complete prepared-work set before advancing authority.
pub(super) const SHARING_POLICY_REVOCATION_SCHEMA_V5: &str = r#"
CREATE TRIGGER authority_process_owner_time_strict_v5
BEFORE UPDATE OF process_owner_epoch ON authority_meta
WHEN NEW.process_owner_epoch IS NOT OLD.process_owner_epoch AND (
    OLD.trusted_time_high_water_ms IS NULL
    OR OLD.clock_status <> 'trusted'
    OR NEW.clock_status <> 'trusted'
    OR NEW.trusted_time_high_water_ms <= OLD.trusted_time_high_water_ms
    OR NEW.updated_at_ms <= OLD.updated_at_ms
    OR NEW.updated_at_ms <> NEW.trusted_time_high_water_ms
)
BEGIN
    SELECT RAISE(ABORT, 'process owner transition requires strictly newer trusted time');
END;

CREATE TABLE sharing_policy_binding_revocation_receipts (
    policy_revision                   INTEGER PRIMARY KEY CHECK (
        policy_revision > 0 AND policy_revision <= 9007199254740991
    ),
    request_digest                    TEXT NOT NULL UNIQUE CHECK (
        length(request_digest) = 64 AND request_digest NOT GLOB '*[^0-9a-f]*'
    ),
    policy_binding_receipt_digest     TEXT NOT NULL UNIQUE CHECK (
        length(policy_binding_receipt_digest) = 64
        AND policy_binding_receipt_digest NOT GLOB '*[^0-9a-f]*'
    ),
    installation_id_digest           TEXT NOT NULL CHECK (
        length(installation_id_digest) = 64
        AND installation_id_digest NOT GLOB '*[^0-9a-f]*'
    ),
    authority_epoch_before           INTEGER NOT NULL CHECK (authority_epoch_before >= 0),
    process_owner_epoch              INTEGER NOT NULL CHECK (process_owner_epoch > 0),
    trusted_time_before_ms           INTEGER NOT NULL CHECK (trusted_time_before_ms >= 0),
    bound_at_ms                      INTEGER NOT NULL CHECK (bound_at_ms > trusted_time_before_ms),
    fetch_claim_count                INTEGER NOT NULL CHECK (
        fetch_claim_count >= 0 AND fetch_claim_count <= 4096
    ),
    verification_count               INTEGER NOT NULL CHECK (
        verification_count >= 0 AND verification_count <= 4096
    ),
    work_item_count                  INTEGER NOT NULL CHECK (
        work_item_count >= 0 AND work_item_count <= 8192
        AND work_item_count = fetch_claim_count + verification_count
    ),
    work_set_json                    TEXT NOT NULL CHECK (
        length(work_set_json) > 0 AND length(work_set_json) <= 4194304
    ),
    work_set_digest                  TEXT NOT NULL CHECK (
        length(work_set_digest) = 64 AND work_set_digest NOT GLOB '*[^0-9a-f]*'
    ),
    fetch_resolution_reason          TEXT NOT NULL CHECK (
        fetch_resolution_reason = 'sharing_policy_transition_aborted'
    ),
    verification_resolution_reason   TEXT NOT NULL CHECK (
        verification_resolution_reason = 'verification_aborted'
    ),
    verification_result_json         TEXT NOT NULL CHECK (
        length(verification_result_json) > 0
        AND length(verification_result_json) <= 4096
    ),
    verification_result_digest       TEXT NOT NULL CHECK (
        length(verification_result_digest) = 64
        AND verification_result_digest NOT GLOB '*[^0-9a-f]*'
    ),
    receipt_json                     TEXT NOT NULL CHECK (
        length(receipt_json) > 0 AND length(receipt_json) <= 131072
    ),
    receipt_digest                   TEXT NOT NULL UNIQUE CHECK (
        length(receipt_digest) = 64 AND receipt_digest NOT GLOB '*[^0-9a-f]*'
    ),
    FOREIGN KEY (policy_revision)
        REFERENCES sharing_policy_binding_receipts(policy_revision)
        ON DELETE RESTRICT DEFERRABLE INITIALLY DEFERRED
);

CREATE TRIGGER sharing_policy_binding_revocation_insert_fenced
BEFORE INSERT ON sharing_policy_binding_revocation_receipts
WHEN EXISTS (
    SELECT 1 FROM sharing_policy_binding_revocation_receipts
    WHERE policy_revision >= NEW.policy_revision
) OR EXISTS (
    SELECT 1 FROM sharing_policy_binding_receipts
    WHERE policy_revision = NEW.policy_revision
       OR request_digest = NEW.request_digest
       OR receipt_digest = NEW.policy_binding_receipt_digest
) OR NOT EXISTS (
    SELECT 1 FROM authority_meta AS meta
    WHERE meta.singleton = 1
      AND meta.schema_version = 3
      AND meta.installation_id_digest = NEW.installation_id_digest
      AND meta.desired_policy_revision < NEW.policy_revision
      AND meta.authority_epoch = NEW.authority_epoch_before
      AND meta.process_owner_epoch = NEW.process_owner_epoch
      AND meta.process_owner_epoch > 0
      AND meta.trusted_time_high_water_ms = NEW.trusted_time_before_ms
      AND meta.clock_status = 'trusted'
      AND meta.updated_at_ms = NEW.trusted_time_before_ms
      AND NEW.bound_at_ms > meta.trusted_time_high_water_ms
      AND NEW.fetch_claim_count = (
          SELECT COUNT(*) FROM fetch_claims WHERE state = 'prepared'
      )
      AND NEW.verification_count = (
          SELECT COUNT(*) FROM candidate_verification_runs WHERE state = 'prepared'
      )
      AND NOT EXISTS (
          SELECT 1 FROM fetch_claims
          WHERE state = 'prepared' AND (
              authority_epoch <> NEW.authority_epoch_before
              OR process_owner_epoch <> NEW.process_owner_epoch
              OR prepared_at_ms > NEW.trusted_time_before_ms
              OR prepared_at_ms >= NEW.bound_at_ms
          )
      )
      AND NOT EXISTS (
          SELECT 1 FROM candidate_verification_runs
          WHERE state = 'prepared' AND (
              authority_state_revision <> meta.state_revision
              OR authority_epoch <> NEW.authority_epoch_before
              OR process_owner_epoch <> NEW.process_owner_epoch
              OR prepared_at_ms > NEW.trusted_time_before_ms
              OR prepared_at_ms >= NEW.bound_at_ms
          )
      )
)
BEGIN
    SELECT RAISE(ABORT, 'policy prepared-work receipt lost its exact authority fence');
END;

CREATE TRIGGER sharing_policy_binding_revocation_update_forbidden
BEFORE UPDATE ON sharing_policy_binding_revocation_receipts
BEGIN
    SELECT RAISE(ABORT, 'policy prepared-work receipts are immutable');
END;

CREATE TRIGGER sharing_policy_binding_revocation_delete_forbidden
BEFORE DELETE ON sharing_policy_binding_revocation_receipts
BEGIN
    SELECT RAISE(ABORT, 'policy prepared-work receipts are append-only');
END;

CREATE TRIGGER fetch_claim_policy_transition_abort_fenced
BEFORE UPDATE OF state, resolved_at_ms, resolution_reason ON fetch_claims
WHEN NEW.state = 'aborted'
 AND NEW.resolution_reason = 'sharing_policy_transition_aborted'
 AND NOT (
    OLD.state = 'prepared'
    AND NEW.redirect_generation = OLD.redirect_generation
    AND NEW.resolved_at_ms IS NOT NULL
    AND EXISTS (
        SELECT 1
        FROM sharing_policy_binding_revocation_receipts AS revocation
        JOIN sharing_policy_binding_receipts AS binding
          ON binding.policy_revision = revocation.policy_revision
         AND binding.request_digest = revocation.request_digest
         AND binding.receipt_digest = revocation.policy_binding_receipt_digest
        JOIN authority_meta AS meta ON meta.singleton = 1
        JOIN planned_downloads AS download
          ON download.plan_id = OLD.plan_id
         AND download.plan_digest = OLD.plan_digest
         AND download.ordinal = OLD.ordinal
         AND download.candidate_token = OLD.candidate_token
        WHERE revocation.installation_id_digest = meta.installation_id_digest
          AND binding.authority_epoch_before = revocation.authority_epoch_before
          AND binding.process_owner_epoch = revocation.process_owner_epoch
          AND binding.trusted_time_before_ms = revocation.trusted_time_before_ms
          AND binding.bound_at_ms = revocation.bound_at_ms
          AND meta.state_revision = binding.state_revision_before
          AND meta.inventory_revision = binding.inventory_revision_before
          AND meta.inventory_digest = binding.inventory_digest_before
          AND meta.authority_epoch = binding.authority_epoch_before
          AND meta.process_owner_epoch = binding.process_owner_epoch
          AND meta.trusted_time_high_water_ms = binding.trusted_time_before_ms
          AND meta.clock_status = 'trusted'
          AND meta.updated_at_ms = binding.authority_updated_at_ms_before
          AND NEW.resolved_at_ms = revocation.bound_at_ms
          AND OLD.prepared_at_ms < revocation.bound_at_ms
          AND OLD.authority_epoch = revocation.authority_epoch_before
          AND OLD.process_owner_epoch = revocation.process_owner_epoch
          AND download.cursor_generation = OLD.cursor_generation
          AND download.committed_offset = OLD.offset_bytes
    )
)
BEGIN
    SELECT RAISE(ABORT, 'policy fetch abort lacks an exact companion receipt');
END;

DROP TRIGGER candidate_verification_abort_fenced;
CREATE TRIGGER candidate_verification_abort_fenced
BEFORE UPDATE OF
    state, resolved_at_ms, resolution_reason, result_json, result_digest,
    observed_artifact_set_digest, mismatch_ordinal, mismatch_observed_digest
ON candidate_verification_runs
WHEN NEW.state = 'aborted' AND CASE WHEN (
    OLD.state = 'prepared'
    AND OLD.resolved_at_ms IS NULL AND OLD.resolution_reason IS NULL
    AND OLD.result_json IS NULL AND OLD.result_digest IS NULL
    AND OLD.observed_artifact_set_digest IS NULL
    AND OLD.mismatch_ordinal IS NULL AND OLD.mismatch_observed_digest IS NULL
    AND NEW.resolved_at_ms IS NOT NULL
    AND NEW.resolved_at_ms >= OLD.prepared_at_ms
    AND NEW.resolution_reason IN ('verification_aborted', 'authority_recovery')
    AND length(NEW.result_json) > 0
    AND length(NEW.result_digest) = 64
    AND NEW.result_digest NOT GLOB '*[^0-9a-f]*'
    AND NEW.observed_artifact_set_digest IS NULL
    AND NEW.mismatch_ordinal IS NULL AND NEW.mismatch_observed_digest IS NULL
    AND EXISTS (
        SELECT 1 FROM authority_meta AS meta
        WHERE meta.singleton = 1 AND meta.clock_status = 'trusted'
          AND meta.trusted_time_high_water_ms = NEW.resolved_at_ms
          AND meta.state_revision = OLD.authority_state_revision
          AND meta.authority_epoch = OLD.authority_epoch
          AND meta.process_owner_epoch = OLD.process_owner_epoch
    )
) OR (
    OLD.state = 'prepared'
    AND OLD.resolved_at_ms IS NULL AND OLD.resolution_reason IS NULL
    AND OLD.result_json IS NULL AND OLD.result_digest IS NULL
    AND OLD.observed_artifact_set_digest IS NULL
    AND OLD.mismatch_ordinal IS NULL AND OLD.mismatch_observed_digest IS NULL
    AND NEW.resolved_at_ms IS NOT NULL AND NEW.resolved_at_ms > OLD.prepared_at_ms
    AND NEW.resolution_reason = 'verification_aborted'
    AND NEW.observed_artifact_set_digest IS NULL
    AND NEW.mismatch_ordinal IS NULL AND NEW.mismatch_observed_digest IS NULL
    AND EXISTS (
        SELECT 1
        FROM sharing_policy_binding_revocation_receipts AS revocation
        JOIN sharing_policy_binding_receipts AS binding
          ON binding.policy_revision = revocation.policy_revision
         AND binding.request_digest = revocation.request_digest
         AND binding.receipt_digest = revocation.policy_binding_receipt_digest
        JOIN authority_meta AS meta ON meta.singleton = 1
        WHERE revocation.installation_id_digest = meta.installation_id_digest
          AND binding.authority_epoch_before = revocation.authority_epoch_before
          AND binding.process_owner_epoch = revocation.process_owner_epoch
          AND binding.trusted_time_before_ms = revocation.trusted_time_before_ms
          AND binding.bound_at_ms = revocation.bound_at_ms
          AND meta.state_revision = binding.state_revision_before
          AND meta.inventory_revision = binding.inventory_revision_before
          AND meta.inventory_digest = binding.inventory_digest_before
          AND meta.authority_epoch = binding.authority_epoch_before
          AND meta.process_owner_epoch = binding.process_owner_epoch
          AND meta.trusted_time_high_water_ms = binding.trusted_time_before_ms
          AND meta.clock_status = 'trusted'
          AND meta.updated_at_ms = binding.authority_updated_at_ms_before
          AND NEW.resolved_at_ms = revocation.bound_at_ms
          AND NEW.result_json = revocation.verification_result_json
          AND NEW.result_digest = revocation.verification_result_digest
          AND OLD.authority_state_revision = meta.state_revision
          AND OLD.authority_epoch = revocation.authority_epoch_before
          AND OLD.process_owner_epoch = revocation.process_owner_epoch
          AND OLD.prepared_at_ms < revocation.bound_at_ms
    )
) THEN 0 ELSE 1 END
BEGIN
    SELECT RAISE(ABORT, 'candidate verification abort lost its authority fence');
END;

DROP TRIGGER sharing_policy_binding_insert_fenced;
CREATE TRIGGER sharing_policy_binding_insert_fenced
BEFORE INSERT ON sharing_policy_binding_receipts
WHEN NOT EXISTS (
    SELECT 1
    FROM authority_meta AS meta
    JOIN sharing_policy_binding_revocation_receipts AS revocation
      ON revocation.policy_revision = NEW.policy_revision
     AND revocation.request_digest = NEW.request_digest
     AND revocation.policy_binding_receipt_digest = NEW.receipt_digest
     AND revocation.installation_id_digest = NEW.installation_id_digest
     AND revocation.authority_epoch_before = NEW.authority_epoch_before
     AND revocation.process_owner_epoch = NEW.process_owner_epoch
     AND revocation.trusted_time_before_ms = NEW.trusted_time_before_ms
     AND revocation.bound_at_ms = NEW.bound_at_ms
    WHERE meta.singleton = 1 AND meta.schema_version = 3
      AND meta.installation_id_digest = NEW.installation_id_digest
      AND meta.desired_policy_revision < NEW.policy_revision
      AND meta.state_revision = NEW.state_revision_before
      AND meta.inventory_revision = NEW.inventory_revision_before
      AND meta.inventory_digest = NEW.inventory_digest_before
      AND meta.authority_epoch = NEW.authority_epoch_before
      AND meta.process_owner_epoch = NEW.process_owner_epoch
      AND meta.process_owner_epoch > 0
      AND meta.trusted_time_high_water_ms = NEW.trusted_time_before_ms
      AND meta.clock_status = 'trusted'
      AND meta.updated_at_ms = NEW.authority_updated_at_ms_before
      AND revocation.fetch_claim_count = (
          SELECT COUNT(*) FROM fetch_claims WHERE state = 'prepared'
      )
      AND revocation.verification_count = (
          SELECT COUNT(*) FROM candidate_verification_runs WHERE state = 'prepared'
      )
)
BEGIN
    SELECT RAISE(ABORT, 'sharing policy binding lost its exact revocation fence');
END;

DROP TRIGGER sharing_policy_binding_apply_authority;
CREATE TRIGGER sharing_policy_binding_apply_authority
AFTER INSERT ON sharing_policy_binding_receipts
BEGIN
    UPDATE fetch_claims SET
        state = 'aborted', resolved_at_ms = NEW.bound_at_ms,
        resolution_reason = (
            SELECT fetch_resolution_reason
            FROM sharing_policy_binding_revocation_receipts
            WHERE policy_revision = NEW.policy_revision
        )
    WHERE state = 'prepared'
      AND authority_epoch = NEW.authority_epoch_before
      AND process_owner_epoch = NEW.process_owner_epoch
      AND prepared_at_ms < NEW.bound_at_ms;
    SELECT RAISE(ABORT, 'sharing policy fetch abort count changed')
    WHERE changes() <> (
        SELECT fetch_claim_count FROM sharing_policy_binding_revocation_receipts
        WHERE policy_revision = NEW.policy_revision
    );

    UPDATE candidate_verification_runs SET
        state = 'aborted', resolved_at_ms = NEW.bound_at_ms,
        resolution_reason = (
            SELECT verification_resolution_reason
            FROM sharing_policy_binding_revocation_receipts
            WHERE policy_revision = NEW.policy_revision
        ),
        result_json = (
            SELECT verification_result_json
            FROM sharing_policy_binding_revocation_receipts
            WHERE policy_revision = NEW.policy_revision
        ),
        result_digest = (
            SELECT verification_result_digest
            FROM sharing_policy_binding_revocation_receipts
            WHERE policy_revision = NEW.policy_revision
        )
    WHERE state = 'prepared'
      AND authority_epoch = NEW.authority_epoch_before
      AND process_owner_epoch = NEW.process_owner_epoch
      AND prepared_at_ms < NEW.bound_at_ms;
    SELECT RAISE(ABORT, 'sharing policy verification abort count changed')
    WHERE changes() <> (
        SELECT verification_count FROM sharing_policy_binding_revocation_receipts
        WHERE policy_revision = NEW.policy_revision
    );
    SELECT RAISE(ABORT, 'sharing policy prepared work was not terminalized')
    WHERE EXISTS (SELECT 1 FROM fetch_claims WHERE state = 'prepared')
       OR EXISTS (SELECT 1 FROM candidate_verification_runs WHERE state = 'prepared');

    UPDATE authority_meta SET
        state_revision = NEW.state_revision_after,
        inventory_revision = NEW.inventory_revision_after,
        inventory_digest = NEW.inventory_digest_after,
        inventory_json = NEW.inventory_after_json,
        desired_policy_revision = NEW.policy_revision,
        sharing_enabled = NEW.sharing_enabled,
        sharing_authorization_ref = NEW.sharing_authorization_ref,
        sharing_authorization_revision = NEW.sharing_authorization_revision,
        sharing_authorization_digest = NEW.sharing_authorization_digest,
        authority_epoch = NEW.authority_epoch_after,
        trusted_time_high_water_ms = NEW.bound_at_ms,
        clock_status = 'trusted', updated_at_ms = NEW.bound_at_ms
    WHERE singleton = 1 AND schema_version = 3
      AND installation_id_digest = NEW.installation_id_digest
      AND desired_policy_revision < NEW.policy_revision
      AND state_revision = NEW.state_revision_before
      AND inventory_revision = NEW.inventory_revision_before
      AND inventory_digest = NEW.inventory_digest_before
      AND authority_epoch = NEW.authority_epoch_before
      AND process_owner_epoch = NEW.process_owner_epoch
      AND process_owner_epoch > 0
      AND trusted_time_high_water_ms = NEW.trusted_time_before_ms
      AND clock_status = 'trusted'
      AND updated_at_ms = NEW.authority_updated_at_ms_before
      AND NOT EXISTS (SELECT 1 FROM fetch_claims WHERE state = 'prepared')
      AND NOT EXISTS (
          SELECT 1 FROM candidate_verification_runs WHERE state = 'prepared'
      );
    SELECT RAISE(ABORT, 'sharing policy binding authority CAS did not update exactly once')
    WHERE changes() <> 1;
END;
"#;
