use serde::Serialize;

use crate::compute_federation::{
    external_pool_adapter_artifact_sandbox_conformance::ExternalPoolAdapterSandboxConformanceDraft,
    external_pool_adapter_sandbox_reattestation::{
        ExternalPoolAdapterSandboxReattestationReceipt,
        ExternalPoolAdapterSandboxReattestationRevocationReceipt,
    },
};

#[derive(Clone)]
pub(crate) struct GetExternalPoolAdapterSandboxReattestationChallenge {
    pub registry_release_id: String,
    pub expected_registry_release_digest: String,
    pub vulnerability_reattestation_receipt_id: String,
    pub expected_vulnerability_reattestation_receipt_digest: String,
    pub sandbox_verifier_key_record_id: String,
    pub expected_sandbox_verifier_key_record_digest: String,
    pub expected_sandbox_verifier_key_id: String,
    pub draft: ExternalPoolAdapterSandboxConformanceDraft,
}

pub(crate) struct CreateExternalPoolAdapterSandboxReattestation {
    pub challenge_id: String,
    pub expected_signature_message_digest: String,
    pub signature_base64: String,
    pub recorded_by_admin_user_id: String,
    pub confirmation: String,
    pub idempotency_scope: String,
    pub idempotency_key: String,
}

pub(crate) struct RevokeExternalPoolAdapterSandboxReattestation {
    pub reattestation_receipt_id: String,
    pub expected_reattestation_receipt_digest: String,
    pub revoked_by_admin_user_id: String,
    pub reason: String,
    pub confirmation: String,
    pub idempotency_scope: String,
    pub idempotency_key: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct ExternalPoolAdapterSandboxReattestationSummary {
    pub reattestation_receipt_id: String,
    pub reattestation_receipt_digest: String,
    pub reattestation_material_digest: String,
    pub challenge_id: String,
    pub registry_release_id: String,
    pub registry_release_digest: String,
    pub registry_release_material_digest: String,
    pub admission_id: String,
    pub admission_digest: String,
    pub package_receipt_id: String,
    pub package_receipt_digest: String,
    pub source_receipt_id: String,
    pub source_receipt_digest: String,
    pub adapter_id: String,
    pub release_version: String,
    pub implementation_digest: String,
    pub capability_set_digest: String,
    pub vulnerability_reattestation_receipt_id: String,
    pub vulnerability_reattestation_receipt_digest: String,
    pub vulnerability_reattestation_material_digest: String,
    pub vulnerability_intelligence_expires_at: String,
    pub sandbox_verifier_key_record_id: String,
    pub sandbox_verifier_key_record_digest: String,
    pub sandbox_verifier_key_id: String,
    pub sequence: u64,
    pub predecessor_receipt_id: Option<String>,
    pub predecessor_receipt_digest: Option<String>,
    pub verifier_report_id: String,
    pub sandbox_runtime_id: String,
    pub runtime_image_digest: String,
    pub run_started_at: String,
    pub run_completed_at: String,
    pub report_generated_at: String,
    pub report_expires_at: String,
    pub capability_count: u64,
    pub passed_capability_count: u64,
    pub policy_violation_count: u64,
    pub verified_at: String,
    pub evidence_scope: String,
    pub sandbox_reattestation_effect: String,
    pub adapter_effect: String,
    pub provider_effect: String,
    pub credential_effect: String,
    pub route_effect: String,
    pub execution_effect: String,
    pub settlement_effect: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct ExternalPoolAdapterSandboxReattestationRevocationSummary {
    pub revocation_receipt_id: String,
    pub revocation_receipt_digest: String,
    pub revocation_material_digest: String,
    pub reattestation_receipt_id: String,
    pub reattestation_receipt_digest: String,
    pub registry_release_id: String,
    pub registry_release_digest: String,
    pub reason: String,
    pub revoked_at: String,
    pub revocation_effect: String,
    pub adapter_effect: String,
    pub provider_effect: String,
    pub credential_effect: String,
    pub route_effect: String,
    pub execution_effect: String,
    pub settlement_effect: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct ExternalPoolAdapterSandboxReattestationWriteReceipt {
    pub reattestation: ExternalPoolAdapterSandboxReattestationSummary,
    pub replayed: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct ExternalPoolAdapterSandboxReattestationRevocationWriteReceipt {
    pub reattestation: ExternalPoolAdapterSandboxReattestationSummary,
    pub revocation: ExternalPoolAdapterSandboxReattestationRevocationSummary,
    pub replayed: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct ExternalPoolAdapterSandboxReattestationCurrentness {
    pub schema: &'static str,
    pub reattestation: ExternalPoolAdapterSandboxReattestationSummary,
    pub revocation: Option<ExternalPoolAdapterSandboxReattestationRevocationSummary>,
    pub current_status: String,
    pub head_status: String,
    pub registry_release_status: String,
    pub vulnerability_reattestation_status: String,
    pub sandbox_verifier_key_status: String,
    pub report_validity_status: String,
    pub revocation_status: String,
}

pub(super) struct StoredSandboxReattestation {
    pub receipt: ExternalPoolAdapterSandboxReattestationReceipt,
    pub receipt_json: String,
}

pub(super) struct StoredSandboxReattestationRevocation {
    pub receipt: ExternalPoolAdapterSandboxReattestationRevocationReceipt,
    pub receipt_json: String,
}

pub(in crate::store) struct HistoricalExternalPoolAdapterSandboxReattestationAuthority {
    receipt: ExternalPoolAdapterSandboxReattestationReceipt,
}

pub(in crate::store) struct CurrentExternalPoolAdapterSandboxReattestationAuthority {
    receipt: ExternalPoolAdapterSandboxReattestationReceipt,
    checked_at: String,
}

impl HistoricalExternalPoolAdapterSandboxReattestationAuthority {
    pub(super) fn new(receipt: ExternalPoolAdapterSandboxReattestationReceipt) -> Self {
        Self { receipt }
    }

    pub(in crate::store) fn receipt(&self) -> &ExternalPoolAdapterSandboxReattestationReceipt {
        &self.receipt
    }
}

impl CurrentExternalPoolAdapterSandboxReattestationAuthority {
    pub(super) fn new(
        receipt: ExternalPoolAdapterSandboxReattestationReceipt,
        checked_at: String,
    ) -> Self {
        Self {
            receipt,
            checked_at,
        }
    }

    pub(in crate::store) fn receipt(&self) -> &ExternalPoolAdapterSandboxReattestationReceipt {
        &self.receipt
    }

    pub(in crate::store) fn checked_at(&self) -> &str {
        &self.checked_at
    }
}

impl StoredSandboxReattestation {
    pub(super) fn summary(&self) -> ExternalPoolAdapterSandboxReattestationSummary {
        let receipt = &self.receipt;
        let item = &receipt.reattestation;
        let binding = &item.binding;
        ExternalPoolAdapterSandboxReattestationSummary {
            reattestation_receipt_id: receipt.reattestation_receipt_id.clone(),
            reattestation_receipt_digest: receipt.reattestation_receipt_digest.clone(),
            reattestation_material_digest: receipt.reattestation_material_digest.clone(),
            challenge_id: binding.challenge_id.clone(),
            registry_release_id: binding.registry_release_id.clone(),
            registry_release_digest: binding.registry_release_digest.clone(),
            registry_release_material_digest: binding.registry_release_material_digest.clone(),
            admission_id: binding.admission_id.clone(),
            admission_digest: binding.admission_digest.clone(),
            package_receipt_id: binding.package_receipt_id.clone(),
            package_receipt_digest: binding.package_receipt_digest.clone(),
            source_receipt_id: binding.source_receipt_id.clone(),
            source_receipt_digest: binding.source_receipt_digest.clone(),
            adapter_id: binding.adapter_id.clone(),
            release_version: binding.release_version.clone(),
            implementation_digest: binding.implementation_digest.clone(),
            capability_set_digest: binding.capability_set_digest.clone(),
            vulnerability_reattestation_receipt_id: binding
                .vulnerability_reattestation_receipt_id
                .clone(),
            vulnerability_reattestation_receipt_digest: binding
                .vulnerability_reattestation_receipt_digest
                .clone(),
            vulnerability_reattestation_material_digest: binding
                .vulnerability_reattestation_material_digest
                .clone(),
            vulnerability_intelligence_expires_at: binding
                .vulnerability_intelligence_expires_at
                .clone(),
            sandbox_verifier_key_record_id: binding.sandbox_verifier_key_record_id.clone(),
            sandbox_verifier_key_record_digest: binding.sandbox_verifier_key_record_digest.clone(),
            sandbox_verifier_key_id: binding.sandbox_verifier_key_id.clone(),
            sequence: binding.sequence,
            predecessor_receipt_id: binding.predecessor_receipt_id.clone(),
            predecessor_receipt_digest: binding.predecessor_receipt_digest.clone(),
            verifier_report_id: binding.verifier_report_id.clone(),
            sandbox_runtime_id: binding.sandbox_runtime_id.clone(),
            runtime_image_digest: binding.runtime_image_digest.clone(),
            run_started_at: binding.run_started_at.clone(),
            run_completed_at: binding.run_completed_at.clone(),
            report_generated_at: binding.report_generated_at.clone(),
            report_expires_at: binding.report_expires_at.clone(),
            capability_count: binding.supported_capabilities.len() as u64,
            passed_capability_count: binding.passed_capability_count,
            policy_violation_count: binding.policy_violation_count,
            verified_at: item.verified_at.clone(),
            evidence_scope: item.evidence_scope.clone(),
            sandbox_reattestation_effect: item.sandbox_reattestation_effect.clone(),
            adapter_effect: item.adapter_effect.clone(),
            provider_effect: item.provider_effect.clone(),
            credential_effect: item.credential_effect.clone(),
            route_effect: item.route_effect.clone(),
            execution_effect: item.execution_effect.clone(),
            settlement_effect: item.settlement_effect.clone(),
        }
    }
}

impl StoredSandboxReattestationRevocation {
    pub(super) fn summary(&self) -> ExternalPoolAdapterSandboxReattestationRevocationSummary {
        let receipt = &self.receipt;
        let item = &receipt.revocation;
        ExternalPoolAdapterSandboxReattestationRevocationSummary {
            revocation_receipt_id: receipt.revocation_receipt_id.clone(),
            revocation_receipt_digest: receipt.revocation_receipt_digest.clone(),
            revocation_material_digest: receipt.revocation_material_digest.clone(),
            reattestation_receipt_id: item.reattestation_receipt_id.clone(),
            reattestation_receipt_digest: item.reattestation_receipt_digest.clone(),
            registry_release_id: item.registry_release_id.clone(),
            registry_release_digest: item.registry_release_digest.clone(),
            reason: item.reason.clone(),
            revoked_at: item.revoked_at.clone(),
            revocation_effect: item.revocation_effect.clone(),
            adapter_effect: item.adapter_effect.clone(),
            provider_effect: item.provider_effect.clone(),
            credential_effect: item.credential_effect.clone(),
            route_effect: item.route_effect.clone(),
            execution_effect: item.execution_effect.clone(),
            settlement_effect: item.settlement_effect.clone(),
        }
    }
}
