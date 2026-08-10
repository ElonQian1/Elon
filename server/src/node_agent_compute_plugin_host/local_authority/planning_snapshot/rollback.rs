use anyhow::{bail, Result};
use sha2::{Digest, Sha256};

use super::{custody::ComputePluginPlanningSnapshotReadCustody, meta::PlanningAuthorityRead};
use crate::node_agent_compute_plugin_host::{
    plugin_manifest::{COMPUTE_PLUGIN_DIGEST_ALGORITHM, COMPUTE_PLUGIN_MANIFEST_CANONICALIZATION},
    rollback_anchor::v2::{
        validate_checkpoint_envelope_v2, ComputePluginAuthorityRollbackCheckpointV2,
        HashedComputePluginAuthorityRollbackCheckpointV2,
        COMPUTE_PLUGIN_AUTHORITY_ROLLBACK_CHECKPOINT_V2_SCHEMA,
        HASHED_COMPUTE_PLUGIN_AUTHORITY_ROLLBACK_CHECKPOINT_V2_SCHEMA,
    },
    signed_artifact_verification::jcs_sha256_hex,
};

use super::super::manifest_catalog_binding::PlanningCatalogBinding;

const SHARING_AUTHORIZATION_REF_DOMAIN: &[u8] = b"ELON_COMPUTE_PLUGIN_SHARING_AUTHORIZATION_REF_V1";

pub(super) fn project_and_match_rollback_checkpoint(
    authority: &PlanningAuthorityRead,
    catalog: &PlanningCatalogBinding,
    custody: &ComputePluginPlanningSnapshotReadCustody<'_>,
) -> Result<HashedComputePluginAuthorityRollbackCheckpointV2> {
    let state = &authority.state;
    let sharing_authorization_ref_digest = state
        .sharing_authorization
        .as_ref()
        .map(|binding| digest_authorization_ref(&binding.authorization_ref));
    let checkpoint = ComputePluginAuthorityRollbackCheckpointV2 {
        schema: COMPUTE_PLUGIN_AUTHORITY_ROLLBACK_CHECKPOINT_V2_SCHEMA.to_string(),
        authority_schema_version: authority.schema_version,
        installation_id_digest: state.installation_id_digest.clone(),
        state_revision: state.state_revision,
        inventory_revision: state.inventory.inventory_revision,
        inventory_digest: state.inventory_digest.clone(),
        desired_policy_revision: state.desired_policy_revision,
        sharing_enabled: state.sharing_enabled,
        sharing_authorization_ref_digest,
        sharing_authorization_revision: state
            .sharing_authorization
            .as_ref()
            .map(|binding| binding.revision),
        sharing_authorization_digest: state
            .sharing_authorization
            .as_ref()
            .map(|binding| binding.digest.clone()),
        node_profile_digest: state.node_profile_digest.clone(),
        manifest_catalog_revision: state.manifest_catalog_revision,
        manifest_catalog_digest: catalog.catalog_digest().to_string(),
        manifest_catalog_binding_receipt_digest: catalog.binding_receipt_digest().to_string(),
        target_id: state.target_id.clone(),
        host_api_protocol_id: state.host_api_protocol_id.clone(),
        host_api_revision: i64::from(state.host_api_revision),
        active_bundle_revision: Some(state.keyring_bundle_revision),
        publisher_keyring_revision: Some(state.publisher_keyring.revision),
        publisher_keyring_digest: Some(state.publisher_keyring.digest.clone()),
        control_keyring_revision: Some(state.control_keyring.revision),
        control_keyring_digest: Some(state.control_keyring.digest.clone()),
        authority_epoch: state.authority_epoch,
        process_owner_epoch: state.process_owner_epoch,
        trusted_time_high_water_ms: state.trusted_time_high_water_ms,
        updated_at_ms: authority.updated_at_ms,
    };
    let checkpoint_digest = jcs_sha256_hex(&checkpoint)?;
    let envelope = HashedComputePluginAuthorityRollbackCheckpointV2 {
        schema: HASHED_COMPUTE_PLUGIN_AUTHORITY_ROLLBACK_CHECKPOINT_V2_SCHEMA.to_string(),
        checkpoint,
        canonicalization: COMPUTE_PLUGIN_MANIFEST_CANONICALIZATION.to_string(),
        checkpoint_digest_algorithm: COMPUTE_PLUGIN_DIGEST_ALGORITHM.to_string(),
        checkpoint_digest,
    };
    validate_checkpoint_envelope_v2(&envelope)?;
    if envelope.checkpoint_digest != custody.rollback_permit().checkpoint_digest() {
        bail!("COMPUTE_PLUGIN_PLANNING_ROLLBACK_CHECKPOINT_CHANGED");
    }
    Ok(envelope)
}

fn digest_authorization_ref(reference: &str) -> String {
    let mut digest = Sha256::new();
    digest.update(SHARING_AUTHORIZATION_REF_DOMAIN);
    digest.update([0]);
    digest.update(reference.as_bytes());
    hex::encode(digest.finalize())
}
