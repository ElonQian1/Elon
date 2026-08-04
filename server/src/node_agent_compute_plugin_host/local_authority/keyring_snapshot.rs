use anyhow::{bail, Context, Result};
use chrono::{DateTime, Utc};
use rusqlite::{params, OptionalExtension, Transaction};

use super::{
    keyring_integrity::{verify_bundle_columns, verify_normalized_keys, StoredBundleRow},
    ComputePluginLocalAuthority,
};
use crate::node_agent_compute_plugin_host::{
    keyring::{
        ComputePluginBootstrapRootKeyResolver, ComputePluginControlPlaneKeyResolver,
        ComputePluginKeyringBinding, ComputePluginPublisherKeyResolver,
        ResolvedComputePluginVerificationKey, SignedComputePluginKeyringBundle,
        ValidatedComputePluginKeyringBundle,
    },
    keyring_validation::{
        resolve_validated_control_key, resolve_validated_publisher_key,
        verify_and_validate_archived_keyring_bundle, verify_and_validate_keyring_bundle,
    },
    signed_artifact_verification::jcs_sha256_hex,
};

#[derive(Debug, Clone)]
pub(super) struct PersistedComputePluginKeyringSnapshot {
    validated: ValidatedComputePluginKeyringBundle,
    signed_envelope_digest: String,
    trusted_time_floor_ms: Option<i64>,
    state_revision: i64,
    authority_epoch: i64,
}

impl PersistedComputePluginKeyringSnapshot {
    pub(super) fn state_revision(&self) -> i64 {
        self.state_revision
    }

    pub(super) fn authority_epoch(&self) -> i64 {
        self.authority_epoch
    }

    pub(super) fn bundle_revision(&self) -> i64 {
        self.validated.signed().bundle.bundle_revision
    }

    pub(super) fn bundle_digest(&self) -> &str {
        &self.validated.signed().bundle_digest
    }

    pub(super) fn signed_envelope_digest(&self) -> &str {
        &self.signed_envelope_digest
    }

    pub(super) fn root_signing_key_id(&self) -> &str {
        &self.validated.signed().signature.signing_key_id
    }

    pub(super) fn publisher_binding(&self) -> &ComputePluginKeyringBinding {
        self.validated.publisher_binding()
    }

    pub(super) fn control_binding(&self) -> &ComputePluginKeyringBinding {
        self.validated.control_binding()
    }

    pub(super) fn root_key_fingerprint(&self) -> &str {
        self.validated.root_key_fingerprint()
    }
}

impl ComputePluginPublisherKeyResolver for PersistedComputePluginKeyringSnapshot {
    fn resolve_publisher_key(
        &self,
        publisher_id: &str,
        signing_key_id: &str,
        expected_keyring: &ComputePluginKeyringBinding,
        trusted_now: DateTime<Utc>,
    ) -> Result<Option<ResolvedComputePluginVerificationKey>> {
        self.require_current_time(trusted_now.timestamp_millis())?;
        resolve_validated_publisher_key(
            &self.validated,
            publisher_id,
            signing_key_id,
            expected_keyring,
            trusted_now,
        )
    }
}

impl ComputePluginControlPlaneKeyResolver for PersistedComputePluginKeyringSnapshot {
    fn resolve_control_plane_key(
        &self,
        signing_key_id: &str,
        expected_keyring: &ComputePluginKeyringBinding,
        trusted_now: DateTime<Utc>,
    ) -> Result<Option<ResolvedComputePluginVerificationKey>> {
        self.require_current_time(trusted_now.timestamp_millis())?;
        resolve_validated_control_key(
            &self.validated,
            signing_key_id,
            expected_keyring,
            trusted_now,
        )
    }
}

impl PersistedComputePluginKeyringSnapshot {
    fn require_current_time(&self, trusted_now_ms: i64) -> Result<()> {
        match self.trusted_time_floor_ms {
            Some(floor) if trusted_now_ms >= floor => Ok(()),
            Some(_) => bail!("COMPUTE_PLUGIN_AUTHORITY_CLOCK_ROLLBACK"),
            None => bail!("COMPUTE_PLUGIN_AUTHORITY_KEYRING_ARCHIVED_ONLY"),
        }
    }
}

impl ComputePluginLocalAuthority {
    /// Rebuilds the active resolver exclusively from the root-signed JSON inside one transaction.
    /// Normalized rows are integrity-checked caches and never become a trust root.
    pub(super) fn load_active_keyring_snapshot(
        &self,
        trusted_now: DateTime<Utc>,
        roots: &dyn ComputePluginBootstrapRootKeyResolver,
    ) -> Result<Option<PersistedComputePluginKeyringSnapshot>> {
        self.with_immediate(|transaction| {
            let state = read_authority_keyring_state(transaction)?;
            if state.active.is_none() {
                return Ok(None);
            }
            let snapshot = load_snapshot_for_state(
                transaction,
                &state,
                KeyringSnapshotValidation::Current(trusted_now.clone()),
                roots,
            )?;
            advance_trusted_time(transaction, &state, trusted_now.timestamp_millis())?;
            Ok(Some(snapshot))
        })
    }
}

#[derive(Debug, Clone)]
pub(super) struct AuthorityKeyringState {
    pub state_revision: i64,
    pub authority_epoch: i64,
    pub active: Option<ActiveKeyringBinding>,
    pub trusted_time_high_water_ms: Option<i64>,
    pub clock_status: String,
}

#[derive(Debug, Clone)]
pub(super) struct ActiveKeyringBinding {
    pub bundle_revision: i64,
    pub publisher: ComputePluginKeyringBinding,
    pub control: ComputePluginKeyringBinding,
}

pub(super) enum KeyringSnapshotValidation {
    Current(DateTime<Utc>),
    Archived,
}

pub(super) fn read_authority_keyring_state(
    transaction: &Transaction<'_>,
) -> Result<AuthorityKeyringState> {
    type MetaRow = (
        i64,
        i64,
        Option<i64>,
        Option<i64>,
        Option<String>,
        Option<i64>,
        Option<String>,
        Option<i64>,
        String,
    );
    let row = transaction
        .query_row(
            r#"SELECT
                state_revision, authority_epoch, active_bundle_revision,
                publisher_keyring_revision, publisher_keyring_digest,
                control_keyring_revision, control_keyring_digest,
                trusted_time_high_water_ms, clock_status
            FROM authority_meta WHERE singleton = 1"#,
            [],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                    row.get(7)?,
                    row.get(8)?,
                ))
            },
        )
        .optional()
        .context("COMPUTE_PLUGIN_AUTHORITY_KEYRING_META_READ")?
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_AUTHORITY_UNINITIALIZED"))?;
    let (
        state_revision,
        authority_epoch,
        bundle,
        publisher_revision,
        publisher_digest,
        control_revision,
        control_digest,
        trusted_time_high_water_ms,
        clock_status,
    ): MetaRow = row;
    if state_revision < 0 || authority_epoch < 0 {
        bail!("COMPUTE_PLUGIN_AUTHORITY_META_CORRUPT: authority fences are invalid");
    }
    let active = match (
        bundle,
        publisher_revision,
        publisher_digest,
        control_revision,
        control_digest,
    ) {
        (None, None, None, None, None) => None,
        (
            Some(bundle_revision),
            Some(publisher_revision),
            Some(publisher_digest),
            Some(control_revision),
            Some(control_digest),
        ) => Some(ActiveKeyringBinding {
            bundle_revision,
            publisher: ComputePluginKeyringBinding {
                revision: publisher_revision,
                digest: publisher_digest,
            },
            control: ComputePluginKeyringBinding {
                revision: control_revision,
                digest: control_digest,
            },
        }),
        _ => bail!("COMPUTE_PLUGIN_AUTHORITY_KEYRING_BINDING_CORRUPT"),
    };
    let maximum_bundle = transaction
        .query_row(
            "SELECT MAX(bundle_revision) FROM keyring_bundles",
            [],
            |row| row.get::<_, Option<i64>>(0),
        )
        .context("COMPUTE_PLUGIN_AUTHORITY_KEYRING_MAX_READ")?;
    if maximum_bundle != active.as_ref().map(|binding| binding.bundle_revision) {
        bail!("COMPUTE_PLUGIN_AUTHORITY_KEYRING_HISTORY: active bundle is not the history tip");
    }
    let history_anomaly = transaction
        .query_row(
            r#"SELECT EXISTS (
                SELECT 1
                FROM keyring_bundles AS newer
                JOIN keyring_bundles AS older
                  ON newer.bundle_revision > older.bundle_revision
                WHERE newer.publisher_revision < older.publisher_revision
                   OR newer.control_revision < older.control_revision
                   OR (newer.publisher_revision = older.publisher_revision
                       AND newer.publisher_digest <> older.publisher_digest)
                   OR (newer.control_revision = older.control_revision
                       AND newer.control_digest <> older.control_digest)
            )"#,
            [],
            |row| row.get::<_, i64>(0),
        )
        .context("COMPUTE_PLUGIN_AUTHORITY_KEYRING_HISTORY_READ")?;
    if history_anomaly != 0 {
        bail!("COMPUTE_PLUGIN_AUTHORITY_KEYRING_HISTORY: ring history is not monotonic");
    }
    Ok(AuthorityKeyringState {
        state_revision,
        authority_epoch,
        active,
        trusted_time_high_water_ms,
        clock_status,
    })
}

pub(super) fn load_snapshot_for_state(
    transaction: &Transaction<'_>,
    state: &AuthorityKeyringState,
    validation: KeyringSnapshotValidation,
    roots: &dyn ComputePluginBootstrapRootKeyResolver,
) -> Result<PersistedComputePluginKeyringSnapshot> {
    let active = state
        .active
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_AUTHORITY_KEYRING_INACTIVE"))?;
    let stored = transaction
        .query_row(
            r#"SELECT
                bundle_digest, signed_envelope_digest, signed_bundle_json, root_signing_key_id,
                root_key_fingerprint, publisher_revision, publisher_digest,
                control_revision, control_digest, publisher_key_count,
                control_key_count, generated_at_ms, expires_at_ms, installed_at_ms
            FROM keyring_bundles WHERE bundle_revision = ?1"#,
            params![active.bundle_revision],
            |row| {
                Ok(StoredBundleRow {
                    bundle_digest: row.get(0)?,
                    signed_envelope_digest: row.get(1)?,
                    signed_bundle_json: row.get(2)?,
                    root_signing_key_id: row.get(3)?,
                    root_key_fingerprint: row.get(4)?,
                    publisher_revision: row.get(5)?,
                    publisher_digest: row.get(6)?,
                    control_revision: row.get(7)?,
                    control_digest: row.get(8)?,
                    publisher_key_count: row.get(9)?,
                    control_key_count: row.get(10)?,
                    generated_at_ms: row.get(11)?,
                    expires_at_ms: row.get(12)?,
                    installed_at_ms: row.get(13)?,
                })
            },
        )
        .optional()
        .context("COMPUTE_PLUGIN_AUTHORITY_KEYRING_BUNDLE_READ")?
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_AUTHORITY_KEYRING_BUNDLE_MISSING"))?;
    let signed: SignedComputePluginKeyringBundle = serde_json::from_str(&stored.signed_bundle_json)
        .context("COMPUTE_PLUGIN_AUTHORITY_KEYRING_JSON")?;
    let (validated, trusted_time_floor_ms) = match validation {
        KeyringSnapshotValidation::Current(trusted_now) => {
            require_trusted_time(state, trusted_now.timestamp_millis(), false)?;
            let floor = trusted_now.timestamp_millis();
            (
                verify_and_validate_keyring_bundle(&signed, trusted_now, roots)?,
                Some(floor),
            )
        }
        KeyringSnapshotValidation::Archived => (
            verify_and_validate_archived_keyring_bundle(&signed, roots)?,
            None,
        ),
    };
    let signed_envelope_digest = jcs_sha256_hex(validated.signed())?;
    verify_bundle_columns(active, &stored, &validated, &signed_envelope_digest)?;
    verify_normalized_keys(transaction, active.bundle_revision, &validated)?;
    let sealed = transaction
        .query_row(
            "SELECT sealed_at_ms FROM keyring_seals WHERE bundle_revision = ?1",
            params![active.bundle_revision],
            |row| row.get::<_, i64>(0),
        )
        .optional()
        .context("COMPUTE_PLUGIN_AUTHORITY_KEYRING_SEAL_READ")?;
    if sealed != Some(stored.installed_at_ms) {
        bail!("COMPUTE_PLUGIN_AUTHORITY_KEYRING_UNSEALED");
    }
    Ok(PersistedComputePluginKeyringSnapshot {
        signed_envelope_digest,
        trusted_time_floor_ms,
        validated,
        state_revision: state.state_revision,
        authority_epoch: state.authority_epoch,
    })
}

pub(super) fn require_trusted_time(
    state: &AuthorityKeyringState,
    trusted_now_ms: i64,
    allow_uninitialized: bool,
) -> Result<()> {
    match (
        state.clock_status.as_str(),
        state.trusted_time_high_water_ms,
    ) {
        ("trusted", Some(high_water)) if trusted_now_ms >= high_water => Ok(()),
        ("uninitialized", None) if allow_uninitialized && state.active.is_none() => Ok(()),
        ("clock_untrusted", _) => bail!("COMPUTE_PLUGIN_AUTHORITY_CLOCK_UNTRUSTED"),
        ("trusted", Some(_)) => bail!("COMPUTE_PLUGIN_AUTHORITY_CLOCK_ROLLBACK"),
        _ => bail!("COMPUTE_PLUGIN_AUTHORITY_CLOCK_STATE_CORRUPT"),
    }
}

pub(super) fn advance_trusted_time(
    transaction: &Transaction<'_>,
    state: &AuthorityKeyringState,
    trusted_now_ms: i64,
) -> Result<()> {
    require_trusted_time(state, trusted_now_ms, false)?;
    let updated = transaction
        .execute(
            r#"UPDATE authority_meta SET
                trusted_time_high_water_ms = ?1,
                clock_status = 'trusted',
                updated_at_ms = ?1
            WHERE singleton = 1
              AND trusted_time_high_water_ms IS ?2
              AND clock_status = ?3"#,
            params![
                trusted_now_ms,
                state.trusted_time_high_water_ms,
                &state.clock_status,
            ],
        )
        .context("COMPUTE_PLUGIN_AUTHORITY_CLOCK_ADVANCE")?;
    if updated != 1 {
        bail!("COMPUTE_PLUGIN_AUTHORITY_CLOCK_CAS");
    }
    Ok(())
}
