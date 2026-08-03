use serde::{Deserialize, Serialize};

pub(crate) const ERASURE_EVIDENCE_KIND_EXTERNAL_RECEIPT: &str = "external_system_receipt";
pub(crate) const ERASURE_EVIDENCE_KIND_MERCHANT_ATTESTATION: &str = "merchant_attestation";
pub(crate) const ERASURE_EVIDENCE_SOURCE_AUTHORITY: &str = "merchant_supplied_unverified";

#[derive(Debug, Clone, Serialize)]
pub(crate) struct OpenCommerceDataErasureEvidence {
    pub id: String,
    pub data_request_id: String,
    pub merchant_id: String,
    pub evidence_kind: String,
    pub external_system: String,
    pub reference_id: String,
    pub receipt_sha256: String,
    pub summary: String,
    pub source_authority: &'static str,
    pub platform_verified: bool,
    pub created_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CreateDataErasureEvidenceRequest {
    pub evidence_kind: String,
    pub external_system: String,
    pub reference_id: String,
    pub receipt_sha256: String,
    pub summary: String,
    #[serde(default)]
    pub merchant_confirmed_unverified: bool,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct OpenCommerceDataErasureEvidenceList {
    pub schema: &'static str,
    pub evidence: Vec<OpenCommerceDataErasureEvidence>,
    pub boundary: Vec<&'static str>,
}
