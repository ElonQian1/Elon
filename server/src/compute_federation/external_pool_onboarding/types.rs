use serde::{Deserialize, Serialize};

use crate::compute_federation::provider::ComputeProvider;

pub(crate) const COMPUTE_EXTERNAL_POOL_ONBOARDING_REQUEST_SCHEMA: &str =
    "compute_federation.external_pool_onboarding_request.v1";
pub(crate) const COMPUTE_EXTERNAL_POOL_ONBOARDING_CANONICALIZATION: &str = "rfc8785_jcs";
pub(crate) const COMPUTE_EXTERNAL_POOL_ONBOARDING_DIGEST_ALGORITHM: &str = "sha256";
pub(crate) const COMPUTE_EXTERNAL_POOL_ONBOARDING_TRUST_TIER: &str = "self_declared";
pub(crate) const COMPUTE_EXTERNAL_POOL_ONBOARDING_CONFIRMATION: &str =
    "confirm_external_pool_onboarding_request";

/// Canonical owner request DTO. Clone/serde support does not grant Store or route authority.
#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeExternalPoolOnboardingRequestEnvelope {
    pub schema: String,
    pub request_id: String,
    pub request_digest: String,
    pub canonicalization: String,
    pub digest_algorithm: String,
    pub request: ComputeExternalPoolOnboardingRequest,
}

/// Owner-declared material only. Review and immutable apply must revalidate every field.
#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeExternalPoolOnboardingRequest {
    pub requested_by_owner_user_id: String,
    pub target_provider: ComputeProvider,
    pub adapter_intent: ComputeExternalPoolOnboardingAdapterIntent,
    pub credential_intent: ComputeExternalPoolOnboardingCredentialIntent,
    pub external_evidence_ref: Option<String>,
    pub external_evidence_sha256: Option<String>,
    pub idempotency_key: String,
    pub confirmation: String,
    pub owner_note: String,
    pub submitted_at: String,
}

/// Expected registry/config identity, not proof that an Adapter exists or can execute.
#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeExternalPoolOnboardingAdapterIntent {
    pub expected_adapter_id: String,
    pub expected_release_version: String,
    pub expected_config_revision: i64,
    pub expected_config_digest: String,
}

/// Optional lookup locator and redacted hint only; neither field is authenticated proof.
#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeExternalPoolOnboardingCredentialIntent {
    pub non_bearer_credential_ref: Option<String>,
    pub credential_hint: Option<String>,
}
