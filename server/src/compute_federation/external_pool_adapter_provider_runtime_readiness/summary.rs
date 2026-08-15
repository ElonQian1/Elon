use serde::{Deserialize, Serialize};

use super::*;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterProviderRuntimeReadinessSafeSummary {
    pub schema: String,
    pub readiness_receipt_id: String,
    pub readiness_receipt_digest: String,
    pub readiness_material_digest: String,
    pub policy_id: String,
    pub policy_revision: u64,
    pub policy_digest: String,
    pub provider_binding_id: String,
    pub provider_binding_digest: String,
    pub registry_release_id: String,
    pub registry_release_digest: String,
    pub registry_release_material_digest: String,
    pub installation_receipt_id: String,
    pub installation_receipt_digest: String,
    pub installation_content_digest: String,
    pub candidate_id: String,
    pub candidate_digest: String,
    pub delegation_id: String,
    pub delegation_digest: String,
    pub profile_id: String,
    pub profile_digest: String,
    pub target_id: String,
    pub target_digest: String,
    pub companion_id: String,
    pub companion_digest: String,
    pub provider_id: String,
    pub provider_policy_revision: i64,
    pub provider_digest: String,
    pub provider_status: String,
    pub vulnerability_reattestation_receipt_id: String,
    pub vulnerability_reattestation_receipt_digest: String,
    pub sandbox_reattestation_receipt_id: String,
    pub sandbox_reattestation_receipt_digest: String,
    pub credential_reattestation_receipt_id: String,
    pub credential_reattestation_receipt_digest: String,
    pub runtime_compatibility_verification_receipt_id: String,
    pub runtime_compatibility_verification_receipt_digest: String,
    pub probe_checked_at: String,
    pub cleanup_completed_at: String,
    pub checked_at: String,
    pub expires_at: String,
    pub sequence: u64,
    pub predecessor_readiness_receipt_id: Option<String>,
    pub predecessor_readiness_receipt_digest: Option<String>,
    pub evidence_scope: String,
    pub receipt_status: String,
    pub effects: ExternalPoolAdapterProviderRuntimeReadinessEffects,
    pub observed_readiness: ExternalPoolAdapterProviderRuntimeReadinessObservedReadiness,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterProviderRuntimeReadinessCurrentnessSummary {
    pub schema: String,
    pub readiness: ExternalPoolAdapterProviderRuntimeReadinessSafeSummary,
    pub currentness_status: String,
    pub head_status: String,
    pub provider_binding_status: String,
    pub provider_status: String,
    pub candidate_status: String,
    pub profile_status: String,
    pub target_status: String,
    pub companion_status: String,
    pub vulnerability_reattestation_status: String,
    pub sandbox_reattestation_status: String,
    pub credential_reattestation_status: String,
    pub runtime_compatibility_verification_status: String,
    pub runtime_custody_epoch_status: String,
    pub runtime_bundle_identity_status: String,
    pub ttl_status: String,
    pub revocation_status: String,
    pub checked_at: String,
    pub effects: ExternalPoolAdapterProviderRuntimeReadinessEffects,
    pub current_readiness: ExternalPoolAdapterProviderRuntimeReadinessObservedReadiness,
}

pub(crate) fn provider_runtime_readiness_safe_summary(
    receipt: &ExternalPoolAdapterProviderRuntimeReadinessReceipt,
) -> ExternalPoolAdapterProviderRuntimeReadinessSafeSummary {
    let value = &receipt.readiness;
    ExternalPoolAdapterProviderRuntimeReadinessSafeSummary {
        schema: PROVIDER_RUNTIME_READINESS_SUMMARY_SCHEMA.into(),
        readiness_receipt_id: receipt.readiness_receipt_id.clone(),
        readiness_receipt_digest: receipt.readiness_receipt_digest.clone(),
        readiness_material_digest: receipt.readiness_material_digest.clone(),
        policy_id: value.policy_id.clone(),
        policy_revision: value.policy_revision,
        policy_digest: value.policy_digest.clone(),
        provider_binding_id: value.provider_binding_id.clone(),
        provider_binding_digest: value.provider_binding_digest.clone(),
        registry_release_id: value.registry_release_id.clone(),
        registry_release_digest: value.registry_release_digest.clone(),
        registry_release_material_digest: value.registry_release_material_digest.clone(),
        installation_receipt_id: value.installation_receipt_id.clone(),
        installation_receipt_digest: value.installation_receipt_digest.clone(),
        installation_content_digest: value.installation_content_digest.clone(),
        candidate_id: value.candidate_id.clone(),
        candidate_digest: value.candidate_digest.clone(),
        delegation_id: value.delegation_id.clone(),
        delegation_digest: value.delegation_digest.clone(),
        profile_id: value.profile_id.clone(),
        profile_digest: value.profile_digest.clone(),
        target_id: value.target_id.clone(),
        target_digest: value.target_digest.clone(),
        companion_id: value.companion_id.clone(),
        companion_digest: value.companion_digest.clone(),
        provider_id: value.provider_id.clone(),
        provider_policy_revision: value.provider_policy_revision,
        provider_digest: value.provider_digest.clone(),
        provider_status: value.provider_status.clone(),
        vulnerability_reattestation_receipt_id: value
            .vulnerability_reattestation_receipt_id
            .clone(),
        vulnerability_reattestation_receipt_digest: value
            .vulnerability_reattestation_receipt_digest
            .clone(),
        sandbox_reattestation_receipt_id: value.sandbox_reattestation_receipt_id.clone(),
        sandbox_reattestation_receipt_digest: value.sandbox_reattestation_receipt_digest.clone(),
        credential_reattestation_receipt_id: value.credential_reattestation_receipt_id.clone(),
        credential_reattestation_receipt_digest: value
            .credential_reattestation_receipt_digest
            .clone(),
        runtime_compatibility_verification_receipt_id: value
            .runtime_compatibility_verification_receipt_id
            .clone(),
        runtime_compatibility_verification_receipt_digest: value
            .runtime_compatibility_verification_receipt_digest
            .clone(),
        probe_checked_at: value.probe_checked_at.clone(),
        cleanup_completed_at: value.cleanup_completed_at.clone(),
        checked_at: value.checked_at.clone(),
        expires_at: value.expires_at.clone(),
        sequence: value.sequence,
        predecessor_readiness_receipt_id: value.predecessor_readiness_receipt_id.clone(),
        predecessor_readiness_receipt_digest: value.predecessor_readiness_receipt_digest.clone(),
        evidence_scope: value.evidence_scope.clone(),
        receipt_status: value.receipt_status.clone(),
        effects: value.effects.clone(),
        observed_readiness: value.observed_readiness.clone(),
    }
}
