use serde::{Deserialize, Serialize};

use crate::compute_federation::external_pool_onboarding::ComputeExternalPoolOnboardingRequestEnvelope;

pub(super) const EXTERNAL_POOL_ONBOARDING_REVIEW_SCHEMA: &str =
    "compute_federation.external_pool_onboarding_review.v1";
pub(super) const EXTERNAL_POOL_ONBOARDING_APPLICATION_SCHEMA: &str =
    "compute_federation.external_pool_onboarding_application.v1";
pub(super) const REVIEW_DECISION_APPROVED: &str = "approved";
pub(super) const REVIEW_DECISION_CHANGES_REQUESTED: &str = "changes_requested";
pub(super) const REVIEW_DECISION_REJECTED: &str = "rejected";
pub(super) const EXTERNAL_POOL_ONBOARDING_APPLY_CONFIRMATION: &str =
    "confirm_external_pool_onboarding_apply";

pub(in crate::store) struct SubmitExternalPoolOnboardingRequest {
    pub request: ComputeExternalPoolOnboardingRequestEnvelope,
    pub idempotency_scope: String,
    pub idempotency_key: String,
}

pub(in crate::store) struct ReviewExternalPoolOnboardingRequest {
    pub request_id: String,
    pub expected_request_digest: String,
    pub decision: String,
    pub review_reason: Option<String>,
    pub reviewed_by_user_id: String,
    pub idempotency_scope: String,
    pub idempotency_key: String,
}

pub(in crate::store) struct ApplyExternalPoolOnboarding {
    pub request_id: String,
    pub expected_request_digest: String,
    pub expected_review_digest: String,
    pub applied_by_user_id: String,
    pub apply_confirmation: String,
    pub idempotency_scope: String,
    pub idempotency_key: String,
}

#[derive(Clone, Serialize)]
pub(in crate::store) struct ExternalPoolOnboardingRequestReceipt {
    pub schema: &'static str,
    pub request_id: String,
    pub request_digest: String,
    pub provider_id: String,
    pub provider_owner_account_id: String,
    pub target_provider_digest: String,
    pub status: String,
    pub credential_ref_present: bool,
    pub credential_hint: Option<String>,
    pub requested_at: String,
    pub replayed: bool,
    pub onboarding_effect: &'static str,
}

#[derive(Clone, Serialize)]
pub(in crate::store) struct ExternalPoolOnboardingReviewReceipt {
    pub schema: &'static str,
    pub review_id: String,
    pub review_digest: String,
    pub request_id: String,
    pub request_digest: String,
    pub provider_id: String,
    pub provider_owner_account_id: String,
    pub decision: String,
    pub review_reason: Option<String>,
    pub reviewed_by_user_id: String,
    pub reviewed_at: String,
    pub replayed: bool,
    pub onboarding_effect: &'static str,
}

#[derive(Clone, Serialize)]
pub(in crate::store) struct ExternalPoolOnboardingApplicationReceipt {
    pub schema: &'static str,
    pub application_id: String,
    pub application_digest: String,
    pub request_id: String,
    pub request_digest: String,
    pub review_id: String,
    pub review_digest: String,
    pub provider_id: String,
    pub provider_digest: String,
    pub approved_by_user_id: String,
    pub reviewed_by_user_id: String,
    pub applied_by_user_id: String,
    pub apply_confirmation: String,
    pub applied_at: String,
    pub replayed: bool,
    pub onboarding_effect: &'static str,
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
    pub provider_id: String,
    pub provider_owner_account_id: String,
    pub decision: String,
    pub review_reason: Option<String>,
    pub reviewed_by_user_id: String,
    pub reviewed_at: String,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct StoredApplicationEnvelope {
    pub schema: String,
    pub application_id: String,
    pub application_digest: String,
    pub canonicalization: String,
    pub digest_algorithm: String,
    pub application: StoredApplicationMaterial,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct StoredApplicationMaterial {
    pub request_id: String,
    pub request_digest: String,
    pub review_id: String,
    pub review_digest: String,
    pub provider_id: String,
    pub provider_kind: String,
    pub provider_owner_account_id: String,
    pub settlement_account_id: String,
    pub target_provider_policy_revision: i64,
    pub target_provider_digest: String,
    pub adapter_id: String,
    pub adapter_release_version: String,
    pub adapter_config_revision: i64,
    pub adapter_config_digest: String,
    pub non_bearer_credential_ref: Option<String>,
    pub credential_hint: Option<String>,
    pub external_evidence_ref: Option<String>,
    pub external_evidence_sha256: Option<String>,
    pub approved_by_user_id: String,
    pub reviewed_by_user_id: String,
    pub applied_by_user_id: String,
    pub apply_confirmation: String,
    pub applied_at: String,
}

pub(super) struct StoredRequest {
    pub envelope: ComputeExternalPoolOnboardingRequestEnvelope,
    pub request_json: String,
    pub target_provider_digest: String,
    pub target_provider_jcs: String,
    pub target_provider_registry_json: String,
    pub status: String,
    pub idempotency_scope: String,
    pub idempotency_key: String,
}

pub(super) struct StoredReview {
    pub envelope: StoredReviewEnvelope,
    pub review_json: String,
    pub idempotency_scope: String,
    pub idempotency_key: String,
}

pub(super) struct StoredApplication {
    pub envelope: StoredApplicationEnvelope,
    pub application_json: String,
    pub target_provider_jcs: String,
    pub target_provider_registry_json: String,
    pub idempotency_scope: String,
    pub idempotency_key: String,
}
