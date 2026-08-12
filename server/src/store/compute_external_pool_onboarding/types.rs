use serde::{Deserialize, Serialize};

use crate::compute_federation::external_pool_onboarding::ComputeExternalPoolOnboardingRequestEnvelope;
use crate::compute_federation::provider::ComputeProvider;

pub(super) const EXTERNAL_POOL_ONBOARDING_REVIEW_SCHEMA: &str =
    "compute_federation.external_pool_onboarding_review.v1";
pub(super) const EXTERNAL_POOL_ONBOARDING_APPLICATION_SCHEMA: &str =
    "compute_federation.external_pool_onboarding_application.v1";
pub(super) const REVIEW_DECISION_APPROVED: &str = "approved";
pub(super) const REVIEW_DECISION_CHANGES_REQUESTED: &str = "changes_requested";
pub(super) const REVIEW_DECISION_REJECTED: &str = "rejected";
pub(crate) const EXTERNAL_POOL_ONBOARDING_APPLY_CONFIRMATION: &str =
    "confirm_external_pool_onboarding_apply";

pub(crate) struct SubmitExternalPoolOnboardingRequest {
    pub request: ComputeExternalPoolOnboardingRequestEnvelope,
    pub idempotency_scope: String,
    pub idempotency_key: String,
}

pub(crate) struct ReviewExternalPoolOnboardingRequest {
    pub request_id: String,
    pub expected_request_digest: String,
    pub decision: String,
    pub review_reason: Option<String>,
    pub reviewed_by_user_id: String,
    pub idempotency_scope: String,
    pub idempotency_key: String,
}

pub(crate) struct ApplyExternalPoolOnboarding {
    pub request_id: String,
    pub expected_request_digest: String,
    pub expected_review_digest: String,
    pub applied_by_user_id: String,
    pub apply_confirmation: String,
    pub idempotency_scope: String,
    pub idempotency_key: String,
}

pub(crate) struct CancelExternalPoolOnboardingRequest {
    pub request_id: String,
    pub expected_request_digest: String,
    pub owner_user_id: String,
}

#[derive(Clone, Serialize)]
pub(crate) struct ExternalPoolOnboardingRequestReceipt {
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
    pub updated_at: String,
    pub canceled_at: Option<String>,
    pub replayed: bool,
    pub onboarding_effect: &'static str,
}

#[derive(Clone, Serialize)]
pub(crate) struct ExternalPoolOnboardingReviewReceipt {
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
pub(crate) struct ExternalPoolOnboardingApplicationReceipt {
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

#[derive(Clone, Serialize)]
pub(crate) struct ExternalPoolOnboardingDetailReceipt {
    pub request: ExternalPoolOnboardingRequestReceipt,
    pub review: Option<ExternalPoolOnboardingReviewReceipt>,
    pub application: Option<ExternalPoolOnboardingApplicationReceipt>,
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
    pub updated_at: String,
    pub canceled_at: Option<String>,
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

/// Same-connection authority over one exact applied onboarding root and unchanged Provider.
///
/// It deliberately has no Clone or Serde implementation. Consumers must reacquire it inside
/// the transaction that consumes the non-bearer credential locator.
pub(in crate::store) struct HistoricalExternalPoolOnboardingApplicationAuthority {
    application_id: String,
    application_digest: String,
    provider: ComputeProvider,
    provider_digest: String,
    adapter_id: String,
    adapter_release_version: String,
    adapter_config_revision: i64,
    adapter_config_digest: String,
    non_bearer_credential_ref: String,
    applied_at: String,
}

impl HistoricalExternalPoolOnboardingApplicationAuthority {
    pub(super) fn new(
        stored: &StoredApplication,
        provider: ComputeProvider,
        provider_digest: String,
    ) -> anyhow::Result<Self> {
        let item = &stored.envelope.application;
        let non_bearer_credential_ref = item
            .non_bearer_credential_ref
            .clone()
            .ok_or_else(|| anyhow::anyhow!("external-pool onboarding has no credential locator"))?;
        Ok(Self {
            application_id: stored.envelope.application_id.clone(),
            application_digest: stored.envelope.application_digest.clone(),
            provider,
            provider_digest,
            adapter_id: item.adapter_id.clone(),
            adapter_release_version: item.adapter_release_version.clone(),
            adapter_config_revision: item.adapter_config_revision,
            adapter_config_digest: item.adapter_config_digest.clone(),
            non_bearer_credential_ref,
            applied_at: item.applied_at.clone(),
        })
    }

    pub(in crate::store) fn application_id(&self) -> &str {
        &self.application_id
    }
    pub(in crate::store) fn application_digest(&self) -> &str {
        &self.application_digest
    }
    pub(in crate::store) fn provider(&self) -> &ComputeProvider {
        &self.provider
    }
    pub(in crate::store) fn provider_digest(&self) -> &str {
        &self.provider_digest
    }
    pub(in crate::store) fn adapter_id(&self) -> &str {
        &self.adapter_id
    }
    pub(in crate::store) fn adapter_release_version(&self) -> &str {
        &self.adapter_release_version
    }
    pub(in crate::store) fn adapter_config_revision(&self) -> i64 {
        self.adapter_config_revision
    }
    pub(in crate::store) fn adapter_config_digest(&self) -> &str {
        &self.adapter_config_digest
    }
    pub(in crate::store) fn non_bearer_credential_ref(&self) -> &str {
        &self.non_bearer_credential_ref
    }
    pub(in crate::store) fn applied_at(&self) -> &str {
        &self.applied_at
    }
}

pub(in crate::store) struct CurrentExternalPoolOnboardingApplicationAuthority {
    historical: HistoricalExternalPoolOnboardingApplicationAuthority,
}

impl CurrentExternalPoolOnboardingApplicationAuthority {
    pub(super) fn new(historical: HistoricalExternalPoolOnboardingApplicationAuthority) -> Self {
        Self { historical }
    }

    pub(in crate::store) fn application_id(&self) -> &str {
        self.historical.application_id()
    }
    pub(in crate::store) fn application_digest(&self) -> &str {
        self.historical.application_digest()
    }
    pub(in crate::store) fn provider(&self) -> &ComputeProvider {
        self.historical.provider()
    }
    pub(in crate::store) fn provider_digest(&self) -> &str {
        self.historical.provider_digest()
    }
    pub(in crate::store) fn adapter_id(&self) -> &str {
        self.historical.adapter_id()
    }
    pub(in crate::store) fn adapter_release_version(&self) -> &str {
        self.historical.adapter_release_version()
    }
    pub(in crate::store) fn adapter_config_revision(&self) -> i64 {
        self.historical.adapter_config_revision()
    }
    pub(in crate::store) fn adapter_config_digest(&self) -> &str {
        self.historical.adapter_config_digest()
    }
    pub(in crate::store) fn non_bearer_credential_ref(&self) -> &str {
        self.historical.non_bearer_credential_ref()
    }
    pub(in crate::store) fn applied_at(&self) -> &str {
        self.historical.applied_at()
    }
}
