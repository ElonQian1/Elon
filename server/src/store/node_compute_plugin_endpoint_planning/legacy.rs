use anyhow::{bail, Result};

use super::super::{
    node_compute_plugin_install_plan_preparation::NodeComputePluginInstallPlanPreparationDispatchIntent,
    node_compute_plugin_sharing::NodeComputePluginSharingDispatchIntent,
    node_credentials::NodeEndpointSessionPermit,
};

const MAX_OBSERVATION_BYTES: usize = 512 * 1024;

pub(super) fn sharing_snapshot(
    permit: &NodeEndpointSessionPermit,
    sharing: &NodeComputePluginSharingDispatchIntent,
) -> Result<(homecli_proto::ComputePluginSharingPolicySnapshotV1, String)> {
    let expected_plugin_installation_digest = crate::compute_plugin_sharing_directive::
        derive_compute_plugin_installation_identity_digest(permit.install_id())
        .map_err(|error| anyhow::anyhow!(error.code()))?;
    if sharing.node_id != permit.binding().agent_id()
        || sharing.owner_user_id != permit.owner_user_id()
        || sharing.installation_identity_digest != expected_plugin_installation_digest
        || !sharing.dispatchable
    {
        bail!("NODE_ENDPOINT_PLANNING_SHARING_SOURCE_MISMATCH");
    }
    let authorization = match (
        sharing.plugin_runtime_requested,
        sharing.authorization.as_ref(),
    ) {
        (true, Some(authorization)) => {
            Some(homecli_proto::ComputePluginSharingAuthorizationBindingV1 {
                authorization_ref: authorization.authorization_ref.clone(),
                revision: u64::try_from(authorization.revision)?,
                digest: authorization.digest.clone(),
            })
        }
        (false, None) => None,
        _ => bail!("NODE_ENDPOINT_PLANNING_SHARING_AUTHORIZATION_MISMATCH"),
    };
    let snapshot =
        crate::compute_plugin_sharing_directive::build_compute_plugin_sharing_policy_snapshot_v1(
            sharing.node_id.clone(),
            sharing.owner_user_id.clone(),
            sharing.installation_identity_digest.clone(),
            u64::try_from(sharing.policy_revision)?,
            sharing.policy_digest.clone(),
            sharing.plugin_runtime_requested,
            authorization,
        )
        .map_err(|error| anyhow::anyhow!(error.code()))?;
    let digest =
        crate::compute_plugin_sharing_directive::compute_plugin_sharing_policy_snapshot_digest(
            &snapshot,
        )
        .map_err(|error| anyhow::anyhow!(error.code()))?;
    Ok((snapshot, digest))
}

pub(super) fn validate_sharing_observation(
    sharing: &NodeComputePluginSharingDispatchIntent,
    policy_snapshot_digest: &str,
    observed: &homecli_proto::ComputePluginSharingPolicyObservedV1,
) -> Result<()> {
    if observed.node_id != sharing.node_id
        || observed.owner_user_id != sharing.owner_user_id
        || observed
            .installation_identity_digest
            .as_deref()
            .is_some_and(|value| value != sharing.installation_identity_digest)
        || observed
            .observed_policy_revision
            .is_some_and(|value| u64::try_from(sharing.policy_revision).ok() != Some(value))
        || observed
            .observed_policy_digest
            .as_deref()
            .is_some_and(|value| value != sharing.policy_digest)
        || observed
            .observed_snapshot_digest
            .as_deref()
            .is_some_and(|value| value != policy_snapshot_digest)
        || !matches!(observed.phase.as_str(), "blocked" | "disabled")
    {
        bail!("NODE_ENDPOINT_PLANNING_SHARING_OBSERVATION_SOURCE_MISMATCH");
    }
    if observed.accepted
        && (observed.installation_identity_digest.as_deref()
            != Some(sharing.installation_identity_digest.as_str())
            || observed.observed_policy_revision != u64::try_from(sharing.policy_revision).ok()
            || observed.observed_policy_digest.as_deref() != Some(sharing.policy_digest.as_str())
            || observed.observed_snapshot_digest.as_deref() != Some(policy_snapshot_digest)
            || observed.phase
                != if sharing.plugin_runtime_requested {
                    "blocked"
                } else {
                    "disabled"
                })
    {
        bail!("NODE_ENDPOINT_PLANNING_SHARING_ACCEPTANCE_MISMATCH");
    }
    Ok(())
}

pub(super) fn canonical_sharing_observation(
    observed: &homecli_proto::ComputePluginSharingPolicyObservedV1,
) -> Result<(String, String)> {
    let (json, digest) =
        crate::compute_plugin_sharing_directive::canonical_compute_plugin_ijson_and_sha256(
            observed,
            MAX_OBSERVATION_BYTES,
        )?;
    let readback: homecli_proto::ComputePluginSharingPolicyObservedV1 =
        serde_json::from_str(&json)?;
    if &readback != observed {
        bail!("NODE_ENDPOINT_PLANNING_SHARING_OBSERVATION_READBACK_MISMATCH");
    }
    Ok((json, digest))
}

pub(super) fn preparation_request(
    intent: &NodeComputePluginInstallPlanPreparationDispatchIntent,
) -> Result<homecli_proto::ComputePluginInstallPlanPreparationRequestV1> {
    Ok(
        homecli_proto::ComputePluginInstallPlanPreparationRequestV1 {
            schema: homecli_proto::COMPUTE_PLUGIN_INSTALL_PLAN_PREPARATION_REQUEST_V1_SCHEMA
                .to_string(),
            preparation_id: intent.preparation_id.clone(),
            node_id: intent.node_id.clone(),
            owner_user_id: intent.owner_user_id.clone(),
            installation_identity_digest: intent.installation_identity_digest.clone(),
            policy_revision: u64::try_from(intent.policy_revision)?,
            policy_digest: intent.policy_digest.clone(),
            policy_snapshot_digest: intent.policy_snapshot_digest.clone(),
            authorization: homecli_proto::ComputePluginSharingAuthorizationBindingV1 {
                authorization_ref: intent.authorization.authorization_ref.clone(),
                revision: u64::try_from(intent.authorization.revision)?,
                digest: intent.authorization.digest.clone(),
            },
        },
    )
}

pub(super) fn planning_request(
    permit: &NodeEndpointSessionPermit,
    intent: &NodeComputePluginInstallPlanPreparationDispatchIntent,
    observation_digest: &str,
) -> Result<homecli_proto::ComputePluginInstallPlanPlanningSnapshotRequestV2> {
    Ok(
        homecli_proto::ComputePluginInstallPlanPlanningSnapshotRequestV2 {
            schema: homecli_proto::COMPUTE_PLUGIN_INSTALL_PLAN_PLANNING_SNAPSHOT_REQUEST_V2_SCHEMA
                .to_string(),
            preparation_id: intent.preparation_id.clone(),
            cloud_session_id: permit.binding().session_id().to_string(),
            source_preparation_delivery_id: intent.delivery_id.clone(),
            source_preparation_observation_digest: observation_digest.to_string(),
            node_id: intent.node_id.clone(),
            owner_user_id: intent.owner_user_id.clone(),
            installation_identity_digest: intent.installation_identity_digest.clone(),
            policy_revision: u64::try_from(intent.policy_revision)?,
            policy_digest: intent.policy_digest.clone(),
            policy_snapshot_digest: intent.policy_snapshot_digest.clone(),
            authorization: homecli_proto::ComputePluginSharingAuthorizationBindingV1 {
                authorization_ref: intent.authorization.authorization_ref.clone(),
                revision: u64::try_from(intent.authorization.revision)?,
                digest: intent.authorization.digest.clone(),
            },
        },
    )
}
