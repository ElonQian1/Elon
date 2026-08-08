/// V4 adds the immutable, no-download sharing-policy binding journal. Existing V3 objects remain
/// byte-for-byte unchanged: a V3 database is verified against the pure V3 reference before these
/// objects are installed.
///
/// `bound_at_ms` must come from the authenticated trusted-time kernel and be strictly newer than
/// the current authority high-water. Applying the receipt advances the high-water and
/// `updated_at_ms` together so rollback checkpoints cannot observe same-time divergent state.
pub(super) const SHARING_POLICY_BINDING_SCHEMA_V4: &str = r#"
DROP TRIGGER plan_application_matches_authority;
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
      AND NEW.applied_at_ms > meta.trusted_time_high_water_ms
)
BEGIN
    SELECT RAISE(ABORT, 'plan application does not match current authority');
END;

CREATE TABLE sharing_policy_binding_receipts (
    policy_revision                    INTEGER PRIMARY KEY CHECK (
        policy_revision > 0 AND policy_revision <= 9007199254740991
    ),
    request_digest                     TEXT NOT NULL UNIQUE CHECK (
        length(request_digest) = 64
        AND request_digest NOT GLOB '*[^0-9a-f]*'
    ),
    node_id                            TEXT NOT NULL CHECK (
        length(node_id) > 0 AND length(node_id) <= 256
        AND node_id = trim(node_id)
    ),
    owner_user_id                      TEXT NOT NULL CHECK (
        length(owner_user_id) > 0 AND length(owner_user_id) <= 256
        AND owner_user_id = trim(owner_user_id)
    ),
    installation_id_digest             TEXT NOT NULL CHECK (
        length(installation_id_digest) = 64
        AND installation_id_digest NOT GLOB '*[^0-9a-f]*'
    ),
    policy_digest                      TEXT NOT NULL CHECK (
        length(policy_digest) = 64
        AND policy_digest NOT GLOB '*[^0-9a-f]*'
    ),
    policy_snapshot_json               TEXT NOT NULL CHECK (
        length(policy_snapshot_json) > 0
        AND length(policy_snapshot_json) <= 65536
    ),
    policy_snapshot_digest             TEXT NOT NULL UNIQUE CHECK (
        length(policy_snapshot_digest) = 64
        AND policy_snapshot_digest NOT GLOB '*[^0-9a-f]*'
    ),
    sharing_enabled                    INTEGER NOT NULL CHECK (sharing_enabled IN (0, 1)),
    sharing_authorization_ref          TEXT CHECK (
        sharing_authorization_ref IS NULL OR (
            length(sharing_authorization_ref) > 0
            AND length(sharing_authorization_ref) <= 256
            AND sharing_authorization_ref = trim(sharing_authorization_ref)
        )
    ),
    sharing_authorization_revision     INTEGER CHECK (
        sharing_authorization_revision IS NULL OR (
            sharing_authorization_revision > 0
            AND sharing_authorization_revision <= 9007199254740991
        )
    ),
    sharing_authorization_digest       TEXT CHECK (
        sharing_authorization_digest IS NULL OR (
            length(sharing_authorization_digest) = 64
            AND sharing_authorization_digest NOT GLOB '*[^0-9a-f]*'
        )
    ),
    state_revision_before              INTEGER NOT NULL CHECK (state_revision_before >= 0),
    state_revision_after               INTEGER NOT NULL UNIQUE CHECK (
        state_revision_after = state_revision_before + 1
    ),
    inventory_revision_before          INTEGER NOT NULL CHECK (inventory_revision_before >= 0),
    inventory_revision_after           INTEGER NOT NULL UNIQUE CHECK (
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
    inventory_after_json               TEXT NOT NULL CHECK (
        length(inventory_after_json) > 0
        AND length(inventory_after_json) <= 16777216
    ),
    authority_epoch_before             INTEGER NOT NULL CHECK (authority_epoch_before >= 0),
    authority_epoch_after              INTEGER NOT NULL UNIQUE CHECK (
        authority_epoch_after = authority_epoch_before + 1
    ),
    process_owner_epoch                INTEGER NOT NULL CHECK (process_owner_epoch > 0),
    trusted_time_before_ms             INTEGER NOT NULL CHECK (trusted_time_before_ms >= 0),
    clock_status_before                TEXT NOT NULL CHECK (
        clock_status_before = 'trusted'
    ),
    authority_updated_at_ms_before     INTEGER NOT NULL CHECK (
        authority_updated_at_ms_before >= 0
    ),
    bound_at_ms                        INTEGER NOT NULL CHECK (
        bound_at_ms > authority_updated_at_ms_before
        AND bound_at_ms > trusted_time_before_ms
    ),
    receipt_json                       TEXT NOT NULL CHECK (
        length(receipt_json) > 0 AND length(receipt_json) <= 131072
    ),
    receipt_digest                     TEXT NOT NULL UNIQUE CHECK (
        length(receipt_digest) = 64
        AND receipt_digest NOT GLOB '*[^0-9a-f]*'
    ),
    CHECK (
        (sharing_enabled = 1
         AND sharing_authorization_ref IS NOT NULL
         AND sharing_authorization_revision = policy_revision
         AND sharing_authorization_digest = policy_digest)
        OR
        (sharing_enabled = 0
         AND sharing_authorization_ref IS NULL
         AND sharing_authorization_revision IS NULL
         AND sharing_authorization_digest IS NULL)
    )
);

CREATE TRIGGER sharing_policy_binding_revision_monotonic
BEFORE INSERT ON sharing_policy_binding_receipts
WHEN EXISTS (
    SELECT 1 FROM sharing_policy_binding_receipts
    WHERE policy_revision >= NEW.policy_revision
)
BEGIN
    SELECT RAISE(ABORT, 'sharing policy binding revision cannot roll back or fork');
END;

CREATE TRIGGER sharing_policy_binding_insert_fenced
BEFORE INSERT ON sharing_policy_binding_receipts
WHEN NOT EXISTS (
    SELECT 1 FROM authority_meta AS meta
    WHERE meta.singleton = 1
      AND meta.schema_version = 3
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
      AND NOT EXISTS (SELECT 1 FROM fetch_claims WHERE state = 'prepared')
      AND NOT EXISTS (
          SELECT 1 FROM candidate_verification_runs WHERE state = 'prepared'
      )
)
BEGIN
    SELECT RAISE(ABORT, 'sharing policy binding lost its exact authority fence');
END;

CREATE TRIGGER authority_sharing_policy_binding_receipt_required
BEFORE UPDATE OF
    desired_policy_revision, sharing_enabled,
    sharing_authorization_ref, sharing_authorization_revision,
    sharing_authorization_digest
ON authority_meta
WHEN (
    NEW.desired_policy_revision IS NOT OLD.desired_policy_revision
    OR NEW.sharing_enabled IS NOT OLD.sharing_enabled
    OR NEW.sharing_authorization_ref IS NOT OLD.sharing_authorization_ref
    OR NEW.sharing_authorization_revision IS NOT OLD.sharing_authorization_revision
    OR NEW.sharing_authorization_digest IS NOT OLD.sharing_authorization_digest
) AND NOT EXISTS (
    SELECT 1 FROM sharing_policy_binding_receipts AS receipt
    WHERE receipt.policy_revision = NEW.desired_policy_revision
      AND receipt.installation_id_digest = OLD.installation_id_digest
      AND receipt.state_revision_before = OLD.state_revision
      AND receipt.state_revision_after = NEW.state_revision
      AND receipt.inventory_revision_before = OLD.inventory_revision
      AND receipt.inventory_revision_after = NEW.inventory_revision
      AND receipt.inventory_digest_before = OLD.inventory_digest
      AND receipt.inventory_digest_after = NEW.inventory_digest
      AND receipt.inventory_after_json = NEW.inventory_json
      AND receipt.authority_epoch_before = OLD.authority_epoch
      AND receipt.authority_epoch_after = NEW.authority_epoch
      AND receipt.process_owner_epoch = OLD.process_owner_epoch
      AND NEW.process_owner_epoch = OLD.process_owner_epoch
      AND receipt.trusted_time_before_ms = OLD.trusted_time_high_water_ms
      AND NEW.trusted_time_high_water_ms = receipt.bound_at_ms
      AND receipt.clock_status_before = 'trusted'
      AND OLD.clock_status = 'trusted'
      AND NEW.clock_status = 'trusted'
      AND receipt.authority_updated_at_ms_before = OLD.updated_at_ms
      AND NEW.updated_at_ms = receipt.bound_at_ms
      AND receipt.sharing_enabled = NEW.sharing_enabled
      AND receipt.sharing_authorization_ref IS NEW.sharing_authorization_ref
      AND receipt.sharing_authorization_revision IS NEW.sharing_authorization_revision
      AND receipt.sharing_authorization_digest IS NEW.sharing_authorization_digest
)
BEGIN
    SELECT RAISE(ABORT, 'sharing policy fields require an exact immutable binding receipt');
END;

CREATE TRIGGER sharing_policy_binding_apply_authority
AFTER INSERT ON sharing_policy_binding_receipts
BEGIN
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
        clock_status = 'trusted',
        updated_at_ms = NEW.bound_at_ms
    WHERE singleton = 1
      AND schema_version = 3
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

CREATE TRIGGER sharing_policy_binding_receipt_update_forbidden
BEFORE UPDATE ON sharing_policy_binding_receipts
BEGIN
    SELECT RAISE(ABORT, 'sharing policy binding receipts are immutable');
END;

CREATE TRIGGER sharing_policy_binding_receipt_delete_forbidden
BEFORE DELETE ON sharing_policy_binding_receipts
BEGIN
    SELECT RAISE(ABORT, 'sharing policy binding receipts are append-only');
END;
"#;
