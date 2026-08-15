use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CreateExternalPoolAdapterRuntimeCompatibilityChallengeInput {
    pub registry_release_id: String,
    pub expected_registry_release_digest: String,
    pub expected_profile_digest: String,
    pub expected_runner_policy_digest: String,
    pub expected_fixture_catalog_digest: String,
    pub sandbox_verifier_key_record_id: String,
    pub expected_sandbox_verifier_key_record_digest: String,
    pub expected_sandbox_verifier_key_id: String,
    pub predecessor_verification_receipt_id: Option<String>,
    pub predecessor_verification_receipt_digest: Option<String>,
    pub idempotency_key: String,
    pub confirmation: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RecordExternalPoolAdapterRuntimeCompatibilityVerificationReceiptInput {
    pub run_observation_id: String,
    pub expected_run_observation_digest: String,
    pub expected_signature_message_digest: String,
    pub signature_base64: String,
    pub idempotency_key: String,
    pub confirmation: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RevokeExternalPoolAdapterRuntimeCompatibilityVerificationReceiptInput {
    pub verification_receipt_id: String,
    pub expected_verification_receipt_digest: String,
    pub reason: String,
    pub idempotency_key: String,
    pub confirmation: String,
}
