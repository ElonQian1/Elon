use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::open_commerce_merchant_evidence_model::MerchantBusinessEvidenceSummary;

pub(crate) const ADAPTER_HANDOFF_CLAIM_SCHEMA: &str =
    "open_commerce.adapter_business_handoff_claim.v1";
pub(crate) const ADAPTER_HANDOFF_CLAIM_POLL_SCHEMA: &str =
    "open_commerce.adapter_business_handoff_claim_poll.v1";
pub(crate) const ADAPTER_HANDOFF_CLAIM_LIST_SCHEMA: &str =
    "open_commerce.adapter_business_handoff_claim_list.v1";
pub(crate) const ADAPTER_HANDOFF_CLAIM_RELEASE_SCHEMA: &str =
    "open_commerce.adapter_business_handoff_claim_release.v1";
pub(crate) const ADAPTER_HANDOFF_CLAIM_RESUME_SCHEMA: &str =
    "open_commerce.adapter_business_handoff_claim_resume.v1";
pub(crate) const ADAPTER_HANDOFF_CLAIM_RENEW_SCHEMA: &str =
    "open_commerce.adapter_business_handoff_claim_renew.v1";

#[derive(Debug, Clone, Serialize)]
pub(crate) struct OpenCommerceAdapterHandoffClaim {
    pub schema: &'static str,
    pub id: String,
    pub project_id: String,
    pub merchant_id: String,
    pub invocation_id: String,
    pub integration_id: String,
    pub adapter_credential_id: String,
    pub adapter_credential_version: i64,
    pub attempt_no: i64,
    pub status: String,
    pub lease_token_hint: String,
    pub lease_expires_at: String,
    pub lease_deadline_at: String,
    pub release_reason_code: Option<String>,
    pub released_at: Option<String>,
    pub completion_status: Option<String>,
    pub retry_not_before: Option<String>,
    pub retry_suspended_at: Option<String>,
    pub retry_suspension_reason: Option<String>,
    pub retry_resumed_at: Option<String>,
    pub completed_receipt_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct OpenCommerceAdapterHandoffTask {
    pub evidence: MerchantBusinessEvidenceSummary,
    pub result: Value,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct OpenCommerceAdapterHandoffClaimIssue {
    pub claim: OpenCommerceAdapterHandoffClaim,
    pub lease_token: String,
    pub lease_token_visible_once: bool,
    pub task: OpenCommerceAdapterHandoffTask,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct OpenCommerceAdapterHandoffClaimPoll {
    pub schema: &'static str,
    pub claimed: bool,
    pub issue: Option<OpenCommerceAdapterHandoffClaimIssue>,
    pub retry_after_seconds: i64,
    pub boundary: Vec<&'static str>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct OpenCommerceAdapterHandoffClaimList {
    pub schema: &'static str,
    pub project_id: String,
    pub claims: Vec<OpenCommerceAdapterHandoffClaim>,
    pub boundary: Vec<&'static str>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct OpenCommerceAdapterHandoffClaimRelease {
    pub schema: &'static str,
    pub claim: OpenCommerceAdapterHandoffClaim,
    pub retryable: bool,
    pub boundary: Vec<&'static str>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct OpenCommerceAdapterHandoffClaimResume {
    pub schema: &'static str,
    pub claim: OpenCommerceAdapterHandoffClaim,
    pub resumed: bool,
    pub funds_moved: bool,
    pub boundary: Vec<&'static str>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct OpenCommerceAdapterHandoffClaimRenew {
    pub schema: &'static str,
    pub claim: OpenCommerceAdapterHandoffClaim,
    pub renewed: bool,
    pub boundary: Vec<&'static str>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ClaimAdapterHandoffRequest {
    #[serde(default = "default_lease_seconds")]
    pub lease_seconds: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CompleteAdapterHandoffClaimRequest {
    pub lease_token: String,
    pub receipt_key: String,
    pub status: String,
    pub target_domain: String,
    #[serde(default)]
    pub target_reference: Option<String>,
    #[serde(default)]
    pub error_code: Option<String>,
    pub completed_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ReleaseAdapterHandoffClaimRequest {
    pub lease_token: String,
    pub reason_code: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ResumeAdapterHandoffClaimRequest {
    pub confirmed_by_user: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RenewAdapterHandoffClaimRequest {
    pub lease_token: String,
    pub extend_seconds: i64,
}

fn default_lease_seconds() -> i64 {
    300
}

pub(crate) fn validate_claim_token_shape(value: &str) -> Result<()> {
    if !value.trim().starts_with("oc_claim_") || value.trim().len() < 46 {
        bail!("衔接任务租约密钥格式无效");
    }
    Ok(())
}

pub(crate) fn validate_release_reason_code(value: &str) -> Result<&str> {
    let value = value.trim();
    match value {
        "adapter_shutdown" | "capacity_pressure" | "transient_failure" | "manual_release" => {
            Ok(value)
        }
        _ => bail!(
            "释放原因只允许 adapter_shutdown、capacity_pressure、transient_failure 或 manual_release"
        ),
    }
}
