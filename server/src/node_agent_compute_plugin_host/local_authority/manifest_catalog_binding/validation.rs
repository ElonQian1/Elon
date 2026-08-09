use anyhow::{bail, Context, Result};
use chrono::{DateTime, Utc};
use rusqlite::{OptionalExtension, Transaction};

mod receipt;
mod session;

pub(super) use receipt::validate_hashed_receipt;
pub(super) use session::{validate_session, ManifestCatalogBindingSession};

use super::types::{
    ComputePluginManifestCatalogBindingReceipt, HashedComputePluginManifestCatalogBindingReceipt,
    ManifestCatalogAuthorityState, ManifestCatalogBindingRequestDigest,
    PreparedManifestCatalogBindingRequest, ProjectedManifestCatalogBinding,
    COMPUTE_PLUGIN_MANIFEST_CATALOG_BINDING_RECEIPT_SCHEMA,
    COMPUTE_PLUGIN_MANIFEST_CATALOG_BINDING_REQUEST_SCHEMA,
    HASHED_COMPUTE_PLUGIN_MANIFEST_CATALOG_BINDING_RECEIPT_SCHEMA,
};
use crate::node_agent_compute_plugin_host::{
    install_plan_admission::validate_inventory,
    keyring::{ComputePluginBootstrapRootKeyResolver, ComputePluginKeyringBinding},
    lifecycle::ComputePluginInventorySnapshot,
    local_authority::keyring_snapshot::{
        load_snapshot_for_state, read_authority_keyring_state, KeyringSnapshotValidation,
        PersistedComputePluginKeyringSnapshot,
    },
    manifest_catalog::{verify_manifest_catalog_candidate, ComputePluginManifestCatalogCandidate},
    manifest_validation::is_sha256,
    plugin_manifest::{COMPUTE_PLUGIN_DIGEST_ALGORITHM, COMPUTE_PLUGIN_MANIFEST_CANONICALIZATION},
    signed_artifact_verification::jcs_sha256_hex,
};

const I_JSON_MAX_SAFE_INTEGER: i64 = 9_007_199_254_740_991;

pub(super) fn read_state_strict(
    transaction: &Transaction<'_>,
    trusted_now: &DateTime<Utc>,
) -> Result<ManifestCatalogAuthorityState> {
    read_state(transaction, trusted_now, true)
}

pub(super) fn read_state_at_or_before(
    transaction: &Transaction<'_>,
    trusted_now: &DateTime<Utc>,
) -> Result<ManifestCatalogAuthorityState> {
    read_state(transaction, trusted_now, false)
}

fn read_state(
    transaction: &Transaction<'_>,
    trusted_now: &DateTime<Utc>,
    require_strict_time: bool,
) -> Result<ManifestCatalogAuthorityState> {
    type Row = (
        String,
        i64,
        i64,
        String,
        String,
        i64,
        i64,
        String,
        i64,
        String,
        String,
        i64,
        i64,
        i64,
        i64,
        Option<i64>,
        String,
        i64,
        Option<i64>,
        Option<i64>,
        Option<String>,
        Option<i64>,
        Option<String>,
    );
    let row: Row = transaction
        .query_row(
            r#"SELECT installation_id_digest, state_revision, inventory_revision,
                inventory_digest, inventory_json, desired_policy_revision, sharing_enabled,
                node_profile_digest, manifest_catalog_revision, target_id,
                host_api_protocol_id, host_api_revision, authority_epoch, process_owner_epoch,
                updated_at_ms, trusted_time_high_water_ms, clock_status, schema_version,
                active_bundle_revision, publisher_keyring_revision, publisher_keyring_digest,
                control_keyring_revision, control_keyring_digest
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
                    row.get(9)?,
                    row.get(10)?,
                    row.get(11)?,
                    row.get(12)?,
                    row.get(13)?,
                    row.get(14)?,
                    row.get(15)?,
                    row.get(16)?,
                    row.get(17)?,
                    row.get(18)?,
                    row.get(19)?,
                    row.get(20)?,
                    row.get(21)?,
                    row.get(22)?,
                ))
            },
        )
        .optional()
        .context("COMPUTE_PLUGIN_MANIFEST_CATALOG_AUTHORITY_READ")?
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_AUTHORITY_UNINITIALIZED"))?;
    let (
        installation_id_digest,
        state_revision,
        inventory_revision,
        inventory_digest,
        inventory_json,
        desired_policy_revision,
        sharing_flag,
        node_profile_digest,
        manifest_catalog_revision,
        target_id,
        host_api_protocol_id,
        host_api_revision,
        authority_epoch,
        process_owner_epoch,
        updated_at_ms,
        trusted_time_high_water_ms,
        clock_status,
        schema_version,
        keyring_bundle_revision,
        publisher_keyring_revision,
        publisher_keyring_digest,
        control_keyring_revision,
        control_keyring_digest,
    ) = row;
    let host_api_revision = u32::try_from(host_api_revision)
        .map_err(|_| anyhow::anyhow!("COMPUTE_PLUGIN_MANIFEST_CATALOG_AUTHORITY_CORRUPT"))?;
    let trusted_time_high_water_ms = trusted_time_high_water_ms
        .filter(|_| clock_status == "trusted")
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_MANIFEST_CATALOG_CLOCK_UNTRUSTED"))?;
    let (keyring_bundle_revision, publisher_keyring, control_keyring) = match (
        keyring_bundle_revision,
        publisher_keyring_revision,
        publisher_keyring_digest,
        control_keyring_revision,
        control_keyring_digest,
    ) {
        (
            Some(bundle),
            Some(publisher_revision),
            Some(publisher_digest),
            Some(control_revision),
            Some(control_digest),
        ) => (
            bundle,
            ComputePluginKeyringBinding {
                revision: publisher_revision,
                digest: publisher_digest,
            },
            ComputePluginKeyringBinding {
                revision: control_revision,
                digest: control_digest,
            },
        ),
        _ => bail!("COMPUTE_PLUGIN_MANIFEST_CATALOG_KEYRING_INACTIVE"),
    };
    let inventory: ComputePluginInventorySnapshot = serde_json::from_str(&inventory_json)
        .context("COMPUTE_PLUGIN_MANIFEST_CATALOG_INVENTORY_JSON")?;
    validate_inventory(&inventory, trusted_now.clone())?;
    let sharing_enabled = match sharing_flag {
        0 => false,
        1 => true,
        _ => bail!("COMPUTE_PLUGIN_MANIFEST_CATALOG_SHARING_FLAG_CORRUPT"),
    };
    if schema_version != 3
        || state_revision < 0
        || state_revision >= I_JSON_MAX_SAFE_INTEGER
        || inventory_revision < 0
        || inventory_revision > I_JSON_MAX_SAFE_INTEGER
        || desired_policy_revision < 0
        || desired_policy_revision > I_JSON_MAX_SAFE_INTEGER
        || manifest_catalog_revision < 0
        || manifest_catalog_revision > I_JSON_MAX_SAFE_INTEGER
        || host_api_revision == 0
        || authority_epoch < 0
        || authority_epoch >= I_JSON_MAX_SAFE_INTEGER
        || process_owner_epoch <= 0
        || process_owner_epoch > I_JSON_MAX_SAFE_INTEGER
        || updated_at_ms < 0
        || updated_at_ms >= I_JSON_MAX_SAFE_INTEGER
        || trusted_time_high_water_ms < 0
        || trusted_time_high_water_ms >= I_JSON_MAX_SAFE_INTEGER
        || updated_at_ms != trusted_time_high_water_ms
        || (require_strict_time && trusted_now.timestamp_millis() <= trusted_time_high_water_ms)
        || (!require_strict_time && trusted_now.timestamp_millis() < trusted_time_high_water_ms)
        || inventory.inventory_revision != inventory_revision
        || inventory.desired_policy_revision != desired_policy_revision
        || inventory.sharing_enabled != sharing_enabled
        || !inventory.plugins.is_empty()
        || jcs_sha256_hex(&inventory)? != inventory_digest
        || !is_sha256(&installation_id_digest)
        || !is_sha256(&inventory_digest)
        || !is_sha256(&node_profile_digest)
        || !identifier_is_valid(&target_id)
        || !identifier_is_valid(&host_api_protocol_id)
        || keyring_bundle_revision <= 0
        || keyring_bundle_revision > I_JSON_MAX_SAFE_INTEGER
        || publisher_keyring.revision <= 0
        || publisher_keyring.revision > I_JSON_MAX_SAFE_INTEGER
        || control_keyring.revision <= 0
        || control_keyring.revision > I_JSON_MAX_SAFE_INTEGER
        || !is_sha256(&publisher_keyring.digest)
        || !is_sha256(&control_keyring.digest)
    {
        bail!("COMPUTE_PLUGIN_MANIFEST_CATALOG_AUTHORITY_CORRUPT");
    }
    let open_owners = transaction
        .query_row(
            "SELECT COUNT(*) FROM candidate_owners WHERE state IN ('owned', 'cleanup_pending')",
            [],
            |row| row.get::<_, i64>(0),
        )
        .context("COMPUTE_PLUGIN_MANIFEST_CATALOG_CANDIDATE_OWNER_COUNT")?;
    let prepared_fetch = transaction
        .query_row(
            "SELECT COUNT(*) FROM fetch_claims WHERE state = 'prepared'",
            [],
            |row| row.get::<_, i64>(0),
        )
        .context("COMPUTE_PLUGIN_MANIFEST_CATALOG_FETCH_COUNT")?;
    let prepared_verification = transaction
        .query_row(
            "SELECT COUNT(*) FROM candidate_verification_runs WHERE state = 'prepared'",
            [],
            |row| row.get::<_, i64>(0),
        )
        .context("COMPUTE_PLUGIN_MANIFEST_CATALOG_VERIFICATION_COUNT")?;
    if open_owners != 0 || prepared_fetch != 0 || prepared_verification != 0 {
        bail!("COMPUTE_PLUGIN_MANIFEST_CATALOG_WORK_NOT_QUIESCENT");
    }
    Ok(ManifestCatalogAuthorityState {
        installation_id_digest,
        state_revision,
        inventory_revision,
        inventory_digest,
        inventory_json,
        desired_policy_revision,
        sharing_enabled,
        node_profile_digest,
        manifest_catalog_revision,
        target_id,
        host_api_protocol_id,
        host_api_revision,
        authority_epoch,
        process_owner_epoch,
        trusted_time_high_water_ms,
        updated_at_ms,
        keyring_bundle_revision,
        publisher_keyring,
        control_keyring,
    })
}

pub(super) fn load_keyring(
    transaction: &Transaction<'_>,
    state: &ManifestCatalogAuthorityState,
    trusted_now: DateTime<Utc>,
    roots: &dyn ComputePluginBootstrapRootKeyResolver,
) -> Result<PersistedComputePluginKeyringSnapshot> {
    let keyring_state = read_authority_keyring_state(transaction)?;
    if keyring_state.state_revision != state.state_revision
        || keyring_state.authority_epoch != state.authority_epoch
    {
        bail!("COMPUTE_PLUGIN_MANIFEST_CATALOG_KEYRING_FENCE_CHANGED");
    }
    let snapshot = load_snapshot_for_state(
        transaction,
        &keyring_state,
        KeyringSnapshotValidation::Current(trusted_now),
        roots,
    )?;
    if snapshot.bundle_revision() != state.keyring_bundle_revision
        || snapshot.publisher_binding() != &state.publisher_keyring
        || snapshot.control_binding() != &state.control_keyring
    {
        bail!("COMPUTE_PLUGIN_MANIFEST_CATALOG_KEYRING_CHANGED");
    }
    Ok(snapshot)
}

pub(super) fn prepare_request(
    candidate: &ComputePluginManifestCatalogCandidate,
    state: &ManifestCatalogAuthorityState,
    trusted_now: DateTime<Utc>,
    keyring: &PersistedComputePluginKeyringSnapshot,
) -> Result<PreparedManifestCatalogBindingRequest> {
    let validated = verify_manifest_catalog_candidate(
        candidate,
        &state.target_id,
        &state.host_api_protocol_id,
        state.host_api_revision,
        state.keyring_bundle_revision,
        &state.publisher_keyring,
        &state.control_keyring,
        trusted_now,
        keyring,
        keyring,
    )?;
    let entry_count = i64::try_from(validated.catalog().entries.len())
        .context("COMPUTE_PLUGIN_MANIFEST_CATALOG_ENTRY_COUNT")?;
    let request_digest = jcs_sha256_hex(&ManifestCatalogBindingRequestDigest {
        schema: COMPUTE_PLUGIN_MANIFEST_CATALOG_BINDING_REQUEST_SCHEMA,
        request_id: candidate.request_id(),
        installation_id_digest: &state.installation_id_digest,
        catalog_revision: validated.catalog().catalog_revision,
        catalog_digest: validated.catalog_digest(),
        signed_catalog_envelope_digest: validated.signed_catalog_envelope_digest(),
        control_signing_key_id: validated.control_signing_key_id(),
        control_signing_key_fingerprint: validated.control_signing_key_fingerprint(),
        signed_manifest_set_digest: validated.signed_manifest_set_digest(),
        node_profile_digest: &state.node_profile_digest,
        target_id: &state.target_id,
        host_api_protocol_id: &state.host_api_protocol_id,
        host_api_revision: state.host_api_revision,
        keyring_bundle_revision: state.keyring_bundle_revision,
        publisher_keyring: &state.publisher_keyring,
        control_keyring: &state.control_keyring,
    })?;
    Ok(PreparedManifestCatalogBindingRequest {
        request_id: candidate.request_id().to_string(),
        request_digest,
        installation_id_digest: state.installation_id_digest.clone(),
        catalog_revision: candidate.catalog_revision(),
        catalog_json: validated.catalog_json().to_string(),
        catalog_digest: validated.catalog_digest().to_string(),
        signed_catalog_json: validated.signed_catalog_json().to_string(),
        signed_catalog_envelope_digest: validated.signed_catalog_envelope_digest().to_string(),
        control_signing_key_id: validated.control_signing_key_id().to_string(),
        control_signing_key_fingerprint: validated.control_signing_key_fingerprint().to_string(),
        signed_manifests_json: validated.signed_manifests_json().to_string(),
        signed_manifest_set_digest: validated.signed_manifest_set_digest().to_string(),
        catalog_entry_count: entry_count,
        node_profile_digest: state.node_profile_digest.clone(),
        target_id: state.target_id.clone(),
        host_api_protocol_id: state.host_api_protocol_id.clone(),
        host_api_revision: state.host_api_revision,
        keyring_bundle_revision: state.keyring_bundle_revision,
        publisher_keyring: state.publisher_keyring.clone(),
        control_keyring: state.control_keyring.clone(),
    })
}

pub(super) fn project(
    request: PreparedManifestCatalogBindingRequest,
    before: ManifestCatalogAuthorityState,
    bound_at_ms: i64,
) -> Result<ProjectedManifestCatalogBinding> {
    if request.installation_id_digest != before.installation_id_digest
        || request.catalog_revision < before.manifest_catalog_revision
        || request.catalog_revision <= 0
        || bound_at_ms <= before.trusted_time_high_water_ms
        || bound_at_ms <= before.updated_at_ms
        || bound_at_ms > I_JSON_MAX_SAFE_INTEGER
    {
        bail!("COMPUTE_PLUGIN_MANIFEST_CATALOG_PROJECTION_INVALID");
    }
    let receipt = ComputePluginManifestCatalogBindingReceipt {
        schema: COMPUTE_PLUGIN_MANIFEST_CATALOG_BINDING_RECEIPT_SCHEMA.to_string(),
        request_id: request.request_id.clone(),
        request_digest: request.request_digest.clone(),
        installation_id_digest: request.installation_id_digest.clone(),
        manifest_catalog_revision_before: before.manifest_catalog_revision,
        catalog_revision: request.catalog_revision,
        catalog_digest: request.catalog_digest.clone(),
        signed_catalog_envelope_digest: request.signed_catalog_envelope_digest.clone(),
        control_signing_key_id: request.control_signing_key_id.clone(),
        control_signing_key_fingerprint: request.control_signing_key_fingerprint.clone(),
        signed_manifest_set_digest: request.signed_manifest_set_digest.clone(),
        catalog_entry_count: request.catalog_entry_count,
        node_profile_digest: request.node_profile_digest.clone(),
        target_id: request.target_id.clone(),
        host_api_protocol_id: request.host_api_protocol_id.clone(),
        host_api_revision: request.host_api_revision,
        keyring_bundle_revision: request.keyring_bundle_revision,
        publisher_keyring: request.publisher_keyring.clone(),
        control_keyring: request.control_keyring.clone(),
        state_revision_before: before.state_revision,
        state_revision_after: before.state_revision + 1,
        inventory_revision: before.inventory_revision,
        inventory_digest: before.inventory_digest.clone(),
        authority_epoch_before: before.authority_epoch,
        authority_epoch_after: before.authority_epoch + 1,
        process_owner_epoch: before.process_owner_epoch,
        trusted_time_before_ms: before.trusted_time_high_water_ms,
        bound_at_ms,
    };
    let hashed_receipt = HashedComputePluginManifestCatalogBindingReceipt {
        schema: HASHED_COMPUTE_PLUGIN_MANIFEST_CATALOG_BINDING_RECEIPT_SCHEMA.to_string(),
        receipt_digest: jcs_sha256_hex(&receipt)?,
        receipt,
        canonicalization: COMPUTE_PLUGIN_MANIFEST_CANONICALIZATION.to_string(),
        receipt_digest_algorithm: COMPUTE_PLUGIN_DIGEST_ALGORITHM.to_string(),
    };
    validate_hashed_receipt(&request, &hashed_receipt)?;
    Ok(ProjectedManifestCatalogBinding {
        request,
        before,
        hashed_receipt,
    })
}

pub(super) fn validate_authority_after(
    transaction: &Transaction<'_>,
    projected: &ProjectedManifestCatalogBinding,
    trusted_now: &DateTime<Utc>,
) -> Result<()> {
    let after = read_state_at_or_before(transaction, trusted_now)?;
    let receipt = &projected.hashed_receipt.receipt;
    if after.installation_id_digest != receipt.installation_id_digest
        || after.state_revision != receipt.state_revision_after
        || after.inventory_revision != receipt.inventory_revision
        || after.inventory_digest != receipt.inventory_digest
        || after.inventory_json != projected.before.inventory_json
        || after.desired_policy_revision != projected.before.desired_policy_revision
        || after.sharing_enabled != projected.before.sharing_enabled
        || after.node_profile_digest != receipt.node_profile_digest
        || after.manifest_catalog_revision != receipt.catalog_revision
        || after.target_id != receipt.target_id
        || after.host_api_protocol_id != receipt.host_api_protocol_id
        || after.host_api_revision != receipt.host_api_revision
        || after.keyring_bundle_revision != receipt.keyring_bundle_revision
        || after.publisher_keyring != receipt.publisher_keyring
        || after.control_keyring != receipt.control_keyring
        || after.authority_epoch != receipt.authority_epoch_after
        || after.process_owner_epoch != receipt.process_owner_epoch
        || after.trusted_time_high_water_ms != receipt.bound_at_ms
        || after.updated_at_ms != receipt.bound_at_ms
    {
        bail!("COMPUTE_PLUGIN_MANIFEST_CATALOG_AUTHORITY_READBACK_CHANGED");
    }
    Ok(())
}

fn identifier_is_valid(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 256
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':' | b'/')
        })
}
