use anyhow::{bail, Context, Result};
use rusqlite::{params, Connection, TransactionBehavior};

mod authority_fences;
mod candidate_cleanup;
mod candidate_health;
mod candidate_health_quarantine;
mod candidate_staging;
mod candidate_verification;
mod fetch_claims;
mod plan_application;
mod schema_integrity;

pub(super) const COMPUTE_PLUGIN_LOCAL_AUTHORITY_SCHEMA_VERSION: i64 = 3;
const COMPUTE_PLUGIN_LOCAL_AUTHORITY_APPLICATION_ID: i64 = 0x454c_4350;

const REQUIRED_TABLES: &[&str] = &[
    "authority_meta",
    "candidate_health_receipts",
    "candidate_health_quarantine_receipts",
    "candidate_cleanup_authorizations",
    "candidate_cleanup_completions",
    "candidate_owners",
    "candidate_staging_receipts",
    "candidate_verification_runs",
    "fetch_claims",
    "keyring_bundles",
    "keyring_keys",
    "keyring_seals",
    "plan_applications",
    "plan_application_seals",
    "plan_events",
    "planned_downloads",
];
pub(super) fn ensure_schema(connection: &mut Connection) -> Result<()> {
    let version = connection
        .pragma_query_value(None, "user_version", |row| row.get::<_, i64>(0))
        .context("COMPUTE_PLUGIN_AUTHORITY_SCHEMA_VERSION_READ")?;
    let application_id = connection
        .pragma_query_value(None, "application_id", |row| row.get::<_, i64>(0))
        .context("COMPUTE_PLUGIN_AUTHORITY_APPLICATION_ID_READ")?;
    match version {
        0 if application_id == 0 => install_schema_v3(connection),
        COMPUTE_PLUGIN_LOCAL_AUTHORITY_SCHEMA_VERSION
            if application_id == COMPUTE_PLUGIN_LOCAL_AUTHORITY_APPLICATION_ID =>
        {
            verify_required_objects(connection)
        }
        COMPUTE_PLUGIN_LOCAL_AUTHORITY_SCHEMA_VERSION => bail!(
            "COMPUTE_PLUGIN_AUTHORITY_APPLICATION_ID: database belongs to another application"
        ),
        0 => bail!(
            "COMPUTE_PLUGIN_AUTHORITY_APPLICATION_ID: unversioned database is already claimed"
        ),
        other => bail!(
            "COMPUTE_PLUGIN_AUTHORITY_SCHEMA_UNSUPPORTED: database version {other} is not supported"
        ),
    }
}

fn install_schema_v3(connection: &mut Connection) -> Result<()> {
    if count_required_tables(connection)? != 0 {
        bail!("COMPUTE_PLUGIN_AUTHORITY_SCHEMA_UNVERSIONED: refusing to adopt unversioned tables");
    }
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .context("COMPUTE_PLUGIN_AUTHORITY_SCHEMA_BEGIN")?;
    create_schema_objects(&transaction)?;
    transaction
        .pragma_update(
            None,
            "user_version",
            COMPUTE_PLUGIN_LOCAL_AUTHORITY_SCHEMA_VERSION,
        )
        .context("COMPUTE_PLUGIN_AUTHORITY_SCHEMA_VERSION_WRITE")?;
    transaction
        .pragma_update(
            None,
            "application_id",
            COMPUTE_PLUGIN_LOCAL_AUTHORITY_APPLICATION_ID,
        )
        .context("COMPUTE_PLUGIN_AUTHORITY_APPLICATION_ID_WRITE")?;
    transaction
        .commit()
        .context("COMPUTE_PLUGIN_AUTHORITY_SCHEMA_COMMIT")?;
    verify_required_objects(connection)
}

fn verify_required_objects(connection: &Connection) -> Result<()> {
    schema_integrity::verify_schema_objects(connection)?;
    let mut statement = connection
        .prepare("PRAGMA foreign_key_check")
        .context("COMPUTE_PLUGIN_AUTHORITY_FOREIGN_KEY_CHECK_PREPARE")?;
    let mut violations = statement
        .query([])
        .context("COMPUTE_PLUGIN_AUTHORITY_FOREIGN_KEY_CHECK")?;
    if violations
        .next()
        .context("COMPUTE_PLUGIN_AUTHORITY_FOREIGN_KEY_CHECK_READ")?
        .is_some()
    {
        bail!("COMPUTE_PLUGIN_AUTHORITY_FOREIGN_KEY_VIOLATION");
    }
    Ok(())
}

fn count_required_tables(connection: &Connection) -> Result<i64> {
    REQUIRED_TABLES.iter().try_fold(0_i64, |count, name| {
        Ok(count + object_exists(connection, "table", name)?)
    })
}

fn object_exists(connection: &Connection, object_type: &str, name: &str) -> Result<i64> {
    connection
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = ?1 AND name = ?2",
            params![object_type, name],
            |row| row.get(0),
        )
        .context("COMPUTE_PLUGIN_AUTHORITY_SCHEMA_INSPECT")
}

fn create_schema_objects(connection: &Connection) -> Result<()> {
    connection
        .execute_batch(SCHEMA_V3)
        .context("COMPUTE_PLUGIN_AUTHORITY_SCHEMA_CREATE_V3")?;
    connection
        .execute_batch(candidate_verification::CANDIDATE_VERIFICATION_SCHEMA_V3)
        .context("COMPUTE_PLUGIN_AUTHORITY_CANDIDATE_VERIFICATION_SCHEMA_CREATE_V3")?;
    connection
        .execute_batch(candidate_staging::CANDIDATE_STAGING_SCHEMA_V3)
        .context("COMPUTE_PLUGIN_AUTHORITY_CANDIDATE_STAGING_SCHEMA_CREATE_V3")?;
    connection
        .execute_batch(candidate_health::CANDIDATE_HEALTH_SCHEMA_V3)
        .context("COMPUTE_PLUGIN_AUTHORITY_CANDIDATE_HEALTH_SCHEMA_CREATE_V3")?;
    connection
        .execute_batch(candidate_health_quarantine::CANDIDATE_HEALTH_QUARANTINE_SCHEMA_V3)
        .context("COMPUTE_PLUGIN_AUTHORITY_CANDIDATE_QUARANTINE_SCHEMA_CREATE_V3")?;
    connection
        .execute_batch(candidate_cleanup::CANDIDATE_CLEANUP_SCHEMA_V3)
        .context("COMPUTE_PLUGIN_AUTHORITY_CANDIDATE_CLEANUP_SCHEMA_CREATE_V3")?;
    connection
        .execute_batch(plan_application::PLAN_APPLICATION_SCHEMA_V3)
        .context("COMPUTE_PLUGIN_AUTHORITY_PLAN_APPLICATION_SCHEMA_CREATE_V3")?;
    connection
        .execute_batch(authority_fences::AUTHORITY_FENCE_SCHEMA_V3)
        .context("COMPUTE_PLUGIN_AUTHORITY_FENCE_SCHEMA_CREATE_V3")?;
    connection
        .execute_batch(fetch_claims::FETCH_CLAIM_SCHEMA_V3)
        .context("COMPUTE_PLUGIN_AUTHORITY_FETCH_CLAIM_SCHEMA_CREATE_V3")?;
    Ok(())
}

const SCHEMA_V3: &str = r#"
CREATE TABLE keyring_bundles (
    bundle_revision          INTEGER PRIMARY KEY CHECK (bundle_revision > 0),
    bundle_digest            TEXT NOT NULL UNIQUE,
    signed_envelope_digest   TEXT NOT NULL UNIQUE,
    signed_bundle_json       TEXT NOT NULL,
    root_signing_key_id      TEXT NOT NULL,
    root_key_fingerprint     TEXT NOT NULL,
    publisher_revision       INTEGER NOT NULL CHECK (publisher_revision > 0),
    publisher_digest         TEXT NOT NULL,
    control_revision         INTEGER NOT NULL CHECK (control_revision > 0),
    control_digest           TEXT NOT NULL,
    publisher_key_count      INTEGER NOT NULL CHECK (
        publisher_key_count >= 0 AND publisher_key_count <= 4096
    ),
    control_key_count        INTEGER NOT NULL CHECK (
        control_key_count >= 0 AND control_key_count <= 4096
    ),
    generated_at_ms          INTEGER NOT NULL,
    expires_at_ms            INTEGER NOT NULL,
    installed_at_ms          INTEGER NOT NULL,
    CHECK (generated_at_ms < expires_at_ms),
    UNIQUE (
        bundle_revision,
        publisher_revision, publisher_digest,
        control_revision, control_digest
    )
);

CREATE TABLE keyring_keys (
    bundle_revision          INTEGER NOT NULL,
    purpose                  TEXT NOT NULL CHECK (
        purpose IN ('publisher_manifest', 'control_install_plan')
    ),
    subject_id               TEXT NOT NULL DEFAULT '',
    signing_key_id           TEXT NOT NULL,
    public_key_base64        TEXT NOT NULL,
    fingerprint_sha256       TEXT NOT NULL,
    status                   TEXT NOT NULL CHECK (status IN ('active', 'revoked')),
    not_before_ms            INTEGER NOT NULL,
    not_after_ms             INTEGER NOT NULL,
    revoked_at_ms            INTEGER,
    PRIMARY KEY (bundle_revision, purpose, subject_id, signing_key_id),
    UNIQUE (bundle_revision, fingerprint_sha256),
    FOREIGN KEY (bundle_revision)
        REFERENCES keyring_bundles(bundle_revision) ON DELETE RESTRICT,
    CHECK (
        (purpose = 'publisher_manifest' AND subject_id <> '')
        OR (purpose = 'control_install_plan' AND subject_id = '')
    ),
    CHECK (not_before_ms < not_after_ms),
    CHECK (
        (status = 'active' AND revoked_at_ms IS NULL)
        OR (status = 'revoked' AND revoked_at_ms IS NOT NULL)
    )
);

CREATE TABLE keyring_seals (
    bundle_revision          INTEGER PRIMARY KEY,
    sealed_at_ms             INTEGER NOT NULL,
    FOREIGN KEY (bundle_revision)
        REFERENCES keyring_bundles(bundle_revision) ON DELETE RESTRICT
);

CREATE TABLE authority_meta (
    singleton                       INTEGER PRIMARY KEY CHECK (singleton = 1),
    schema_version                  INTEGER NOT NULL CHECK (schema_version = 3),
    installation_id_digest         TEXT NOT NULL,
    state_revision                  INTEGER NOT NULL CHECK (state_revision >= 0),
    inventory_revision              INTEGER NOT NULL CHECK (inventory_revision >= 0),
    inventory_digest                TEXT NOT NULL,
    inventory_json                  TEXT NOT NULL,
    desired_policy_revision         INTEGER NOT NULL CHECK (desired_policy_revision >= 0),
    sharing_enabled                 INTEGER NOT NULL CHECK (sharing_enabled IN (0, 1)),
    sharing_authorization_ref       TEXT,
    sharing_authorization_revision  INTEGER,
    sharing_authorization_digest    TEXT,
    node_profile_digest             TEXT NOT NULL,
    manifest_catalog_revision       INTEGER NOT NULL CHECK (manifest_catalog_revision >= 0),
    target_id                       TEXT NOT NULL,
    host_api_protocol_id            TEXT NOT NULL,
    host_api_revision               INTEGER NOT NULL CHECK (host_api_revision >= 0),
    active_bundle_revision          INTEGER REFERENCES keyring_seals(bundle_revision)
                                        ON DELETE RESTRICT,
    publisher_keyring_revision      INTEGER,
    publisher_keyring_digest        TEXT,
    control_keyring_revision        INTEGER,
    control_keyring_digest          TEXT,
    authority_epoch                 INTEGER NOT NULL CHECK (authority_epoch >= 0),
    process_owner_epoch             INTEGER NOT NULL CHECK (process_owner_epoch >= 0),
    trusted_time_high_water_ms      INTEGER CHECK (
        trusted_time_high_water_ms IS NULL OR trusted_time_high_water_ms >= 0
    ),
    clock_status                    TEXT NOT NULL CHECK (
        clock_status IN ('uninitialized', 'trusted', 'clock_untrusted')
    ),
    updated_at_ms                   INTEGER NOT NULL,
    FOREIGN KEY (
        active_bundle_revision,
        publisher_keyring_revision, publisher_keyring_digest,
        control_keyring_revision, control_keyring_digest
    ) REFERENCES keyring_bundles (
        bundle_revision,
        publisher_revision, publisher_digest,
        control_revision, control_digest
    ) ON DELETE RESTRICT,
    CHECK (
        (sharing_authorization_ref IS NULL
         AND sharing_authorization_revision IS NULL
         AND sharing_authorization_digest IS NULL)
        OR
        (sharing_authorization_ref IS NOT NULL
         AND sharing_authorization_revision >= 0
         AND sharing_authorization_digest IS NOT NULL)
    ),
    CHECK (sharing_enabled = 0 OR sharing_authorization_ref IS NOT NULL),
    CHECK (
        (active_bundle_revision IS NULL
         AND publisher_keyring_revision IS NULL
         AND publisher_keyring_digest IS NULL
         AND control_keyring_revision IS NULL
         AND control_keyring_digest IS NULL)
        OR
        (active_bundle_revision IS NOT NULL
         AND publisher_keyring_revision IS NOT NULL
         AND publisher_keyring_digest IS NOT NULL
         AND control_keyring_revision IS NOT NULL
         AND control_keyring_digest IS NOT NULL)
    ),
    CHECK (
        (clock_status = 'uninitialized' AND trusted_time_high_water_ms IS NULL)
        OR
        (clock_status IN ('trusted', 'clock_untrusted')
         AND trusted_time_high_water_ms IS NOT NULL)
    )
);

CREATE TABLE plan_applications (
    plan_id                         TEXT PRIMARY KEY,
    plan_digest                     TEXT NOT NULL,
    application_request_digest      TEXT NOT NULL UNIQUE,
    signed_plan_envelope_digest     TEXT NOT NULL,
    signed_manifest_set_digest      TEXT NOT NULL,
    signed_plan_json                TEXT NOT NULL,
    signed_manifests_json           TEXT NOT NULL,
    admission_bindings_json         TEXT NOT NULL,
    admission_bindings_digest       TEXT NOT NULL,
    expected_inventory_revision     INTEGER NOT NULL,
    expected_inventory_digest       TEXT NOT NULL,
    application_inventory_revision  INTEGER NOT NULL UNIQUE,
    inventory_after_digest          TEXT NOT NULL,
    inventory_after_json            TEXT NOT NULL,
    application_state_revision      INTEGER NOT NULL,
    authority_epoch_at_apply        INTEGER NOT NULL,
    keyring_bundle_revision         INTEGER NOT NULL,
    publisher_keyring_revision      INTEGER NOT NULL,
    publisher_keyring_digest        TEXT NOT NULL,
    control_keyring_revision        INTEGER NOT NULL,
    control_keyring_digest          TEXT NOT NULL,
    control_signing_key_fingerprint TEXT NOT NULL,
    new_candidate_count             INTEGER NOT NULL,
    closed_candidate_count          INTEGER NOT NULL,
    download_count                  INTEGER NOT NULL,
    download_bytes                  INTEGER NOT NULL,
    applied_at_ms                   INTEGER NOT NULL,
    expires_at_ms                   INTEGER NOT NULL,
    receipt_json                    TEXT NOT NULL,
    receipt_digest                  TEXT NOT NULL,
    UNIQUE (plan_id, plan_digest),
    UNIQUE (plan_id, plan_digest, application_inventory_revision),
    CHECK (application_inventory_revision = expected_inventory_revision + 1),
    CHECK (
        expected_inventory_revision >= 0
        AND application_state_revision > 0
        AND authority_epoch_at_apply > 0
    ),
    CHECK (
        new_candidate_count >= 0
        AND closed_candidate_count >= 0
        AND download_count >= 0
        AND download_bytes >= 0
    ),
    CHECK (applied_at_ms >= 0 AND applied_at_ms < expires_at_ms),
    FOREIGN KEY (
        keyring_bundle_revision,
        publisher_keyring_revision, publisher_keyring_digest,
        control_keyring_revision, control_keyring_digest
    ) REFERENCES keyring_bundles (
        bundle_revision,
        publisher_revision, publisher_digest,
        control_revision, control_digest
    ) ON DELETE RESTRICT,
    FOREIGN KEY (keyring_bundle_revision)
        REFERENCES keyring_seals(bundle_revision) ON DELETE RESTRICT
);

CREATE TABLE plan_application_seals (
    plan_id                     TEXT PRIMARY KEY,
    plan_digest                 TEXT NOT NULL,
    application_request_digest  TEXT NOT NULL,
    receipt_digest              TEXT NOT NULL,
    sealed_at_ms                INTEGER NOT NULL CHECK (sealed_at_ms >= 0),
    UNIQUE (plan_id, plan_digest),
    FOREIGN KEY (plan_id, plan_digest)
        REFERENCES plan_applications(plan_id, plan_digest) ON DELETE RESTRICT
);

CREATE TABLE plan_events (
    plan_id             TEXT NOT NULL,
    plan_digest         TEXT NOT NULL,
    event_index         INTEGER NOT NULL CHECK (event_index >= 0),
    event_type          TEXT NOT NULL,
    event_digest        TEXT NOT NULL,
    payload_json        TEXT NOT NULL,
    recorded_at_ms      INTEGER NOT NULL,
    PRIMARY KEY (plan_id, event_index),
    UNIQUE (event_digest),
    FOREIGN KEY (plan_id, plan_digest)
        REFERENCES plan_applications(plan_id, plan_digest) ON DELETE RESTRICT
);

CREATE TABLE candidate_owners (
    candidate_token                 TEXT PRIMARY KEY,
    plugin_id                       TEXT NOT NULL,
    slot_ref                        TEXT NOT NULL,
    candidate_generation            INTEGER NOT NULL CHECK (candidate_generation > 0),
    release_json                    TEXT NOT NULL,
    permission_grant_digest         TEXT NOT NULL,
    owner_plan_id                   TEXT NOT NULL,
    owner_plan_digest               TEXT NOT NULL,
    application_inventory_revision  INTEGER NOT NULL,
    state                           TEXT NOT NULL CHECK (
        state IN ('owned', 'cleanup_pending', 'released', 'promoted', 'cleaned')
    ),
    created_at_ms                   INTEGER NOT NULL,
    closed_at_ms                    INTEGER,
    closed_by_plan_id               TEXT,
    closed_by_plan_digest           TEXT,
    close_reason                    TEXT,
    UNIQUE (candidate_token, owner_plan_id, owner_plan_digest),
    UNIQUE (plugin_id, slot_ref),
    UNIQUE (plugin_id, candidate_generation),
    FOREIGN KEY (
        owner_plan_id, owner_plan_digest, application_inventory_revision
    ) REFERENCES plan_applications (
        plan_id, plan_digest, application_inventory_revision
    ) ON DELETE RESTRICT,
    FOREIGN KEY (closed_by_plan_id, closed_by_plan_digest)
        REFERENCES plan_applications(plan_id, plan_digest) ON DELETE RESTRICT,
    CHECK (
        (state IN ('owned', 'cleanup_pending')
         AND closed_at_ms IS NULL
         AND closed_by_plan_id IS NULL
         AND closed_by_plan_digest IS NULL
         AND close_reason IS NULL)
        OR
        (state = 'released'
         AND closed_at_ms IS NOT NULL
         AND closed_at_ms >= created_at_ms
         AND closed_by_plan_id IS NOT NULL
         AND closed_by_plan_digest IS NOT NULL
         AND close_reason IS NOT NULL
         AND close_reason <> '')
        OR
        (state = 'promoted'
         AND closed_at_ms IS NOT NULL
         AND closed_at_ms >= created_at_ms
         AND closed_by_plan_id IS NULL
         AND closed_by_plan_digest IS NULL
         AND close_reason IS NOT NULL
         AND close_reason <> '')
        OR
        (state = 'cleaned'
         AND closed_at_ms IS NOT NULL
         AND closed_at_ms >= created_at_ms
         AND closed_by_plan_id IS NULL
         AND closed_by_plan_digest IS NULL
         AND close_reason = 'candidate_cleanup_completed')
    )
);

CREATE UNIQUE INDEX one_owned_candidate_per_plugin
    ON candidate_owners(plugin_id) WHERE state IN ('owned', 'cleanup_pending');

CREATE TABLE planned_downloads (
    plan_id                 TEXT NOT NULL,
    plan_digest             TEXT NOT NULL,
    ordinal                 INTEGER NOT NULL CHECK (ordinal >= 0),
    item_index              INTEGER NOT NULL CHECK (item_index >= 0),
    candidate_token         TEXT NOT NULL,
    artifact_kind           TEXT NOT NULL,
    artifact_id             TEXT NOT NULL,
    artifact_digest         TEXT NOT NULL,
    source_ref              TEXT NOT NULL,
    cache_class             TEXT NOT NULL,
    part_relative_path      TEXT NOT NULL,
    size_bytes              INTEGER NOT NULL CHECK (size_bytes >= 0),
    committed_offset        INTEGER NOT NULL DEFAULT 0,
    cursor_generation       INTEGER NOT NULL DEFAULT 0,
    state                   TEXT NOT NULL DEFAULT 'pending' CHECK (
        state IN ('pending', 'downloading', 'complete', 'canceled', 'failed')
    ),
    created_at_ms           INTEGER NOT NULL,
    updated_at_ms           INTEGER NOT NULL,
    PRIMARY KEY (plan_id, ordinal),
    UNIQUE (plan_id, plan_digest, ordinal),
    UNIQUE (part_relative_path),
    FOREIGN KEY (plan_id, plan_digest)
        REFERENCES plan_applications(plan_id, plan_digest) ON DELETE RESTRICT,
    FOREIGN KEY (candidate_token, plan_id, plan_digest)
        REFERENCES candidate_owners(candidate_token, owner_plan_id, owner_plan_digest)
        ON DELETE RESTRICT,
    CHECK (size_bytes > 0),
    CHECK (cursor_generation >= 0),
    CHECK (updated_at_ms >= created_at_ms),
    CHECK (committed_offset >= 0 AND committed_offset <= size_bytes),
    CHECK (state <> 'complete' OR committed_offset = size_bytes)
);

CREATE TABLE fetch_claims (
    claim_id                TEXT PRIMARY KEY,
    plan_id                 TEXT NOT NULL,
    plan_digest             TEXT NOT NULL,
    ordinal                 INTEGER NOT NULL CHECK (ordinal >= 0),
    candidate_token         TEXT NOT NULL,
    authority_epoch         INTEGER NOT NULL CHECK (authority_epoch > 0),
    process_owner_epoch     INTEGER NOT NULL CHECK (process_owner_epoch > 0),
    cursor_generation       INTEGER NOT NULL CHECK (cursor_generation > 0),
    redirect_generation     INTEGER NOT NULL DEFAULT 0 CHECK (
        redirect_generation >= 0 AND redirect_generation <= 5
    ),
    offset_bytes            INTEGER NOT NULL CHECK (offset_bytes >= 0),
    length_bytes            INTEGER NOT NULL CHECK (
        length_bytes > 0 AND length_bytes <= 16777216
    ),
    end_offset_bytes        INTEGER NOT NULL,
    state                   TEXT NOT NULL CHECK (
        state IN ('prepared', 'committed', 'aborted', 'revoked')
    ),
    prepared_at_ms          INTEGER NOT NULL CHECK (prepared_at_ms >= 0),
    resolved_at_ms          INTEGER,
    resolution_reason       TEXT,
    UNIQUE (plan_id, ordinal, cursor_generation),
    FOREIGN KEY (plan_id, plan_digest, ordinal)
        REFERENCES planned_downloads(plan_id, plan_digest, ordinal) ON DELETE RESTRICT,
    FOREIGN KEY (candidate_token, plan_id, plan_digest)
        REFERENCES candidate_owners(candidate_token, owner_plan_id, owner_plan_digest)
        ON DELETE RESTRICT,
    CHECK (
        end_offset_bytes > offset_bytes
        AND length_bytes = end_offset_bytes - offset_bytes
    ),
    CHECK (
        (state = 'prepared'
         AND resolved_at_ms IS NULL
         AND resolution_reason IS NULL)
        OR
        (state IN ('committed', 'aborted', 'revoked')
         AND resolved_at_ms IS NOT NULL
         AND resolved_at_ms >= prepared_at_ms
         AND resolution_reason IS NOT NULL
         AND resolution_reason <> '')
    ),
    CHECK (
        state <> 'revoked'
        OR resolution_reason IN (
            'authority_epoch_advanced_by_keyring',
            'authority_epoch_advanced_by_plan',
            'authority_epoch_advanced_by_verification',
            'process_owner_epoch_advanced',
            'candidate_released_by_plan'
        )
    )
);

CREATE UNIQUE INDEX one_prepared_claim_per_download
    ON fetch_claims(plan_id, ordinal) WHERE state = 'prepared';

CREATE TRIGGER keyring_bundle_revision_monotonic
BEFORE INSERT ON keyring_bundles
WHEN EXISTS (
    SELECT 1 FROM keyring_bundles
    WHERE bundle_revision > NEW.bundle_revision
) BEGIN
    SELECT RAISE(ABORT, 'keyring bundle revision cannot roll back');
END;
CREATE TRIGGER keyring_publisher_revision_consistent
BEFORE INSERT ON keyring_bundles
WHEN EXISTS (
    SELECT 1 FROM keyring_bundles
    WHERE publisher_revision > NEW.publisher_revision
       OR (publisher_revision = NEW.publisher_revision
           AND publisher_digest <> NEW.publisher_digest)
) BEGIN
    SELECT RAISE(ABORT, 'publisher keyring revision cannot roll back or change digest');
END;
CREATE TRIGGER keyring_control_revision_consistent
BEFORE INSERT ON keyring_bundles
WHEN EXISTS (
    SELECT 1 FROM keyring_bundles
    WHERE control_revision > NEW.control_revision
       OR (control_revision = NEW.control_revision
           AND control_digest <> NEW.control_digest)
) BEGIN
    SELECT RAISE(ABORT, 'control keyring revision cannot roll back or change digest');
END;
CREATE TRIGGER sealed_keyring_keys_insert
BEFORE INSERT ON keyring_keys
WHEN EXISTS (
    SELECT 1 FROM keyring_seals WHERE bundle_revision = NEW.bundle_revision
) BEGIN
    SELECT RAISE(ABORT, 'sealed keyring bundle cannot accept keys');
END;
CREATE TRIGGER keyring_seal_counts
BEFORE INSERT ON keyring_seals
WHEN (
    (SELECT COUNT(*) FROM keyring_keys
     WHERE bundle_revision = NEW.bundle_revision
       AND purpose = 'publisher_manifest')
        <> (SELECT publisher_key_count FROM keyring_bundles
            WHERE bundle_revision = NEW.bundle_revision)
    OR
    (SELECT COUNT(*) FROM keyring_keys
     WHERE bundle_revision = NEW.bundle_revision
       AND purpose = 'control_install_plan')
        <> (SELECT control_key_count FROM keyring_bundles
            WHERE bundle_revision = NEW.bundle_revision)
) BEGIN
    SELECT RAISE(ABORT, 'keyring bundle cannot seal with incomplete keys');
END;
CREATE TRIGGER authority_keyring_binding_monotonic
BEFORE UPDATE OF
    active_bundle_revision,
    publisher_keyring_revision, publisher_keyring_digest,
    control_keyring_revision, control_keyring_digest
ON authority_meta
WHEN OLD.active_bundle_revision IS NOT NULL AND (
    NEW.active_bundle_revision IS NULL
    OR NEW.active_bundle_revision < OLD.active_bundle_revision
    OR NEW.publisher_keyring_revision IS NULL
    OR NEW.publisher_keyring_revision < OLD.publisher_keyring_revision
    OR (NEW.publisher_keyring_revision = OLD.publisher_keyring_revision
        AND NEW.publisher_keyring_digest <> OLD.publisher_keyring_digest)
    OR NEW.control_keyring_revision IS NULL
    OR NEW.control_keyring_revision < OLD.control_keyring_revision
    OR (NEW.control_keyring_revision = OLD.control_keyring_revision
        AND NEW.control_keyring_digest <> OLD.control_keyring_digest)
) BEGIN
    SELECT RAISE(ABORT, 'active keyring binding cannot roll back');
END;
CREATE TRIGGER authority_keyring_binding_fenced
BEFORE UPDATE OF
    active_bundle_revision,
    publisher_keyring_revision, publisher_keyring_digest,
    control_keyring_revision, control_keyring_digest
ON authority_meta
WHEN (
    OLD.active_bundle_revision IS NOT NEW.active_bundle_revision
    OR OLD.publisher_keyring_revision IS NOT NEW.publisher_keyring_revision
    OR OLD.publisher_keyring_digest IS NOT NEW.publisher_keyring_digest
    OR OLD.control_keyring_revision IS NOT NEW.control_keyring_revision
    OR OLD.control_keyring_digest IS NOT NEW.control_keyring_digest
) AND (
    NEW.state_revision <> OLD.state_revision + 1
    OR NEW.authority_epoch <> OLD.authority_epoch + 1
) BEGIN
    SELECT RAISE(ABORT, 'active keyring binding change must advance fences');
END;
CREATE TRIGGER authority_trusted_time_monotonic
BEFORE UPDATE OF trusted_time_high_water_ms ON authority_meta
WHEN OLD.trusted_time_high_water_ms IS NOT NULL AND (
    NEW.trusted_time_high_water_ms IS NULL
    OR NEW.trusted_time_high_water_ms < OLD.trusted_time_high_water_ms
) BEGIN
    SELECT RAISE(ABORT, 'trusted time high-water cannot roll back');
END;
CREATE TRIGGER authority_trusted_time_recovery_fenced
BEFORE UPDATE OF trusted_time_high_water_ms, clock_status ON authority_meta
WHEN OLD.clock_status = 'clock_untrusted'
 AND NEW.clock_status = 'trusted'
 AND NEW.trusted_time_high_water_ms <= OLD.trusted_time_high_water_ms
BEGIN
    SELECT RAISE(ABORT, 'clock recovery must advance trusted time');
END;

CREATE TRIGGER immutable_keyring_bundles_update
BEFORE UPDATE ON keyring_bundles BEGIN
    SELECT RAISE(ABORT, 'keyring bundles are immutable');
END;
CREATE TRIGGER immutable_keyring_bundles_delete
BEFORE DELETE ON keyring_bundles BEGIN
    SELECT RAISE(ABORT, 'keyring bundles are immutable');
END;
CREATE TRIGGER immutable_keyring_keys_update
BEFORE UPDATE ON keyring_keys BEGIN
    SELECT RAISE(ABORT, 'keyring keys are immutable');
END;
CREATE TRIGGER immutable_keyring_keys_delete
BEFORE DELETE ON keyring_keys BEGIN
    SELECT RAISE(ABORT, 'keyring keys are immutable');
END;
CREATE TRIGGER immutable_keyring_seals_update
BEFORE UPDATE ON keyring_seals BEGIN
    SELECT RAISE(ABORT, 'keyring seals are immutable');
END;
CREATE TRIGGER immutable_keyring_seals_delete
BEFORE DELETE ON keyring_seals BEGIN
    SELECT RAISE(ABORT, 'keyring seals are immutable');
END;
CREATE TRIGGER immutable_plan_applications_update
BEFORE UPDATE ON plan_applications BEGIN
    SELECT RAISE(ABORT, 'plan applications are immutable');
END;
CREATE TRIGGER immutable_plan_applications_delete
BEFORE DELETE ON plan_applications BEGIN
    SELECT RAISE(ABORT, 'plan applications are immutable');
END;
CREATE TRIGGER immutable_plan_application_seals_update
BEFORE UPDATE ON plan_application_seals BEGIN
    SELECT RAISE(ABORT, 'plan application seals are immutable');
END;
CREATE TRIGGER immutable_plan_application_seals_delete
BEFORE DELETE ON plan_application_seals BEGIN
    SELECT RAISE(ABORT, 'plan application seals are immutable');
END;
CREATE TRIGGER immutable_plan_events_update
BEFORE UPDATE ON plan_events BEGIN
    SELECT RAISE(ABORT, 'plan events are append-only');
END;
CREATE TRIGGER immutable_plan_events_delete
BEFORE DELETE ON plan_events BEGIN
    SELECT RAISE(ABORT, 'plan events are append-only');
END;
"#;

#[cfg(test)]
mod tests;
