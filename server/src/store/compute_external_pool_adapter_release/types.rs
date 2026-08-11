use anyhow::{bail, Result};
use chrono::{DateTime, SecondsFormat};
use serde::{Deserialize, Serialize};

use crate::compute_federation::external_pool_adapter_release::{
    ComputeExternalPoolAdapterReleaseCapability, ComputeExternalPoolAdapterReleaseIntent,
    ComputeExternalPoolAdapterReleaseRequestEnvelope,
    ComputeExternalPoolAdapterReleaseVerifierIntent,
    COMPUTE_EXTERNAL_POOL_ADAPTER_RELEASE_REQUEST_SCHEMA,
};

pub(super) const EXTERNAL_POOL_ADAPTER_RELEASE_REVIEW_SCHEMA: &str =
    "compute_federation.external_pool_adapter_release_review.v1";
pub(super) const EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_SCHEMA: &str =
    "compute_federation.external_pool_adapter_release_admission.v1";
pub(super) const EXTERNAL_POOL_ADAPTER_RELEASE_REVIEW_CONFIRMATION: &str =
    "confirm_external_pool_adapter_release_review";
pub(super) const EXTERNAL_POOL_ADAPTER_RELEASE_APPLY_CONFIRMATION: &str =
    "confirm_external_pool_adapter_release_stage";
pub(super) const REVIEW_DECISION_APPROVED: &str = "approved";
pub(super) const REVIEW_DECISION_CHANGES_REQUESTED: &str = "changes_requested";
pub(super) const REVIEW_DECISION_REJECTED: &str = "rejected";
pub(super) const ADMISSION_STATUS_STAGED: &str = "staged";

pub(in crate::store) struct SubmitExternalPoolAdapterReleaseRequest {
    pub submitted_by_admin_user_id: String,
    pub release: ComputeExternalPoolAdapterReleaseIntent,
    pub idempotency_key: String,
    pub confirmation: String,
    pub submission_note: String,
    pub idempotency_scope: String,
}

pub(in crate::store) struct ReviewExternalPoolAdapterReleaseRequest {
    pub request_id: String,
    pub expected_request_digest: String,
    pub expected_request_material_digest: String,
    pub decision: String,
    pub review_confirmation: String,
    pub review_note: Option<String>,
    pub reviewed_by_admin_user_id: String,
    pub idempotency_scope: String,
    pub idempotency_key: String,
}

pub(in crate::store) struct ApplyExternalPoolAdapterRelease {
    pub request_id: String,
    pub expected_request_digest: String,
    pub expected_request_material_digest: String,
    pub expected_review_digest: String,
    pub applied_by_admin_user_id: String,
    pub apply_confirmation: String,
    pub apply_note: String,
    pub idempotency_scope: String,
    pub idempotency_key: String,
}

#[derive(Clone, Serialize)]
pub(in crate::store) struct ExternalPoolAdapterReleaseRequestReceipt {
    pub schema: &'static str,
    pub request_id: String,
    pub request_digest: String,
    pub request_material_digest: String,
    pub adapter_id: String,
    pub release_version: String,
    pub status: String,
    pub submitted_by_admin_user_id: String,
    pub submitted_at: String,
    pub replayed: bool,
    pub release_effect: &'static str,
}

#[derive(Clone, Serialize)]
pub(in crate::store) struct ExternalPoolAdapterReleaseReviewReceipt {
    pub schema: &'static str,
    pub review_id: String,
    pub review_digest: String,
    pub request_id: String,
    pub request_digest: String,
    pub request_material_digest: String,
    pub adapter_id: String,
    pub release_version: String,
    pub decision: String,
    pub reviewed_by_admin_user_id: String,
    pub reviewed_at: String,
    pub replayed: bool,
    pub release_effect: &'static str,
}

#[derive(Clone, Serialize)]
pub(in crate::store) struct ExternalPoolAdapterReleaseAdmissionReceipt {
    pub schema: &'static str,
    pub admission_id: String,
    pub admission_digest: String,
    pub request_id: String,
    pub request_digest: String,
    pub request_material_digest: String,
    pub review_id: String,
    pub review_digest: String,
    pub adapter_id: String,
    pub release_version: String,
    pub submitted_by_admin_user_id: String,
    pub reviewed_by_admin_user_id: String,
    pub applied_by_admin_user_id: String,
    pub status: String,
    pub applied_at: String,
    pub replayed: bool,
    pub release_effect: &'static str,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct StoredReviewEnvelope {
    pub schema: String,
    pub review_id: String,
    pub review_digest: String,
    pub canonicalization: String,
    pub digest_algorithm: String,
    pub review: StoredReviewMaterial,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct StoredReviewMaterial {
    pub request_id: String,
    pub request_digest: String,
    pub request_material_digest: String,
    pub adapter_id: String,
    pub release_version: String,
    pub decision: String,
    pub review_confirmation: String,
    pub review_note: Option<String>,
    pub reviewed_by_admin_user_id: String,
    pub reviewed_at: String,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct StoredAdmissionEnvelope {
    pub schema: String,
    pub admission_id: String,
    pub admission_digest: String,
    pub canonicalization: String,
    pub digest_algorithm: String,
    pub admission: StoredAdmissionMaterial,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct StoredAdmissionMaterial {
    pub request_id: String,
    pub request_digest: String,
    pub request_material_digest: String,
    pub review_id: String,
    pub review_digest: String,
    pub adapter_id: String,
    pub release_version: String,
    pub route_kind: String,
    pub supported_provider_kinds: Vec<String>,
    pub candidate_artifact_ref: String,
    pub declared_implementation_sha256: String,
    pub supported_capabilities: Vec<ComputeExternalPoolAdapterReleaseCapability>,
    pub capability_set_digest: String,
    pub expected_credential_verifier: ComputeExternalPoolAdapterReleaseVerifierIntent,
    pub submitted_by_admin_user_id: String,
    pub reviewed_by_admin_user_id: String,
    pub applied_by_admin_user_id: String,
    pub apply_confirmation: String,
    pub apply_note: String,
    pub applied_at: String,
    pub status: String,
}

pub(super) struct StoredRequest {
    pub envelope: ComputeExternalPoolAdapterReleaseRequestEnvelope,
    pub request_json: String,
    pub supported_provider_kinds_json: String,
    pub capabilities_json: String,
    pub capability_set_digest: String,
    pub status: String,
    pub reviewed_by_admin_user_id: Option<String>,
    pub reviewed_at: Option<String>,
    pub applied_by_admin_user_id: Option<String>,
    pub applied_at: Option<String>,
    pub idempotency_scope: String,
    pub idempotency_key: String,
    pub created_at: String,
    pub updated_at: String,
}

pub(super) struct StoredReview {
    pub envelope: StoredReviewEnvelope,
    pub review_json: String,
    pub idempotency_scope: String,
    pub idempotency_key: String,
}

pub(super) struct StoredAdmission {
    pub envelope: StoredAdmissionEnvelope,
    pub admission_json: String,
    pub supported_provider_kinds_json: String,
    pub capabilities_json: String,
    pub idempotency_scope: String,
    pub idempotency_key: String,
}

impl StoredRequest {
    pub(super) fn state_is_exact(&self) -> bool {
        let submitted = &self.envelope.request.submitted_at;
        let submitted_by = self.envelope.request.submitted_by_admin_user_id.as_str();
        let reviewed_by_other = self
            .reviewed_by_admin_user_id
            .as_deref()
            .is_some_and(|reviewed_by| reviewed_by != submitted_by);
        if canonical_nanos(submitted).is_err() || self.created_at != *submitted {
            return false;
        }
        match self.status.as_str() {
            "submitted" => {
                self.reviewed_by_admin_user_id.is_none()
                    && self.reviewed_at.is_none()
                    && self.applied_by_admin_user_id.is_none()
                    && self.applied_at.is_none()
                    && self.updated_at == *submitted
            }
            "approved" | "changes_requested" | "rejected" => match &self.reviewed_at {
                Some(at) => {
                    canonical_nanos(at).is_ok()
                        && submitted <= at
                        && reviewed_by_other
                        && self.applied_by_admin_user_id.is_none()
                        && self.applied_at.is_none()
                        && self.updated_at == *at
                }
                None => false,
            },
            ADMISSION_STATUS_STAGED => match (&self.reviewed_at, &self.applied_at) {
                (Some(reviewed), Some(applied)) => {
                    canonical_nanos(reviewed).is_ok()
                        && canonical_nanos(applied).is_ok()
                        && submitted <= reviewed
                        && reviewed <= applied
                        && reviewed_by_other
                        && self.applied_by_admin_user_id.is_some()
                        && self.updated_at == *applied
                }
                _ => false,
            },
            _ => false,
        }
    }

    pub(super) fn into_receipt(self, replayed: bool) -> ExternalPoolAdapterReleaseRequestReceipt {
        let request = &self.envelope.request;
        ExternalPoolAdapterReleaseRequestReceipt {
            schema: COMPUTE_EXTERNAL_POOL_ADAPTER_RELEASE_REQUEST_SCHEMA,
            request_id: self.envelope.request_id,
            request_digest: self.envelope.request_digest,
            request_material_digest: self.envelope.request_material_digest,
            adapter_id: request.release.adapter_id.clone(),
            release_version: request.release.release_version.clone(),
            status: self.status,
            submitted_by_admin_user_id: request.submitted_by_admin_user_id.clone(),
            submitted_at: request.submitted_at.clone(),
            replayed,
            release_effect: "none",
        }
    }
}

impl StoredReview {
    pub(super) fn into_receipt(self, replayed: bool) -> ExternalPoolAdapterReleaseReviewReceipt {
        let review = self.envelope.review;
        ExternalPoolAdapterReleaseReviewReceipt {
            schema: EXTERNAL_POOL_ADAPTER_RELEASE_REVIEW_SCHEMA,
            review_id: self.envelope.review_id,
            review_digest: self.envelope.review_digest,
            request_id: review.request_id,
            request_digest: review.request_digest,
            request_material_digest: review.request_material_digest,
            adapter_id: review.adapter_id,
            release_version: review.release_version,
            decision: review.decision,
            reviewed_by_admin_user_id: review.reviewed_by_admin_user_id,
            reviewed_at: review.reviewed_at,
            replayed,
            release_effect: "none",
        }
    }
}

impl StoredAdmission {
    pub(super) fn into_receipt(self, replayed: bool) -> ExternalPoolAdapterReleaseAdmissionReceipt {
        let admission = self.envelope.admission;
        ExternalPoolAdapterReleaseAdmissionReceipt {
            schema: EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_SCHEMA,
            admission_id: self.envelope.admission_id,
            admission_digest: self.envelope.admission_digest,
            request_id: admission.request_id,
            request_digest: admission.request_digest,
            request_material_digest: admission.request_material_digest,
            review_id: admission.review_id,
            review_digest: admission.review_digest,
            adapter_id: admission.adapter_id,
            release_version: admission.release_version,
            submitted_by_admin_user_id: admission.submitted_by_admin_user_id,
            reviewed_by_admin_user_id: admission.reviewed_by_admin_user_id,
            applied_by_admin_user_id: admission.applied_by_admin_user_id,
            status: admission.status,
            applied_at: admission.applied_at,
            replayed,
            release_effect: "staged_admission_only",
        }
    }
}

pub(super) fn canonical_nanos(value: &str) -> Result<()> {
    let parsed = DateTime::parse_from_rfc3339(value)?;
    if parsed.offset().local_minus_utc() != 0
        || parsed.to_rfc3339_opts(SecondsFormat::Nanos, true) != value
    {
        bail!("Adapter release timestamp is not canonical UTC nanoseconds");
    }
    Ok(())
}
