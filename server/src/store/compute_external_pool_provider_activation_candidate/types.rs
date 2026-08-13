use serde::Serialize;

use crate::{
    compute_federation::{
        external_pool_adapter_installation::{
            ExternalPoolAdapterInstallationBinding, PreparedExternalPoolAdapterInstallation,
        },
        external_pool_provider_activation_candidate::{
            ExternalPoolProviderActivationCandidateReceipt,
            ExternalPoolProviderActivationDelegationReceipt,
            ExternalPoolProviderActivationDelegationRevocationReceipt,
        },
    },
    store::{
        compute_external_pool_adapter_credential_reattestation::CurrentExternalPoolAdapterCredentialReattestationAuthority,
        compute_external_pool_adapter_registry::CurrentExternalPoolAdapterRegistryProviderBindingAuthority,
        compute_external_pool_adapter_sandbox_reattestation::CurrentExternalPoolAdapterSandboxReattestationAuthority,
        compute_external_pool_adapter_vulnerability_reattestation::CurrentExternalPoolAdapterVulnerabilityReattestationAuthority,
    },
};

pub(crate) struct CreateExternalPoolProviderActivationCandidate {
    pub prepared: PreparedExternalPoolAdapterInstallation,
    pub provider_binding_id: String,
    pub expected_provider_binding_digest: String,
    pub expected_registry_release_digest: String,
    pub issued_by_owner_user_id: String,
    pub idempotency_scope: String,
    pub idempotency_key: String,
    pub confirmation: String,
}

pub(crate) struct GetCurrentExternalPoolProviderActivationPreflight {
    pub prepared: PreparedExternalPoolAdapterInstallation,
    pub candidate_id: String,
    pub expected_candidate_digest: String,
    pub vulnerability_reattestation_receipt_id: String,
    pub expected_vulnerability_reattestation_receipt_digest: String,
    pub sandbox_reattestation_receipt_id: String,
    pub expected_sandbox_reattestation_receipt_digest: String,
    pub credential_reattestation_receipt_id: String,
    pub expected_credential_reattestation_receipt_digest: String,
}

pub(crate) struct RevokeExternalPoolProviderActivationDelegation {
    pub delegation_id: String,
    pub expected_delegation_digest: String,
    pub expected_candidate_digest: String,
    pub revoked_by_owner_user_id: String,
    pub reason: String,
    pub idempotency_scope: String,
    pub idempotency_key: String,
    pub confirmation: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct ExternalPoolProviderActivationDelegationSummary {
    pub delegation_id: String,
    pub delegation_digest: String,
    pub delegation_material_digest: String,
    pub provider_binding_id: String,
    pub provider_binding_digest: String,
    pub registry_release_id: String,
    pub registry_release_digest: String,
    pub route_adapter_projection_id: String,
    pub provider_id: String,
    pub provider_owner_account_id: String,
    pub provider_policy_revision: i64,
    pub provider_digest: String,
    pub provider_status: String,
    pub logical_adapter_id: String,
    pub release_version: String,
    pub adapter_config_revision: i64,
    pub adapter_config_digest: String,
    pub service_actor_id: String,
    pub service_actor_kind: String,
    pub issued_at: String,
    pub sequence: u64,
    pub delegation_effect: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct ExternalPoolProviderActivationCandidateSummary {
    pub candidate_id: String,
    pub candidate_digest: String,
    pub candidate_material_digest: String,
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
    pub provider_owner_account_id: String,
    pub provider_policy_revision: i64,
    pub provider_digest: String,
    pub provider_status: String,
    pub logical_adapter_id: String,
    pub release_version: String,
    pub adapter_config_revision: i64,
    pub adapter_config_digest: String,
    pub implementation_digest: String,
    pub capability_set_digest: String,
    pub credential_verifier_digest: String,
    pub logical_adapter_binding_digest: String,
    pub logical_projection_compatibility_digest: String,
    pub service_actor_id: String,
    pub sequence: u64,
    pub checked_at: String,
    pub candidate_status: String,
    pub activation_closure_status: String,
    pub candidate_effect: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct ExternalPoolProviderActivationDelegationRevocationSummary {
    pub revocation_id: String,
    pub revocation_digest: String,
    pub revocation_material_digest: String,
    pub delegation_id: String,
    pub delegation_digest: String,
    pub candidate_id: String,
    pub candidate_digest: String,
    pub provider_binding_id: String,
    pub provider_id: String,
    pub reason: String,
    pub revoked_at: String,
    pub revocation_effect: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct ExternalPoolProviderActivationCandidateWriteReceipt {
    pub delegation: ExternalPoolProviderActivationDelegationSummary,
    pub candidate: ExternalPoolProviderActivationCandidateSummary,
    pub replayed: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct ExternalPoolProviderActivationDelegationRevocationWriteReceipt {
    pub delegation: ExternalPoolProviderActivationDelegationSummary,
    pub candidate: ExternalPoolProviderActivationCandidateSummary,
    pub revocation: ExternalPoolProviderActivationDelegationRevocationSummary,
    pub replayed: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct ExternalPoolProviderActivationCandidateCurrentness {
    pub schema: &'static str,
    pub delegation: ExternalPoolProviderActivationDelegationSummary,
    pub candidate: ExternalPoolProviderActivationCandidateSummary,
    pub current_status: String,
    pub provider_status: String,
    pub file_inventory_status: String,
    pub delegation_status: String,
    pub route_projection_status: String,
    pub activation_closure_status: String,
    pub activation_ready: bool,
    pub checked_at: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct ExternalPoolProviderActivationPreflightReceipt {
    pub schema: &'static str,
    pub delegation: ExternalPoolProviderActivationDelegationSummary,
    pub candidate: ExternalPoolProviderActivationCandidateSummary,
    pub checked_at: String,
    pub inputs_status: String,
    pub activation_closure_status: String,
    pub activation_ready: bool,
}

pub(crate) struct ExternalPoolProviderActivationCandidateAuditTarget {
    pub candidate_id: String,
    pub candidate_digest: String,
    pub provider_binding_id: String,
    pub provider_binding_digest: String,
    pub provider_owner_account_id: String,
    pub installation_binding: ExternalPoolAdapterInstallationBinding,
}

pub(super) struct StoredDelegation {
    pub receipt: ExternalPoolProviderActivationDelegationReceipt,
    pub receipt_json: String,
}
pub(super) struct StoredCandidate {
    pub receipt: ExternalPoolProviderActivationCandidateReceipt,
    pub receipt_json: String,
}
pub(super) struct StoredRevocation {
    pub receipt: ExternalPoolProviderActivationDelegationRevocationReceipt,
    pub receipt_json: String,
}

impl StoredDelegation {
    pub(super) fn summary(&self) -> ExternalPoolProviderActivationDelegationSummary {
        delegation_summary(&self.receipt)
    }
}

pub(super) fn delegation_summary(
    r: &ExternalPoolProviderActivationDelegationReceipt,
) -> ExternalPoolProviderActivationDelegationSummary {
    let d = &r.delegation;
    ExternalPoolProviderActivationDelegationSummary {
        delegation_id: r.delegation_id.clone(),
        delegation_digest: r.delegation_digest.clone(),
        delegation_material_digest: r.delegation_material_digest.clone(),
        provider_binding_id: d.provider_binding_id.clone(),
        provider_binding_digest: d.provider_binding_digest.clone(),
        registry_release_id: d.registry_release_id.clone(),
        registry_release_digest: d.registry_release_digest.clone(),
        route_adapter_projection_id: d.route_adapter_projection_id.clone(),
        provider_id: d.provider_id.clone(),
        provider_owner_account_id: d.provider_owner_account_id.clone(),
        provider_policy_revision: d.provider_policy_revision,
        provider_digest: d.provider_digest.clone(),
        provider_status: d.provider_status.clone(),
        logical_adapter_id: d.logical_adapter_id.clone(),
        release_version: d.release_version.clone(),
        adapter_config_revision: d.adapter_config_revision,
        adapter_config_digest: d.adapter_config_digest.clone(),
        service_actor_id: d.service_actor_id.clone(),
        service_actor_kind: d.service_actor_kind.clone(),
        issued_at: d.issued_at.clone(),
        sequence: d.sequence,
        delegation_effect: d.delegation_effect.clone(),
    }
}

impl StoredCandidate {
    pub(super) fn summary(&self) -> ExternalPoolProviderActivationCandidateSummary {
        candidate_summary(&self.receipt)
    }
}

pub(super) fn candidate_summary(
    r: &ExternalPoolProviderActivationCandidateReceipt,
) -> ExternalPoolProviderActivationCandidateSummary {
    let c = &r.candidate;
    ExternalPoolProviderActivationCandidateSummary {
        candidate_id: r.candidate_id.clone(),
        candidate_digest: r.candidate_digest.clone(),
        candidate_material_digest: r.candidate_material_digest.clone(),
        delegation_id: c.delegation_id.clone(),
        delegation_digest: c.delegation_digest.clone(),
        provider_binding_id: c.provider_binding_id.clone(),
        provider_binding_digest: c.provider_binding_digest.clone(),
        registry_release_id: c.registry_release_id.clone(),
        registry_release_digest: c.registry_release_digest.clone(),
        installation_receipt_id: c.installation_receipt_id.clone(),
        installation_receipt_digest: c.installation_receipt_digest.clone(),
        installation_content_digest: c.installation_content_digest.clone(),
        route_adapter_projection_id: c.route_adapter_projection_id.clone(),
        provider_id: c.provider_id.clone(),
        provider_owner_account_id: c.provider_owner_account_id.clone(),
        provider_policy_revision: c.provider_policy_revision,
        provider_digest: c.provider_digest.clone(),
        provider_status: c.provider_status.clone(),
        logical_adapter_id: c.logical_adapter_id.clone(),
        release_version: c.release_version.clone(),
        adapter_config_revision: c.adapter_config_revision,
        adapter_config_digest: c.adapter_config_digest.clone(),
        implementation_digest: c.implementation_digest.clone(),
        capability_set_digest: c.capability_set_digest.clone(),
        credential_verifier_digest: c.credential_verifier_digest.clone(),
        logical_adapter_binding_digest: c.logical_adapter_binding_digest.clone(),
        logical_projection_compatibility_digest: c.logical_projection_compatibility_digest.clone(),
        service_actor_id: c.service_actor_id.clone(),
        sequence: c.sequence,
        checked_at: c.checked_at.clone(),
        candidate_status: c.candidate_status.clone(),
        activation_closure_status: c.activation_closure_status.clone(),
        candidate_effect: c.candidate_effect.clone(),
    }
}

impl StoredRevocation {
    pub(super) fn summary(&self) -> ExternalPoolProviderActivationDelegationRevocationSummary {
        let r = &self.receipt;
        let v = &r.revocation;
        ExternalPoolProviderActivationDelegationRevocationSummary {
            revocation_id: r.revocation_id.clone(),
            revocation_digest: r.revocation_digest.clone(),
            revocation_material_digest: r.revocation_material_digest.clone(),
            delegation_id: v.delegation_id.clone(),
            delegation_digest: v.delegation_digest.clone(),
            candidate_id: v.candidate_id.clone(),
            candidate_digest: v.candidate_digest.clone(),
            provider_binding_id: v.provider_binding_id.clone(),
            provider_id: v.provider_id.clone(),
            reason: v.reason.clone(),
            revoked_at: v.revoked_at.clone(),
            revocation_effect: v.revocation_effect.clone(),
        }
    }
}

/// Store-only, non-Clone/non-Serde dynamic proof. Short-TTL heads never enter candidate storage.
pub(in crate::store) struct CurrentExternalPoolProviderActivationPreflightAuthority {
    pub(super) registry: CurrentExternalPoolAdapterRegistryProviderBindingAuthority,
    pub(super) vulnerability: CurrentExternalPoolAdapterVulnerabilityReattestationAuthority,
    pub(super) sandbox: CurrentExternalPoolAdapterSandboxReattestationAuthority,
    pub(super) credential: CurrentExternalPoolAdapterCredentialReattestationAuthority,
    pub(super) delegation: ExternalPoolProviderActivationDelegationReceipt,
    pub(super) candidate: ExternalPoolProviderActivationCandidateReceipt,
    pub(super) checked_at: String,
}

impl CurrentExternalPoolProviderActivationPreflightAuthority {
    pub(in crate::store) fn registry(
        &self,
    ) -> &CurrentExternalPoolAdapterRegistryProviderBindingAuthority {
        &self.registry
    }

    pub(in crate::store) fn vulnerability(
        &self,
    ) -> &CurrentExternalPoolAdapterVulnerabilityReattestationAuthority {
        &self.vulnerability
    }

    pub(in crate::store) fn sandbox(
        &self,
    ) -> &CurrentExternalPoolAdapterSandboxReattestationAuthority {
        &self.sandbox
    }

    pub(in crate::store) fn credential(
        &self,
    ) -> &CurrentExternalPoolAdapterCredentialReattestationAuthority {
        &self.credential
    }

    pub(in crate::store) fn delegation(&self) -> &ExternalPoolProviderActivationDelegationReceipt {
        &self.delegation
    }

    pub(in crate::store) fn candidate(&self) -> &ExternalPoolProviderActivationCandidateReceipt {
        &self.candidate
    }

    pub(in crate::store) fn checked_at(&self) -> &str {
        &self.checked_at
    }
}
