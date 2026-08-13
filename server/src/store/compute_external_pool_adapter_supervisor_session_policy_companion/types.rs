use serde::Serialize;

use crate::{
    compute_federation::{
        external_pool_adapter_installation::{
            ExternalPoolAdapterInstallationBinding, PreparedExternalPoolAdapterInstallation,
        },
        external_pool_adapter_supervisor_session_policy_companion::{
            ExternalPoolAdapterSupervisorSessionPolicy,
            ExternalPoolAdapterSupervisorSessionPolicyCompanionReceipt,
            ExternalPoolAdapterSupervisorSessionPolicyCompanionRevocationReceipt,
        },
    },
    store::compute_external_pool_adapter_upstream_transport_target::CurrentExternalPoolAdapterUpstreamTransportTargetAuthority,
};

pub(crate) struct CreateExternalPoolAdapterSupervisorSessionPolicyCompanion {
    pub prepared: PreparedExternalPoolAdapterInstallation,
    pub target_id: String,
    pub expected_target_digest: String,
    pub expected_profile_digest: String,
    pub expected_candidate_digest: String,
    pub expected_provider_binding_digest: String,
    pub expected_supervisor_session_policy_digest: String,
    pub predecessor_companion_id: Option<String>,
    pub expected_predecessor_companion_digest: Option<String>,
    pub recorded_by_actor_kind: String,
    pub recorded_by_actor_user_id: String,
    pub idempotency_scope: String,
    pub idempotency_key: String,
    pub confirmation: String,
}

pub(crate) struct RevokeExternalPoolAdapterSupervisorSessionPolicyCompanion {
    pub companion_id: String,
    pub expected_companion_digest: String,
    pub expected_target_digest: String,
    pub expected_profile_digest: String,
    pub revoked_by_actor_kind: String,
    pub revoked_by_actor_user_id: String,
    pub reason: String,
    pub idempotency_scope: String,
    pub idempotency_key: String,
    pub confirmation: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct ExternalPoolAdapterSupervisorSessionPolicySummary {
    pub schema: &'static str,
    pub policy_digest: String,
    pub policy: ExternalPoolAdapterSupervisorSessionPolicy,
    pub companion_effect: String,
    pub adapter_effect: String,
    pub runtime_effect: String,
    pub provider_effect: String,
    pub credential_effect: String,
    pub route_effect: String,
    pub execution_effect: String,
    pub usage_effect: String,
    pub market_effect: String,
    pub settlement_effect: String,
    pub process_spawn_ready: bool,
    pub ipc_session_ready: bool,
    pub secret_delivery_ready: bool,
    pub broker_connect_ready: bool,
    pub upstream_probe_observed: bool,
    pub runtime_launch_ready: bool,
    pub activation_ready: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct ExternalPoolAdapterSupervisorSessionPolicyCompanionSummary {
    pub companion_id: String,
    pub companion_digest: String,
    pub companion_material_digest: String,
    pub profile_id: String,
    pub profile_digest: String,
    pub candidate_id: String,
    pub candidate_digest: String,
    pub provider_binding_id: String,
    pub provider_binding_digest: String,
    pub provider_id: String,
    pub provider_status: String,
    pub target_id: String,
    pub target_digest: String,
    pub target_policy_digest: String,
    pub launch_policy_digest: String,
    pub entrypoint_capsule_policy_digest: String,
    pub supervisor_session_policy_digest: String,
    pub sequence: u64,
    pub predecessor_companion_id: Option<String>,
    pub predecessor_companion_digest: Option<String>,
    pub recorded_by_actor_kind: String,
    pub recorded_at: String,
    pub companion_status: String,
    pub companion_effect: String,
    pub adapter_effect: String,
    pub runtime_effect: String,
    pub provider_effect: String,
    pub credential_effect: String,
    pub route_effect: String,
    pub execution_effect: String,
    pub usage_effect: String,
    pub market_effect: String,
    pub settlement_effect: String,
    pub process_spawn_ready: bool,
    pub ipc_session_ready: bool,
    pub secret_delivery_ready: bool,
    pub broker_connect_ready: bool,
    pub upstream_probe_observed: bool,
    pub runtime_launch_ready: bool,
    pub activation_ready: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct ExternalPoolAdapterSupervisorSessionPolicyCompanionRevocationSummary {
    pub revocation_id: String,
    pub revocation_digest: String,
    pub revocation_material_digest: String,
    pub companion_id: String,
    pub companion_digest: String,
    pub target_id: String,
    pub target_digest: String,
    pub profile_id: String,
    pub profile_digest: String,
    pub provider_binding_id: String,
    pub provider_binding_digest: String,
    pub provider_id: String,
    pub revoked_by_actor_kind: String,
    pub reason: String,
    pub revoked_at: String,
    pub revocation_effect: String,
    pub adapter_effect: String,
    pub runtime_effect: String,
    pub provider_effect: String,
    pub credential_effect: String,
    pub route_effect: String,
    pub execution_effect: String,
    pub usage_effect: String,
    pub market_effect: String,
    pub settlement_effect: String,
    pub process_spawn_ready: bool,
    pub ipc_session_ready: bool,
    pub secret_delivery_ready: bool,
    pub broker_connect_ready: bool,
    pub upstream_probe_observed: bool,
    pub runtime_launch_ready: bool,
    pub activation_ready: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct ExternalPoolAdapterSupervisorSessionPolicyCompanionWriteReceipt {
    pub companion: ExternalPoolAdapterSupervisorSessionPolicyCompanionSummary,
    pub replayed: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct ExternalPoolAdapterSupervisorSessionPolicyCompanionRevocationWriteReceipt {
    pub companion: ExternalPoolAdapterSupervisorSessionPolicyCompanionSummary,
    pub revocation: ExternalPoolAdapterSupervisorSessionPolicyCompanionRevocationSummary,
    pub replayed: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct ExternalPoolAdapterSupervisorSessionPolicyCompanionCurrentness {
    pub schema: &'static str,
    pub companion: ExternalPoolAdapterSupervisorSessionPolicyCompanionSummary,
    pub current_status: String,
    pub provider_status: String,
    pub profile_status: String,
    pub target_status: String,
    pub policy_status: String,
    pub revocation_status: String,
    pub adapter_effect: String,
    pub runtime_effect: String,
    pub provider_effect: String,
    pub credential_effect: String,
    pub route_effect: String,
    pub execution_effect: String,
    pub usage_effect: String,
    pub market_effect: String,
    pub settlement_effect: String,
    pub process_spawn_ready: bool,
    pub ipc_session_ready: bool,
    pub secret_delivery_ready: bool,
    pub broker_connect_ready: bool,
    pub upstream_probe_observed: bool,
    pub runtime_launch_ready: bool,
    pub activation_ready: bool,
    pub checked_at: String,
}

pub(crate) struct ExternalPoolAdapterSupervisorSessionPolicyCompanionAuditTarget {
    pub companion_id: String,
    pub companion_digest: String,
    pub target_id: String,
    pub target_digest: String,
    pub profile_id: String,
    pub profile_digest: String,
    pub candidate_id: String,
    pub provider_binding_id: String,
    pub provider_owner_account_id: String,
    pub installation_binding: ExternalPoolAdapterInstallationBinding,
}

pub(super) struct StoredSupervisorSessionPolicyCompanion {
    pub receipt: ExternalPoolAdapterSupervisorSessionPolicyCompanionReceipt,
    pub receipt_json: String,
}
pub(super) struct StoredSupervisorSessionPolicyCompanionRevocation {
    pub receipt: ExternalPoolAdapterSupervisorSessionPolicyCompanionRevocationReceipt,
    pub receipt_json: String,
}
pub(super) struct SupervisorSessionPolicyCatalogEntry {
    pub policy: ExternalPoolAdapterSupervisorSessionPolicy,
    pub digest: String,
}

/// Store-private future supervisor seam; intentionally not Clone, Debug, or serializable.
pub(in crate::store) struct CurrentExternalPoolAdapterSupervisorSessionPolicyCompanionAuthority {
    companion: ExternalPoolAdapterSupervisorSessionPolicyCompanionReceipt,
    target: CurrentExternalPoolAdapterUpstreamTransportTargetAuthority,
    checked_at: String,
}

impl CurrentExternalPoolAdapterSupervisorSessionPolicyCompanionAuthority {
    pub(super) fn new(
        companion: ExternalPoolAdapterSupervisorSessionPolicyCompanionReceipt,
        target: CurrentExternalPoolAdapterUpstreamTransportTargetAuthority,
        checked_at: String,
    ) -> Self {
        Self {
            companion,
            target,
            checked_at,
        }
    }
    pub(in crate::store) fn companion(
        &self,
    ) -> &ExternalPoolAdapterSupervisorSessionPolicyCompanionReceipt {
        &self.companion
    }
    pub(in crate::store) fn target(
        &self,
    ) -> &CurrentExternalPoolAdapterUpstreamTransportTargetAuthority {
        &self.target
    }
    pub(in crate::store) fn checked_at(&self) -> &str {
        &self.checked_at
    }
}

impl StoredSupervisorSessionPolicyCompanion {
    pub(super) fn summary(&self) -> ExternalPoolAdapterSupervisorSessionPolicyCompanionSummary {
        companion_summary(&self.receipt)
    }
}

pub(super) fn companion_summary(
    r: &ExternalPoolAdapterSupervisorSessionPolicyCompanionReceipt,
) -> ExternalPoolAdapterSupervisorSessionPolicyCompanionSummary {
    let c = &r.companion;
    ExternalPoolAdapterSupervisorSessionPolicyCompanionSummary {
        companion_id: r.companion_id.clone(),
        companion_digest: r.companion_digest.clone(),
        companion_material_digest: r.companion_material_digest.clone(),
        profile_id: c.profile_id.clone(),
        profile_digest: c.profile_digest.clone(),
        candidate_id: c.candidate_id.clone(),
        candidate_digest: c.candidate_digest.clone(),
        provider_binding_id: c.provider_binding_id.clone(),
        provider_binding_digest: c.provider_binding_digest.clone(),
        provider_id: c.provider_id.clone(),
        provider_status: c.provider_status.clone(),
        target_id: c.target_id.clone(),
        target_digest: c.target_digest.clone(),
        target_policy_digest: c.target_policy_digest.clone(),
        launch_policy_digest: c.launch_policy_digest.clone(),
        entrypoint_capsule_policy_digest: c.entrypoint_capsule_policy_digest.clone(),
        supervisor_session_policy_digest: c.supervisor_session_policy_digest.clone(),
        sequence: c.sequence,
        predecessor_companion_id: c.predecessor_companion_id.clone(),
        predecessor_companion_digest: c.predecessor_companion_digest.clone(),
        recorded_by_actor_kind: c.recorded_by_actor_kind.clone(),
        recorded_at: c.recorded_at.clone(),
        companion_status: c.companion_status.clone(),
        companion_effect: c.companion_effect.clone(),
        adapter_effect: c.adapter_effect.clone(),
        runtime_effect: c.runtime_effect.clone(),
        provider_effect: c.provider_effect.clone(),
        credential_effect: c.credential_effect.clone(),
        route_effect: c.route_effect.clone(),
        execution_effect: c.execution_effect.clone(),
        usage_effect: c.usage_effect.clone(),
        market_effect: c.market_effect.clone(),
        settlement_effect: c.settlement_effect.clone(),
        process_spawn_ready: c.process_spawn_ready,
        ipc_session_ready: c.ipc_session_ready,
        secret_delivery_ready: c.secret_delivery_ready,
        broker_connect_ready: c.broker_connect_ready,
        upstream_probe_observed: c.upstream_probe_observed,
        runtime_launch_ready: c.runtime_launch_ready,
        activation_ready: c.activation_ready,
    }
}

impl StoredSupervisorSessionPolicyCompanionRevocation {
    pub(super) fn summary(
        &self,
    ) -> ExternalPoolAdapterSupervisorSessionPolicyCompanionRevocationSummary {
        let r = &self.receipt;
        let v = &r.revocation;
        ExternalPoolAdapterSupervisorSessionPolicyCompanionRevocationSummary {
            revocation_id: r.revocation_id.clone(),
            revocation_digest: r.revocation_digest.clone(),
            revocation_material_digest: r.revocation_material_digest.clone(),
            companion_id: v.companion_id.clone(),
            companion_digest: v.companion_digest.clone(),
            target_id: v.target_id.clone(),
            target_digest: v.target_digest.clone(),
            profile_id: v.profile_id.clone(),
            profile_digest: v.profile_digest.clone(),
            provider_binding_id: v.provider_binding_id.clone(),
            provider_binding_digest: v.provider_binding_digest.clone(),
            provider_id: v.provider_id.clone(),
            revoked_by_actor_kind: v.revoked_by_actor_kind.clone(),
            reason: v.reason.clone(),
            revoked_at: v.revoked_at.clone(),
            revocation_effect: v.revocation_effect.clone(),
            adapter_effect: v.adapter_effect.clone(),
            runtime_effect: v.runtime_effect.clone(),
            provider_effect: v.provider_effect.clone(),
            credential_effect: v.credential_effect.clone(),
            route_effect: v.route_effect.clone(),
            execution_effect: v.execution_effect.clone(),
            usage_effect: v.usage_effect.clone(),
            market_effect: v.market_effect.clone(),
            settlement_effect: v.settlement_effect.clone(),
            process_spawn_ready: v.process_spawn_ready,
            ipc_session_ready: v.ipc_session_ready,
            secret_delivery_ready: v.secret_delivery_ready,
            broker_connect_ready: v.broker_connect_ready,
            upstream_probe_observed: v.upstream_probe_observed,
            runtime_launch_ready: v.runtime_launch_ready,
            activation_ready: v.activation_ready,
        }
    }
}
