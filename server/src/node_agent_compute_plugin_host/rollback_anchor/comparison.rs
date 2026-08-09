use std::{fmt, time::Instant};

use anyhow::{bail, Result};

use super::{
    v2::{
        build_startup_witness_v2, validate_checkpoint_envelope_v2,
        ComputePluginAuthorityRollbackCheckpointV2,
        HashedComputePluginAuthorityRollbackCheckpointV2,
        HashedComputePluginRollbackAnchorStartupWitnessV2, VerifiedComputePluginRollbackAnchorV2,
    },
    validate_checkpoint_envelope, VerifiedComputePluginRollbackAnchor,
};
use crate::node_agent_compute_plugin_host::local_authority::{
    ComputePluginAuthorityRollbackCheckpoint,
    GuardedLocalComputePluginAuthorityRollbackCheckpointV2,
    HashedComputePluginAuthorityRollbackCheckpoint,
};

#[derive(Debug)]
pub(in crate::node_agent_compute_plugin_host) enum ComputePluginRollbackAnchorAssessment {
    StartupPermitted(ComputePluginRollbackAnchorStartupPermit),
    PublishRequired(ComputePluginRollbackAnchorPublishRequired),
}

/// V2 never accepts a V1 verified anchor or returns a V1 startup permit. A caller that requires
/// catalog-aware rollback evidence must name this type explicitly.
#[derive(Debug)]
pub(in crate::node_agent_compute_plugin_host) enum ComputePluginRollbackAnchorAssessmentV2 {
    StartupPermitted(ComputePluginRollbackAnchorStartupPermitV2),
    PublishRequired(ComputePluginRollbackAnchorPublishRequiredV2),
}

pub(in crate::node_agent_compute_plugin_host) struct ComputePluginRollbackAnchorStartupPermit {
    anchor_id: String,
    anchor_sequence: i64,
    checkpoint_digest: String,
    attestation_digest: String,
    signing_key_fingerprint: String,
    verified_at: Instant,
}

pub(in crate::node_agent_compute_plugin_host) struct ComputePluginRollbackAnchorStartupPermitV2 {
    witness: HashedComputePluginRollbackAnchorStartupWitnessV2,
    verified_at: Instant,
}

pub(in crate::node_agent_compute_plugin_host) struct ComputePluginRollbackAnchorPublishRequired {
    anchor_id: String,
    anchor_sequence: i64,
    anchored_checkpoint_digest: String,
    anchored_high_water_ms: i64,
    local_high_water_ms: i64,
    local_checkpoint: HashedComputePluginAuthorityRollbackCheckpoint,
    verified_at: Instant,
}

/// No V2 publication wire exists yet. This value deliberately has no conversion to the V1
/// publication payload, preventing a V2 checkpoint from being sent under a V1 schema or domain.
pub(in crate::node_agent_compute_plugin_host) struct ComputePluginRollbackAnchorPublishRequiredV2 {
    anchor_id: String,
    anchor_sequence: i64,
    anchored_checkpoint_digest: String,
    anchored_high_water_ms: i64,
    local_high_water_ms: i64,
    local_checkpoint: HashedComputePluginAuthorityRollbackCheckpointV2,
    verified_at: Instant,
}

pub(super) struct ComputePluginRollbackAnchorPublicationParts {
    pub(super) anchor_id: String,
    pub(super) anchor_sequence: i64,
    pub(super) anchored_checkpoint_digest: String,
    pub(super) local_checkpoint: HashedComputePluginAuthorityRollbackCheckpoint,
}

impl ComputePluginRollbackAnchorPublishRequired {
    pub(super) fn into_publication_parts(self) -> ComputePluginRollbackAnchorPublicationParts {
        ComputePluginRollbackAnchorPublicationParts {
            anchor_id: self.anchor_id,
            anchor_sequence: self.anchor_sequence,
            anchored_checkpoint_digest: self.anchored_checkpoint_digest,
            local_checkpoint: self.local_checkpoint,
        }
    }
}

impl ComputePluginRollbackAnchorStartupPermitV2 {
    pub(in crate::node_agent_compute_plugin_host) fn anchor_id(&self) -> &str {
        &self.witness.witness.anchor_id
    }

    pub(in crate::node_agent_compute_plugin_host) fn anchor_sequence(&self) -> i64 {
        self.witness.witness.anchor_sequence
    }

    pub(in crate::node_agent_compute_plugin_host) fn checkpoint_digest(&self) -> &str {
        &self.witness.witness.checkpoint_digest
    }

    pub(in crate::node_agent_compute_plugin_host) fn attestation_digest(&self) -> &str {
        &self.witness.witness.attestation_digest
    }

    pub(in crate::node_agent_compute_plugin_host) fn signing_key_fingerprint(&self) -> &str {
        &self.witness.witness.signing_key_fingerprint
    }

    pub(in crate::node_agent_compute_plugin_host) fn witness_digest(&self) -> &str {
        &self.witness.witness_digest
    }

    pub(in crate::node_agent_compute_plugin_host) fn verified_at(&self) -> Instant {
        self.verified_at
    }
}

impl fmt::Debug for ComputePluginRollbackAnchorStartupPermit {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ComputePluginRollbackAnchorStartupPermit")
            .field("anchor_id", &self.anchor_id)
            .field("anchor_sequence", &self.anchor_sequence)
            .field("checkpoint_digest", &"<redacted>")
            .field("attestation_digest", &"<redacted>")
            .field("signing_key_fingerprint", &"<redacted>")
            .field("verified_at", &"<monotonic>")
            .finish()
    }
}

impl fmt::Debug for ComputePluginRollbackAnchorStartupPermitV2 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ComputePluginRollbackAnchorStartupPermitV2")
            .field("anchor_id", &self.witness.witness.anchor_id)
            .field("anchor_sequence", &self.witness.witness.anchor_sequence)
            .field("checkpoint_digest", &"<redacted>")
            .field("attestation_digest", &"<redacted>")
            .field("signing_key_fingerprint", &"<redacted>")
            .field("witness_digest", &"<redacted>")
            .field("verified_at", &"<monotonic>")
            .finish()
    }
}

impl fmt::Debug for ComputePluginRollbackAnchorPublishRequired {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ComputePluginRollbackAnchorPublishRequired")
            .field("anchor_id", &self.anchor_id)
            .field("anchor_sequence", &self.anchor_sequence)
            .field("anchored_checkpoint_digest", &"<redacted>")
            .field("anchored_high_water_ms", &self.anchored_high_water_ms)
            .field("local_high_water_ms", &self.local_high_water_ms)
            .field("local_checkpoint_digest", &"<redacted>")
            .field("verified_at", &"<monotonic>")
            .finish()
    }
}

impl fmt::Debug for ComputePluginRollbackAnchorPublishRequiredV2 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ComputePluginRollbackAnchorPublishRequiredV2")
            .field("anchor_id", &self.anchor_id)
            .field("anchor_sequence", &self.anchor_sequence)
            .field("anchored_checkpoint_digest", &"<redacted>")
            .field("anchored_high_water_ms", &self.anchored_high_water_ms)
            .field("local_high_water_ms", &self.local_high_water_ms)
            .field("local_checkpoint_digest", &"<redacted>")
            .field("verified_at", &"<monotonic>")
            .finish()
    }
}

pub(in crate::node_agent_compute_plugin_host) fn assess_rollback_anchor(
    local: HashedComputePluginAuthorityRollbackCheckpoint,
    verified: VerifiedComputePluginRollbackAnchor,
) -> Result<ComputePluginRollbackAnchorAssessment> {
    validate_checkpoint_envelope(&local)?;
    let anchored = &verified.attestation().checkpoint;
    validate_checkpoint_envelope(anchored)?;
    validate_same_authority(&local.checkpoint, &anchored.checkpoint)?;

    let local_high_water = local.checkpoint.trusted_time_high_water_ms;
    let anchored_high_water = anchored.checkpoint.trusted_time_high_water_ms;
    if local_high_water < anchored_high_water {
        bail!("COMPUTE_PLUGIN_ROLLBACK_ANCHOR_LOCAL_BEHIND");
    }
    if local_high_water == anchored_high_water {
        if local.checkpoint_digest != anchored.checkpoint_digest {
            bail!("COMPUTE_PLUGIN_ROLLBACK_ANCHOR_SAME_TIME_DIVERGENCE");
        }
        return Ok(ComputePluginRollbackAnchorAssessment::StartupPermitted(
            ComputePluginRollbackAnchorStartupPermit {
                anchor_id: verified.attestation().anchor_id.clone(),
                anchor_sequence: verified.attestation().anchor_sequence,
                checkpoint_digest: local.checkpoint_digest,
                attestation_digest: verified.attestation_digest().to_string(),
                signing_key_fingerprint: verified.signing_key_fingerprint().to_string(),
                verified_at: verified.verified_at(),
            },
        ));
    }

    validate_local_not_behind_monotonic_facts(&local.checkpoint, &anchored.checkpoint)?;
    Ok(ComputePluginRollbackAnchorAssessment::PublishRequired(
        ComputePluginRollbackAnchorPublishRequired {
            anchor_id: verified.attestation().anchor_id.clone(),
            anchor_sequence: verified.attestation().anchor_sequence,
            anchored_checkpoint_digest: anchored.checkpoint_digest.clone(),
            anchored_high_water_ms: anchored_high_water,
            local_high_water_ms: local_high_water,
            local_checkpoint: local,
            verified_at: verified.verified_at(),
        },
    ))
}

pub(in crate::node_agent_compute_plugin_host) fn assess_rollback_anchor_v2(
    local: GuardedLocalComputePluginAuthorityRollbackCheckpointV2,
    verified: VerifiedComputePluginRollbackAnchorV2,
) -> Result<ComputePluginRollbackAnchorAssessmentV2> {
    let local = local.into_checkpoint();
    validate_checkpoint_envelope_v2(&local)?;
    let anchored = &verified.attestation().checkpoint;
    validate_checkpoint_envelope_v2(anchored)?;
    validate_same_authority_v2(&local.checkpoint, &anchored.checkpoint)?;

    let local_high_water = local.checkpoint.trusted_time_high_water_ms;
    let anchored_high_water = anchored.checkpoint.trusted_time_high_water_ms;
    if local_high_water < anchored_high_water {
        bail!("COMPUTE_PLUGIN_ROLLBACK_ANCHOR_V2_LOCAL_BEHIND");
    }
    if local_high_water == anchored_high_water {
        if local.checkpoint_digest != anchored.checkpoint_digest {
            bail!("COMPUTE_PLUGIN_ROLLBACK_ANCHOR_V2_SAME_TIME_DIVERGENCE");
        }
        let witness = build_startup_witness_v2(
            verified.attestation().anchor_id.clone(),
            verified.attestation().anchor_sequence,
            local.checkpoint_digest,
            verified.attestation_digest().to_string(),
            verified.signing_key_fingerprint().to_string(),
        )?;
        return Ok(ComputePluginRollbackAnchorAssessmentV2::StartupPermitted(
            ComputePluginRollbackAnchorStartupPermitV2 {
                witness,
                verified_at: verified.verified_at(),
            },
        ));
    }

    validate_local_not_behind_monotonic_facts_v2(&local.checkpoint, &anchored.checkpoint)?;
    Ok(ComputePluginRollbackAnchorAssessmentV2::PublishRequired(
        ComputePluginRollbackAnchorPublishRequiredV2 {
            anchor_id: verified.attestation().anchor_id.clone(),
            anchor_sequence: verified.attestation().anchor_sequence,
            anchored_checkpoint_digest: anchored.checkpoint_digest.clone(),
            anchored_high_water_ms: anchored_high_water,
            local_high_water_ms: local_high_water,
            local_checkpoint: local,
            verified_at: verified.verified_at(),
        },
    ))
}

fn validate_same_authority_v2(
    local: &ComputePluginAuthorityRollbackCheckpointV2,
    anchored: &ComputePluginAuthorityRollbackCheckpointV2,
) -> Result<()> {
    if local.installation_id_digest != anchored.installation_id_digest
        || local.authority_schema_version != anchored.authority_schema_version
        || local.target_id != anchored.target_id
        || local.host_api_protocol_id != anchored.host_api_protocol_id
        || local.host_api_revision != anchored.host_api_revision
    {
        bail!("COMPUTE_PLUGIN_ROLLBACK_ANCHOR_V2_AUTHORITY_MISMATCH");
    }
    Ok(())
}

fn validate_local_not_behind_monotonic_facts_v2(
    local: &ComputePluginAuthorityRollbackCheckpointV2,
    anchored: &ComputePluginAuthorityRollbackCheckpointV2,
) -> Result<()> {
    if local.manifest_catalog_revision == anchored.manifest_catalog_revision
        && (local.manifest_catalog_digest != anchored.manifest_catalog_digest
            || local.manifest_catalog_binding_receipt_digest
                != anchored.manifest_catalog_binding_receipt_digest)
    {
        bail!("COMPUTE_PLUGIN_ROLLBACK_ANCHOR_V2_CATALOG_REVISION_FORK");
    }
    if local.state_revision < anchored.state_revision
        || local.inventory_revision < anchored.inventory_revision
        || local.desired_policy_revision < anchored.desired_policy_revision
        || local.manifest_catalog_revision < anchored.manifest_catalog_revision
        || local.authority_epoch < anchored.authority_epoch
        || local.process_owner_epoch < anchored.process_owner_epoch
        || (local.inventory_revision == anchored.inventory_revision
            && local.inventory_digest != anchored.inventory_digest)
        || (local.state_revision == anchored.state_revision
            && !same_state_binding_v2(local, anchored))
        || !keyring_is_not_behind_v2(local, anchored)
    {
        bail!("COMPUTE_PLUGIN_ROLLBACK_ANCHOR_V2_MONOTONIC_FACT_ROLLBACK");
    }
    Ok(())
}

fn same_state_binding_v2(
    local: &ComputePluginAuthorityRollbackCheckpointV2,
    anchored: &ComputePluginAuthorityRollbackCheckpointV2,
) -> bool {
    local.inventory_revision == anchored.inventory_revision
        && local.inventory_digest == anchored.inventory_digest
        && local.desired_policy_revision == anchored.desired_policy_revision
        && local.sharing_enabled == anchored.sharing_enabled
        && local.sharing_authorization_ref_digest == anchored.sharing_authorization_ref_digest
        && local.sharing_authorization_revision == anchored.sharing_authorization_revision
        && local.sharing_authorization_digest == anchored.sharing_authorization_digest
        && local.node_profile_digest == anchored.node_profile_digest
        && local.manifest_catalog_revision == anchored.manifest_catalog_revision
        && local.manifest_catalog_digest == anchored.manifest_catalog_digest
        && local.manifest_catalog_binding_receipt_digest
            == anchored.manifest_catalog_binding_receipt_digest
        && local.active_bundle_revision == anchored.active_bundle_revision
        && local.publisher_keyring_revision == anchored.publisher_keyring_revision
        && local.publisher_keyring_digest == anchored.publisher_keyring_digest
        && local.control_keyring_revision == anchored.control_keyring_revision
        && local.control_keyring_digest == anchored.control_keyring_digest
        && local.authority_epoch == anchored.authority_epoch
        && local.process_owner_epoch == anchored.process_owner_epoch
}

fn keyring_is_not_behind_v2(
    local: &ComputePluginAuthorityRollbackCheckpointV2,
    anchored: &ComputePluginAuthorityRollbackCheckpointV2,
) -> bool {
    let Some(anchored_bundle) = anchored.active_bundle_revision else {
        return true;
    };
    let (
        Some(local_bundle),
        Some(local_publisher_revision),
        Some(local_control_revision),
        Some(anchored_publisher_revision),
        Some(anchored_control_revision),
    ) = (
        local.active_bundle_revision,
        local.publisher_keyring_revision,
        local.control_keyring_revision,
        anchored.publisher_keyring_revision,
        anchored.control_keyring_revision,
    )
    else {
        return false;
    };
    local_bundle >= anchored_bundle
        && local_publisher_revision >= anchored_publisher_revision
        && local_control_revision >= anchored_control_revision
        && (local_publisher_revision != anchored_publisher_revision
            || local.publisher_keyring_digest == anchored.publisher_keyring_digest)
        && (local_control_revision != anchored_control_revision
            || local.control_keyring_digest == anchored.control_keyring_digest)
}

fn validate_same_authority(
    local: &ComputePluginAuthorityRollbackCheckpoint,
    anchored: &ComputePluginAuthorityRollbackCheckpoint,
) -> Result<()> {
    if local.installation_id_digest != anchored.installation_id_digest
        || local.authority_schema_version != anchored.authority_schema_version
        || local.target_id != anchored.target_id
        || local.host_api_protocol_id != anchored.host_api_protocol_id
        || local.host_api_revision != anchored.host_api_revision
    {
        bail!("COMPUTE_PLUGIN_ROLLBACK_ANCHOR_AUTHORITY_MISMATCH");
    }
    Ok(())
}

fn validate_local_not_behind_monotonic_facts(
    local: &ComputePluginAuthorityRollbackCheckpoint,
    anchored: &ComputePluginAuthorityRollbackCheckpoint,
) -> Result<()> {
    if local.state_revision < anchored.state_revision
        || local.inventory_revision < anchored.inventory_revision
        || local.desired_policy_revision < anchored.desired_policy_revision
        || local.manifest_catalog_revision < anchored.manifest_catalog_revision
        || local.authority_epoch < anchored.authority_epoch
        || local.process_owner_epoch < anchored.process_owner_epoch
        || (local.inventory_revision == anchored.inventory_revision
            && local.inventory_digest != anchored.inventory_digest)
        || (local.state_revision == anchored.state_revision && !same_state_binding(local, anchored))
        || !keyring_is_not_behind(local, anchored)
    {
        bail!("COMPUTE_PLUGIN_ROLLBACK_ANCHOR_MONOTONIC_FACT_ROLLBACK");
    }
    Ok(())
}

fn same_state_binding(
    local: &ComputePluginAuthorityRollbackCheckpoint,
    anchored: &ComputePluginAuthorityRollbackCheckpoint,
) -> bool {
    local.inventory_revision == anchored.inventory_revision
        && local.inventory_digest == anchored.inventory_digest
        && local.desired_policy_revision == anchored.desired_policy_revision
        && local.sharing_enabled == anchored.sharing_enabled
        && local.sharing_authorization_ref_digest == anchored.sharing_authorization_ref_digest
        && local.sharing_authorization_revision == anchored.sharing_authorization_revision
        && local.sharing_authorization_digest == anchored.sharing_authorization_digest
        && local.node_profile_digest == anchored.node_profile_digest
        && local.manifest_catalog_revision == anchored.manifest_catalog_revision
        && local.active_bundle_revision == anchored.active_bundle_revision
        && local.publisher_keyring_revision == anchored.publisher_keyring_revision
        && local.publisher_keyring_digest == anchored.publisher_keyring_digest
        && local.control_keyring_revision == anchored.control_keyring_revision
        && local.control_keyring_digest == anchored.control_keyring_digest
        && local.authority_epoch == anchored.authority_epoch
        && local.process_owner_epoch == anchored.process_owner_epoch
}

fn keyring_is_not_behind(
    local: &ComputePluginAuthorityRollbackCheckpoint,
    anchored: &ComputePluginAuthorityRollbackCheckpoint,
) -> bool {
    let Some(anchored_bundle) = anchored.active_bundle_revision else {
        return true;
    };
    let (
        Some(local_bundle),
        Some(local_publisher_revision),
        Some(local_control_revision),
        Some(anchored_publisher_revision),
        Some(anchored_control_revision),
    ) = (
        local.active_bundle_revision,
        local.publisher_keyring_revision,
        local.control_keyring_revision,
        anchored.publisher_keyring_revision,
        anchored.control_keyring_revision,
    )
    else {
        return false;
    };
    local_bundle >= anchored_bundle
        && local_publisher_revision >= anchored_publisher_revision
        && local_control_revision >= anchored_control_revision
        && (local_publisher_revision != anchored_publisher_revision
            || local.publisher_keyring_digest == anchored.publisher_keyring_digest)
        && (local_control_revision != anchored_control_revision
            || local.control_keyring_digest == anchored.control_keyring_digest)
}
