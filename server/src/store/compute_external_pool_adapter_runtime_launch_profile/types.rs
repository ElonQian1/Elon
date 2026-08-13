use serde::Serialize;

use crate::compute_federation::{
    external_pool_adapter_installation::{
        ExternalPoolAdapterInstallationBinding, PreparedExternalPoolAdapterInstallation,
    },
    external_pool_adapter_runtime_launch_profile::{
        ExternalPoolAdapterRuntimeLaunchPolicy, ExternalPoolAdapterRuntimeLaunchProfileReceipt,
        ExternalPoolAdapterRuntimeLaunchProfileRevocationReceipt,
    },
};
use crate::store::compute_external_pool_provider_activation_candidate::CurrentExternalPoolProviderActivationCandidateStaticAuthority;

pub(crate) struct CreateExternalPoolAdapterRuntimeLaunchProfile {
    pub prepared: PreparedExternalPoolAdapterInstallation,
    pub candidate_id: String,
    pub expected_candidate_digest: String,
    pub expected_provider_binding_digest: String,
    pub expected_launch_policy_digest: String,
    pub predecessor_profile_id: Option<String>,
    pub expected_predecessor_profile_digest: Option<String>,
    pub recorded_by_actor_kind: String,
    pub recorded_by_actor_user_id: String,
    pub idempotency_scope: String,
    pub idempotency_key: String,
    pub confirmation: String,
}

pub(crate) struct RevokeExternalPoolAdapterRuntimeLaunchProfile {
    pub profile_id: String,
    pub expected_profile_digest: String,
    pub expected_candidate_digest: String,
    pub revoked_by_actor_kind: String,
    pub revoked_by_actor_user_id: String,
    pub reason: String,
    pub idempotency_scope: String,
    pub idempotency_key: String,
    pub confirmation: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct ExternalPoolAdapterRuntimeLaunchPolicySummary {
    pub schema: &'static str,
    pub policy_id: String,
    pub policy_revision: u64,
    pub policy_digest: String,
    pub runtime_kind: String,
    pub host_os: String,
    pub host_arch: String,
    pub host_environment: String,
    pub executable_kind: String,
    pub binary_format: String,
    pub resolver_backend_policy_id: String,
    pub resolver_backend_policy_revision: u64,
    pub process_isolation_policy_id: String,
    pub resource_policy_id: String,
    pub network_egress_policy_id: String,
    pub profile_effect: String,
    pub adapter_effect: String,
    pub runtime_effect: String,
    pub usage_effect: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct ExternalPoolAdapterRuntimeLaunchProfileSummary {
    pub profile_id: String,
    pub profile_digest: String,
    pub profile_material_digest: String,
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
    pub provider_id: String,
    pub provider_status: String,
    pub logical_adapter_id: String,
    pub release_version: String,
    pub adapter_config_revision: i64,
    pub adapter_config_digest: String,
    pub implementation_digest: String,
    pub capability_set_digest: String,
    pub credential_verifier_digest: String,
    pub credential_ref_scheme: String,
    pub entrypoint_path_digest: String,
    pub entrypoint_sha256: String,
    pub entrypoint_size_bytes: u64,
    pub entry_inventory_digest: String,
    pub installed_file_count: u64,
    pub installed_total_bytes: u64,
    pub launch_policy_digest: String,
    pub launch_policy: ExternalPoolAdapterRuntimeLaunchPolicy,
    pub sequence: u64,
    pub predecessor_profile_id: Option<String>,
    pub predecessor_profile_digest: Option<String>,
    pub recorded_by_actor_kind: String,
    pub recorded_at: String,
    pub profile_status: String,
    pub profile_effect: String,
    pub adapter_effect: String,
    pub runtime_effect: String,
    pub usage_effect: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct ExternalPoolAdapterRuntimeLaunchProfileRevocationSummary {
    pub revocation_id: String,
    pub revocation_digest: String,
    pub revocation_material_digest: String,
    pub profile_id: String,
    pub profile_digest: String,
    pub provider_binding_id: String,
    pub candidate_id: String,
    pub candidate_digest: String,
    pub revoked_by_actor_kind: String,
    pub reason: String,
    pub revoked_at: String,
    pub revocation_effect: String,
    pub adapter_effect: String,
    pub runtime_effect: String,
    pub usage_effect: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct ExternalPoolAdapterRuntimeLaunchProfileWriteReceipt {
    pub profile: ExternalPoolAdapterRuntimeLaunchProfileSummary,
    pub replayed: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct ExternalPoolAdapterRuntimeLaunchProfileRevocationWriteReceipt {
    pub profile: ExternalPoolAdapterRuntimeLaunchProfileSummary,
    pub revocation: ExternalPoolAdapterRuntimeLaunchProfileRevocationSummary,
    pub replayed: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct ExternalPoolAdapterRuntimeLaunchProfileCurrentness {
    pub schema: &'static str,
    pub profile: ExternalPoolAdapterRuntimeLaunchProfileSummary,
    pub current_status: String,
    pub provider_status: String,
    pub candidate_status: String,
    pub file_inventory_status: String,
    pub launch_policy_status: String,
    pub revocation_status: String,
    pub runtime_launch_ready: bool,
    pub checked_at: String,
}

pub(crate) struct ExternalPoolAdapterRuntimeLaunchProfileAuditTarget {
    pub profile_id: String,
    pub profile_digest: String,
    pub candidate_id: String,
    pub candidate_digest: String,
    pub provider_binding_id: String,
    pub provider_owner_account_id: String,
    pub installation_binding: ExternalPoolAdapterInstallationBinding,
}

pub(super) struct StoredRuntimeLaunchProfile {
    pub receipt: ExternalPoolAdapterRuntimeLaunchProfileReceipt,
    pub receipt_json: String,
}

pub(super) struct StoredRuntimeLaunchProfileRevocation {
    pub receipt: ExternalPoolAdapterRuntimeLaunchProfileRevocationReceipt,
    pub receipt_json: String,
}

pub(super) struct RuntimeLaunchPolicyCatalogEntry {
    pub policy: ExternalPoolAdapterRuntimeLaunchPolicy,
    pub digest: String,
}

/// Store-only future-consumer seam retaining the exact Prepared installation through V254/V249.
/// It intentionally implements neither Clone nor Serde.
pub(in crate::store) struct CurrentExternalPoolAdapterRuntimeLaunchProfileAuthority {
    profile: ExternalPoolAdapterRuntimeLaunchProfileReceipt,
    candidate: CurrentExternalPoolProviderActivationCandidateStaticAuthority,
    checked_at: String,
}

impl CurrentExternalPoolAdapterRuntimeLaunchProfileAuthority {
    pub(super) fn new(
        profile: ExternalPoolAdapterRuntimeLaunchProfileReceipt,
        candidate: CurrentExternalPoolProviderActivationCandidateStaticAuthority,
        checked_at: String,
    ) -> Self {
        Self {
            profile,
            candidate,
            checked_at,
        }
    }

    pub(in crate::store) fn profile(&self) -> &ExternalPoolAdapterRuntimeLaunchProfileReceipt {
        &self.profile
    }

    pub(in crate::store) fn candidate(
        &self,
    ) -> &CurrentExternalPoolProviderActivationCandidateStaticAuthority {
        &self.candidate
    }

    pub(in crate::store) fn checked_at(&self) -> &str {
        &self.checked_at
    }
}

impl StoredRuntimeLaunchProfile {
    pub(super) fn summary(&self) -> ExternalPoolAdapterRuntimeLaunchProfileSummary {
        profile_summary(&self.receipt)
    }
}

pub(super) fn profile_summary(
    r: &ExternalPoolAdapterRuntimeLaunchProfileReceipt,
) -> ExternalPoolAdapterRuntimeLaunchProfileSummary {
    let p = &r.profile;
    ExternalPoolAdapterRuntimeLaunchProfileSummary {
        profile_id: r.profile_id.clone(),
        profile_digest: r.profile_digest.clone(),
        profile_material_digest: r.profile_material_digest.clone(),
        candidate_id: p.candidate_id.clone(),
        candidate_digest: p.candidate_digest.clone(),
        delegation_id: p.delegation_id.clone(),
        delegation_digest: p.delegation_digest.clone(),
        provider_binding_id: p.provider_binding_id.clone(),
        provider_binding_digest: p.provider_binding_digest.clone(),
        registry_release_id: p.registry_release_id.clone(),
        registry_release_digest: p.registry_release_digest.clone(),
        installation_receipt_id: p.installation_receipt_id.clone(),
        installation_receipt_digest: p.installation_receipt_digest.clone(),
        installation_content_digest: p.installation_content_digest.clone(),
        provider_id: p.provider_id.clone(),
        provider_status: p.provider_status.clone(),
        logical_adapter_id: p.logical_adapter_id.clone(),
        release_version: p.release_version.clone(),
        adapter_config_revision: p.adapter_config_revision,
        adapter_config_digest: p.adapter_config_digest.clone(),
        implementation_digest: p.implementation_digest.clone(),
        capability_set_digest: p.capability_set_digest.clone(),
        credential_verifier_digest: p.credential_verifier_digest.clone(),
        credential_ref_scheme: p.credential_ref_scheme.clone(),
        entrypoint_path_digest: p.entrypoint_path_digest.clone(),
        entrypoint_sha256: p.entrypoint_sha256.clone(),
        entrypoint_size_bytes: p.entrypoint_size_bytes,
        entry_inventory_digest: p.entry_inventory_digest.clone(),
        installed_file_count: p.installed_file_count,
        installed_total_bytes: p.installed_total_bytes,
        launch_policy_digest: p.launch_policy_digest.clone(),
        launch_policy: p.launch_policy.clone(),
        sequence: p.sequence,
        predecessor_profile_id: p.predecessor_profile_id.clone(),
        predecessor_profile_digest: p.predecessor_profile_digest.clone(),
        recorded_by_actor_kind: p.recorded_by_actor_kind.clone(),
        recorded_at: p.recorded_at.clone(),
        profile_status: p.profile_status.clone(),
        profile_effect: p.profile_effect.clone(),
        runtime_effect: p.runtime_effect.clone(),
        adapter_effect: p.adapter_effect.clone(),
        usage_effect: p.usage_effect.clone(),
    }
}

impl StoredRuntimeLaunchProfileRevocation {
    pub(super) fn summary(&self) -> ExternalPoolAdapterRuntimeLaunchProfileRevocationSummary {
        let r = &self.receipt;
        let v = &r.revocation;
        ExternalPoolAdapterRuntimeLaunchProfileRevocationSummary {
            revocation_id: r.revocation_id.clone(),
            revocation_digest: r.revocation_digest.clone(),
            revocation_material_digest: r.revocation_material_digest.clone(),
            profile_id: v.profile_id.clone(),
            profile_digest: v.profile_digest.clone(),
            provider_binding_id: v.provider_binding_id.clone(),
            candidate_id: v.candidate_id.clone(),
            candidate_digest: v.candidate_digest.clone(),
            revoked_by_actor_kind: v.revoked_by_actor_kind.clone(),
            reason: v.reason.clone(),
            revoked_at: v.revoked_at.clone(),
            revocation_effect: v.revocation_effect.clone(),
            runtime_effect: v.runtime_effect.clone(),
            adapter_effect: v.adapter_effect.clone(),
            usage_effect: v.usage_effect.clone(),
        }
    }
}
