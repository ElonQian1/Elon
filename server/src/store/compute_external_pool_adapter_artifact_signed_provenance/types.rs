use serde::Serialize;

use crate::compute_federation::external_pool_adapter_artifact_signed_provenance::{
    ExternalPoolAdapterArtifactSignatureBinding, ExternalPoolAdapterArtifactSignedProvenanceReceipt,
};

pub(crate) struct CreateExternalPoolAdapterArtifactSignedProvenance {
    pub admission_id: String,
    pub expected_admission_digest: String,
    pub expected_source_receipt_digest: String,
    pub key_record_id: String,
    pub expected_key_record_digest: String,
    pub expected_key_id: String,
    pub expected_signature_message_digest: String,
    pub signature_base64: String,
    pub verified_by_admin_user_id: String,
    pub confirmation: String,
    pub idempotency_scope: String,
    pub idempotency_key: String,
}

pub(crate) struct GetExternalPoolAdapterArtifactSignatureChallenge {
    pub admission_id: String,
    pub expected_admission_digest: String,
    pub expected_source_receipt_digest: String,
    pub key_record_id: String,
    pub expected_key_record_digest: String,
    pub expected_key_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct ExternalPoolAdapterArtifactSignedProvenanceSummary {
    pub provenance_receipt_id: String,
    pub provenance_receipt_digest: String,
    pub verification_material_digest: String,
    pub binding: ExternalPoolAdapterArtifactSignatureBinding,
    pub signature_message_digest: String,
    pub signature_digest: String,
    pub verified_by_admin_user_id: String,
    pub verified_at: String,
    pub evidence_scope: String,
    pub artifact_ref_resolution_effect: String,
    pub artifact_format_effect: String,
    pub conformance_effect: String,
    pub adapter_effect: String,
    pub route_effect: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct ExternalPoolAdapterArtifactSignedProvenanceWriteReceipt {
    pub provenance: ExternalPoolAdapterArtifactSignedProvenanceSummary,
    pub replayed: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct ExternalPoolAdapterArtifactSignedProvenanceCurrentnessReceipt {
    pub schema: &'static str,
    pub provenance: ExternalPoolAdapterArtifactSignedProvenanceSummary,
    pub current_status: String,
    pub admission_current_status: String,
    pub signer_current_status: String,
}

pub(super) struct StoredSignedProvenanceReceipt {
    pub receipt: ExternalPoolAdapterArtifactSignedProvenanceReceipt,
    pub receipt_json: String,
}

impl StoredSignedProvenanceReceipt {
    pub(super) fn summary(&self) -> ExternalPoolAdapterArtifactSignedProvenanceSummary {
        let material = &self.receipt.provenance;
        ExternalPoolAdapterArtifactSignedProvenanceSummary {
            provenance_receipt_id: self.receipt.provenance_receipt_id.clone(),
            provenance_receipt_digest: self.receipt.provenance_receipt_digest.clone(),
            verification_material_digest: self.receipt.verification_material_digest.clone(),
            binding: material.binding.clone(),
            signature_message_digest: material.signature_message_digest.clone(),
            signature_digest: material.signature_digest.clone(),
            verified_by_admin_user_id: material.verified_by_admin_user_id.clone(),
            verified_at: material.verified_at.clone(),
            evidence_scope: material.evidence_scope.clone(),
            artifact_ref_resolution_effect: material.artifact_ref_resolution_effect.clone(),
            artifact_format_effect: material.artifact_format_effect.clone(),
            conformance_effect: material.conformance_effect.clone(),
            adapter_effect: material.adapter_effect.clone(),
            route_effect: material.route_effect.clone(),
        }
    }
}
