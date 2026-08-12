use serde::Serialize;

use crate::compute_federation::external_pool_adapter_artifact_sandbox_conformance::{
    ExternalPoolAdapterSandboxConformanceDraft, ExternalPoolAdapterSandboxConformanceReceipt,
};

#[derive(Clone)]
pub(crate) struct GetExternalPoolAdapterSandboxConformanceChallenge {
    pub admission_id: String,
    pub expected_vulnerability_report_receipt_digest: String,
    pub sandbox_verifier_key_record_id: String,
    pub expected_sandbox_verifier_key_record_digest: String,
    pub expected_sandbox_verifier_key_id: String,
    pub draft: ExternalPoolAdapterSandboxConformanceDraft,
}

pub(crate) struct CreateExternalPoolAdapterSandboxConformance {
    pub challenge: GetExternalPoolAdapterSandboxConformanceChallenge,
    pub expected_signature_message_digest: String,
    pub signature_base64: String,
    pub verified_by_admin_user_id: String,
    pub confirmation: String,
    pub idempotency_scope: String,
    pub idempotency_key: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct ExternalPoolAdapterSandboxConformanceSummary {
    pub sandbox_conformance_receipt_id: String,
    pub sandbox_conformance_receipt_digest: String,
    pub conformance_material_digest: String,
    pub admission_id: String,
    pub adapter_id: String,
    pub release_version: String,
    pub vulnerability_report_receipt_id: String,
    pub vulnerability_report_receipt_digest: String,
    pub sandbox_verifier_key_record_id: String,
    pub sandbox_verifier_key_record_digest: String,
    pub sandbox_verifier_key_id: String,
    pub sandbox_verifier_operator: String,
    pub sandbox_verifier_product: String,
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
    pub signature_message_digest: String,
    pub signature_digest: String,
    pub verified_by_admin_user_id: String,
    pub verified_at: String,
    pub evidence_scope: String,
    pub conformance_effect: String,
    pub credential_effect: String,
    pub adapter_effect: String,
    pub route_effect: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct ExternalPoolAdapterSandboxConformanceWriteReceipt {
    pub sandbox_conformance: ExternalPoolAdapterSandboxConformanceSummary,
    pub replayed: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct ExternalPoolAdapterSandboxConformanceCurrentness {
    pub schema: &'static str,
    pub sandbox_conformance: ExternalPoolAdapterSandboxConformanceSummary,
    pub current_status: String,
    pub vulnerability_report_status: String,
    pub sandbox_verifier_key_status: String,
    pub report_validity_status: String,
}

pub(super) struct StoredExternalPoolAdapterSandboxConformance {
    pub receipt: ExternalPoolAdapterSandboxConformanceReceipt,
    pub receipt_json: String,
}

impl StoredExternalPoolAdapterSandboxConformance {
    pub(super) fn summary(&self) -> ExternalPoolAdapterSandboxConformanceSummary {
        let item = &self.receipt.conformance;
        let binding = &item.binding;
        ExternalPoolAdapterSandboxConformanceSummary {
            sandbox_conformance_receipt_id: self.receipt.sandbox_conformance_receipt_id.clone(),
            sandbox_conformance_receipt_digest: self
                .receipt
                .sandbox_conformance_receipt_digest
                .clone(),
            conformance_material_digest: self.receipt.conformance_material_digest.clone(),
            admission_id: binding.admission_id.clone(),
            adapter_id: binding.adapter_id.clone(),
            release_version: binding.release_version.clone(),
            vulnerability_report_receipt_id: binding.vulnerability_report_receipt_id.clone(),
            vulnerability_report_receipt_digest: binding
                .vulnerability_report_receipt_digest
                .clone(),
            sandbox_verifier_key_record_id: binding.sandbox_verifier_key_record_id.clone(),
            sandbox_verifier_key_record_digest: binding.sandbox_verifier_key_record_digest.clone(),
            sandbox_verifier_key_id: binding.sandbox_verifier_key_id.clone(),
            sandbox_verifier_operator: binding.sandbox_verifier_operator.clone(),
            sandbox_verifier_product: binding.sandbox_verifier_product.clone(),
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
            signature_message_digest: item.signature_message_digest.clone(),
            signature_digest: item.signature_digest.clone(),
            verified_by_admin_user_id: item.verified_by_admin_user_id.clone(),
            verified_at: item.verified_at.clone(),
            evidence_scope: item.evidence_scope.clone(),
            conformance_effect: item.conformance_effect.clone(),
            credential_effect: item.credential_effect.clone(),
            adapter_effect: item.adapter_effect.clone(),
            route_effect: item.route_effect.clone(),
        }
    }
}

pub(super) fn write_receipt(
    stored: &StoredExternalPoolAdapterSandboxConformance,
    replayed: bool,
) -> ExternalPoolAdapterSandboxConformanceWriteReceipt {
    ExternalPoolAdapterSandboxConformanceWriteReceipt {
        sandbox_conformance: stored.summary(),
        replayed,
    }
}
