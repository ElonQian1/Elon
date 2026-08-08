use std::time::Instant;

use anyhow::{bail, Context, Result};
use chrono::{DateTime, SecondsFormat, Utc};
use rusqlite::{OptionalExtension, Transaction};

use super::{
    super::{process_ownership::ComputePluginFetchProcessFence, ComputePluginLocalAuthority},
    types::{
        ComputePluginSharingPolicyBindingReceipt, HashedComputePluginSharingPolicyBindingReceipt,
        PolicyBindingAuthorityState, PreparedSharingPolicyBindingRequest,
        ProjectedSharingPolicyBinding, SharingPolicyBindingRequestDigest,
        COMPUTE_PLUGIN_SHARING_POLICY_BINDING_RECEIPT_SCHEMA,
        COMPUTE_PLUGIN_SHARING_POLICY_BINDING_REQUEST_SCHEMA,
        HASHED_COMPUTE_PLUGIN_SHARING_POLICY_BINDING_RECEIPT_SCHEMA,
    },
};
use crate::{
    compute_plugin_sharing_directive::{
        compute_plugin_sharing_policy_snapshot_digest,
        validate_compute_plugin_sharing_policy_snapshot_v1,
    },
    node_agent_compute_plugin_host::{
        bootstrap::ComputePluginLocalPolicyBindingIntent,
        fetch_file::PinnedComputePluginRoot,
        install_plan_admission::validate_inventory,
        lifecycle::{ComputePluginInventorySnapshot, ACTIVATION_DISABLED, ADMISSION_REVOKED},
        manifest_validation::is_sha256,
        plugin_manifest::{
            COMPUTE_PLUGIN_DIGEST_ALGORITHM, COMPUTE_PLUGIN_MANIFEST_CANONICALIZATION,
        },
        signed_artifact_verification::jcs_sha256_hex,
        trusted_time::ComputePluginTrustedTimeObservation,
    },
};

const MAX_IDENTIFIER_BYTES: usize = 256;
const MAX_IJSON_SAFE_INTEGER_I64: i64 = 9_007_199_254_740_991;
const MAX_IJSON_SAFE_INTEGER_U64: u64 = 9_007_199_254_740_991;

pub(super) struct PolicyBindingSessionFacts {
    pub trusted_now: DateTime<Utc>,
    pub prepared_at: Instant,
    pub clock_epoch_digest: String,
}

pub(super) struct ReadPolicyBindingState {
    pub authority: PolicyBindingAuthorityState,
    pub inventory: ComputePluginInventorySnapshot,
}

pub(super) fn validate_session_and_prepare_request(
    authority: &ComputePluginLocalAuthority,
    intent: &ComputePluginLocalPolicyBindingIntent,
    root: &PinnedComputePluginRoot,
    process_fence: &ComputePluginFetchProcessFence,
    observation: &ComputePluginTrustedTimeObservation,
) -> Result<(
    PreparedSharingPolicyBindingRequest,
    PolicyBindingSessionFacts,
)> {
    let snapshot = intent.snapshot();
    let authority_root = authority
        .path()
        .parent()
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_POLICY_BINDING_AUTHORITY_PATH_INVALID"))?;
    validate_compute_plugin_sharing_policy_snapshot_v1(snapshot)
        .map_err(|error| anyhow::anyhow!(error.code()))?;
    let calculated_snapshot_digest = compute_plugin_sharing_policy_snapshot_digest(snapshot)
        .map_err(|error| anyhow::anyhow!(error.code()))?;
    if calculated_snapshot_digest != intent.snapshot_digest()
        || snapshot.installation_identity_digest != intent.installation_identity_digest()
        || root.installation_id_digest() != intent.installation_identity_digest()
        || !is_sha256(root.root_identity_digest())
        || root.node_data_paths() != intent.node_data_paths()
        || root.compute_plugin_root() != intent.compute_plugin_root()
        || root.compute_plugin_root() != authority_root
        || intent.compute_plugin_root() != authority_root
        || intent.bootstrap_instance_id().is_empty()
        || intent.configuration_generation() == 0
        || intent.cancellation_generation() == 0
    {
        bail!("COMPUTE_PLUGIN_POLICY_BINDING_INTENT_CHANGED");
    }
    if !authority
        .instance_binding()
        .matches(process_fence.authority_instance_binding())
        || process_fence.installation_id_digest() != intent.installation_identity_digest()
        || observation.installation_id_digest() != intent.installation_identity_digest()
        || observation.clock_epoch_digest() != process_fence.clock_epoch_digest()
        || observation.observed_at() <= process_fence.acquired_observed_at()
        || observation.trusted_now().timestamp_millis() <= process_fence.acquired_at_ms()
    {
        bail!("COMPUTE_PLUGIN_POLICY_BINDING_AUTHORITY_SESSION_CHANGED");
    }
    let policy_revision = i64::try_from(snapshot.policy_revision)
        .context("COMPUTE_PLUGIN_POLICY_BINDING_REVISION_RANGE")?;
    let (authorization_ref, authorization_revision, authorization_digest) =
        if let Some(binding) = snapshot.authorization.as_ref() {
            let revision = i64::try_from(binding.revision)
                .context("COMPUTE_PLUGIN_POLICY_BINDING_AUTHORIZATION_REVISION_RANGE")?;
            (
                Some(binding.authorization_ref.clone()),
                Some(revision),
                Some(binding.digest.clone()),
            )
        } else {
            (None, None, None)
        };
    let policy_snapshot_json =
        serde_json::to_string(snapshot).context("COMPUTE_PLUGIN_POLICY_BINDING_SNAPSHOT_JSON")?;
    if policy_snapshot_json.len() > 65_536 {
        bail!("COMPUTE_PLUGIN_POLICY_BINDING_SNAPSHOT_LIMIT");
    }
    let request_digest = jcs_sha256_hex(&SharingPolicyBindingRequestDigest {
        schema: COMPUTE_PLUGIN_SHARING_POLICY_BINDING_REQUEST_SCHEMA,
        policy_snapshot: snapshot,
        policy_snapshot_digest: intent.snapshot_digest(),
    })?;
    Ok((
        PreparedSharingPolicyBindingRequest {
            node_id: snapshot.node_id.clone(),
            owner_user_id: snapshot.owner_user_id.clone(),
            installation_id_digest: snapshot.installation_identity_digest.clone(),
            policy_revision,
            policy_digest: snapshot.policy_digest.clone(),
            policy_snapshot_json,
            policy_snapshot_digest: intent.snapshot_digest().to_string(),
            sharing_enabled: snapshot.plugin_runtime_requested,
            sharing_authorization_ref: authorization_ref,
            sharing_authorization_revision: authorization_revision,
            sharing_authorization_digest: authorization_digest,
            source_preparation_id: intent.preparation_id().map(str::to_string),
            source_bootstrap_instance_id: intent.bootstrap_instance_id().to_string(),
            source_configuration_generation: intent.configuration_generation(),
            source_cancellation_generation: intent.cancellation_generation(),
            request_digest,
        },
        PolicyBindingSessionFacts {
            trusted_now: observation.trusted_now().to_owned(),
            prepared_at: observation.observed_at(),
            clock_epoch_digest: observation.clock_epoch_digest().to_string(),
        },
    ))
}

pub(super) fn read_state(
    transaction: &Transaction<'_>,
    trusted_now: &DateTime<Utc>,
) -> Result<ReadPolicyBindingState> {
    type Row = (
        String,
        i64,
        i64,
        String,
        String,
        i64,
        i64,
        Option<String>,
        Option<i64>,
        Option<String>,
        i64,
        i64,
        Option<i64>,
        String,
        i64,
    );
    let row: Row = transaction
        .query_row(
            r#"SELECT installation_id_digest, state_revision, inventory_revision,
                inventory_digest, inventory_json, desired_policy_revision, sharing_enabled,
                sharing_authorization_ref, sharing_authorization_revision,
                sharing_authorization_digest, authority_epoch, process_owner_epoch,
                trusted_time_high_water_ms, clock_status, updated_at_ms
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
                ))
            },
        )
        .optional()
        .context("COMPUTE_PLUGIN_POLICY_BINDING_AUTHORITY_READ")?
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_AUTHORITY_UNINITIALIZED"))?;
    let sharing_enabled = match row.6 {
        0 => false,
        1 => true,
        _ => bail!("COMPUTE_PLUGIN_POLICY_BINDING_SHARING_FLAG_CORRUPT"),
    };
    let trusted_time_high_water_ms = row
        .12
        .filter(|_| row.13 == "trusted")
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_POLICY_BINDING_CLOCK_UNTRUSTED"))?;
    let inventory: ComputePluginInventorySnapshot =
        serde_json::from_str(&row.4).context("COMPUTE_PLUGIN_POLICY_BINDING_INVENTORY_JSON")?;
    validate_inventory(&inventory, trusted_now.to_owned())?;
    let canonical_inventory_json = serde_json::to_string(&inventory)
        .context("COMPUTE_PLUGIN_POLICY_BINDING_INVENTORY_CANONICAL_JSON")?;
    if canonical_inventory_json != row.4
        || jcs_sha256_hex(&inventory)? != row.3
        || inventory.inventory_revision != row.2
        || inventory.desired_policy_revision != row.5
        || inventory.sharing_enabled != sharing_enabled
        || row.1 < 0
        || row.2 < 0
        || row.5 < 0
        || row.10 < 0
        || row.11 <= 0
        || trusted_time_high_water_ms < 0
        || row.14 != trusted_time_high_water_ms
        || trusted_now.timestamp_millis() <= trusted_time_high_water_ms
        || !is_sha256(&row.0)
        || !is_sha256(&row.3)
        || !authorization_shape_is_valid(sharing_enabled, &row.7, row.8, &row.9)
    {
        bail!("COMPUTE_PLUGIN_POLICY_BINDING_AUTHORITY_CORRUPT");
    }
    Ok(ReadPolicyBindingState {
        authority: PolicyBindingAuthorityState {
            installation_id_digest: row.0,
            state_revision: row.1,
            inventory_revision: row.2,
            inventory_digest: row.3,
            inventory_json: row.4,
            desired_policy_revision: row.5,
            sharing_enabled,
            sharing_authorization_ref: row.7,
            sharing_authorization_revision: row.8,
            sharing_authorization_digest: row.9,
            authority_epoch: row.10,
            process_owner_epoch: row.11,
            trusted_time_high_water_ms,
            updated_at_ms: row.14,
        },
        inventory,
    })
}

pub(super) fn project(
    request: PreparedSharingPolicyBindingRequest,
    current: ReadPolicyBindingState,
    bound_at: &DateTime<Utc>,
) -> Result<ProjectedSharingPolicyBinding> {
    let before = current.authority;
    if request.installation_id_digest != before.installation_id_digest
        || request.policy_revision <= before.desired_policy_revision
        || bound_at.timestamp_millis() <= before.trusted_time_high_water_ms
        || bound_at.timestamp_millis() <= before.updated_at_ms
    {
        bail!("COMPUTE_PLUGIN_POLICY_BINDING_AUTHORITY_CHANGED");
    }
    let mut inventory_after = current.inventory;
    inventory_after.inventory_revision = inventory_after
        .inventory_revision
        .checked_add(1)
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_INVENTORY_REVISION_OVERFLOW"))?;
    inventory_after.desired_policy_revision = request.policy_revision;
    inventory_after.sharing_enabled = request.sharing_enabled;
    inventory_after.observed_at = bound_at.to_rfc3339_opts(SecondsFormat::Millis, true);
    for record in &mut inventory_after.plugins {
        record.desired_activation = ACTIVATION_DISABLED.to_string();
        record.admission = ADMISSION_REVOKED.to_string();
        record.health = None;
        record.state_changed_at = inventory_after.observed_at.clone();
    }
    validate_inventory(&inventory_after, bound_at.to_owned())?;
    let inventory_after_json = serde_json::to_string(&inventory_after)
        .context("COMPUTE_PLUGIN_POLICY_BINDING_INVENTORY_AFTER_JSON")?;
    let inventory_after_digest = jcs_sha256_hex(&inventory_after)?;
    let receipt = ComputePluginSharingPolicyBindingReceipt {
        schema: COMPUTE_PLUGIN_SHARING_POLICY_BINDING_RECEIPT_SCHEMA.to_string(),
        request_digest: request.request_digest.clone(),
        node_id: request.node_id.clone(),
        owner_user_id: request.owner_user_id.clone(),
        installation_id_digest: request.installation_id_digest.clone(),
        policy_revision: request.policy_revision,
        policy_digest: request.policy_digest.clone(),
        policy_snapshot_digest: request.policy_snapshot_digest.clone(),
        sharing_enabled: request.sharing_enabled,
        sharing_authorization_ref: request.sharing_authorization_ref.clone(),
        sharing_authorization_revision: request.sharing_authorization_revision,
        sharing_authorization_digest: request.sharing_authorization_digest.clone(),
        source_preparation_id: request.source_preparation_id.clone(),
        source_bootstrap_instance_id: request.source_bootstrap_instance_id.clone(),
        source_configuration_generation: request.source_configuration_generation,
        source_cancellation_generation: request.source_cancellation_generation,
        state_revision_before: before.state_revision,
        state_revision_after: checked_next(before.state_revision, "STATE")?,
        inventory_revision_before: before.inventory_revision,
        inventory_revision_after: inventory_after.inventory_revision,
        inventory_digest_before: before.inventory_digest.clone(),
        inventory_digest_after: inventory_after_digest,
        authority_epoch_before: before.authority_epoch,
        authority_epoch_after: checked_next(before.authority_epoch, "AUTHORITY_EPOCH")?,
        process_owner_epoch: before.process_owner_epoch,
        trusted_time_before_ms: before.trusted_time_high_water_ms,
        bound_at_ms: bound_at.timestamp_millis(),
    };
    let receipt_digest = jcs_sha256_hex(&receipt)?;
    Ok(ProjectedSharingPolicyBinding {
        request,
        before,
        inventory_after_json,
        hashed_receipt: HashedComputePluginSharingPolicyBindingReceipt {
            schema: HASHED_COMPUTE_PLUGIN_SHARING_POLICY_BINDING_RECEIPT_SCHEMA.to_string(),
            receipt,
            canonicalization: COMPUTE_PLUGIN_MANIFEST_CANONICALIZATION.to_string(),
            receipt_digest_algorithm: COMPUTE_PLUGIN_DIGEST_ALGORITHM.to_string(),
            receipt_digest,
        },
    })
}

pub(super) fn validate_hashed_receipt(
    hashed: &HashedComputePluginSharingPolicyBindingReceipt,
) -> Result<()> {
    let receipt = &hashed.receipt;
    if hashed.schema != HASHED_COMPUTE_PLUGIN_SHARING_POLICY_BINDING_RECEIPT_SCHEMA
        || hashed.canonicalization != COMPUTE_PLUGIN_MANIFEST_CANONICALIZATION
        || hashed.receipt_digest_algorithm != COMPUTE_PLUGIN_DIGEST_ALGORITHM
        || !is_sha256(&hashed.receipt_digest)
        || receipt.schema != COMPUTE_PLUGIN_SHARING_POLICY_BINDING_RECEIPT_SCHEMA
        || !is_sha256(&receipt.request_digest)
        || !is_sha256(&receipt.installation_id_digest)
        || !is_sha256(&receipt.policy_digest)
        || !is_sha256(&receipt.policy_snapshot_digest)
        || !bounded_identifier(&receipt.node_id)
        || !bounded_identifier(&receipt.owner_user_id)
        || !(1..=MAX_IJSON_SAFE_INTEGER_I64).contains(&receipt.policy_revision)
        || receipt.state_revision_before < 0
        || receipt.state_revision_after != checked_next(receipt.state_revision_before, "STATE")?
        || receipt.inventory_revision_before < 0
        || receipt.inventory_revision_after
            != checked_next(receipt.inventory_revision_before, "INVENTORY")?
        || !is_sha256(&receipt.inventory_digest_before)
        || !is_sha256(&receipt.inventory_digest_after)
        || receipt.inventory_digest_before == receipt.inventory_digest_after
        || receipt.authority_epoch_before < 0
        || receipt.authority_epoch_after
            != checked_next(receipt.authority_epoch_before, "AUTHORITY_EPOCH")?
        || receipt.process_owner_epoch <= 0
        || !bounded_identifier(&receipt.source_bootstrap_instance_id)
        || !(1..=MAX_IJSON_SAFE_INTEGER_U64).contains(&receipt.source_configuration_generation)
        || !(1..=MAX_IJSON_SAFE_INTEGER_U64).contains(&receipt.source_cancellation_generation)
        || receipt
            .source_preparation_id
            .as_deref()
            .is_some_and(|value| !bounded_identifier(value))
        || receipt.sharing_enabled != receipt.source_preparation_id.is_some()
        || !authorization_shape_is_valid(
            receipt.sharing_enabled,
            &receipt.sharing_authorization_ref,
            receipt.sharing_authorization_revision,
            &receipt.sharing_authorization_digest,
        )
        || (receipt.sharing_enabled
            && (receipt.sharing_authorization_revision != Some(receipt.policy_revision)
                || receipt.sharing_authorization_digest.as_deref()
                    != Some(receipt.policy_digest.as_str())))
        || receipt.trusted_time_before_ms < 0
        || receipt.bound_at_ms <= receipt.trusted_time_before_ms
        || jcs_sha256_hex(receipt)? != hashed.receipt_digest
    {
        bail!("COMPUTE_PLUGIN_POLICY_BINDING_RECEIPT_INVALID");
    }
    Ok(())
}

fn authorization_shape_is_valid(
    sharing_enabled: bool,
    reference: &Option<String>,
    revision: Option<i64>,
    digest: &Option<String>,
) -> bool {
    match (
        sharing_enabled,
        reference.as_deref(),
        revision,
        digest.as_deref(),
    ) {
        (false, None, None, None) => true,
        (true, Some(reference), Some(revision), Some(digest)) => {
            bounded_identifier(reference)
                && (1..=MAX_IJSON_SAFE_INTEGER_I64).contains(&revision)
                && is_sha256(digest)
        }
        _ => false,
    }
}

fn bounded_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_IDENTIFIER_BYTES
        && value.trim() == value
        && !value.chars().any(char::is_control)
}

fn checked_next(value: i64, label: &'static str) -> Result<i64> {
    value
        .checked_add(1)
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_POLICY_BINDING_{label}_EXHAUSTED"))
}
