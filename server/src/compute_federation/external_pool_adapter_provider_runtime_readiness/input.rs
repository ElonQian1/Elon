use serde::Deserialize;

#[derive(Clone, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExpectedProviderRuntimeReadinessPredecessor {
    pub readiness_receipt_id: String,
    pub readiness_receipt_digest: String,
}

#[derive(Clone, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct CreateProviderRuntimeReadinessReceiptBody {
    pub expected_provider_binding_digest: String,
    pub expected_installation_receipt_id: String,
    pub expected_installation_receipt_digest: String,
    pub expected_candidate_digest: String,
    pub expected_profile_digest: String,
    pub expected_target_digest: String,
    pub expected_companion_digest: String,
    pub runtime_compatibility_verification_receipt_id: String,
    pub expected_runtime_compatibility_verification_receipt_digest: String,
    pub expected_predecessor: Option<ExpectedProviderRuntimeReadinessPredecessor>,
    pub idempotency_key: String,
    pub confirm_provider_runtime_readiness: bool,
}

#[derive(Clone, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct RevokeProviderRuntimeReadinessReceiptBody {
    pub expected_readiness_receipt_digest: String,
    pub reason: String,
    pub idempotency_key: String,
    pub confirm_revocation: bool,
}
