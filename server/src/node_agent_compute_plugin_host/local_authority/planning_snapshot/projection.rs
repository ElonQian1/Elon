use std::fmt;

use anyhow::{bail, Result};
use homecli_proto::{
    ComputePluginInstallPlanKeyringBindingV1, ComputePluginInstallPlanPlanningInstalledRecordV2,
    ComputePluginSharingAuthorizationBindingV1,
    MAX_COMPUTE_PLUGIN_INSTALL_PLAN_PLANNING_INSTALLED_RECORDS,
    MAX_COMPUTE_PLUGIN_INSTALL_PLAN_PLANNING_SAFE_INTEGER,
    MAX_COMPUTE_PLUGIN_INSTALL_PLAN_PLANNING_SNAPSHOT_BYTES,
};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::node_agent_compute_plugin_host::{
    install_plan_admission_validation::is_identifier, lifecycle::ComputePluginInventorySnapshot,
    local_authority_schema::COMPUTE_PLUGIN_LOCAL_AUTHORITY_SCHEMA_VERSION,
    manifest_validation::is_sha256, signed_artifact_verification::jcs_sha256_hex,
};

use super::custody::ComputePluginPlanningSnapshotReadCustody;

const PROJECTION_SCHEMA: &str = "elon.compute_plugin.planning_authority_projection.v1";

/// Full authority facts captured by A1. This material is deliberately private: it can be hashed
/// inside the projector, but it cannot be cloned or serialized by a caller and is not a wire DTO.
#[derive(Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ComputePluginPlanningAuthorityProjectionMaterial {
    schema: String,
    installation_id_digest: String,
    bootstrap_instance_id: String,
    configuration_generation: u64,
    cancellation_generation: u64,
    planning_request_digest: String,
    account_binding_digest: String,
    bootstrap_root_set_digest: String,
    authority_schema_version: u64,
    state_revision: u64,
    authority_epoch: u64,
    process_owner_epoch: u64,
    clock_epoch_digest: String,
    trusted_time_high_water_ms: u64,
    captured_at_ms: u64,
    inventory_revision: u64,
    inventory_digest: String,
    inventory: ComputePluginInventorySnapshot,
    node_id: String,
    owner_user_id: String,
    sharing_enabled: bool,
    authorization: Option<ComputePluginSharingAuthorizationBindingV1>,
    policy_revision: u64,
    policy_digest: String,
    policy_snapshot_digest: String,
    policy_binding_receipt_digest: String,
    policy_revocation_receipt_digest: String,
    policy_source_preparation_id: Option<String>,
    policy_source_bootstrap_instance_id: String,
    policy_source_configuration_generation: u64,
    policy_source_cancellation_generation: u64,
    policy_binding_authority_epoch: u64,
    policy_binding_process_owner_epoch: u64,
    policy_trusted_time_before_ms: u64,
    policy_bound_at_ms: u64,
    node_profile_digest: String,
    manifest_catalog_revision: u64,
    manifest_catalog_digest: String,
    manifest_catalog_binding_receipt_digest: String,
    keyring_bundle_revision: u64,
    publisher_keyring: ComputePluginInstallPlanKeyringBindingV1,
    control_keyring: ComputePluginInstallPlanKeyringBindingV1,
    target_id: String,
    host_api_protocol_id: String,
    host_api_revision: u32,
    rollback_anchor_id: String,
    rollback_anchor_sequence: u64,
    rollback_checkpoint_digest: String,
    rollback_attestation_digest: String,
    rollback_signing_key_fingerprint: String,
    rollback_witness_digest: String,
    installed_records: Vec<ComputePluginInstallPlanPlanningInstalledRecordV2>,
}

pub(super) struct ComputePluginPlanningAuthorityProjectionFields {
    pub(super) installation_id_digest: String,
    pub(super) bootstrap_instance_id: String,
    pub(super) configuration_generation: u64,
    pub(super) cancellation_generation: u64,
    pub(super) planning_request_digest: String,
    pub(super) account_binding_digest: String,
    pub(super) bootstrap_root_set_digest: String,
    pub(super) authority_schema_version: u64,
    pub(super) state_revision: u64,
    pub(super) authority_epoch: u64,
    pub(super) process_owner_epoch: u64,
    pub(super) clock_epoch_digest: String,
    pub(super) trusted_time_high_water_ms: u64,
    pub(super) captured_at_ms: u64,
    pub(super) inventory_revision: u64,
    pub(super) inventory_digest: String,
    pub(super) inventory: ComputePluginInventorySnapshot,
    pub(super) node_id: String,
    pub(super) owner_user_id: String,
    pub(super) sharing_enabled: bool,
    pub(super) authorization: Option<ComputePluginSharingAuthorizationBindingV1>,
    pub(super) policy_revision: u64,
    pub(super) policy_digest: String,
    pub(super) policy_snapshot_digest: String,
    pub(super) policy_binding_receipt_digest: String,
    pub(super) policy_revocation_receipt_digest: String,
    pub(super) policy_source_preparation_id: Option<String>,
    pub(super) policy_source_bootstrap_instance_id: String,
    pub(super) policy_source_configuration_generation: u64,
    pub(super) policy_source_cancellation_generation: u64,
    pub(super) policy_binding_authority_epoch: u64,
    pub(super) policy_binding_process_owner_epoch: u64,
    pub(super) policy_trusted_time_before_ms: u64,
    pub(super) policy_bound_at_ms: u64,
    pub(super) node_profile_digest: String,
    pub(super) manifest_catalog_revision: u64,
    pub(super) manifest_catalog_digest: String,
    pub(super) manifest_catalog_binding_receipt_digest: String,
    pub(super) keyring_bundle_revision: u64,
    pub(super) publisher_keyring: ComputePluginInstallPlanKeyringBindingV1,
    pub(super) control_keyring: ComputePluginInstallPlanKeyringBindingV1,
    pub(super) target_id: String,
    pub(super) host_api_protocol_id: String,
    pub(super) host_api_revision: u32,
    pub(super) rollback_anchor_id: String,
    pub(super) rollback_anchor_sequence: u64,
    pub(super) rollback_checkpoint_digest: String,
    pub(super) rollback_attestation_digest: String,
    pub(super) rollback_signing_key_fingerprint: String,
    pub(super) rollback_witness_digest: String,
    pub(super) installed_records: Vec<ComputePluginInstallPlanPlanningInstalledRecordV2>,
}

/// Linear A1 output. No field accessor exposes serializable snapshot material; a future v15
/// producer must be implemented inside this sealed module and consume the complete value.
#[must_use = "a coherent projection is evidence, not a planning or execution capability"]
pub(in crate::node_agent_compute_plugin_host) struct ComputePluginPlanningAuthorityProjection<'a> {
    material: ComputePluginPlanningAuthorityProjectionMaterial,
    projection_digest: String,
    _opened: &'a super::super::OpenedComputePluginLocalAuthority,
    _custody: ComputePluginPlanningSnapshotReadCustody<'a>,
}

pub(super) struct PreparedComputePluginPlanningAuthorityProjection {
    material: ComputePluginPlanningAuthorityProjectionMaterial,
    projection_digest: String,
}

impl PreparedComputePluginPlanningAuthorityProjection {
    pub(super) fn seal(fields: ComputePluginPlanningAuthorityProjectionFields) -> Result<Self> {
        validate_fields(&fields)?;
        let material = ComputePluginPlanningAuthorityProjectionMaterial {
            schema: PROJECTION_SCHEMA.to_string(),
            installation_id_digest: fields.installation_id_digest,
            bootstrap_instance_id: fields.bootstrap_instance_id,
            configuration_generation: fields.configuration_generation,
            cancellation_generation: fields.cancellation_generation,
            planning_request_digest: fields.planning_request_digest,
            account_binding_digest: fields.account_binding_digest,
            bootstrap_root_set_digest: fields.bootstrap_root_set_digest,
            authority_schema_version: fields.authority_schema_version,
            state_revision: fields.state_revision,
            authority_epoch: fields.authority_epoch,
            process_owner_epoch: fields.process_owner_epoch,
            clock_epoch_digest: fields.clock_epoch_digest,
            trusted_time_high_water_ms: fields.trusted_time_high_water_ms,
            captured_at_ms: fields.captured_at_ms,
            inventory_revision: fields.inventory_revision,
            inventory_digest: fields.inventory_digest,
            inventory: fields.inventory,
            node_id: fields.node_id,
            owner_user_id: fields.owner_user_id,
            sharing_enabled: fields.sharing_enabled,
            authorization: fields.authorization,
            policy_revision: fields.policy_revision,
            policy_digest: fields.policy_digest,
            policy_snapshot_digest: fields.policy_snapshot_digest,
            policy_binding_receipt_digest: fields.policy_binding_receipt_digest,
            policy_revocation_receipt_digest: fields.policy_revocation_receipt_digest,
            policy_source_preparation_id: fields.policy_source_preparation_id,
            policy_source_bootstrap_instance_id: fields.policy_source_bootstrap_instance_id,
            policy_source_configuration_generation: fields.policy_source_configuration_generation,
            policy_source_cancellation_generation: fields.policy_source_cancellation_generation,
            policy_binding_authority_epoch: fields.policy_binding_authority_epoch,
            policy_binding_process_owner_epoch: fields.policy_binding_process_owner_epoch,
            policy_trusted_time_before_ms: fields.policy_trusted_time_before_ms,
            policy_bound_at_ms: fields.policy_bound_at_ms,
            node_profile_digest: fields.node_profile_digest,
            manifest_catalog_revision: fields.manifest_catalog_revision,
            manifest_catalog_digest: fields.manifest_catalog_digest,
            manifest_catalog_binding_receipt_digest: fields.manifest_catalog_binding_receipt_digest,
            keyring_bundle_revision: fields.keyring_bundle_revision,
            publisher_keyring: fields.publisher_keyring,
            control_keyring: fields.control_keyring,
            target_id: fields.target_id,
            host_api_protocol_id: fields.host_api_protocol_id,
            host_api_revision: fields.host_api_revision,
            rollback_anchor_id: fields.rollback_anchor_id,
            rollback_anchor_sequence: fields.rollback_anchor_sequence,
            rollback_checkpoint_digest: fields.rollback_checkpoint_digest,
            rollback_attestation_digest: fields.rollback_attestation_digest,
            rollback_signing_key_fingerprint: fields.rollback_signing_key_fingerprint,
            rollback_witness_digest: fields.rollback_witness_digest,
            installed_records: fields.installed_records,
        };
        let canonical_size = serde_json::to_vec(&material)?.len();
        if canonical_size > MAX_COMPUTE_PLUGIN_INSTALL_PLAN_PLANNING_SNAPSHOT_BYTES {
            bail!("COMPUTE_PLUGIN_PLANNING_AUTHORITY_PROJECTION_TOO_LARGE");
        }
        let projection_digest = jcs_sha256_hex(&material)?;
        Ok(Self {
            material,
            projection_digest,
        })
    }

    pub(super) fn bind_custody<'a>(
        self,
        custody: ComputePluginPlanningSnapshotReadCustody<'a>,
        opened: &'a super::super::OpenedComputePluginLocalAuthority,
    ) -> ComputePluginPlanningAuthorityProjection<'a> {
        ComputePluginPlanningAuthorityProjection {
            material: self.material,
            projection_digest: self.projection_digest,
            _opened: opened,
            _custody: custody,
        }
    }
}

impl ComputePluginPlanningAuthorityProjection<'_> {
    pub(in crate::node_agent_compute_plugin_host) fn projection_digest(&self) -> &str {
        &self.projection_digest
    }

    pub(in crate::node_agent_compute_plugin_host) fn installed_record_count(&self) -> usize {
        self.material.installed_records.len()
    }
}

impl fmt::Debug for ComputePluginPlanningAuthorityProjection<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ComputePluginPlanningAuthorityProjection")
            .field("installation_id_digest", &"<redacted>")
            .field("projection_digest", &"<redacted>")
            .field("state_revision", &self.material.state_revision)
            .field("inventory_revision", &self.material.inventory_revision)
            .field("installed_records", &self.material.installed_records.len())
            .finish()
    }
}

/// Stable, redacted reason why A1 could not seal a complete projection. No partial material is
/// retained, so a blocked result cannot be resumed or upgraded into ready authority.
pub(in crate::node_agent_compute_plugin_host) struct ComputePluginPlanningAuthorityProjectionBlocked
{
    code: &'static str,
    diagnostic_digest: String,
}

impl ComputePluginPlanningAuthorityProjectionBlocked {
    pub(super) fn from_error(code: &'static str, error: &anyhow::Error) -> Self {
        let mut digest = Sha256::new();
        digest.update(error.to_string().as_bytes());
        Self {
            code,
            diagnostic_digest: hex::encode(digest.finalize()),
        }
    }

    pub(in crate::node_agent_compute_plugin_host) fn code(&self) -> &'static str {
        self.code
    }

    pub(in crate::node_agent_compute_plugin_host) fn diagnostic_digest(&self) -> &str {
        &self.diagnostic_digest
    }
}

impl fmt::Debug for ComputePluginPlanningAuthorityProjectionBlocked {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ComputePluginPlanningAuthorityProjectionBlocked")
            .field("code", &self.code)
            .field("diagnostic_digest", &"<redacted>")
            .finish()
    }
}

pub(in crate::node_agent_compute_plugin_host) enum ComputePluginPlanningAuthorityProjectionOutcome<
    'a,
> {
    Projected(ComputePluginPlanningAuthorityProjection<'a>),
    Blocked(ComputePluginPlanningAuthorityProjectionBlocked),
}

fn validate_fields(fields: &ComputePluginPlanningAuthorityProjectionFields) -> Result<()> {
    let records_are_sorted = fields
        .installed_records
        .windows(2)
        .all(|pair| pair[0].plugin_id < pair[1].plugin_id);
    let integer_facts = [
        fields.configuration_generation,
        fields.cancellation_generation,
        fields.authority_schema_version,
        fields.state_revision,
        fields.authority_epoch,
        fields.process_owner_epoch,
        fields.trusted_time_high_water_ms,
        fields.captured_at_ms,
        fields.inventory_revision,
        fields.policy_revision,
        fields.policy_source_configuration_generation,
        fields.policy_source_cancellation_generation,
        fields.policy_binding_authority_epoch,
        fields.policy_binding_process_owner_epoch,
        fields.policy_trusted_time_before_ms,
        fields.policy_bound_at_ms,
        fields.manifest_catalog_revision,
        fields.keyring_bundle_revision,
        fields.publisher_keyring.revision,
        fields.control_keyring.revision,
        fields.rollback_anchor_sequence,
    ];
    let authorization_matches = fields.authorization.as_ref().is_some_and(|authorization| {
        authorization.revision == fields.policy_revision
            && authorization.digest == fields.policy_digest
    });
    let inventory_matches = u64::try_from(fields.inventory.inventory_revision).ok()
        == Some(fields.inventory_revision)
        && fields.inventory.plugins.len() == fields.installed_records.len()
        && jcs_sha256_hex(&fields.inventory)? == fields.inventory_digest
        && fields
            .inventory
            .plugins
            .iter()
            .zip(&fields.installed_records)
            .all(|(source, projected)| {
                source.plugin_id == projected.plugin_id
                    && u64::try_from(source.install_generation).ok()
                        == Some(projected.install_generation)
                    && source.active_slot_ref == projected.active_slot_ref
                    && source.candidate_slot_ref == projected.candidate_slot_ref
                    && source.desired_presence == projected.desired_presence
                    && source.desired_activation == projected.desired_activation
                    && source.admission == projected.admission
                    && source.runtime.phase == projected.runtime_phase
                    && u64::try_from(source.runtime.runtime_generation).ok()
                        == Some(projected.runtime_generation)
                    && u64::try_from(source.active_attempts).ok() == Some(projected.active_attempts)
                    && source.permission_grant_digest == projected.permission_grant_digest
            });
    if !is_sha256(&fields.installation_id_digest)
        || !is_identifier(&fields.bootstrap_instance_id)
        || !(1..=MAX_COMPUTE_PLUGIN_INSTALL_PLAN_PLANNING_SAFE_INTEGER)
            .contains(&fields.configuration_generation)
        || !(1..=MAX_COMPUTE_PLUGIN_INSTALL_PLAN_PLANNING_SAFE_INTEGER)
            .contains(&fields.cancellation_generation)
        || !is_sha256(&fields.planning_request_digest)
        || !is_sha256(&fields.account_binding_digest)
        || !is_sha256(&fields.bootstrap_root_set_digest)
        || i64::try_from(fields.authority_schema_version).ok()
            != Some(COMPUTE_PLUGIN_LOCAL_AUTHORITY_SCHEMA_VERSION)
        || fields.authority_epoch == 0
        || integer_facts
            .into_iter()
            .any(|value| value > MAX_COMPUTE_PLUGIN_INSTALL_PLAN_PLANNING_SAFE_INTEGER)
        || fields.process_owner_epoch == 0
        || fields.trusted_time_high_water_ms == 0
        || fields.captured_at_ms <= fields.trusted_time_high_water_ms
        || !is_sha256(&fields.clock_epoch_digest)
        || !is_sha256(&fields.inventory_digest)
        || !inventory_matches
        || !is_identifier(&fields.node_id)
        || !is_identifier(&fields.owner_user_id)
        || fields.policy_revision == 0
        || !is_sha256(&fields.policy_digest)
        || !is_sha256(&fields.policy_snapshot_digest)
        || !is_sha256(&fields.policy_binding_receipt_digest)
        || !is_sha256(&fields.policy_revocation_receipt_digest)
        || !fields.sharing_enabled
        || !authorization_matches
        || fields.policy_source_configuration_generation == 0
        || fields.policy_source_cancellation_generation == 0
        || fields.policy_source_configuration_generation != fields.configuration_generation
        || fields.policy_source_cancellation_generation != fields.cancellation_generation
        || fields.policy_binding_authority_epoch == 0
        || fields.policy_binding_process_owner_epoch == 0
        || fields.policy_binding_authority_epoch > fields.authority_epoch
        || fields.policy_binding_process_owner_epoch > fields.process_owner_epoch
        || fields.policy_trusted_time_before_ms > fields.policy_bound_at_ms
        || fields.policy_bound_at_ms > fields.trusted_time_high_water_ms
        || fields
            .policy_source_preparation_id
            .as_deref()
            .is_none_or(|value| !is_identifier(value))
        || !is_identifier(&fields.policy_source_bootstrap_instance_id)
        || !is_sha256(&fields.node_profile_digest)
        || fields.manifest_catalog_revision == 0
        || !is_sha256(&fields.manifest_catalog_digest)
        || !is_sha256(&fields.manifest_catalog_binding_receipt_digest)
        || fields.keyring_bundle_revision == 0
        || fields.publisher_keyring.revision == 0
        || fields.control_keyring.revision == 0
        || !is_sha256(&fields.publisher_keyring.digest)
        || !is_sha256(&fields.control_keyring.digest)
        || fields.publisher_keyring == fields.control_keyring
        || !is_identifier(&fields.target_id)
        || !is_identifier(&fields.host_api_protocol_id)
        || fields.host_api_revision == 0
        || !is_identifier(&fields.rollback_anchor_id)
        || fields.rollback_anchor_sequence == 0
        || !is_sha256(&fields.rollback_checkpoint_digest)
        || !is_sha256(&fields.rollback_attestation_digest)
        || !is_sha256(&fields.rollback_signing_key_fingerprint)
        || !is_sha256(&fields.rollback_witness_digest)
        || fields.installed_records.len()
            > MAX_COMPUTE_PLUGIN_INSTALL_PLAN_PLANNING_INSTALLED_RECORDS
        || !records_are_sorted
    {
        bail!("COMPUTE_PLUGIN_PLANNING_AUTHORITY_PROJECTION_INVALID");
    }
    Ok(())
}
