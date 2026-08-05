use anyhow::{bail, Result};

use super::{
    install_plan_admission_validation::is_identifier,
    local_authority::{
        HashedComputePluginAuthorityRollbackCheckpoint,
        COMPUTE_PLUGIN_AUTHORITY_ROLLBACK_CHECKPOINT_SCHEMA,
        HASHED_COMPUTE_PLUGIN_AUTHORITY_ROLLBACK_CHECKPOINT_SCHEMA,
    },
    local_authority_schema::COMPUTE_PLUGIN_LOCAL_AUTHORITY_SCHEMA_VERSION,
    manifest_validation::is_sha256,
    plugin_manifest::{COMPUTE_PLUGIN_DIGEST_ALGORITHM, COMPUTE_PLUGIN_MANIFEST_CANONICALIZATION},
    signed_artifact_verification::jcs_sha256_hex,
};

mod attestation;
mod comparison;

pub(in crate::node_agent_compute_plugin_host) use attestation::{
    begin_rollback_anchor_challenge, verify_rollback_anchor_attestation,
    ComputePluginRollbackAnchorAttestation, ComputePluginRollbackAnchorChallenge,
    ComputePluginRollbackAnchorChallengePayload, ComputePluginRollbackAnchorChallengeRequest,
    ComputePluginRollbackAnchorKeyResolver, ComputePluginSignedRollbackAnchorAttestation,
    VerifiedComputePluginRollbackAnchor,
};
pub(in crate::node_agent_compute_plugin_host) use comparison::{
    assess_rollback_anchor, ComputePluginRollbackAnchorAssessment,
    ComputePluginRollbackAnchorPublishRequired, ComputePluginRollbackAnchorStartupPermit,
};

pub(super) fn validate_checkpoint_envelope(
    envelope: &HashedComputePluginAuthorityRollbackCheckpoint,
) -> Result<()> {
    let checkpoint = &envelope.checkpoint;
    if envelope.schema != HASHED_COMPUTE_PLUGIN_AUTHORITY_ROLLBACK_CHECKPOINT_SCHEMA
        || envelope.canonicalization != COMPUTE_PLUGIN_MANIFEST_CANONICALIZATION
        || envelope.checkpoint_digest_algorithm != COMPUTE_PLUGIN_DIGEST_ALGORITHM
        || !is_sha256(&envelope.checkpoint_digest)
        || checkpoint.schema != COMPUTE_PLUGIN_AUTHORITY_ROLLBACK_CHECKPOINT_SCHEMA
        || checkpoint.authority_schema_version != COMPUTE_PLUGIN_LOCAL_AUTHORITY_SCHEMA_VERSION
        || !is_sha256(&checkpoint.installation_id_digest)
        || checkpoint.state_revision < 0
        || checkpoint.inventory_revision < 0
        || !is_sha256(&checkpoint.inventory_digest)
        || checkpoint.desired_policy_revision < 0
        || !is_sha256(&checkpoint.node_profile_digest)
        || checkpoint.manifest_catalog_revision < 0
        || !is_identifier(&checkpoint.target_id)
        || !is_identifier(&checkpoint.host_api_protocol_id)
        || !(0..=i64::from(u32::MAX)).contains(&checkpoint.host_api_revision)
        || checkpoint.authority_epoch < 0
        || checkpoint.process_owner_epoch < 0
        || checkpoint.trusted_time_high_water_ms < 0
        || checkpoint.updated_at_ms != checkpoint.trusted_time_high_water_ms
        || !sharing_binding_is_valid(envelope)
        || !keyring_binding_is_valid(envelope)
        || jcs_sha256_hex(checkpoint)? != envelope.checkpoint_digest
    {
        bail!("COMPUTE_PLUGIN_ROLLBACK_ANCHOR_CHECKPOINT_INVALID");
    }
    Ok(())
}

pub(super) fn validate_opaque_identifier(
    code: &str,
    value: &str,
    maximum_bytes: usize,
) -> Result<()> {
    if value.is_empty()
        || value.len() > maximum_bytes
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
    {
        bail!("{code}: identifier is empty, oversized or non-canonical");
    }
    Ok(())
}

fn sharing_binding_is_valid(envelope: &HashedComputePluginAuthorityRollbackCheckpoint) -> bool {
    let checkpoint = &envelope.checkpoint;
    match (
        checkpoint.sharing_authorization_ref_digest.as_deref(),
        checkpoint.sharing_authorization_revision,
        checkpoint.sharing_authorization_digest.as_deref(),
    ) {
        (None, None, None) => !checkpoint.sharing_enabled,
        (Some(reference_digest), Some(revision), Some(authorization_digest)) => {
            is_sha256(reference_digest) && revision >= 0 && is_sha256(authorization_digest)
        }
        _ => false,
    }
}

fn keyring_binding_is_valid(envelope: &HashedComputePluginAuthorityRollbackCheckpoint) -> bool {
    let checkpoint = &envelope.checkpoint;
    match (
        checkpoint.active_bundle_revision,
        checkpoint.publisher_keyring_revision,
        checkpoint.publisher_keyring_digest.as_deref(),
        checkpoint.control_keyring_revision,
        checkpoint.control_keyring_digest.as_deref(),
    ) {
        (None, None, None, None, None) => true,
        (
            Some(bundle),
            Some(publisher_revision),
            Some(publisher_digest),
            Some(control_revision),
            Some(control_digest),
        ) => {
            bundle > 0
                && publisher_revision > 0
                && control_revision > 0
                && is_sha256(publisher_digest)
                && is_sha256(control_digest)
        }
        _ => false,
    }
}
