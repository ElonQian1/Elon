use serde::{Deserialize, Serialize};

use super::types::{
    ExternalPoolAdapterRuntimeCompatibilityEffects,
    ExternalPoolAdapterRuntimeCompatibilityReadiness,
};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterRuntimeCompatibilityRevocationMaterial {
    pub verification_receipt_id: String,
    pub verification_receipt_digest: String,
    pub registry_release_id: String,
    pub registry_release_digest: String,
    pub revoked_by_admin_user_id: String,
    pub reason: String,
    pub confirmation: String,
    pub idempotency_scope: String,
    pub idempotency_key: String,
    pub revoked_at: String,
    pub recorded_at: String,
    pub revocation_status: String,
    pub effects: ExternalPoolAdapterRuntimeCompatibilityEffects,
    pub readiness: ExternalPoolAdapterRuntimeCompatibilityReadiness,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterRuntimeCompatibilityRevocationReceipt {
    pub schema: String,
    pub revocation_receipt_id: String,
    pub revocation_receipt_digest: String,
    pub revocation_material_digest: String,
    pub canonicalization: String,
    pub digest_algorithm: String,
    pub revocation: ExternalPoolAdapterRuntimeCompatibilityRevocationMaterial,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterRuntimeCompatibilityCurrentnessSummary {
    pub schema: String,
    pub registry_release_id: String,
    pub adapter_id: String,
    pub release_version: String,
    pub profile_id: String,
    pub profile_revision: u64,
    pub verification_receipt_id: String,
    pub verification_receipt_digest: String,
    pub sequence: u64,
    pub verified_at: String,
    pub expires_at: String,
    pub revoked_at: Option<String>,
    pub currentness_status: String,
    pub historical_reasons: Vec<String>,
    pub effects: ExternalPoolAdapterRuntimeCompatibilityEffects,
    pub readiness: ExternalPoolAdapterRuntimeCompatibilityReadiness,
}
