use serde::{Deserialize, Serialize};

use crate::open_commerce_portability_model::ConsumerPortabilityExport;

pub(crate) const CONSUMER_PORTABILITY_IMPORT_SCHEMA: &str =
    "open_commerce.consumer_portability_import.v1";
pub(crate) const CONSUMER_PORTABILITY_IMPORT_TRUST_STATUS: &str =
    "integrity_verified_source_untrusted";
pub(crate) const CONSUMER_PORTABILITY_IMPORT_TRUSTED_STATUS: &str =
    "trusted_operator_signature_verified";
pub(crate) const CONSUMER_PORTABILITY_IMPORT_MERGE_STATUS: &str = "isolated_snapshot";
pub(crate) const CONSUMER_PORTABILITY_SIGNATURE_ALGORITHM: &str = "rsa-pkcs1v15-sha256";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ConsumerPortabilityPackageSignature {
    pub algorithm: String,
    pub key_id: String,
    pub signature_base64: String,
}

#[derive(Debug, Clone)]
pub(crate) struct VerifiedConsumerPortabilitySignature {
    pub key_record_id: String,
    pub signature: ConsumerPortabilityPackageSignature,
    pub verified_at: String,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct CreateConsumerPortabilityImportRequest {
    pub source_operator: String,
    pub package: ConsumerPortabilityExport,
    #[serde(default)]
    pub signature: Option<ConsumerPortabilityPackageSignature>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ConsumerPortabilityImport {
    pub schema: String,
    pub id: String,
    pub destination_project_id: String,
    pub source_operator: String,
    pub source_project_id: String,
    pub source_package_id: String,
    pub source_package_schema: String,
    pub envelope_sha256: String,
    pub payload_sha256: String,
    pub package_json: String,
    pub package: ConsumerPortabilityExport,
    pub trust_status: String,
    pub merge_status: String,
    #[serde(skip)]
    pub(crate) signer_key_record_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signature: Option<ConsumerPortabilityPackageSignature>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signature_verified_at: Option<String>,
    pub imported_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ConsumerPortabilityImportSummary {
    pub id: String,
    pub source_operator: String,
    pub source_project_id: String,
    pub source_package_id: String,
    pub source_package_schema: String,
    pub envelope_sha256: String,
    pub payload_sha256: String,
    pub relationship_count: usize,
    pub data_request_count: usize,
    pub preference_profile_included: bool,
    pub invocation_receipt_count: usize,
    pub merchant_identity_claim_count: usize,
    pub trust_status: String,
    pub merge_status: String,
    pub signer_key_id: Option<String>,
    pub signature_verified_at: Option<String>,
    pub imported_at: String,
}

impl ConsumerPortabilityImport {
    pub(crate) fn summary(&self) -> ConsumerPortabilityImportSummary {
        ConsumerPortabilityImportSummary {
            id: self.id.clone(),
            source_operator: self.source_operator.clone(),
            source_project_id: self.source_project_id.clone(),
            source_package_id: self.source_package_id.clone(),
            source_package_schema: self.source_package_schema.clone(),
            envelope_sha256: self.envelope_sha256.clone(),
            payload_sha256: self.payload_sha256.clone(),
            relationship_count: self.package.payload.relationships.len(),
            data_request_count: self.package.payload.data_requests.len(),
            preference_profile_included: self.package.payload.preference_profile.is_some(),
            invocation_receipt_count: self.package.payload.invocation_receipts.len(),
            merchant_identity_claim_count: self.package.payload.merchant_identity_claims.len(),
            trust_status: self.trust_status.clone(),
            merge_status: self.merge_status.clone(),
            signer_key_id: self.signature.as_ref().map(|value| value.key_id.clone()),
            signature_verified_at: self.signature_verified_at.clone(),
            imported_at: self.imported_at.clone(),
        }
    }
}
