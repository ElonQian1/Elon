use serde::{Deserialize, Serialize};

use crate::{
    open_commerce_consumer_preference_model::{
        ConsumerPreferenceDisclosure, ConsumerPreferenceProfile,
    },
    open_commerce_data_request_model::OpenCommerceConsumerDataRequest,
    open_commerce_relationship_model::OpenCommerceConsumerRelationship,
};

pub(crate) const CONSUMER_PORTABILITY_EXPORT_SCHEMA: &str =
    "open_commerce.consumer_portability_export.v4";
pub(crate) const CONSUMER_PORTABILITY_PAYLOAD_SCHEMA: &str =
    "open_commerce.consumer_portability_payload.v4";
pub(crate) const CONSUMER_PORTABILITY_EXPORT_SCHEMA_V3: &str =
    "open_commerce.consumer_portability_export.v3";
pub(crate) const CONSUMER_PORTABILITY_PAYLOAD_SCHEMA_V3: &str =
    "open_commerce.consumer_portability_payload.v3";
pub(crate) const CONSUMER_PORTABILITY_EXPORT_SCHEMA_V2: &str =
    "open_commerce.consumer_portability_export.v2";
pub(crate) const CONSUMER_PORTABILITY_PAYLOAD_SCHEMA_V2: &str =
    "open_commerce.consumer_portability_payload.v2";
pub(crate) const CONSUMER_PORTABILITY_EXPORT_SCHEMA_V1: &str =
    "open_commerce.consumer_portability_export.v1";
pub(crate) const CONSUMER_PORTABILITY_PAYLOAD_SCHEMA_V1: &str =
    "open_commerce.consumer_portability_payload.v1";

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct CreateConsumerPortabilityExportRequest {
    pub idempotency_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct ConsumerRelationshipRenewalLink {
    pub source_relationship_id: String,
    pub renewed_relationship_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ConsumerPortableInvocationReceipt {
    pub schema: String,
    pub payload_sha256: String,
    pub payload_json: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ConsumerPortableMerchantIdentityClaim {
    pub source_merchant_id: String,
    pub key_ids: Vec<String>,
    pub authority: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ConsumerPortabilityPayload {
    pub schema: String,
    pub source_project_id: String,
    pub generated_at: String,
    pub relationships: Vec<OpenCommerceConsumerRelationship>,
    pub relationship_renewals: Vec<ConsumerRelationshipRenewalLink>,
    pub data_requests: Vec<OpenCommerceConsumerDataRequest>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preference_profile: Option<ConsumerPreferenceProfile>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub preference_disclosures: Vec<ConsumerPreferenceDisclosure>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub invocation_receipt_scope: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub invocation_receipts: Vec<ConsumerPortableInvocationReceipt>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub merchant_identity_claims: Vec<ConsumerPortableMerchantIdentityClaim>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ConsumerPortabilityExport {
    pub schema: String,
    pub id: String,
    pub source_project_id: String,
    pub idempotency_key: String,
    pub payload_sha256: String,
    pub payload_json: String,
    pub payload: ConsumerPortabilityPayload,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ConsumerPortabilityExportSummary {
    pub id: String,
    pub source_project_id: String,
    pub idempotency_key: String,
    pub payload_sha256: String,
    pub relationship_count: usize,
    pub renewal_count: usize,
    pub data_request_count: usize,
    pub preference_profile_included: bool,
    pub preference_disclosure_count: usize,
    pub invocation_receipt_count: usize,
    pub merchant_identity_claim_count: usize,
    pub created_at: String,
}

impl ConsumerPortabilityExport {
    pub(crate) fn summary(&self) -> ConsumerPortabilityExportSummary {
        ConsumerPortabilityExportSummary {
            id: self.id.clone(),
            source_project_id: self.source_project_id.clone(),
            idempotency_key: self.idempotency_key.clone(),
            payload_sha256: self.payload_sha256.clone(),
            relationship_count: self.payload.relationships.len(),
            renewal_count: self.payload.relationship_renewals.len(),
            data_request_count: self.payload.data_requests.len(),
            preference_profile_included: self.payload.preference_profile.is_some(),
            preference_disclosure_count: self.payload.preference_disclosures.len(),
            invocation_receipt_count: self.payload.invocation_receipts.len(),
            merchant_identity_claim_count: self.payload.merchant_identity_claims.len(),
            created_at: self.created_at.clone(),
        }
    }
}
