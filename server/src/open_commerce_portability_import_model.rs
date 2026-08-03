use serde::{Deserialize, Serialize};

use crate::open_commerce_portability_model::ConsumerPortabilityExport;

pub(crate) const CONSUMER_PORTABILITY_IMPORT_SCHEMA: &str =
    "open_commerce.consumer_portability_import.v1";
pub(crate) const CONSUMER_PORTABILITY_IMPORT_TRUST_STATUS: &str =
    "integrity_verified_source_untrusted";
pub(crate) const CONSUMER_PORTABILITY_IMPORT_MERGE_STATUS: &str = "isolated_snapshot";

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct CreateConsumerPortabilityImportRequest {
    pub source_operator: String,
    pub package: ConsumerPortabilityExport,
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
    pub trust_status: String,
    pub merge_status: String,
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
            trust_status: self.trust_status.clone(),
            merge_status: self.merge_status.clone(),
            imported_at: self.imported_at.clone(),
        }
    }
}
