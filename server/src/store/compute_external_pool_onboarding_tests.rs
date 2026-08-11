use std::path::{Path, PathBuf};

use uuid::Uuid;

use crate::{
    compute_federation::{
        external_pool_onboarding::{
            canonical_external_pool_onboarding_request_json_and_digest,
            ComputeExternalPoolOnboardingAdapterIntent,
            ComputeExternalPoolOnboardingCredentialIntent, ComputeExternalPoolOnboardingRequest,
            ComputeExternalPoolOnboardingRequestEnvelope,
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
    },
    store::Store,
};

use super::types::{
    ApplyExternalPoolOnboarding, ReviewExternalPoolOnboardingRequest,
    SubmitExternalPoolOnboardingRequest, EXTERNAL_POOL_ONBOARDING_APPLY_CONFIRMATION,
    REVIEW_DECISION_APPROVED, REVIEW_DECISION_CHANGES_REQUESTED,
};

const OWNER: &str = "external-pool-owner";
const REVIEWER: &str = "external-pool-reviewer";
const APPLIER: &str = "external-pool-applier";
const REQUESTED_AT: &str = "2026-08-11T00:00:00.000000000Z";

fn temporary_store() -> (Store, PathBuf) {
    let path = std::env::temp_dir().join(format!(
        "elon_external_pool_onboarding_{}.db",
        Uuid::new_v4().simple()
    ));
    (Store::open(&path).expect("store opens"), path)
}

fn remove_store_files(path: &Path) {
    for suffix in ["", "-wal", "-shm"] {
        let candidate = PathBuf::from(format!("{}{}", path.display(), suffix));
        let _ = std::fs::remove_file(candidate);
    }
}

fn adapter_intent() -> ComputeExternalPoolOnboardingAdapterIntent {
    ComputeExternalPoolOnboardingAdapterIntent {
        expected_adapter_id: "community-external-pool".to_string(),
        expected_release_version: "1.0.0".to_string(),
        expected_config_revision: 1,
        expected_config_digest: "community-config-v1".to_string(),
    }
}

fn target_provider(case: &str) -> ComputeProvider {
    let adapter = adapter_intent();
    ComputeProvider {
        schema: COMPUTE_PROVIDER_SCHEMA.to_string(),
        provider_id: format!("external-pool-provider-{case}"),
        provider_kind: PROVIDER_KIND_EXTERNAL_POOL.to_string(),
        owner_account_id: OWNER.to_string(),
        settlement_account_id: Some(OWNER.to_string()),
        display_name: format!("External pool {case}"),
        status: PROVIDER_STATUS_REGISTERING.to_string(),
        trust_tier: COMPUTE_EXTERNAL_POOL_ONBOARDING_TRUST_TIER.to_string(),
        home_region: Some("cn-east".to_string()),
        policy_revision: 1,
        capabilities: ComputeProviderCapabilities {
            task_kinds: vec!["image_generation".to_string(), "llm_inference".to_string()],
            accelerator_kinds: vec!["consumer_gpu".to_string()],
            regions: vec!["cn-east".to_string()],
            allowed_data_classes: vec!["public".to_string()],
            supports_streaming: true,
            supports_checkpointing: false,
        },
        endpoint: None,
        adapter: Some(ComputeProviderAdapterRef {
            adapter_id: adapter.expected_adapter_id,
            adapter_version: adapter.expected_release_version,
            config_revision: adapter.expected_config_revision,
            config_digest: adapter.expected_config_digest,
        }),
        evidence_profile: ComputeProviderEvidenceProfile {
            declared_hardware_digest: Some("4".repeat(64)),
            observed_hardware_digest: None,
            verified_hardware_digest: None,
            last_observed_at: None,
            last_verified_at: None,
        },
        created_at: REQUESTED_AT.to_string(),
        updated_at: REQUESTED_AT.to_string(),
    }
}

fn request_envelope(case: &str) -> ComputeExternalPoolOnboardingRequestEnvelope {
    let idempotency_key = format!("onboard-{case}");
    let mut envelope = ComputeExternalPoolOnboardingRequestEnvelope {
        schema: COMPUTE_EXTERNAL_POOL_ONBOARDING_REQUEST_SCHEMA.to_string(),
        request_id: format!("external-pool-request-{case}"),
        request_digest: String::new(),
        canonicalization: COMPUTE_EXTERNAL_POOL_ONBOARDING_CANONICALIZATION.to_string(),
        digest_algorithm: COMPUTE_EXTERNAL_POOL_ONBOARDING_DIGEST_ALGORITHM.to_string(),
        request: ComputeExternalPoolOnboardingRequest {
            requested_by_owner_user_id: OWNER.to_string(),
            target_provider: target_provider(case),
            adapter_intent: adapter_intent(),
            credential_intent: ComputeExternalPoolOnboardingCredentialIntent {
                non_bearer_credential_ref: Some(format!("vault-ref:external-pool-{case}")),
                credential_hint: Some("server-held test credential".to_string()),
            },
            external_evidence_ref: Some(format!("evidence-ref:external-pool-{case}")),
            external_evidence_sha256: Some("5".repeat(64)),
            idempotency_key,
            confirmation: COMPUTE_EXTERNAL_POOL_ONBOARDING_CONFIRMATION.to_string(),
            owner_note: "register metadata only; do not grant route authority".to_string(),
            submitted_at: REQUESTED_AT.to_string(),
        },
    };
    let (_, digest) = canonical_external_pool_onboarding_request_json_and_digest(&envelope)
        .expect("request digest");
    envelope.request_digest = digest;
    envelope
}

fn submit_input(case: &str) -> SubmitExternalPoolOnboardingRequest {
    SubmitExternalPoolOnboardingRequest {
        request: request_envelope(case),
        idempotency_scope: "external-pool-onboarding-submit".to_string(),
        idempotency_key: format!("onboard-{case}"),
    }
}

fn review_input(
    request_id: &str,
    request_digest: &str,
    decision: &str,
) -> ReviewExternalPoolOnboardingRequest {
    ReviewExternalPoolOnboardingRequest {
        request_id: request_id.to_string(),
        expected_request_digest: request_digest.to_string(),
        decision: decision.to_string(),
        review_reason: (decision != REVIEW_DECISION_APPROVED)
            .then(|| "owner must revise the onboarding declaration".to_string()),
        reviewed_by_user_id: REVIEWER.to_string(),
        idempotency_scope: "external-pool-onboarding-review".to_string(),
        idempotency_key: format!("review-{request_id}"),
    }
}

fn apply_input(
    request_id: &str,
    request_digest: &str,
    review_digest: &str,
) -> ApplyExternalPoolOnboarding {
    ApplyExternalPoolOnboarding {
        request_id: request_id.to_string(),
        expected_request_digest: request_digest.to_string(),
        expected_review_digest: review_digest.to_string(),
        applied_by_user_id: APPLIER.to_string(),
        apply_confirmation: EXTERNAL_POOL_ONBOARDING_APPLY_CONFIRMATION.to_string(),
        idempotency_scope: "external-pool-onboarding-apply".to_string(),
        idempotency_key: format!("apply-{request_id}"),
    }
}

#[test]
fn approved_onboarding_registers_provider_and_replays_after_reopen() {
    let (store, path) = temporary_store();
    let request = store
        .submit_external_pool_onboarding_request(submit_input("success"))
        .expect("owner request submits");
    assert_eq!(request.status, "submitted");
    assert!(request.credential_ref_present);
    assert!(!request.replayed);
    let request_replay = store
        .submit_external_pool_onboarding_request(submit_input("success"))
        .expect("owner request replays");
    assert_eq!(request_replay.request_id, request.request_id);
    assert!(request_replay.replayed);

    let review = store
        .review_external_pool_onboarding_request(review_input(
            &request.request_id,
            &request.request_digest,
            REVIEW_DECISION_APPROVED,
        ))
        .expect("independent review approves");
    assert_eq!(review.reviewed_by_user_id, REVIEWER);
    assert!(!review.replayed);
    let review_replay = store
        .review_external_pool_onboarding_request(review_input(
            &request.request_id,
            &request.request_digest,
            REVIEW_DECISION_APPROVED,
        ))
        .expect("review replays");
    assert_eq!(review_replay.review_id, review.review_id);
    assert!(review_replay.replayed);

    let application = store
        .apply_external_pool_onboarding(apply_input(
            &request.request_id,
            &request.request_digest,
            &review.review_digest,
        ))
        .expect("approved onboarding applies");
    assert_eq!(application.onboarding_effect, "provider_registered_only");
    assert_eq!(application.approved_by_user_id, OWNER);
    assert_eq!(application.reviewed_by_user_id, REVIEWER);
    assert!(!application.replayed);
    let provider = store
        .compute_provider(&request.provider_id)
        .expect("registered Provider reads back");
    assert_eq!(provider.provider.provider_kind, PROVIDER_KIND_EXTERNAL_POOL);
    assert_eq!(provider.provider.status, PROVIDER_STATUS_REGISTERING);
    assert_eq!(provider.provider_digest, application.provider_digest);

    let application_replay = store
        .apply_external_pool_onboarding(apply_input(
            &request.request_id,
            &request.request_digest,
            &review.review_digest,
        ))
        .expect("application replays");
    assert_eq!(
        application_replay.application_id,
        application.application_id
    );
    assert!(application_replay.replayed);
    drop(store);

    let reopened = Store::open(&path).expect("store reopens");
    let reopened_request = reopened
        .submit_external_pool_onboarding_request(submit_input("success"))
        .expect("request history survives reopen");
    assert_eq!(reopened_request.status, "applied");
    assert!(reopened_request.replayed);
    let reopened_review = reopened
        .review_external_pool_onboarding_request(review_input(
            &request.request_id,
            &request.request_digest,
            REVIEW_DECISION_APPROVED,
        ))
        .expect("review history survives reopen");
    assert_eq!(reopened_review.review_id, review.review_id);
    let reopened_application = reopened
        .apply_external_pool_onboarding(apply_input(
            &request.request_id,
            &request.request_digest,
            &review.review_digest,
        ))
        .expect("application history survives reopen");
    assert_eq!(
        reopened_application.application_id,
        application.application_id
    );
    assert_eq!(
        reopened
            .compute_provider(&request.provider_id)
            .expect("Provider survives reopen")
            .provider_digest,
        application.provider_digest
    );
    drop(reopened);
    remove_store_files(&path);
}

#[test]
fn owner_review_and_non_approval_cannot_register_provider() {
    let (store, path) = temporary_store();
    let request = store
        .submit_external_pool_onboarding_request(submit_input("closed"))
        .expect("owner request submits");
    let mut owner_review = review_input(
        &request.request_id,
        &request.request_digest,
        REVIEW_DECISION_APPROVED,
    );
    owner_review.reviewed_by_user_id = OWNER.to_string();
    assert!(store
        .review_external_pool_onboarding_request(owner_review)
        .err()
        .expect("owner review must fail")
        .to_string()
        .contains("owner cannot review"));

    let review = store
        .review_external_pool_onboarding_request(review_input(
            &request.request_id,
            &request.request_digest,
            REVIEW_DECISION_CHANGES_REQUESTED,
        ))
        .expect("review closes with changes requested");
    let mut wrong_confirmation = apply_input(
        &request.request_id,
        &request.request_digest,
        &review.review_digest,
    );
    wrong_confirmation.apply_confirmation = "confirm-something-else".to_string();
    assert!(store
        .apply_external_pool_onboarding(wrong_confirmation)
        .err()
        .expect("wrong confirmation must fail")
        .to_string()
        .contains("confirmation is not exact"));
    assert!(store
        .apply_external_pool_onboarding(apply_input(
            &request.request_id,
            &request.request_digest,
            &review.review_digest,
        ))
        .err()
        .expect("non-approved request cannot apply")
        .to_string()
        .contains("only the exact approved"));
    assert!(store
        .compute_provider_if_exists(&request.provider_id)
        .expect("Provider lookup succeeds")
        .is_none());

    let mut conflicting_replay = review_input(
        &request.request_id,
        &request.request_digest,
        REVIEW_DECISION_CHANGES_REQUESTED,
    );
    conflicting_replay.review_reason = Some("a different immutable reason".to_string());
    assert!(store
        .review_external_pool_onboarding_request(conflicting_replay)
        .err()
        .expect("changed replay must fail")
        .to_string()
        .contains("conflicts with immutable history"));
    drop(store);
    remove_store_files(&path);
}
