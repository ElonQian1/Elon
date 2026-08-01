use serde::{Deserialize, Serialize};

pub(crate) const SUI_CORRECTION_PROJECTION_SCHEMA: &str =
    "task_economy.sui_correction_projection.v1";
pub(crate) const SUI_CORRECTION_PACKAGE_SCHEMA: &str =
    "task_economy.sui_correction_projection_package.v1";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct SuiCorrectionProjectionLeg {
    pub receipt_id: String,
    pub receipt_kind: String,
    pub intent_id: String,
    pub posting_key: String,
    pub compute_amount_micros: i64,
    pub provider_amount_micros: i64,
    pub platform_amount_micros: i64,
    pub currency: String,
    pub source_receipt_digest: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct SuiCorrectionProjectionEnvelope {
    pub schema: String,
    pub correction_id: String,
    pub correction_matter_id: String,
    pub original_receipt_id: String,
    pub project_object_key: String,
    pub reversal: SuiCorrectionProjectionLeg,
    pub replacement: SuiCorrectionProjectionLeg,
    pub shadow_only: bool,
    pub atomic_bundle: bool,
    pub ptb_steps: Vec<String>,
    pub network_submission: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct SuiCorrectionProjectionPackage {
    pub id: String,
    pub project_id: String,
    pub correction_id: String,
    pub reversal_receipt_id: String,
    pub replacement_receipt_id: String,
    pub target_network: String,
    pub package_schema: String,
    pub projection_digest: String,
    pub source_bundle_digest: String,
    pub envelope: SuiCorrectionProjectionEnvelope,
    pub integrity_status: String,
    pub submission_readiness: String,
    pub network_submission: String,
    pub submission_attempts: i64,
    pub last_error: Option<String>,
    pub created_by_user_id: String,
    pub verified_at: String,
    pub created_at: String,
    pub updated_at: String,
}

pub(crate) struct CreateSuiCorrectionProjectionPackage<'a> {
    pub project_id: &'a str,
    pub correction_id: &'a str,
    pub reversal_receipt_id: &'a str,
    pub replacement_receipt_id: &'a str,
    pub target_network: &'a str,
    pub package_schema: &'a str,
    pub projection_digest: &'a str,
    pub source_bundle_digest: &'a str,
    pub envelope_json: &'a str,
    pub created_by_user_id: &'a str,
}
