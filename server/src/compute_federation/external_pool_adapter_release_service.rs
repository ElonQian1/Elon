//! Administrator-only orchestration for external-pool Adapter release staging.

use anyhow::{bail, Result};
use chrono::{SecondsFormat, Utc};
use serde::{Deserialize, Serialize};

use crate::store::{
    ApplyExternalPoolAdapterRelease, ExternalPoolAdapterReleaseAdmissionReceipt,
    ExternalPoolAdapterReleaseDetailReceipt, ExternalPoolAdapterReleaseRequestReceipt,
    ExternalPoolAdapterReleaseReviewReceipt, ReviewExternalPoolAdapterReleaseRequest, Store,
    SubmitExternalPoolAdapterReleaseRequest, EXTERNAL_POOL_ADAPTER_RELEASE_APPLY_CONFIRMATION,
    EXTERNAL_POOL_ADAPTER_RELEASE_REVIEW_CONFIRMATION,
};

use super::external_pool_adapter_release::{
    canonical_external_pool_adapter_release_capability_set_digest,
    ComputeExternalPoolAdapterReleaseCapability, ComputeExternalPoolAdapterReleaseIntent,
    ComputeExternalPoolAdapterReleaseVerifierIntent,
    COMPUTE_EXTERNAL_POOL_ADAPTER_RELEASE_CONFIRMATION,
    COMPUTE_EXTERNAL_POOL_ADAPTER_RELEASE_PROVIDER_KIND,
    COMPUTE_EXTERNAL_POOL_ADAPTER_RELEASE_ROUTE_KIND,
};

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SubmitExternalPoolAdapterReleaseBody {
    pub idempotency_key: String,
    pub adapter_id: String,
    pub release_version: String,
    pub candidate_artifact_ref: String,
    pub declared_implementation_sha256: String,
    pub supported_capabilities: Vec<ComputeExternalPoolAdapterReleaseCapability>,
    pub expected_credential_verifier: ComputeExternalPoolAdapterReleaseVerifierIntent,
    #[serde(default)]
    pub submission_note: String,
    pub confirm_submission: bool,
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ReviewExternalPoolAdapterReleaseBody {
    pub idempotency_key: String,
    pub expected_request_digest: String,
    pub expected_request_material_digest: String,
    pub decision: String,
    #[serde(default)]
    pub review_note: Option<String>,
    pub confirm_review: bool,
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct StageExternalPoolAdapterReleaseBody {
    pub idempotency_key: String,
    pub expected_request_digest: String,
    pub expected_request_material_digest: String,
    pub expected_review_digest: String,
    #[serde(default)]
    pub apply_note: String,
    pub confirm_stage: bool,
}

#[derive(Clone, Serialize)]
pub(crate) struct ExternalPoolAdapterReleasePreflightReport {
    pub schema: &'static str,
    pub request_id: String,
    pub request_digest: String,
    pub adapter_id: String,
    pub release_version: String,
    pub submitted_by_admin_user_id: String,
    pub request_status: String,
    pub checked_at: String,
    pub review_present: bool,
    pub admission_present: bool,
    pub admin_review_allowed: bool,
    pub admin_stage_allowed: bool,
    pub blockers: Vec<String>,
    pub release_effect: &'static str,
}

pub(crate) fn submit_for_admin(
    store: &Store,
    admin_user_id: &str,
    body: SubmitExternalPoolAdapterReleaseBody,
) -> Result<ExternalPoolAdapterReleaseRequestReceipt> {
    if !body.confirm_submission {
        bail!("提交 external-pool Adapter release 前必须显式确认");
    }
    let capability_set_digest = canonical_external_pool_adapter_release_capability_set_digest(
        &body.supported_capabilities,
    )?;
    store.submit_external_pool_adapter_release_request(SubmitExternalPoolAdapterReleaseRequest {
        submitted_by_admin_user_id: admin_user_id.to_string(),
        release: ComputeExternalPoolAdapterReleaseIntent {
            adapter_id: body.adapter_id,
            release_version: body.release_version,
            route_kind: COMPUTE_EXTERNAL_POOL_ADAPTER_RELEASE_ROUTE_KIND.to_string(),
            supported_provider_kinds: vec![
                COMPUTE_EXTERNAL_POOL_ADAPTER_RELEASE_PROVIDER_KIND.to_string()
            ],
            candidate_artifact_ref: body.candidate_artifact_ref,
            declared_implementation_sha256: body.declared_implementation_sha256,
            supported_capabilities: body.supported_capabilities,
            capability_set_digest,
            expected_credential_verifier: body.expected_credential_verifier,
        },
        idempotency_key: body.idempotency_key,
        confirmation: COMPUTE_EXTERNAL_POOL_ADAPTER_RELEASE_CONFIRMATION.to_string(),
        submission_note: body.submission_note,
        idempotency_scope: operation_scope("submit", admin_user_id),
    })
}

pub(crate) fn review_for_admin(
    store: &Store,
    admin_user_id: &str,
    request_id: &str,
    body: ReviewExternalPoolAdapterReleaseBody,
) -> Result<ExternalPoolAdapterReleaseReviewReceipt> {
    if !body.confirm_review {
        bail!("复核 external-pool Adapter release 前必须显式确认");
    }
    store.review_external_pool_adapter_release_request(ReviewExternalPoolAdapterReleaseRequest {
        request_id: request_id.to_string(),
        expected_request_digest: body.expected_request_digest,
        expected_request_material_digest: body.expected_request_material_digest,
        decision: body.decision,
        review_confirmation: EXTERNAL_POOL_ADAPTER_RELEASE_REVIEW_CONFIRMATION.to_string(),
        review_note: body.review_note,
        reviewed_by_admin_user_id: admin_user_id.to_string(),
        idempotency_scope: operation_scope("review", admin_user_id),
        idempotency_key: body.idempotency_key,
    })
}

pub(crate) fn stage_for_admin(
    store: &Store,
    admin_user_id: &str,
    request_id: &str,
    body: StageExternalPoolAdapterReleaseBody,
) -> Result<ExternalPoolAdapterReleaseAdmissionReceipt> {
    if !body.confirm_stage {
        bail!("暂存 external-pool Adapter release 前必须显式确认");
    }
    store.apply_external_pool_adapter_release(ApplyExternalPoolAdapterRelease {
        request_id: request_id.to_string(),
        expected_request_digest: body.expected_request_digest,
        expected_request_material_digest: body.expected_request_material_digest,
        expected_review_digest: body.expected_review_digest,
        applied_by_admin_user_id: admin_user_id.to_string(),
        apply_confirmation: EXTERNAL_POOL_ADAPTER_RELEASE_APPLY_CONFIRMATION.to_string(),
        apply_note: body.apply_note,
        idempotency_scope: operation_scope("stage", admin_user_id),
        idempotency_key: body.idempotency_key,
    })
}

pub(crate) fn get_for_admin(
    store: &Store,
    request_id: &str,
) -> Result<ExternalPoolAdapterReleaseDetailReceipt> {
    store.external_pool_adapter_release_request(request_id)
}

pub(crate) fn list_for_admin(
    store: &Store,
    status: Option<&str>,
    limit: usize,
) -> Result<Vec<ExternalPoolAdapterReleaseDetailReceipt>> {
    store.list_external_pool_adapter_release_requests_for_admin(status, limit)
}

pub(crate) fn preflight_for_admin(
    store: &Store,
    admin_user_id: &str,
    request_id: &str,
) -> Result<ExternalPoolAdapterReleasePreflightReport> {
    preflight(get_for_admin(store, request_id)?, admin_user_id)
}

fn preflight(
    detail: ExternalPoolAdapterReleaseDetailReceipt,
    admin_user_id: &str,
) -> Result<ExternalPoolAdapterReleasePreflightReport> {
    let status = detail.request.status.as_str();
    let review_approved = detail
        .review
        .as_ref()
        .is_some_and(|review| review.decision == "approved");
    let admin_review_allowed =
        status == "submitted" && detail.request.submitted_by_admin_user_id != admin_user_id;
    let admin_stage_allowed = status == "approved" && review_approved && detail.admission.is_none();
    let blockers = match status {
        "submitted" if !admin_review_allowed => {
            vec!["current_admin_cannot_review_own_submission".to_string()]
        }
        "submitted" | "approved" => Vec::new(),
        "changes_requested" => vec!["changes_requested_requires_new_submission".to_string()],
        "rejected" => vec!["release_request_rejected".to_string()],
        "staged" => vec!["adapter_release_already_staged".to_string()],
        _ => bail!("external-pool Adapter release request status is unsupported"),
    };
    Ok(ExternalPoolAdapterReleasePreflightReport {
        schema: "compute_federation.external_pool_adapter_release_preflight.v1",
        request_id: detail.request.request_id,
        request_digest: detail.request.request_digest,
        adapter_id: detail.request.adapter_id,
        release_version: detail.request.release_version,
        submitted_by_admin_user_id: detail.request.submitted_by_admin_user_id,
        request_status: detail.request.status,
        checked_at: Utc::now().to_rfc3339_opts(SecondsFormat::Nanos, true),
        review_present: detail.review.is_some(),
        admission_present: detail.admission.is_some(),
        admin_review_allowed,
        admin_stage_allowed,
        blockers,
        release_effect: "none",
    })
}

fn operation_scope(operation: &str, admin_user_id: &str) -> String {
    format!("external-pool-adapter-release:{operation}:{admin_user_id}")
}
