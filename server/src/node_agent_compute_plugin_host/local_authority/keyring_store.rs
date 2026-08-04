use anyhow::{bail, Context, Result};
use chrono::{DateTime, Utc};
use rusqlite::{params, Transaction};

use super::{
    fetch_claim_revocation::revoke_for_keyring_authority_epoch_advance,
    keyring_integrity::timestamp_millis,
    keyring_snapshot::{
        advance_trusted_time, load_snapshot_for_state, read_authority_keyring_state,
        require_trusted_time, AuthorityKeyringState, KeyringSnapshotValidation,
        PersistedComputePluginKeyringSnapshot,
    },
    ComputePluginLocalAuthority,
};
use crate::node_agent_compute_plugin_host::{
    keyring::{
        ComputePluginBootstrapRootKeyResolver, ComputePluginKeyring, ComputePluginKeyringBinding,
        SignedComputePluginKeyringBundle, ValidatedComputePluginKeyringBundle,
    },
    keyring_validation::verify_and_validate_keyring_bundle,
    signed_artifact_verification::jcs_sha256_hex,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ComputePluginKeyringInstallDisposition {
    Installed,
    AlreadyInstalled,
}

#[derive(Debug, Clone)]
pub(crate) struct ComputePluginKeyringInstallResult {
    disposition: ComputePluginKeyringInstallDisposition,
    snapshot: PersistedComputePluginKeyringSnapshot,
}

impl ComputePluginKeyringInstallResult {
    pub(crate) fn disposition(&self) -> ComputePluginKeyringInstallDisposition {
        self.disposition
    }

    pub(crate) fn state_revision(&self) -> i64 {
        self.snapshot.state_revision()
    }

    pub(crate) fn authority_epoch(&self) -> i64 {
        self.snapshot.authority_epoch()
    }

    pub(crate) fn bundle_revision(&self) -> i64 {
        self.snapshot.bundle_revision()
    }

    pub(crate) fn publisher_binding(&self) -> &ComputePluginKeyringBinding {
        self.snapshot.publisher_binding()
    }

    pub(crate) fn control_binding(&self) -> &ComputePluginKeyringBinding {
        self.snapshot.control_binding()
    }

    pub(crate) fn root_key_fingerprint(&self) -> &str {
        self.snapshot.root_key_fingerprint()
    }
}

impl ComputePluginLocalAuthority {
    /// Verifies, persists, seals and activates one root-signed bundle atomically. The same bundle
    /// revision and digest is an idempotent replay; every rollback or equivocation fails closed.
    pub(crate) fn install_keyring_bundle(
        &self,
        signed: &SignedComputePluginKeyringBundle,
        trusted_now: DateTime<Utc>,
        roots: &dyn ComputePluginBootstrapRootKeyResolver,
    ) -> Result<ComputePluginKeyringInstallResult> {
        self.with_immediate(|transaction| {
            let state = read_authority_keyring_state(transaction)?;
            require_trusted_time(&state, trusted_now.timestamp_millis(), true)?;
            let candidate = verify_and_validate_keyring_bundle(signed, trusted_now.clone(), roots)?;
            if let Some(current) = state
                .active
                .as_ref()
                .map(|_| {
                    load_snapshot_for_state(
                        transaction,
                        &state,
                        KeyringSnapshotValidation::Archived,
                        roots,
                    )
                })
                .transpose()?
            {
                match classify_candidate(&current, &candidate)? {
                    CandidateDisposition::Replay => {
                        advance_trusted_time(transaction, &state, trusted_now.timestamp_millis())?;
                        let replay_state = read_authority_keyring_state(transaction)?;
                        let snapshot = load_snapshot_for_state(
                            transaction,
                            &replay_state,
                            KeyringSnapshotValidation::Current(trusted_now),
                            roots,
                        )?;
                        return Ok(ComputePluginKeyringInstallResult {
                            disposition: ComputePluginKeyringInstallDisposition::AlreadyInstalled,
                            snapshot,
                        });
                    }
                    CandidateDisposition::Advance => {}
                }
            }
            if state.state_revision == i64::MAX || state.authority_epoch == i64::MAX {
                bail!("COMPUTE_PLUGIN_AUTHORITY_FENCE_EXHAUSTED");
            }
            insert_validated_bundle(transaction, &candidate, trusted_now.timestamp_millis())?;
            revoke_for_keyring_authority_epoch_advance(
                transaction,
                state.authority_epoch,
                state.authority_epoch + 1,
                trusted_now.timestamp_millis(),
            )?;
            activate_bundle(
                transaction,
                &state,
                &candidate,
                trusted_now.timestamp_millis(),
            )?;
            let committed_state = read_authority_keyring_state(transaction)?;
            let snapshot = load_snapshot_for_state(
                transaction,
                &committed_state,
                KeyringSnapshotValidation::Current(trusted_now),
                roots,
            )?;
            Ok(ComputePluginKeyringInstallResult {
                disposition: ComputePluginKeyringInstallDisposition::Installed,
                snapshot,
            })
        })
    }
}

enum CandidateDisposition {
    Replay,
    Advance,
}

fn classify_candidate(
    current: &PersistedComputePluginKeyringSnapshot,
    candidate: &ValidatedComputePluginKeyringBundle,
) -> Result<CandidateDisposition> {
    let candidate_revision = candidate.signed().bundle.bundle_revision;
    if candidate_revision < current.bundle_revision() {
        bail!("COMPUTE_PLUGIN_KEYRING_ROLLBACK: bundle revision decreased");
    }
    if candidate_revision == current.bundle_revision() {
        if jcs_sha256_hex(candidate.signed())? == current.signed_envelope_digest()
            && candidate.signed().bundle_digest == current.bundle_digest()
            && candidate.signed().signature.signing_key_id == current.root_signing_key_id()
            && candidate.root_key_fingerprint() == current.root_key_fingerprint()
            && candidate.publisher_binding() == current.publisher_binding()
            && candidate.control_binding() == current.control_binding()
        {
            return Ok(CandidateDisposition::Replay);
        }
        bail!("COMPUTE_PLUGIN_KEYRING_REVISION_CONFLICT: bundle revision changed contents");
    }
    validate_ring_advance(
        "PUBLISHER",
        current.publisher_binding(),
        candidate.publisher_binding(),
    )?;
    validate_ring_advance(
        "CONTROL",
        current.control_binding(),
        candidate.control_binding(),
    )?;
    Ok(CandidateDisposition::Advance)
}

fn validate_ring_advance(
    label: &str,
    current: &ComputePluginKeyringBinding,
    candidate: &ComputePluginKeyringBinding,
) -> Result<()> {
    if candidate.revision < current.revision {
        bail!("COMPUTE_PLUGIN_KEYRING_{label}_ROLLBACK: ring revision decreased");
    }
    if candidate.revision == current.revision && candidate.digest != current.digest {
        bail!("COMPUTE_PLUGIN_KEYRING_{label}_CONFLICT: the same ring revision changed digest");
    }
    Ok(())
}

fn insert_validated_bundle(
    transaction: &Transaction<'_>,
    validated: &ValidatedComputePluginKeyringBundle,
    installed_at_ms: i64,
) -> Result<()> {
    let signed = validated.signed();
    let publisher_count = i64::try_from(signed.bundle.publisher_keyring.keys.len())
        .context("COMPUTE_PLUGIN_AUTHORITY_KEYRING_KEY_COUNT")?;
    let control_count = i64::try_from(signed.bundle.control_keyring.keys.len())
        .context("COMPUTE_PLUGIN_AUTHORITY_KEYRING_KEY_COUNT")?;
    let signed_json =
        serde_json::to_string(signed).context("COMPUTE_PLUGIN_AUTHORITY_KEYRING_JSON")?;
    let signed_envelope_digest = jcs_sha256_hex(signed)?;
    let inserted = transaction
        .execute(
            r#"INSERT INTO keyring_bundles (
                bundle_revision, bundle_digest, signed_envelope_digest, signed_bundle_json,
                root_signing_key_id, root_key_fingerprint,
                publisher_revision, publisher_digest,
                control_revision, control_digest,
                publisher_key_count, control_key_count,
                generated_at_ms, expires_at_ms, installed_at_ms
            ) VALUES (
                ?1, ?2, ?3, ?4,
                ?5, ?6,
                ?7, ?8,
                ?9, ?10,
                ?11, ?12,
                ?13, ?14, ?15
            )"#,
            params![
                signed.bundle.bundle_revision,
                &signed.bundle_digest,
                signed_envelope_digest,
                signed_json,
                &signed.signature.signing_key_id,
                validated.root_key_fingerprint(),
                validated.publisher_binding().revision,
                &validated.publisher_binding().digest,
                validated.control_binding().revision,
                &validated.control_binding().digest,
                publisher_count,
                control_count,
                timestamp_millis(&signed.bundle.generated_at)?,
                timestamp_millis(&signed.bundle.expires_at)?,
                installed_at_ms,
            ],
        )
        .context("COMPUTE_PLUGIN_AUTHORITY_KEYRING_BUNDLE_INSERT")?;
    if inserted != 1 {
        bail!("COMPUTE_PLUGIN_AUTHORITY_KEYRING_BUNDLE_CAS");
    }
    insert_ring_keys(
        transaction,
        signed.bundle.bundle_revision,
        &signed.bundle.publisher_keyring,
    )?;
    insert_ring_keys(
        transaction,
        signed.bundle.bundle_revision,
        &signed.bundle.control_keyring,
    )?;
    let sealed = transaction
        .execute(
            "INSERT INTO keyring_seals (bundle_revision, sealed_at_ms) VALUES (?1, ?2)",
            params![signed.bundle.bundle_revision, installed_at_ms],
        )
        .context("COMPUTE_PLUGIN_AUTHORITY_KEYRING_SEAL")?;
    if sealed != 1 {
        bail!("COMPUTE_PLUGIN_AUTHORITY_KEYRING_SEAL_CAS");
    }
    Ok(())
}

fn insert_ring_keys(
    transaction: &Transaction<'_>,
    bundle_revision: i64,
    ring: &ComputePluginKeyring,
) -> Result<()> {
    for key in &ring.keys {
        let inserted = transaction
            .execute(
                r#"INSERT INTO keyring_keys (
                    bundle_revision, purpose, subject_id, signing_key_id,
                    public_key_base64, fingerprint_sha256, status,
                    not_before_ms, not_after_ms, revoked_at_ms
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)"#,
                params![
                    bundle_revision,
                    &key.purpose,
                    key.publisher_id.as_deref().unwrap_or_default(),
                    &key.signing_key_id,
                    &key.public_key_base64,
                    &key.fingerprint_sha256,
                    &key.status,
                    timestamp_millis(&key.not_before)?,
                    timestamp_millis(&key.not_after)?,
                    key.revoked_at
                        .as_deref()
                        .map(timestamp_millis)
                        .transpose()?,
                ],
            )
            .context("COMPUTE_PLUGIN_AUTHORITY_KEYRING_KEY_INSERT")?;
        if inserted != 1 {
            bail!("COMPUTE_PLUGIN_AUTHORITY_KEYRING_KEY_CAS");
        }
    }
    Ok(())
}

fn activate_bundle(
    transaction: &Transaction<'_>,
    state: &AuthorityKeyringState,
    validated: &ValidatedComputePluginKeyringBundle,
    updated_at_ms: i64,
) -> Result<()> {
    let old_bundle = state.active.as_ref().map(|value| value.bundle_revision);
    let old_publisher_revision = state.active.as_ref().map(|value| value.publisher.revision);
    let old_publisher_digest = state
        .active
        .as_ref()
        .map(|value| value.publisher.digest.as_str());
    let old_control_revision = state.active.as_ref().map(|value| value.control.revision);
    let old_control_digest = state
        .active
        .as_ref()
        .map(|value| value.control.digest.as_str());
    let updated = transaction
        .execute(
            r#"UPDATE authority_meta SET
                active_bundle_revision = ?1,
                publisher_keyring_revision = ?2,
                publisher_keyring_digest = ?3,
                control_keyring_revision = ?4,
                control_keyring_digest = ?5,
                state_revision = state_revision + 1,
                authority_epoch = authority_epoch + 1,
                trusted_time_high_water_ms = ?6,
                clock_status = 'trusted',
                updated_at_ms = ?6
            WHERE singleton = 1
              AND state_revision = ?7
              AND authority_epoch = ?8
              AND active_bundle_revision IS ?9
              AND publisher_keyring_revision IS ?10
              AND publisher_keyring_digest IS ?11
              AND control_keyring_revision IS ?12
              AND control_keyring_digest IS ?13
              AND trusted_time_high_water_ms IS ?14
              AND clock_status = ?15"#,
            params![
                validated.signed().bundle.bundle_revision,
                validated.publisher_binding().revision,
                &validated.publisher_binding().digest,
                validated.control_binding().revision,
                &validated.control_binding().digest,
                updated_at_ms,
                state.state_revision,
                state.authority_epoch,
                old_bundle,
                old_publisher_revision,
                old_publisher_digest,
                old_control_revision,
                old_control_digest,
                state.trusted_time_high_water_ms,
                &state.clock_status,
            ],
        )
        .context("COMPUTE_PLUGIN_AUTHORITY_KEYRING_ACTIVATE")?;
    if updated != 1 {
        bail!("COMPUTE_PLUGIN_AUTHORITY_KEYRING_ACTIVATION_CAS");
    }
    Ok(())
}
