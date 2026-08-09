/// V7 seals installation and activation as one locally authoritative transition.
///
/// Both receipts are inserted before the authority projection advances. Their deferred cycle
/// prevents either half from committing alone; the owner transition then consumes their exact
/// installed/active projection. Health is only installation input, never runtime readiness.
pub(super) const CANDIDATE_PROMOTION_CORE_SCHEMA_V7: &str = r#"
DROP TRIGGER candidate_promotion_requires_receipt;

CREATE TABLE candidate_install_receipts (
    install_id                         TEXT PRIMARY KEY CHECK (length(install_id) > 0),
    promotion_id                       TEXT NOT NULL UNIQUE CHECK (length(promotion_id) > 0),
    candidate_token                    TEXT NOT NULL UNIQUE,
    candidate_token_digest             TEXT NOT NULL CHECK (
        length(candidate_token_digest) = 64
        AND candidate_token_digest NOT GLOB '*[^0-9a-f]*'
    ),
    installation_id_digest             TEXT NOT NULL CHECK (
        length(installation_id_digest) = 64
        AND installation_id_digest NOT GLOB '*[^0-9a-f]*'
    ),
    plugin_id                          TEXT NOT NULL CHECK (length(plugin_id) > 0),
    slot_ref                           TEXT NOT NULL CHECK (length(slot_ref) > 0),
    candidate_generation               INTEGER NOT NULL CHECK (candidate_generation > 0),
    owner_plan_id                      TEXT NOT NULL,
    owner_plan_digest                  TEXT NOT NULL CHECK (
        length(owner_plan_digest) = 64 AND owner_plan_digest NOT GLOB '*[^0-9a-f]*'
    ),
    application_inventory_revision    INTEGER NOT NULL CHECK (
        application_inventory_revision > 0
    ),
    staging_id                         TEXT NOT NULL UNIQUE,
    staging_receipt_digest             TEXT NOT NULL CHECK (
        length(staging_receipt_digest) = 64
        AND staging_receipt_digest NOT GLOB '*[^0-9a-f]*'
    ),
    staging_run_digest                 TEXT NOT NULL CHECK (
        length(staging_run_digest) = 64 AND staging_run_digest NOT GLOB '*[^0-9a-f]*'
    ),
    health_id                          TEXT NOT NULL UNIQUE,
    health_receipt_digest              TEXT NOT NULL CHECK (
        length(health_receipt_digest) = 64
        AND health_receipt_digest NOT GLOB '*[^0-9a-f]*'
    ),
    health_observation_digest          TEXT NOT NULL CHECK (
        length(health_observation_digest) = 64
        AND health_observation_digest NOT GLOB '*[^0-9a-f]*'
    ),
    release_json                       TEXT NOT NULL CHECK (
        length(release_json) > 0 AND json_valid(release_json)
    ),
    permission_grant_digest            TEXT NOT NULL CHECK (
        length(permission_grant_digest) = 64
        AND permission_grant_digest NOT GLOB '*[^0-9a-f]*'
    ),
    signed_manifest_envelope_digest    TEXT NOT NULL CHECK (
        length(signed_manifest_envelope_digest) = 64
        AND signed_manifest_envelope_digest NOT GLOB '*[^0-9a-f]*'
    ),
    install_state                      TEXT NOT NULL CHECK (install_state = 'installed'),
    install_evidence_json              TEXT NOT NULL CHECK (
        length(install_evidence_json) > 0
        AND length(install_evidence_json) <= 131072
        AND json_valid(install_evidence_json)
    ),
    install_evidence_digest            TEXT NOT NULL UNIQUE CHECK (
        length(install_evidence_digest) = 64
        AND install_evidence_digest NOT GLOB '*[^0-9a-f]*'
    ),
    install_generation_before          INTEGER NOT NULL CHECK (install_generation_before >= 0),
    install_generation_after           INTEGER NOT NULL CHECK (
        install_generation_after = candidate_generation
        AND install_generation_after > install_generation_before
    ),
    authority_state_revision_before   INTEGER NOT NULL CHECK (
        authority_state_revision_before > 0
    ),
    authority_state_revision_after    INTEGER NOT NULL CHECK (
        authority_state_revision_after = authority_state_revision_before + 1
    ),
    inventory_revision_before         INTEGER NOT NULL CHECK (inventory_revision_before > 0),
    inventory_revision_after          INTEGER NOT NULL CHECK (
        inventory_revision_after = inventory_revision_before + 1
    ),
    inventory_digest_before           TEXT NOT NULL CHECK (
        length(inventory_digest_before) = 64
        AND inventory_digest_before NOT GLOB '*[^0-9a-f]*'
    ),
    inventory_digest_after            TEXT NOT NULL CHECK (
        length(inventory_digest_after) = 64
        AND inventory_digest_after NOT GLOB '*[^0-9a-f]*'
        AND inventory_digest_after <> inventory_digest_before
    ),
    inventory_json_after              TEXT NOT NULL CHECK (
        length(inventory_json_after) > 0 AND json_valid(inventory_json_after)
    ),
    authority_epoch_before            INTEGER NOT NULL CHECK (authority_epoch_before > 0),
    authority_epoch_after             INTEGER NOT NULL CHECK (
        authority_epoch_after = authority_epoch_before + 1
    ),
    process_owner_epoch               INTEGER NOT NULL CHECK (process_owner_epoch > 0),
    trusted_time_before_ms            INTEGER NOT NULL CHECK (trusted_time_before_ms > 0),
    authority_updated_at_ms_before    INTEGER NOT NULL CHECK (authority_updated_at_ms_before > 0),
    installed_at_ms                   INTEGER NOT NULL CHECK (
        installed_at_ms > trusted_time_before_ms
    ),
    receipt_json                      TEXT NOT NULL CHECK (
        length(receipt_json) > 0 AND length(receipt_json) <= 131072
        AND json_valid(receipt_json)
    ),
    receipt_digest                    TEXT NOT NULL UNIQUE CHECK (
        length(receipt_digest) = 64 AND receipt_digest NOT GLOB '*[^0-9a-f]*'
    ),
    UNIQUE (install_id, candidate_token, promotion_id, receipt_digest),
    UNIQUE (candidate_token, owner_plan_id, owner_plan_digest),
    FOREIGN KEY (candidate_token, owner_plan_id, owner_plan_digest)
        REFERENCES candidate_owners(candidate_token, owner_plan_id, owner_plan_digest)
        ON DELETE RESTRICT,
    FOREIGN KEY (staging_id)
        REFERENCES candidate_staging_receipts(staging_id) ON DELETE RESTRICT,
    FOREIGN KEY (health_id)
        REFERENCES candidate_health_receipts(health_id) ON DELETE RESTRICT,
    FOREIGN KEY (promotion_id, candidate_token, install_id)
        REFERENCES candidate_promotion_receipts(promotion_id, candidate_token, install_id)
        ON DELETE RESTRICT DEFERRABLE INITIALLY DEFERRED
) WITHOUT ROWID;

CREATE TABLE candidate_promotion_receipts (
    promotion_id                       TEXT PRIMARY KEY CHECK (length(promotion_id) > 0),
    install_id                         TEXT NOT NULL UNIQUE CHECK (length(install_id) > 0),
    install_receipt_digest             TEXT NOT NULL UNIQUE CHECK (
        length(install_receipt_digest) = 64
        AND install_receipt_digest NOT GLOB '*[^0-9a-f]*'
    ),
    candidate_token                    TEXT NOT NULL UNIQUE,
    candidate_token_digest             TEXT NOT NULL CHECK (
        length(candidate_token_digest) = 64
        AND candidate_token_digest NOT GLOB '*[^0-9a-f]*'
    ),
    installation_id_digest             TEXT NOT NULL CHECK (
        length(installation_id_digest) = 64
        AND installation_id_digest NOT GLOB '*[^0-9a-f]*'
    ),
    plugin_id                          TEXT NOT NULL CHECK (length(plugin_id) > 0),
    slot_ref                           TEXT NOT NULL CHECK (length(slot_ref) > 0),
    candidate_generation               INTEGER NOT NULL CHECK (candidate_generation > 0),
    owner_plan_id                      TEXT NOT NULL,
    owner_plan_digest                  TEXT NOT NULL CHECK (
        length(owner_plan_digest) = 64 AND owner_plan_digest NOT GLOB '*[^0-9a-f]*'
    ),
    application_inventory_revision    INTEGER NOT NULL CHECK (
        application_inventory_revision > 0
    ),
    staging_id                         TEXT NOT NULL UNIQUE,
    staging_receipt_digest             TEXT NOT NULL CHECK (
        length(staging_receipt_digest) = 64
        AND staging_receipt_digest NOT GLOB '*[^0-9a-f]*'
    ),
    health_id                          TEXT NOT NULL UNIQUE,
    health_receipt_digest              TEXT NOT NULL CHECK (
        length(health_receipt_digest) = 64
        AND health_receipt_digest NOT GLOB '*[^0-9a-f]*'
    ),
    release_json                       TEXT NOT NULL CHECK (
        length(release_json) > 0 AND json_valid(release_json)
    ),
    permission_grant_digest            TEXT NOT NULL CHECK (
        length(permission_grant_digest) = 64
        AND permission_grant_digest NOT GLOB '*[^0-9a-f]*'
    ),
    signed_manifest_envelope_digest    TEXT NOT NULL CHECK (
        length(signed_manifest_envelope_digest) = 64
        AND signed_manifest_envelope_digest NOT GLOB '*[^0-9a-f]*'
    ),
    promotion_state                    TEXT NOT NULL CHECK (promotion_state = 'active'),
    active_provenance_json             TEXT NOT NULL CHECK (
        length(active_provenance_json) > 0
        AND length(active_provenance_json) <= 131072
        AND json_valid(active_provenance_json)
    ),
    active_provenance_digest           TEXT NOT NULL UNIQUE CHECK (
        length(active_provenance_digest) = 64
        AND active_provenance_digest NOT GLOB '*[^0-9a-f]*'
    ),
    install_generation_after           INTEGER NOT NULL CHECK (install_generation_after > 0),
    activation_generation_before      INTEGER NOT NULL CHECK (activation_generation_before >= 0),
    activation_generation_after       INTEGER NOT NULL CHECK (
        activation_generation_after = activation_generation_before + 1
    ),
    previous_active_slot_ref           TEXT,
    previous_active_release_json       TEXT,
    previous_active_install_receipt_digest TEXT,
    previous_active_promotion_receipt_digest TEXT,
    authority_state_revision_before   INTEGER NOT NULL CHECK (
        authority_state_revision_before > 0
    ),
    authority_state_revision_after    INTEGER NOT NULL CHECK (
        authority_state_revision_after = authority_state_revision_before + 1
    ),
    inventory_revision_before         INTEGER NOT NULL CHECK (inventory_revision_before > 0),
    inventory_revision_after          INTEGER NOT NULL CHECK (
        inventory_revision_after = inventory_revision_before + 1
    ),
    inventory_digest_before           TEXT NOT NULL CHECK (
        length(inventory_digest_before) = 64
        AND inventory_digest_before NOT GLOB '*[^0-9a-f]*'
    ),
    inventory_digest_after            TEXT NOT NULL CHECK (
        length(inventory_digest_after) = 64
        AND inventory_digest_after NOT GLOB '*[^0-9a-f]*'
        AND inventory_digest_after <> inventory_digest_before
    ),
    inventory_json_after              TEXT NOT NULL CHECK (
        length(inventory_json_after) > 0 AND json_valid(inventory_json_after)
    ),
    authority_epoch_before            INTEGER NOT NULL CHECK (authority_epoch_before > 0),
    authority_epoch_after             INTEGER NOT NULL CHECK (
        authority_epoch_after = authority_epoch_before + 1
    ),
    process_owner_epoch               INTEGER NOT NULL CHECK (process_owner_epoch > 0),
    trusted_time_before_ms            INTEGER NOT NULL CHECK (trusted_time_before_ms > 0),
    authority_updated_at_ms_before    INTEGER NOT NULL CHECK (authority_updated_at_ms_before > 0),
    installed_at_ms                   INTEGER NOT NULL CHECK (
        installed_at_ms > trusted_time_before_ms
    ),
    promoted_at_ms                    INTEGER NOT NULL CHECK (promoted_at_ms >= installed_at_ms),
    close_reason                      TEXT NOT NULL CHECK (
        close_reason = 'candidate_promotion_completed'
    ),
    receipt_json                      TEXT NOT NULL CHECK (
        length(receipt_json) > 0 AND length(receipt_json) <= 131072
        AND json_valid(receipt_json)
    ),
    receipt_digest                    TEXT NOT NULL UNIQUE CHECK (
        length(receipt_digest) = 64 AND receipt_digest NOT GLOB '*[^0-9a-f]*'
    ),
    UNIQUE (promotion_id, candidate_token, install_id),
    FOREIGN KEY (install_id, candidate_token, promotion_id, install_receipt_digest)
        REFERENCES candidate_install_receipts(
            install_id, candidate_token, promotion_id, receipt_digest
        ) ON DELETE RESTRICT DEFERRABLE INITIALLY DEFERRED,
    CHECK (
        (previous_active_slot_ref IS NULL
         AND previous_active_release_json IS NULL
         AND previous_active_install_receipt_digest IS NULL
         AND previous_active_promotion_receipt_digest IS NULL)
        OR
        (previous_active_slot_ref IS NOT NULL
         AND previous_active_release_json IS NOT NULL
         AND previous_active_install_receipt_digest IS NOT NULL
         AND previous_active_promotion_receipt_digest IS NOT NULL
         AND length(previous_active_slot_ref) > 0
         AND length(previous_active_release_json) > 0
         AND json_valid(previous_active_release_json)
         AND length(previous_active_install_receipt_digest) = 64
         AND previous_active_install_receipt_digest NOT GLOB '*[^0-9a-f]*'
         AND length(previous_active_promotion_receipt_digest) = 64
         AND previous_active_promotion_receipt_digest NOT GLOB '*[^0-9a-f]*')
    )
) WITHOUT ROWID;

CREATE TRIGGER candidate_install_insert_fenced
BEFORE INSERT ON candidate_install_receipts
WHEN NOT EXISTS (
    SELECT 1
    FROM authority_meta AS meta
    JOIN plan_applications AS application
      ON application.plan_id = NEW.owner_plan_id
     AND application.plan_digest = NEW.owner_plan_digest
    JOIN candidate_owners AS candidate
      ON candidate.candidate_token = NEW.candidate_token
     AND candidate.owner_plan_id = application.plan_id
     AND candidate.owner_plan_digest = application.plan_digest
    JOIN candidate_staging_receipts AS staging
      ON staging.staging_id = NEW.staging_id
     AND staging.candidate_token = candidate.candidate_token
     AND staging.receipt_digest = NEW.staging_receipt_digest
     AND staging.staging_run_digest = NEW.staging_run_digest
    JOIN candidate_health_receipts AS health
      ON health.health_id = NEW.health_id
     AND health.candidate_token = candidate.candidate_token
     AND health.staging_id = staging.staging_id
     AND health.staging_receipt_digest = staging.receipt_digest
     AND health.receipt_digest = NEW.health_receipt_digest
     AND health.health_observation_digest = NEW.health_observation_digest
    WHERE meta.singleton = 1
      AND meta.clock_status = 'trusted'
      AND meta.sharing_enabled = 1
      AND meta.installation_id_digest = NEW.installation_id_digest
      AND meta.state_revision = NEW.authority_state_revision_before
      AND meta.inventory_revision = NEW.inventory_revision_before
      AND meta.inventory_digest = NEW.inventory_digest_before
      AND meta.authority_epoch = NEW.authority_epoch_before
      AND meta.process_owner_epoch = NEW.process_owner_epoch
      AND meta.trusted_time_high_water_ms = NEW.trusted_time_before_ms
      AND meta.updated_at_ms = NEW.authority_updated_at_ms_before
      AND meta.active_bundle_revision = application.keyring_bundle_revision
      AND meta.publisher_keyring_revision = application.publisher_keyring_revision
      AND meta.publisher_keyring_digest = application.publisher_keyring_digest
      AND meta.control_keyring_revision = application.control_keyring_revision
      AND meta.control_keyring_digest = application.control_keyring_digest
      AND candidate.state = 'owned'
      AND candidate.plugin_id = NEW.plugin_id
      AND candidate.slot_ref = NEW.slot_ref
      AND candidate.candidate_generation = NEW.candidate_generation
      AND candidate.application_inventory_revision = NEW.application_inventory_revision
      AND candidate.release_json = NEW.release_json
      AND candidate.permission_grant_digest = NEW.permission_grant_digest
      AND health.authority_state_revision = NEW.authority_state_revision_before
      AND health.inventory_revision = NEW.inventory_revision_before
      AND health.inventory_digest = NEW.inventory_digest_before
      AND health.authority_epoch = NEW.authority_epoch_before
      AND health.process_owner_epoch = NEW.process_owner_epoch
      AND health.recorded_at_ms = NEW.trusted_time_before_ms
      AND health.recorded_at_ms < NEW.installed_at_ms
      AND NEW.installed_at_ms < health.expires_at_ms
      AND NEW.installed_at_ms < application.expires_at_ms
)
BEGIN
    SELECT RAISE(ABORT, 'candidate install receipt lost its exact staged authority fence');
END;

CREATE TRIGGER candidate_promotion_insert_fenced
BEFORE INSERT ON candidate_promotion_receipts
WHEN NOT EXISTS (
    SELECT 1
    FROM candidate_install_receipts AS installation
    JOIN authority_meta AS meta ON meta.singleton = 1
    JOIN candidate_health_receipts AS health
      ON health.health_id = installation.health_id
     AND health.receipt_digest = installation.health_receipt_digest
    WHERE installation.install_id = NEW.install_id
      AND installation.promotion_id = NEW.promotion_id
      AND installation.receipt_digest = NEW.install_receipt_digest
      AND installation.candidate_token = NEW.candidate_token
      AND installation.candidate_token_digest = NEW.candidate_token_digest
      AND installation.installation_id_digest = NEW.installation_id_digest
      AND installation.plugin_id = NEW.plugin_id
      AND installation.slot_ref = NEW.slot_ref
      AND installation.candidate_generation = NEW.candidate_generation
      AND installation.owner_plan_id = NEW.owner_plan_id
      AND installation.owner_plan_digest = NEW.owner_plan_digest
      AND installation.application_inventory_revision = NEW.application_inventory_revision
      AND installation.staging_id = NEW.staging_id
      AND installation.staging_receipt_digest = NEW.staging_receipt_digest
      AND installation.health_id = NEW.health_id
      AND installation.health_receipt_digest = NEW.health_receipt_digest
      AND installation.release_json = NEW.release_json
      AND installation.permission_grant_digest = NEW.permission_grant_digest
      AND installation.signed_manifest_envelope_digest = NEW.signed_manifest_envelope_digest
      AND installation.install_generation_after = NEW.install_generation_after
      AND installation.authority_state_revision_before = NEW.authority_state_revision_before
      AND installation.authority_state_revision_after = NEW.authority_state_revision_after
      AND installation.inventory_revision_before = NEW.inventory_revision_before
      AND installation.inventory_revision_after = NEW.inventory_revision_after
      AND installation.inventory_digest_before = NEW.inventory_digest_before
      AND installation.inventory_digest_after = NEW.inventory_digest_after
      AND installation.inventory_json_after = NEW.inventory_json_after
      AND installation.authority_epoch_before = NEW.authority_epoch_before
      AND installation.authority_epoch_after = NEW.authority_epoch_after
      AND installation.process_owner_epoch = NEW.process_owner_epoch
      AND installation.trusted_time_before_ms = NEW.trusted_time_before_ms
      AND installation.authority_updated_at_ms_before = NEW.authority_updated_at_ms_before
      AND installation.installed_at_ms = NEW.installed_at_ms
      AND NEW.promoted_at_ms < health.expires_at_ms
      AND meta.state_revision = NEW.authority_state_revision_before
      AND meta.installation_id_digest = NEW.installation_id_digest
      AND meta.inventory_revision = NEW.inventory_revision_before
      AND meta.inventory_digest = NEW.inventory_digest_before
      AND meta.authority_epoch = NEW.authority_epoch_before
      AND meta.process_owner_epoch = NEW.process_owner_epoch
      AND meta.trusted_time_high_water_ms = NEW.trusted_time_before_ms
      AND meta.updated_at_ms = NEW.authority_updated_at_ms_before
      AND (
          NEW.previous_active_slot_ref IS NULL
          OR EXISTS (
              SELECT 1
              FROM candidate_promotion_receipts AS prior_promotion
              JOIN candidate_install_receipts AS prior_install
                ON prior_install.install_id = prior_promotion.install_id
               AND prior_install.receipt_digest = prior_promotion.install_receipt_digest
              JOIN candidate_owners AS prior_candidate
                ON prior_candidate.candidate_token = prior_promotion.candidate_token
              WHERE prior_promotion.plugin_id = NEW.plugin_id
                AND prior_promotion.activation_generation_after =
                    NEW.activation_generation_before
                AND prior_promotion.slot_ref = NEW.previous_active_slot_ref
                AND prior_promotion.release_json = NEW.previous_active_release_json
                AND prior_install.receipt_digest =
                    NEW.previous_active_install_receipt_digest
                AND prior_promotion.receipt_digest =
                    NEW.previous_active_promotion_receipt_digest
                AND prior_candidate.state = 'promoted'
                AND prior_candidate.slot_ref = NEW.previous_active_slot_ref
                AND prior_candidate.release_json = NEW.previous_active_release_json
          )
      )
)
BEGIN
    SELECT RAISE(ABORT, 'candidate promotion receipt lost its exact install or active fence');
END;

CREATE TRIGGER candidate_install_update_forbidden
BEFORE UPDATE ON candidate_install_receipts
BEGIN
    SELECT RAISE(ABORT, 'candidate install receipts are immutable');
END;

CREATE TRIGGER candidate_install_delete_forbidden
BEFORE DELETE ON candidate_install_receipts
BEGIN
    SELECT RAISE(ABORT, 'candidate install receipts are append-only');
END;

CREATE TRIGGER candidate_promotion_update_forbidden
BEFORE UPDATE ON candidate_promotion_receipts
BEGIN
    SELECT RAISE(ABORT, 'candidate promotion receipts are immutable');
END;

CREATE TRIGGER candidate_promotion_delete_forbidden
BEFORE DELETE ON candidate_promotion_receipts
BEGIN
    SELECT RAISE(ABORT, 'candidate promotion receipts are append-only');
END;

CREATE TRIGGER candidate_promotion_requires_receipt
BEFORE UPDATE OF state, closed_at_ms, closed_by_plan_id, closed_by_plan_digest, close_reason
ON candidate_owners
WHEN NEW.state = 'promoted' AND NOT EXISTS (
    SELECT 1
    FROM candidate_install_receipts AS installation
    JOIN candidate_promotion_receipts AS promotion
      ON promotion.install_id = installation.install_id
     AND promotion.promotion_id = installation.promotion_id
     AND promotion.install_receipt_digest = installation.receipt_digest
     AND promotion.candidate_token = installation.candidate_token
    JOIN authority_meta AS meta ON meta.singleton = 1
    WHERE installation.candidate_token = OLD.candidate_token
      AND installation.plugin_id = OLD.plugin_id
      AND installation.slot_ref = OLD.slot_ref
      AND installation.candidate_generation = OLD.candidate_generation
      AND installation.owner_plan_id = OLD.owner_plan_id
      AND installation.owner_plan_digest = OLD.owner_plan_digest
      AND installation.application_inventory_revision = OLD.application_inventory_revision
      AND installation.release_json = OLD.release_json
      AND installation.permission_grant_digest = OLD.permission_grant_digest
      AND installation.install_state = 'installed'
      AND promotion.promotion_state = 'active'
      AND promotion.signed_manifest_envelope_digest =
          installation.signed_manifest_envelope_digest
      AND promotion.installation_id_digest = installation.installation_id_digest
      AND promotion.health_id = installation.health_id
      AND promotion.health_receipt_digest = installation.health_receipt_digest
      AND promotion.install_generation_after = installation.install_generation_after
      AND promotion.authority_state_revision_after =
          installation.authority_state_revision_after
      AND promotion.inventory_revision_after = installation.inventory_revision_after
      AND promotion.inventory_digest_after = installation.inventory_digest_after
      AND promotion.inventory_json_after = installation.inventory_json_after
      AND promotion.authority_epoch_after = installation.authority_epoch_after
      AND promotion.process_owner_epoch = installation.process_owner_epoch
      AND promotion.installed_at_ms = installation.installed_at_ms
      AND meta.state_revision = promotion.authority_state_revision_after
      AND meta.installation_id_digest = promotion.installation_id_digest
      AND meta.inventory_revision = promotion.inventory_revision_after
      AND meta.inventory_digest = promotion.inventory_digest_after
      AND meta.inventory_json = promotion.inventory_json_after
      AND meta.authority_epoch = promotion.authority_epoch_after
      AND meta.process_owner_epoch = promotion.process_owner_epoch
      AND meta.trusted_time_high_water_ms = promotion.promoted_at_ms
      AND meta.updated_at_ms = promotion.promoted_at_ms
      AND NEW.closed_at_ms = promotion.promoted_at_ms
      AND NEW.closed_by_plan_id IS NULL
      AND NEW.closed_by_plan_digest IS NULL
      AND NEW.close_reason = promotion.close_reason
)
BEGIN
    SELECT RAISE(ABORT, 'candidate promotion requires exact install and active receipts');
END;
"#;
