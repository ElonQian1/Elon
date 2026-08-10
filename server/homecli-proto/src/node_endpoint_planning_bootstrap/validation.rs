use crate::{
    node_endpoint_wire::{
        bounded_identifier, bounded_string_list, positive_safe_integer, safe_integer, sha256_digest,
    },
    ComputePluginInstallPlanPlanningSnapshotObservedV2,
    ComputePluginInstallPlanPlanningSnapshotRequestV2,
    ComputePluginInstallPlanPreparationObservedV1, ComputePluginInstallPlanPreparationRequestV1,
    ComputePluginSharingAuthorizationBindingV1, ComputePluginSharingPolicyObservedV1,
    ComputePluginSharingPolicySnapshotV1,
    COMPUTE_PLUGIN_INSTALL_PLAN_PLANNING_SNAPSHOT_OBSERVED_V2_SCHEMA,
    COMPUTE_PLUGIN_INSTALL_PLAN_PLANNING_SNAPSHOT_REQUEST_V2_SCHEMA,
    COMPUTE_PLUGIN_INSTALL_PLAN_PREPARATION_OBSERVED_V1_SCHEMA,
    COMPUTE_PLUGIN_INSTALL_PLAN_PREPARATION_REQUEST_V1_SCHEMA,
    COMPUTE_PLUGIN_SHARING_POLICY_OBSERVED_V1_SCHEMA,
    COMPUTE_PLUGIN_SHARING_POLICY_SNAPSHOT_V1_SCHEMA,
};

use super::NodeEndpointPlanningBootstrapSessionBindingV1;

pub(super) fn validate_sharing_request(
    value: &ComputePluginSharingPolicySnapshotV1,
    session: &NodeEndpointPlanningBootstrapSessionBindingV1,
) -> Result<(), &'static str> {
    if value.schema != COMPUTE_PLUGIN_SHARING_POLICY_SNAPSHOT_V1_SCHEMA
        || value.node_id != session.agent_id()
        || value.owner_user_id != session.owner_user_id()
        || !sha256_digest(&value.installation_identity_digest)
        || !positive_safe_integer(value.policy_revision)
        || !sha256_digest(&value.policy_digest)
        || value.plugin_runtime_requested != value.authorization.is_some()
        || value.authorization.as_ref().is_some_and(|authorization| {
            !valid_authorization(authorization, value.policy_revision, &value.policy_digest)
        })
    {
        return Err("NODE_ENDPOINT_PLANNING_BOOTSTRAP_SHARING_REQUEST_INVALID");
    }
    Ok(())
}

pub(super) fn validate_sharing_observed(
    value: &ComputePluginSharingPolicyObservedV1,
    session: &NodeEndpointPlanningBootstrapSessionBindingV1,
) -> Result<(), &'static str> {
    if value.schema != COMPUTE_PLUGIN_SHARING_POLICY_OBSERVED_V1_SCHEMA
        || value.node_id != session.agent_id()
        || value.owner_user_id != session.owner_user_id()
        || value
            .installation_identity_digest
            .as_deref()
            .is_some_and(|digest| !sha256_digest(digest))
        || !safe_integer(value.configuration_generation)
        || !safe_integer(value.cancellation_generation)
        || !matches!(value.phase.as_str(), "blocked" | "disabled")
        || value.side_effects_started
        || value.blocked_reasons.is_empty()
        || !bounded_string_list(&value.blocked_reasons, 64)
        || value
            .error_code
            .as_deref()
            .is_some_and(|code| !stable_code(code))
    {
        return Err("NODE_ENDPOINT_PLANNING_BOOTSTRAP_SHARING_OBSERVED_INVALID");
    }
    if value.accepted {
        if value.installation_identity_digest.is_none()
            || value
                .observed_policy_revision
                .is_none_or(|revision| !positive_safe_integer(revision))
            || value
                .observed_policy_digest
                .as_deref()
                .is_none_or(|digest| !sha256_digest(digest))
            || value
                .observed_snapshot_digest
                .as_deref()
                .is_none_or(|digest| !sha256_digest(digest))
            || value.error_code.is_some()
        {
            return Err("NODE_ENDPOINT_PLANNING_BOOTSTRAP_SHARING_ACCEPTANCE_INVALID");
        }
    } else if value.error_code.is_none() {
        return Err("NODE_ENDPOINT_PLANNING_BOOTSTRAP_SHARING_REJECTION_INVALID");
    }
    Ok(())
}

pub(super) fn validate_preparation_request(
    value: &ComputePluginInstallPlanPreparationRequestV1,
    session: &NodeEndpointPlanningBootstrapSessionBindingV1,
) -> Result<(), &'static str> {
    if value.schema != COMPUTE_PLUGIN_INSTALL_PLAN_PREPARATION_REQUEST_V1_SCHEMA
        || !bounded_identifier(&value.preparation_id, 256)
        || value.node_id != session.agent_id()
        || value.owner_user_id != session.owner_user_id()
        || !sha256_digest(&value.installation_identity_digest)
        || !positive_safe_integer(value.policy_revision)
        || !sha256_digest(&value.policy_digest)
        || !sha256_digest(&value.policy_snapshot_digest)
        || !valid_authorization(
            &value.authorization,
            value.policy_revision,
            &value.policy_digest,
        )
    {
        return Err("NODE_ENDPOINT_PLANNING_BOOTSTRAP_PREPARATION_REQUEST_INVALID");
    }
    Ok(())
}

pub(super) fn validate_preparation_observed(
    value: &ComputePluginInstallPlanPreparationObservedV1,
    session: &NodeEndpointPlanningBootstrapSessionBindingV1,
) -> Result<(), &'static str> {
    if value.schema != COMPUTE_PLUGIN_INSTALL_PLAN_PREPARATION_OBSERVED_V1_SCHEMA
        || !bounded_identifier(&value.preparation_id, 256)
        || value.node_id != session.agent_id()
        || value.owner_user_id != session.owner_user_id()
        || value
            .installation_identity_digest
            .as_deref()
            .is_some_and(|digest| !sha256_digest(digest))
        || !bounded_identifier(&value.bootstrap_instance_id, 256)
        || value.context_ready
        || value.context.is_some()
        || value.phase != "blocked"
        || !safe_integer(value.configuration_generation)
        || !safe_integer(value.cancellation_generation)
        || value.compute_plugin_root_lock_acquired
        || value.trusted_time_authority_configured
        || value.rollback_anchor_witness_configured
        || value.root_pinned
        || value.authority_opened
        || value.process_fence_acquired
        || value.new_work_admission_enabled
        || value.downloads_allowed
        || value.side_effects_started
        || value.blocked_reasons.is_empty()
        || !bounded_string_list(&value.blocked_reasons, 64)
        || value
            .error_code
            .as_deref()
            .is_some_and(|code| !stable_code(code))
    {
        return Err("NODE_ENDPOINT_PLANNING_BOOTSTRAP_PREPARATION_OBSERVED_INVALID");
    }
    if value.accepted {
        if value.installation_identity_digest.is_none()
            || value
                .observed_policy_revision
                .is_none_or(|revision| !positive_safe_integer(revision))
            || value
                .observed_policy_digest
                .as_deref()
                .is_none_or(|digest| !sha256_digest(digest))
            || value
                .observed_policy_snapshot_digest
                .as_deref()
                .is_none_or(|digest| !sha256_digest(digest))
            || value
                .observed_authorization
                .as_ref()
                .is_none_or(|authorization| {
                    value.observed_policy_revision.is_none_or(|revision| {
                        value
                            .observed_policy_digest
                            .as_deref()
                            .is_none_or(|digest| {
                                !valid_authorization(authorization, revision, digest)
                            })
                    })
                })
            || value.error_code.is_some()
        {
            return Err("NODE_ENDPOINT_PLANNING_BOOTSTRAP_PREPARATION_ACCEPTANCE_INVALID");
        }
    } else if value.error_code.is_none() {
        return Err("NODE_ENDPOINT_PLANNING_BOOTSTRAP_PREPARATION_REJECTION_INVALID");
    }
    Ok(())
}

pub(super) fn validate_snapshot_request(
    value: &ComputePluginInstallPlanPlanningSnapshotRequestV2,
    session: &NodeEndpointPlanningBootstrapSessionBindingV1,
) -> Result<(), &'static str> {
    if value.schema != COMPUTE_PLUGIN_INSTALL_PLAN_PLANNING_SNAPSHOT_REQUEST_V2_SCHEMA
        || !bounded_identifier(&value.preparation_id, 256)
        || value.cloud_session_id != session.session_id()
        || !bounded_identifier(&value.source_preparation_delivery_id, 256)
        || !sha256_digest(&value.source_preparation_observation_digest)
        || value.node_id != session.agent_id()
        || value.owner_user_id != session.owner_user_id()
        || !sha256_digest(&value.installation_identity_digest)
        || !positive_safe_integer(value.policy_revision)
        || !sha256_digest(&value.policy_digest)
        || !sha256_digest(&value.policy_snapshot_digest)
        || !valid_authorization(
            &value.authorization,
            value.policy_revision,
            &value.policy_digest,
        )
    {
        return Err("NODE_ENDPOINT_PLANNING_BOOTSTRAP_SNAPSHOT_REQUEST_INVALID");
    }
    Ok(())
}

pub(super) fn validate_snapshot_observed(
    value: &ComputePluginInstallPlanPlanningSnapshotObservedV2,
    session: &NodeEndpointPlanningBootstrapSessionBindingV1,
) -> Result<(), &'static str> {
    if value.schema != COMPUTE_PLUGIN_INSTALL_PLAN_PLANNING_SNAPSHOT_OBSERVED_V2_SCHEMA
        || !bounded_identifier(&value.preparation_id, 256)
        || value.cloud_session_id != session.session_id()
        || !bounded_identifier(&value.source_preparation_delivery_id, 256)
        || !sha256_digest(&value.source_preparation_observation_digest)
        || value.node_id != session.agent_id()
        || value.owner_user_id != session.owner_user_id()
        || value
            .installation_identity_digest
            .as_deref()
            .is_some_and(|digest| !sha256_digest(digest))
        || value.snapshot_ready
        || value.snapshot.is_some()
        || !bounded_identifier(&value.bootstrap_instance_id, 256)
        || value.phase != "blocked"
        || !safe_integer(value.configuration_generation)
        || !safe_integer(value.cancellation_generation)
        || value.local_confirmation_available
        || value.compute_plugin_root_lock_acquired
        || value.trusted_time_authority_configured
        || value.rollback_anchor_witness_configured
        || value.root_pinned
        || value.authority_opened
        || value.process_fence_acquired
        || value.plan_apply_allowed
        || value.new_work_admission_enabled
        || value.downloads_allowed
        || value.sidecar_launch_allowed
        || value.side_effects_started
        || value.blocked_reasons.is_empty()
        || !bounded_string_list(&value.blocked_reasons, 64)
        || value
            .error_code
            .as_deref()
            .is_some_and(|code| !stable_code(code))
    {
        return Err("NODE_ENDPOINT_PLANNING_BOOTSTRAP_SNAPSHOT_OBSERVED_INVALID");
    }
    if value.accepted {
        if value.installation_identity_digest.is_none()
            || value
                .observed_policy_revision
                .is_none_or(|revision| !positive_safe_integer(revision))
            || value
                .observed_policy_digest
                .as_deref()
                .is_none_or(|digest| !sha256_digest(digest))
            || value
                .observed_policy_snapshot_digest
                .as_deref()
                .is_none_or(|digest| !sha256_digest(digest))
            || value
                .observed_authorization
                .as_ref()
                .is_none_or(|authorization| {
                    value.observed_policy_revision.is_none_or(|revision| {
                        value
                            .observed_policy_digest
                            .as_deref()
                            .is_none_or(|digest| {
                                !valid_authorization(authorization, revision, digest)
                            })
                    })
                })
            || value.error_code.is_some()
        {
            return Err("NODE_ENDPOINT_PLANNING_BOOTSTRAP_SNAPSHOT_ACCEPTANCE_INVALID");
        }
    } else if value.error_code.is_none() {
        return Err("NODE_ENDPOINT_PLANNING_BOOTSTRAP_SNAPSHOT_REJECTION_INVALID");
    }
    Ok(())
}

fn valid_authorization(
    value: &ComputePluginSharingAuthorizationBindingV1,
    policy_revision: u64,
    policy_digest: &str,
) -> bool {
    bounded_identifier(&value.authorization_ref, 256)
        && positive_safe_integer(value.revision)
        && value.revision == policy_revision
        && sha256_digest(&value.digest)
        && value.digest == policy_digest
}

fn stable_code(value: &str) -> bool {
    bounded_identifier(value, 256)
        && value
            .bytes()
            .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit() || byte == b'_')
}
