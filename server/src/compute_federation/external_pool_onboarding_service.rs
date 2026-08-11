//! Owner submission and administrator review/application for external-pool onboarding.

use anyhow::{bail, Result};
use serde::Deserialize;

use crate::store::{
    ApplyExternalPoolOnboarding, ExternalPoolOnboardingApplicationReceipt,
    ExternalPoolOnboardingRequestReceipt, ExternalPoolOnboardingReviewReceipt,
    ReviewExternalPoolOnboardingRequest, Store, SubmitExternalPoolOnboardingRequest,
    EXTERNAL_POOL_ONBOARDING_APPLY_CONFIRMATION,
};

use super::{
    external_pool_onboarding::{
        canonical_external_pool_onboarding_request_json_and_digest,
        ComputeExternalPoolOnboardingAdapterIntent, ComputeExternalPoolOnboardingCredentialIntent,
        ComputeExternalPoolOnboardingRequest, ComputeExternalPoolOnboardingRequestEnvelope,
        COMPUTE_EXTERNAL_POOL_ONBOARDING_CANONICALIZATION,
        COMPUTE_EXTERNAL_POOL_ONBOARDING_CONFIRMATION,
        COMPUTE_EXTERNAL_POOL_ONBOARDING_DIGEST_ALGORITHM,
        COMPUTE_EXTERNAL_POOL_ONBOARDING_REQUEST_SCHEMA,
        COMPUTE_EXTERNAL_POOL_ONBOARDING_TRUST_TIER,
    },
    provider::{
        ComputeProvider, ComputeProviderAdapterRef, ComputeProviderCapabilities,
        ComputeProviderEvidenceProfile, COMPUTE_PROVIDER_SCHEMA, PROVIDER_KIND_EXTERNAL_POOL,
        PROVIDER_STATUS_REGISTERING,
    },
};

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SubmitExternalPoolOnboardingBody {
    pub request_id: String,
    pub idempotency_key: String,
    pub submitted_at: String,
    pub provider_id: String,
    pub display_name: String,
    pub home_region: String,
    pub task_kinds: Vec<String>,
    pub accelerator_kinds: Vec<String>,
    pub regions: Vec<String>,
    pub allowed_data_classes: Vec<String>,
    pub supports_streaming: bool,
    pub supports_checkpointing: bool,
    pub declared_hardware_digest: Option<String>,
    pub adapter_intent: ComputeExternalPoolOnboardingAdapterIntent,
    pub credential_intent: ComputeExternalPoolOnboardingCredentialIntent,
    pub external_evidence_ref: Option<String>,
    pub external_evidence_sha256: Option<String>,
    #[serde(default)]
    pub owner_note: String,
    pub confirm_submission: bool,
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ReviewExternalPoolOnboardingBody {
    pub idempotency_key: String,
    pub expected_request_digest: String,
    pub decision: String,
    pub review_reason: Option<String>,
    pub confirm_review: bool,
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ApplyExternalPoolOnboardingBody {
    pub idempotency_key: String,
    pub expected_request_digest: String,
    pub expected_review_digest: String,
    pub confirm_application: bool,
}

pub(crate) fn submit_for_owner(
    store: &Store,
    owner_user_id: &str,
    mut body: SubmitExternalPoolOnboardingBody,
) -> Result<ExternalPoolOnboardingRequestReceipt> {
    if !body.confirm_submission {
        bail!("提交 external-pool onboarding 前必须显式确认");
    }
    normalize(&mut body.task_kinds);
    normalize(&mut body.accelerator_kinds);
    normalize(&mut body.regions);
    normalize(&mut body.allowed_data_classes);

    let adapter = ComputeProviderAdapterRef {
        adapter_id: body.adapter_intent.expected_adapter_id.clone(),
        adapter_version: body.adapter_intent.expected_release_version.clone(),
        config_revision: body.adapter_intent.expected_config_revision,
        config_digest: body.adapter_intent.expected_config_digest.clone(),
    };
    let target_provider = ComputeProvider {
        schema: COMPUTE_PROVIDER_SCHEMA.to_string(),
        provider_id: body.provider_id,
        provider_kind: PROVIDER_KIND_EXTERNAL_POOL.to_string(),
        owner_account_id: owner_user_id.to_string(),
        settlement_account_id: Some(owner_user_id.to_string()),
        display_name: body.display_name,
        status: PROVIDER_STATUS_REGISTERING.to_string(),
        trust_tier: COMPUTE_EXTERNAL_POOL_ONBOARDING_TRUST_TIER.to_string(),
        home_region: Some(body.home_region),
        policy_revision: 1,
        capabilities: ComputeProviderCapabilities {
            task_kinds: body.task_kinds,
            accelerator_kinds: body.accelerator_kinds,
            regions: body.regions,
            allowed_data_classes: body.allowed_data_classes,
            supports_streaming: body.supports_streaming,
            supports_checkpointing: body.supports_checkpointing,
        },
        endpoint: None,
        adapter: Some(adapter),
        evidence_profile: ComputeProviderEvidenceProfile {
            declared_hardware_digest: body.declared_hardware_digest,
            observed_hardware_digest: None,
            verified_hardware_digest: None,
            last_observed_at: None,
            last_verified_at: None,
        },
        created_at: body.submitted_at.clone(),
        updated_at: body.submitted_at.clone(),
    };
    let mut envelope = ComputeExternalPoolOnboardingRequestEnvelope {
        schema: COMPUTE_EXTERNAL_POOL_ONBOARDING_REQUEST_SCHEMA.to_string(),
        request_id: body.request_id,
        request_digest: String::new(),
        canonicalization: COMPUTE_EXTERNAL_POOL_ONBOARDING_CANONICALIZATION.to_string(),
        digest_algorithm: COMPUTE_EXTERNAL_POOL_ONBOARDING_DIGEST_ALGORITHM.to_string(),
        request: ComputeExternalPoolOnboardingRequest {
            requested_by_owner_user_id: owner_user_id.to_string(),
            target_provider,
            adapter_intent: body.adapter_intent,
            credential_intent: body.credential_intent,
            external_evidence_ref: body.external_evidence_ref,
            external_evidence_sha256: body.external_evidence_sha256,
            idempotency_key: body.idempotency_key.clone(),
            confirmation: COMPUTE_EXTERNAL_POOL_ONBOARDING_CONFIRMATION.to_string(),
            owner_note: body.owner_note,
            submitted_at: body.submitted_at,
        },
    };
    let (_, request_digest) =
        canonical_external_pool_onboarding_request_json_and_digest(&envelope)?;
    envelope.request_digest = request_digest;
    store.submit_external_pool_onboarding_request(SubmitExternalPoolOnboardingRequest {
        request: envelope,
        idempotency_scope: operation_scope("submit", owner_user_id),
        idempotency_key: body.idempotency_key,
    })
}

pub(crate) fn review_for_admin(
    store: &Store,
    admin_user_id: &str,
    request_id: &str,
    body: ReviewExternalPoolOnboardingBody,
) -> Result<ExternalPoolOnboardingReviewReceipt> {
    if !body.confirm_review {
        bail!("复核 external-pool onboarding 前必须显式确认");
    }
    store.review_external_pool_onboarding_request(ReviewExternalPoolOnboardingRequest {
        request_id: request_id.to_string(),
        expected_request_digest: body.expected_request_digest,
        decision: body.decision,
        review_reason: body.review_reason,
        reviewed_by_user_id: admin_user_id.to_string(),
        idempotency_scope: operation_scope("review", admin_user_id),
        idempotency_key: body.idempotency_key,
    })
}

pub(crate) fn apply_for_admin(
    store: &Store,
    admin_user_id: &str,
    request_id: &str,
    body: ApplyExternalPoolOnboardingBody,
) -> Result<ExternalPoolOnboardingApplicationReceipt> {
    if !body.confirm_application {
        bail!("应用 external-pool onboarding 前必须显式确认");
    }
    store.apply_external_pool_onboarding(ApplyExternalPoolOnboarding {
        request_id: request_id.to_string(),
        expected_request_digest: body.expected_request_digest,
        expected_review_digest: body.expected_review_digest,
        applied_by_user_id: admin_user_id.to_string(),
        apply_confirmation: EXTERNAL_POOL_ONBOARDING_APPLY_CONFIRMATION.to_string(),
        idempotency_scope: operation_scope("apply", admin_user_id),
        idempotency_key: body.idempotency_key,
    })
}

fn normalize(values: &mut Vec<String>) {
    values.sort();
    values.dedup();
}

fn operation_scope(operation: &str, user_id: &str) -> String {
    format!("external-pool-onboarding:{operation}:{user_id}")
}
