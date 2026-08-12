use serde::{Deserialize, Serialize};

pub(crate) const ARTIFACT_SIGNATURE_BINDING_SCHEMA: &str =
    "compute_federation.external_pool_adapter_artifact_signature_binding.v1";
pub(crate) const ARTIFACT_SIGNATURE_CHALLENGE_SCHEMA: &str =
    "compute_federation.external_pool_adapter_artifact_signature_challenge.v1";
pub(crate) const ARTIFACT_SIGNED_PROVENANCE_RECEIPT_SCHEMA: &str =
    "compute_federation.external_pool_adapter_artifact_signed_provenance_receipt.v1";
pub(crate) const ARTIFACT_SIGNED_PROVENANCE_CURRENTNESS_SCHEMA: &str =
    "compute_federation.external_pool_adapter_artifact_signed_provenance_currentness.v1";
pub(crate) const ARTIFACT_SIGNED_PROVENANCE_CANONICALIZATION: &str = "rfc8785_jcs";
pub(crate) const ARTIFACT_SIGNED_PROVENANCE_DIGEST_ALGORITHM: &str = "sha256";
pub(crate) const ARTIFACT_SIGNED_PROVENANCE_SIGNATURE_ALGORITHM: &str = "rsa-pkcs1v15-sha256";
pub(crate) const ARTIFACT_SIGNED_PROVENANCE_CONFIRMATION: &str =
    "confirm_external_pool_adapter_artifact_signed_provenance";
pub(crate) const ARTIFACT_SIGNED_PROVENANCE_EVIDENCE_SCOPE: &str =
    "rsa_signature_over_exact_artifact_binding";
pub(crate) const ARTIFACT_SIGNED_PROVENANCE_CURRENT: &str = "verified_current";
pub(crate) const ARTIFACT_SIGNED_PROVENANCE_HISTORICAL: &str = "historical_only";
pub(crate) const ARTIFACT_SIGNED_PROVENANCE_NO_EFFECT: &str = "none";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterArtifactSignatureBinding {
    pub schema: String,
    pub admission_id: String,
    pub admission_digest: String,
    pub adapter_id: String,
    pub release_version: String,
    pub candidate_artifact_ref_digest: String,
    pub source_receipt_id: String,
    pub source_receipt_digest: String,
    pub artifact_sha256: String,
    pub artifact_size_bytes: u64,
    pub key_record_id: String,
    pub key_record_digest: String,
    pub key_id: String,
    pub source_operator: String,
    pub signature_algorithm: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct ExternalPoolAdapterArtifactSignatureChallengeReceipt {
    pub schema: &'static str,
    pub canonicalization: &'static str,
    pub digest_algorithm: &'static str,
    pub signature_algorithm: &'static str,
    pub signature_message_base64: String,
    pub signature_message_digest: String,
    pub binding: ExternalPoolAdapterArtifactSignatureBinding,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterArtifactSignedProvenanceMaterial {
    pub binding: ExternalPoolAdapterArtifactSignatureBinding,
    pub signature_message_digest: String,
    pub signature_base64: String,
    pub signature_digest: String,
    pub verified_by_admin_user_id: String,
    pub confirmation: String,
    pub idempotency_scope: String,
    pub idempotency_key: String,
    pub verified_at: String,
    pub recorded_at: String,
    pub evidence_scope: String,
    pub artifact_ref_resolution_effect: String,
    pub artifact_format_effect: String,
    pub conformance_effect: String,
    pub adapter_effect: String,
    pub route_effect: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterArtifactSignedProvenanceReceipt {
    pub schema: String,
    pub provenance_receipt_id: String,
    pub provenance_receipt_digest: String,
    pub verification_material_digest: String,
    pub canonicalization: String,
    pub digest_algorithm: String,
    pub provenance: ExternalPoolAdapterArtifactSignedProvenanceMaterial,
}
