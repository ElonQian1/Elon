use serde::Serialize;

use crate::{
    compute_federation::{
        external_pool_adapter_installation::{
            ExternalPoolAdapterInstallationBinding, PreparedExternalPoolAdapterInstallation,
        },
        external_pool_adapter_upstream_transport_target::{
            ExternalPoolAdapterUpstreamTransportTargetPolicy,
            ExternalPoolAdapterUpstreamTransportTargetReceipt,
            ExternalPoolAdapterUpstreamTransportTargetRevocationReceipt,
        },
    },
    store::compute_external_pool_adapter_runtime_launch_profile::CurrentExternalPoolAdapterRuntimeLaunchProfileAuthority,
};

pub(crate) struct ExternalPoolAdapterUpstreamTransportTargetDraft {
    pub dns_hostname: String,
    pub port: u16,
    pub expected_tls_leaf_spki_sha256: String,
}

pub(crate) struct CreateExternalPoolAdapterUpstreamTransportTarget {
    pub prepared: PreparedExternalPoolAdapterInstallation,
    pub profile_id: String,
    pub expected_profile_digest: String,
    pub expected_candidate_digest: String,
    pub expected_provider_binding_digest: String,
    pub expected_target_policy_digest: String,
    pub target: ExternalPoolAdapterUpstreamTransportTargetDraft,
    pub predecessor_target_id: Option<String>,
    pub expected_predecessor_target_digest: Option<String>,
    pub recorded_by_actor_kind: String,
    pub recorded_by_actor_user_id: String,
    pub idempotency_scope: String,
    pub idempotency_key: String,
    pub confirmation: String,
}

pub(crate) struct RevokeExternalPoolAdapterUpstreamTransportTarget {
    pub target_id: String,
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
pub(crate) struct ExternalPoolAdapterUpstreamTransportTargetPolicySummary {
    pub schema: &'static str,
    pub policy_id: String,
    pub policy_revision: u64,
    pub policy_digest: String,
    pub transport_owner: String,
    pub transport_kind: String,
    pub hostname_policy: String,
    pub port_policy: String,
    pub dns_resolution_policy: String,
    pub address_selection_policy: String,
    pub tls_version_policy: String,
    pub tls_server_name_policy: String,
    pub tls_chain_policy: String,
    pub tls_trust_anchor_policy: String,
    pub tls_leaf_identity_policy: String,
    pub proxy_policy: String,
    pub redirect_policy: String,
    pub zero_rtt_policy: String,
    pub client_certificate_policy: String,
    pub adapter_network_policy: String,
    pub max_hostname_bytes: u64,
    pub max_dns_answers: u64,
    pub dns_timeout_ms: u64,
    pub connect_timeout_ms: u64,
    pub tls_handshake_timeout_ms: u64,
    pub max_connect_attempts: u64,
    pub target_effect: String,
    pub adapter_effect: String,
    pub runtime_effect: String,
    pub provider_effect: String,
    pub credential_effect: String,
    pub route_effect: String,
    pub execution_effect: String,
    pub usage_effect: String,
    pub market_effect: String,
    pub settlement_effect: String,
    pub broker_connect_ready: bool,
    pub upstream_probe_observed: bool,
    pub runtime_launch_ready: bool,
    pub activation_ready: bool,
}

/// Public-safe summary. Host, port, SNI, and expected SPKI pin are intentionally absent.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct ExternalPoolAdapterUpstreamTransportTargetSummary {
    pub target_id: String,
    pub target_digest: String,
    pub target_material_digest: String,
    pub profile_id: String,
    pub profile_digest: String,
    pub candidate_id: String,
    pub candidate_digest: String,
    pub delegation_id: String,
    pub delegation_digest: String,
    pub provider_binding_id: String,
    pub provider_binding_digest: String,
    pub registry_release_id: String,
    pub registry_release_digest: String,
    pub installation_receipt_id: String,
    pub installation_receipt_digest: String,
    pub installation_content_digest: String,
    pub route_adapter_projection_id: String,
    pub provider_id: String,
    pub provider_status: String,
    pub logical_adapter_id: String,
    pub release_version: String,
    pub adapter_config_revision: i64,
    pub adapter_config_digest: String,
    pub implementation_digest: String,
    pub capability_set_digest: String,
    pub credential_verifier_digest: String,
    pub launch_policy_digest: String,
    pub network_egress_policy_id: String,
    pub network_egress_policy_revision: u64,
    pub network_egress_policy_digest: String,
    pub service_actor_id: String,
    pub target_policy_digest: String,
    pub sequence: u64,
    pub predecessor_target_id: Option<String>,
    pub predecessor_target_digest: Option<String>,
    pub recorded_by_actor_kind: String,
    pub recorded_at: String,
    pub target_status: String,
    pub target_effect: String,
    pub adapter_effect: String,
    pub runtime_effect: String,
    pub provider_effect: String,
    pub credential_effect: String,
    pub route_effect: String,
    pub execution_effect: String,
    pub usage_effect: String,
    pub market_effect: String,
    pub settlement_effect: String,
    pub broker_connect_ready: bool,
    pub upstream_probe_observed: bool,
    pub runtime_launch_ready: bool,
    pub activation_ready: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct ExternalPoolAdapterUpstreamTransportTargetRevocationSummary {
    pub revocation_id: String,
    pub revocation_digest: String,
    pub revocation_material_digest: String,
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
    pub broker_connect_ready: bool,
    pub upstream_probe_observed: bool,
    pub runtime_launch_ready: bool,
    pub activation_ready: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct ExternalPoolAdapterUpstreamTransportTargetWriteReceipt {
    pub target: ExternalPoolAdapterUpstreamTransportTargetSummary,
    pub replayed: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct ExternalPoolAdapterUpstreamTransportTargetRevocationWriteReceipt {
    pub target: ExternalPoolAdapterUpstreamTransportTargetSummary,
    pub revocation: ExternalPoolAdapterUpstreamTransportTargetRevocationSummary,
    pub replayed: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct ExternalPoolAdapterUpstreamTransportTargetCurrentness {
    pub schema: &'static str,
    pub target: ExternalPoolAdapterUpstreamTransportTargetSummary,
    pub current_status: String,
    pub provider_status: String,
    pub profile_status: String,
    pub target_policy_status: String,
    pub revocation_status: String,
    pub broker_connect_ready: bool,
    pub upstream_probe_observed: bool,
    pub runtime_launch_ready: bool,
    pub activation_ready: bool,
    pub checked_at: String,
}

pub(crate) struct ExternalPoolAdapterUpstreamTransportTargetAuditTarget {
    pub target_id: String,
    pub target_digest: String,
    pub profile_id: String,
    pub profile_digest: String,
    pub candidate_id: String,
    pub provider_binding_id: String,
    pub provider_owner_account_id: String,
    pub installation_binding: ExternalPoolAdapterInstallationBinding,
}

pub(super) struct StoredUpstreamTransportTarget {
    pub receipt: ExternalPoolAdapterUpstreamTransportTargetReceipt,
    pub receipt_json: String,
}

pub(super) struct StoredUpstreamTransportTargetRevocation {
    pub receipt: ExternalPoolAdapterUpstreamTransportTargetRevocationReceipt,
    pub receipt_json: String,
}

pub(super) struct UpstreamTransportTargetPolicyCatalogEntry {
    pub policy: ExternalPoolAdapterUpstreamTransportTargetPolicy,
    pub digest: String,
}

/// Store-only future broker seam. It intentionally implements neither Clone, Debug nor Serde.
pub(in crate::store) struct CurrentExternalPoolAdapterUpstreamTransportTargetAuthority {
    target: ExternalPoolAdapterUpstreamTransportTargetReceipt,
    profile: CurrentExternalPoolAdapterRuntimeLaunchProfileAuthority,
    checked_at: String,
}

impl CurrentExternalPoolAdapterUpstreamTransportTargetAuthority {
    pub(super) fn new(
        target: ExternalPoolAdapterUpstreamTransportTargetReceipt,
        profile: CurrentExternalPoolAdapterRuntimeLaunchProfileAuthority,
        checked_at: String,
    ) -> Self {
        Self {
            target,
            profile,
            checked_at,
        }
    }

    pub(in crate::store) fn target(&self) -> &ExternalPoolAdapterUpstreamTransportTargetReceipt {
        &self.target
    }

    pub(in crate::store) fn profile(
        &self,
    ) -> &CurrentExternalPoolAdapterRuntimeLaunchProfileAuthority {
        &self.profile
    }

    pub(in crate::store) fn checked_at(&self) -> &str {
        &self.checked_at
    }
}

impl StoredUpstreamTransportTarget {
    pub(super) fn summary(&self) -> ExternalPoolAdapterUpstreamTransportTargetSummary {
        target_summary(&self.receipt)
    }
}

pub(super) fn target_summary(
    r: &ExternalPoolAdapterUpstreamTransportTargetReceipt,
) -> ExternalPoolAdapterUpstreamTransportTargetSummary {
    let t = &r.target;
    ExternalPoolAdapterUpstreamTransportTargetSummary {
        target_id: r.target_id.clone(),
        target_digest: r.target_digest.clone(),
        target_material_digest: r.target_material_digest.clone(),
        profile_id: t.profile_id.clone(),
        profile_digest: t.profile_digest.clone(),
        candidate_id: t.candidate_id.clone(),
        candidate_digest: t.candidate_digest.clone(),
        delegation_id: t.delegation_id.clone(),
        delegation_digest: t.delegation_digest.clone(),
        provider_binding_id: t.provider_binding_id.clone(),
        provider_binding_digest: t.provider_binding_digest.clone(),
        registry_release_id: t.registry_release_id.clone(),
        registry_release_digest: t.registry_release_digest.clone(),
        installation_receipt_id: t.installation_receipt_id.clone(),
        installation_receipt_digest: t.installation_receipt_digest.clone(),
        installation_content_digest: t.installation_content_digest.clone(),
        route_adapter_projection_id: t.route_adapter_projection_id.clone(),
        provider_id: t.provider_id.clone(),
        provider_status: t.provider_status.clone(),
        logical_adapter_id: t.logical_adapter_id.clone(),
        release_version: t.release_version.clone(),
        adapter_config_revision: t.adapter_config_revision,
        adapter_config_digest: t.adapter_config_digest.clone(),
        implementation_digest: t.implementation_digest.clone(),
        capability_set_digest: t.capability_set_digest.clone(),
        credential_verifier_digest: t.credential_verifier_digest.clone(),
        launch_policy_digest: t.launch_policy_digest.clone(),
        network_egress_policy_id: t.network_egress_policy_id.clone(),
        network_egress_policy_revision: t.network_egress_policy_revision,
        network_egress_policy_digest: t.network_egress_policy_digest.clone(),
        service_actor_id: t.service_actor_id.clone(),
        target_policy_digest: t.target_policy_digest.clone(),
        sequence: t.sequence,
        predecessor_target_id: t.predecessor_target_id.clone(),
        predecessor_target_digest: t.predecessor_target_digest.clone(),
        recorded_by_actor_kind: t.recorded_by_actor_kind.clone(),
        recorded_at: t.recorded_at.clone(),
        target_status: t.target_status.clone(),
        target_effect: t.target_effect.clone(),
        adapter_effect: t.adapter_effect.clone(),
        runtime_effect: t.runtime_effect.clone(),
        provider_effect: t.provider_effect.clone(),
        credential_effect: t.credential_effect.clone(),
        route_effect: t.route_effect.clone(),
        execution_effect: t.execution_effect.clone(),
        usage_effect: t.usage_effect.clone(),
        market_effect: t.market_effect.clone(),
        settlement_effect: t.settlement_effect.clone(),
        broker_connect_ready: t.broker_connect_ready,
        upstream_probe_observed: t.upstream_probe_observed,
        runtime_launch_ready: t.runtime_launch_ready,
        activation_ready: t.activation_ready,
    }
}

impl StoredUpstreamTransportTargetRevocation {
    pub(super) fn summary(&self) -> ExternalPoolAdapterUpstreamTransportTargetRevocationSummary {
        let r = &self.receipt;
        let v = &r.revocation;
        ExternalPoolAdapterUpstreamTransportTargetRevocationSummary {
            revocation_id: r.revocation_id.clone(),
            revocation_digest: r.revocation_digest.clone(),
            revocation_material_digest: r.revocation_material_digest.clone(),
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
            broker_connect_ready: v.broker_connect_ready,
            upstream_probe_observed: v.upstream_probe_observed,
            runtime_launch_ready: v.runtime_launch_ready,
            activation_ready: v.activation_ready,
        }
    }
}
