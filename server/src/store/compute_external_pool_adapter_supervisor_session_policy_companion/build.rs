use anyhow::Result;

use crate::{
    compute_federation::external_pool_adapter_supervisor_session_policy_companion::*,
    store::{
        compute_external_pool_adapter_upstream_transport_target::CurrentExternalPoolAdapterUpstreamTransportTargetAuthority,
        new_id,
    },
};

use super::{policy::supervisor_session_policy_catalog, types::*};

#[allow(clippy::too_many_arguments)]
pub(super) fn build_companion(
    authority: &CurrentExternalPoolAdapterUpstreamTransportTargetAuthority,
    capsule_policy_id: &str,
    capsule_policy_revision: u64,
    capsule_policy_digest: &str,
    predecessor: Option<&StoredSupervisorSessionPolicyCompanion>,
    sequence: u64,
    now: &str,
    actor_kind: &str,
    actor_id: &str,
    scope: &str,
    key: &str,
    confirmation: &str,
) -> Result<ExternalPoolAdapterSupervisorSessionPolicyCompanionReceipt> {
    let target = authority.target();
    let t = &target.target;
    let launch = &authority.profile().profile().profile.launch_policy;
    let policy = supervisor_session_policy_catalog()?;
    let c = ExternalPoolAdapterSupervisorSessionPolicyCompanionMaterial {
        profile_id: t.profile_id.clone(),
        profile_digest: t.profile_digest.clone(),
        candidate_id: t.candidate_id.clone(),
        candidate_digest: t.candidate_digest.clone(),
        provider_binding_id: t.provider_binding_id.clone(),
        provider_binding_digest: t.provider_binding_digest.clone(),
        provider_id: t.provider_id.clone(),
        provider_owner_account_id: t.provider_owner_account_id.clone(),
        provider_policy_revision: t.provider_policy_revision,
        provider_digest: t.provider_digest.clone(),
        provider_status: t.provider_status.clone(),
        launch_policy_digest: t.launch_policy_digest.clone(),
        process_isolation_policy_id: launch.process_isolation_policy_id.clone(),
        process_isolation_policy_revision: launch.process_isolation_policy_revision,
        process_isolation_policy_digest: launch.process_isolation_policy_digest.clone(),
        resource_policy_id: launch.resource_policy_id.clone(),
        resource_policy_revision: launch.resource_policy_revision,
        resource_policy_digest: launch.resource_policy_digest.clone(),
        network_egress_policy_id: launch.network_egress_policy_id.clone(),
        network_egress_policy_revision: launch.network_egress_policy_revision,
        network_egress_policy_digest: launch.network_egress_policy_digest.clone(),
        delegation_id: t.delegation_id.clone(),
        delegation_digest: t.delegation_digest.clone(),
        registry_release_id: t.registry_release_id.clone(),
        registry_release_digest: t.registry_release_digest.clone(),
        installation_receipt_id: t.installation_receipt_id.clone(),
        installation_receipt_digest: t.installation_receipt_digest.clone(),
        installation_content_digest: t.installation_content_digest.clone(),
        route_adapter_projection_id: t.route_adapter_projection_id.clone(),
        logical_adapter_id: t.logical_adapter_id.clone(),
        release_version: t.release_version.clone(),
        adapter_config_revision: t.adapter_config_revision,
        adapter_config_digest: t.adapter_config_digest.clone(),
        implementation_digest: t.implementation_digest.clone(),
        capability_set_digest: t.capability_set_digest.clone(),
        credential_verifier_digest: t.credential_verifier_digest.clone(),
        service_actor_id: t.service_actor_id.clone(),
        entrypoint_capsule_policy_id: capsule_policy_id.into(),
        entrypoint_capsule_policy_revision: capsule_policy_revision,
        entrypoint_capsule_policy_digest: capsule_policy_digest.into(),
        target_id: target.target_id.clone(),
        target_digest: target.target_digest.clone(),
        target_policy_digest: t.target_policy_digest.clone(),
        supervisor_session_policy_digest: policy.digest,
        supervisor_session_policy: policy.policy,
        sequence,
        predecessor_companion_id: predecessor.map(|x| x.receipt.companion_id.clone()),
        predecessor_companion_digest: predecessor.map(|x| x.receipt.companion_digest.clone()),
        recorded_by_actor_kind: actor_kind.into(),
        recorded_by_actor_user_id: actor_id.into(),
        recorded_at: now.into(),
        idempotency_scope: scope.into(),
        idempotency_key: key.into(),
        confirmation: confirmation.into(),
        companion_status: SUPERVISOR_SESSION_COMPANION_STATUS.into(),
        companion_effect: SUPERVISOR_SESSION_COMPANION_EFFECT.into(),
        adapter_effect: none(),
        runtime_effect: none(),
        provider_effect: none(),
        credential_effect: none(),
        route_effect: none(),
        execution_effect: none(),
        usage_effect: none(),
        market_effect: none(),
        settlement_effect: none(),
        process_spawn_ready: false,
        ipc_session_ready: false,
        secret_delivery_ready: false,
        broker_connect_ready: false,
        upstream_probe_observed: false,
        runtime_launch_ready: false,
        activation_ready: false,
    };
    seal_companion(c)
}
pub(super) fn build_revocation(
    input: &RevokeExternalPoolAdapterSupervisorSessionPolicyCompanion,
    companion: &StoredSupervisorSessionPolicyCompanion,
    now: &str,
) -> Result<ExternalPoolAdapterSupervisorSessionPolicyCompanionRevocationReceipt> {
    let c = &companion.receipt.companion;
    let r = ExternalPoolAdapterSupervisorSessionPolicyCompanionRevocationMaterial {
        companion_id: companion.receipt.companion_id.clone(),
        companion_digest: companion.receipt.companion_digest.clone(),
        target_id: c.target_id.clone(),
        target_digest: c.target_digest.clone(),
        profile_id: c.profile_id.clone(),
        profile_digest: c.profile_digest.clone(),
        provider_binding_id: c.provider_binding_id.clone(),
        provider_binding_digest: c.provider_binding_digest.clone(),
        provider_id: c.provider_id.clone(),
        revoked_by_actor_kind: input.revoked_by_actor_kind.clone(),
        revoked_by_actor_user_id: input.revoked_by_actor_user_id.clone(),
        reason: input.reason.clone(),
        revoked_at: now.into(),
        recorded_at: now.into(),
        idempotency_scope: input.idempotency_scope.clone(),
        idempotency_key: input.idempotency_key.clone(),
        confirmation: input.confirmation.clone(),
        revocation_effect: SUPERVISOR_SESSION_COMPANION_REVOCATION_EFFECT.into(),
        adapter_effect: none(),
        runtime_effect: none(),
        provider_effect: none(),
        credential_effect: none(),
        route_effect: none(),
        execution_effect: none(),
        usage_effect: none(),
        market_effect: none(),
        settlement_effect: none(),
        process_spawn_ready: false,
        ipc_session_ready: false,
        secret_delivery_ready: false,
        broker_connect_ready: false,
        upstream_probe_observed: false,
        runtime_launch_ready: false,
        activation_ready: false,
    };
    seal_revocation(r)
}
fn seal_companion(
    c: ExternalPoolAdapterSupervisorSessionPolicyCompanionMaterial,
) -> Result<ExternalPoolAdapterSupervisorSessionPolicyCompanionReceipt> {
    validate_supervisor_session_policy(&c.supervisor_session_policy)?;
    let mut r = ExternalPoolAdapterSupervisorSessionPolicyCompanionReceipt {
        schema: SUPERVISOR_SESSION_COMPANION_SCHEMA.into(),
        companion_id: new_id("external_pool_adapter_supervisor_session_policy_companion"),
        companion_digest: String::new(),
        companion_material_digest: supervisor_session_companion_material_digest(&c)?,
        canonicalization: SUPERVISOR_SESSION_COMPANION_CANONICALIZATION.into(),
        digest_algorithm: SUPERVISOR_SESSION_COMPANION_DIGEST_ALGORITHM.into(),
        companion: c,
    };
    r.companion_digest = canonical_supervisor_session_companion_json_and_digest(&r)?.1;
    validate_supervisor_session_companion_receipt(&r)?;
    Ok(r)
}
fn seal_revocation(
    v: ExternalPoolAdapterSupervisorSessionPolicyCompanionRevocationMaterial,
) -> Result<ExternalPoolAdapterSupervisorSessionPolicyCompanionRevocationReceipt> {
    let mut r = ExternalPoolAdapterSupervisorSessionPolicyCompanionRevocationReceipt {
        schema: SUPERVISOR_SESSION_COMPANION_REVOCATION_SCHEMA.into(),
        revocation_id: new_id(
            "external_pool_adapter_supervisor_session_policy_companion_revocation",
        ),
        revocation_digest: String::new(),
        revocation_material_digest: supervisor_session_companion_revocation_material_digest(&v)?,
        canonicalization: SUPERVISOR_SESSION_COMPANION_CANONICALIZATION.into(),
        digest_algorithm: SUPERVISOR_SESSION_COMPANION_DIGEST_ALGORITHM.into(),
        revocation: v,
    };
    r.revocation_digest = canonical_supervisor_session_companion_revocation_json_and_digest(&r)?.1;
    validate_supervisor_session_companion_revocation_receipt(&r)?;
    Ok(r)
}
fn none() -> String {
    SUPERVISOR_SESSION_COMPANION_NO_EFFECT.into()
}
