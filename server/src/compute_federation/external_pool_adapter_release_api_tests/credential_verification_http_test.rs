use base64::{engine::general_purpose::STANDARD, Engine as _};
use chrono::{Duration, SecondsFormat, Utc};
use rsa::{
    pkcs1v15::SigningKey,
    signature::{SignatureEncoding, Signer},
};
use sha2::Sha256;

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
    store::{
        ApplyExternalPoolOnboarding, ReviewExternalPoolOnboardingRequest,
        SubmitExternalPoolOnboardingRequest, EXTERNAL_POOL_ONBOARDING_APPLY_CONFIRMATION,
    },
};

use super::{
    credential_verifier_http_test,
    credential_verifier_key_http_test::{self, create_active_credential_verifier_key},
    *,
};

#[tokio::test]
async fn credential_verification_http_enforces_exact_binding_revocation_and_redaction() {
    let fixture = fixture();
    let verifier = create_release_credential_verifier(&fixture, "credential-verification").await;
    let (private, key) =
        create_active_credential_verifier_key(&fixture, &verifier, "credential-verification").await;
    let staged = lifecycle_support::stage_release(
        &fixture,
        "credential-verification",
        "43.0.0",
        b"credential verification fixture artifact",
    )
    .await;
    let application = create_onboarding_application(&fixture, "credential-verification", "43.0.0");
    let path = verification_path();
    let body = verification_body(&application, &staged, &key, "credential-verification");
    let locator = "vault-ref:credential-verification";

    assert_eq!(
        call(
            &fixture.router,
            Method::POST,
            &format!("{path}/challenge"),
            None,
            &body,
        )
        .await
        .0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        call(
            &fixture.router,
            Method::POST,
            &format!("{path}/challenge"),
            Some(&fixture.member_token),
            &body,
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    let mut unknown = body.clone();
    unknown["credential_ref"] = json!(locator);
    assert_eq!(
        call(
            &fixture.router,
            Method::POST,
            &format!("{path}/challenge"),
            Some(&fixture.applier_token),
            &unknown,
        )
        .await
        .0,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    let (status, challenge) = call(
        &fixture.router,
        Method::POST,
        &format!("{path}/challenge"),
        Some(&fixture.applier_token),
        &body,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{challenge}");
    assert_eq!(challenge["binding"]["provider_status"], "registering");
    assert_eq!(challenge["binding"]["credential_ref_scheme"], "vault_ref");
    assert_ne!(
        challenge["binding"]["credential_locator_commitment"],
        locator
    );
    assert_redacted(&challenge, locator);

    let message = STANDARD
        .decode(challenge["signature_message_base64"].as_str().unwrap())
        .unwrap();
    let signature = SigningKey::<Sha256>::new(private).sign(&message).to_vec();
    let mut record = body.clone();
    record["expected_signature_message_digest"] = challenge["signature_message_digest"].clone();
    record["signature_base64"] = json!(STANDARD.encode(&signature));
    record["idempotency_key"] = json!("credential-verification-record");
    record["confirm_verification"] = json!(true);

    let mut bad_signature = record.clone();
    bad_signature["signature_base64"] = json!(STANDARD.encode(vec![7u8; 256]));
    assert_eq!(
        call(
            &fixture.router,
            Method::POST,
            path,
            Some(&fixture.applier_token),
            &bad_signature,
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    let (status, created) = call(
        &fixture.router,
        Method::POST,
        path,
        Some(&fixture.applier_token),
        &record,
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{created}");
    assert_eq!(
        created["credential_verification"]["credential_effect"],
        "signed_credential_verification_current"
    );
    for effect in [
        "adapter_effect",
        "route_effect",
        "execution_effect",
        "settlement_effect",
    ] {
        assert_eq!(created["credential_verification"][effect], "none");
    }
    assert_redacted(&created, locator);
    let (status, replayed) = call(
        &fixture.router,
        Method::POST,
        path,
        Some(&fixture.applier_token),
        &record,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{replayed}");
    assert_eq!(replayed["replayed"], true);

    let receipt_id = created["credential_verification"]["credential_verification_receipt_id"]
        .as_str()
        .unwrap();
    let receipt_digest = created["credential_verification"]
        ["credential_verification_receipt_digest"]
        .as_str()
        .unwrap();
    assert_eq!(receipt_digest.len(), 64);
    {
        let connection = fixture.state.store.conn().unwrap();
        assert!(connection
            .execute(
                "INSERT OR REPLACE INTO compute_external_pool_adapter_credential_verification_receipts
                 SELECT * FROM compute_external_pool_adapter_credential_verification_receipts
                 WHERE credential_verification_receipt_id=?1",
                [receipt_id],
            )
            .is_err());
    }
    let currentness_path = format!("{path}/{receipt_id}/currentness");
    let (status, current) = call(
        &fixture.router,
        Method::GET,
        &currentness_path,
        Some(&fixture.applier_token),
        &Value::Null,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{current}");
    assert_eq!(current["current_status"], "verified_current");
    assert_eq!(current["onboarding_status"], "exact");
    assert_eq!(current["provider_status"], "exact_registering");
    assert_eq!(current["admission_status"], "staged");
    assert_eq!(current["verifier_key_status"], "active");
    assert_redacted(&current, locator);

    let key_id = key["key_record"]["key_record_id"].as_str().unwrap();
    let key_digest = key["key_record"]["key_record_digest"].as_str().unwrap();
    let (status, revoked) = call(
        &fixture.router,
        Method::POST,
        &format!(
            "{}/{key_id}/revoke",
            credential_verifier_key_http_test::path()
        ),
        Some(&fixture.applier_token),
        &json!({
            "expected_key_record_digest": key_digest,
            "idempotency_key": "credential-verification-key-revoke",
            "reason": "credential verification fixture key intentionally retired",
            "confirm_revocation": true
        }),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{revoked}");
    let (_, historical) = call(
        &fixture.router,
        Method::GET,
        &currentness_path,
        Some(&fixture.applier_token),
        &Value::Null,
    )
    .await;
    assert_eq!(historical["current_status"], "historical_only");
    assert_eq!(historical["verifier_key_status"], "revoked");
    assert_redacted(&historical, locator);
    fixture.cleanup();
}

pub(super) async fn create_release_credential_verifier(fixture: &Fixture, suffix: &str) -> Value {
    let body = json!({
        "verifier_operator": "fixture-verification-lab",
        "verifier_product": "fixture-credential-verifier-v1",
        "verification_kind": "signed_challenge",
        "verifier_id": "community-pool-verifier",
        "verifier_revision": 1,
        "verifier_digest": "2".repeat(64),
        "idempotency_key": format!("{suffix}-verifier-register"),
        "confirm_registration": true
    });
    let (status, created) = call(
        &fixture.router,
        Method::POST,
        credential_verifier_http_test::path(),
        Some(&fixture.submitter_token),
        &body,
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{created}");
    let id = created["verifier_record"]["verifier_record_id"]
        .as_str()
        .unwrap();
    let digest = created["verifier_record"]["verifier_record_digest"]
        .as_str()
        .unwrap();
    let (status, activated) = call(
        &fixture.router,
        Method::POST,
        &format!("{}/{id}/activate", credential_verifier_http_test::path()),
        Some(&fixture.reviewer_token),
        &json!({
            "expected_verifier_record_digest": digest,
            "idempotency_key": format!("{suffix}-verifier-activate"),
            "confirm_activation": true
        }),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{activated}");
    created
}

pub(super) fn create_onboarding_application(
    fixture: &Fixture,
    suffix: &str,
    release_version: &str,
) -> crate::store::ExternalPoolOnboardingApplicationReceipt {
    let timestamp = Utc::now().to_rfc3339_opts(SecondsFormat::Nanos, true);
    let adapter = ComputeExternalPoolOnboardingAdapterIntent {
        expected_adapter_id: "community-external-pool".to_string(),
        expected_release_version: release_version.to_string(),
        expected_config_revision: 1,
        expected_config_digest: "community-config-v1".to_string(),
    };
    let provider = ComputeProvider {
        schema: COMPUTE_PROVIDER_SCHEMA.to_string(),
        provider_id: format!("external-pool-provider-{suffix}"),
        provider_kind: PROVIDER_KIND_EXTERNAL_POOL.to_string(),
        owner_account_id: fixture.member.id.clone(),
        settlement_account_id: Some(fixture.member.id.clone()),
        display_name: format!("External pool {suffix}"),
        status: PROVIDER_STATUS_REGISTERING.to_string(),
        trust_tier: COMPUTE_EXTERNAL_POOL_ONBOARDING_TRUST_TIER.to_string(),
        home_region: Some("cn-east".to_string()),
        policy_revision: 1,
        capabilities: ComputeProviderCapabilities {
            task_kinds: vec!["llm_inference".to_string()],
            accelerator_kinds: vec!["consumer_gpu".to_string()],
            regions: vec!["cn-east".to_string()],
            allowed_data_classes: vec!["public".to_string()],
            supports_streaming: true,
            supports_checkpointing: false,
        },
        endpoint: None,
        adapter: Some(ComputeProviderAdapterRef {
            adapter_id: adapter.expected_adapter_id.clone(),
            adapter_version: adapter.expected_release_version.clone(),
            config_revision: adapter.expected_config_revision,
            config_digest: adapter.expected_config_digest.clone(),
        }),
        evidence_profile: ComputeProviderEvidenceProfile {
            declared_hardware_digest: Some("4".repeat(64)),
            observed_hardware_digest: None,
            verified_hardware_digest: None,
            last_observed_at: None,
            last_verified_at: None,
        },
        created_at: timestamp.clone(),
        updated_at: timestamp.clone(),
    };
    let mut envelope = ComputeExternalPoolOnboardingRequestEnvelope {
        schema: COMPUTE_EXTERNAL_POOL_ONBOARDING_REQUEST_SCHEMA.to_string(),
        request_id: format!("external-pool-request-{suffix}"),
        request_digest: String::new(),
        canonicalization: COMPUTE_EXTERNAL_POOL_ONBOARDING_CANONICALIZATION.to_string(),
        digest_algorithm: COMPUTE_EXTERNAL_POOL_ONBOARDING_DIGEST_ALGORITHM.to_string(),
        request: ComputeExternalPoolOnboardingRequest {
            requested_by_owner_user_id: fixture.member.id.clone(),
            target_provider: provider,
            adapter_intent: adapter,
            credential_intent: ComputeExternalPoolOnboardingCredentialIntent {
                non_bearer_credential_ref: Some(format!("vault-ref:{suffix}")),
                credential_hint: Some("server-held credential".to_string()),
            },
            external_evidence_ref: None,
            external_evidence_sha256: None,
            idempotency_key: format!("{suffix}-onboarding-submit"),
            confirmation: COMPUTE_EXTERNAL_POOL_ONBOARDING_CONFIRMATION.to_string(),
            owner_note: "register metadata only; credential must be independently verified"
                .to_string(),
            submitted_at: timestamp,
        },
    };
    envelope.request_digest = canonical_external_pool_onboarding_request_json_and_digest(&envelope)
        .unwrap()
        .1;
    let request = fixture
        .state
        .store
        .submit_external_pool_onboarding_request(SubmitExternalPoolOnboardingRequest {
            request: envelope,
            idempotency_scope: "credential-verification-onboarding-submit".to_string(),
            idempotency_key: format!("{suffix}-onboarding-submit"),
        })
        .unwrap();
    let review = fixture
        .state
        .store
        .review_external_pool_onboarding_request(ReviewExternalPoolOnboardingRequest {
            request_id: request.request_id.clone(),
            expected_request_digest: request.request_digest.clone(),
            decision: "approved".to_string(),
            review_reason: None,
            reviewed_by_user_id: fixture.reviewer.id.clone(),
            idempotency_scope: "credential-verification-onboarding-review".to_string(),
            idempotency_key: format!("{suffix}-onboarding-review"),
        })
        .unwrap();
    fixture
        .state
        .store
        .apply_external_pool_onboarding(ApplyExternalPoolOnboarding {
            request_id: request.request_id,
            expected_request_digest: request.request_digest,
            expected_review_digest: review.review_digest,
            applied_by_user_id: fixture.applier.id.clone(),
            apply_confirmation: EXTERNAL_POOL_ONBOARDING_APPLY_CONFIRMATION.to_string(),
            idempotency_scope: "credential-verification-onboarding-apply".to_string(),
            idempotency_key: format!("{suffix}-onboarding-apply"),
        })
        .unwrap()
}

pub(super) fn verification_body(
    application: &crate::store::ExternalPoolOnboardingApplicationReceipt,
    staged: &Value,
    key: &Value,
    suffix: &str,
) -> Value {
    let now = Utc::now();
    json!({
        "application_id": application.application_id,
        "expected_application_digest": application.application_digest,
        "admission_id": staged["admission_id"],
        "expected_admission_digest": staged["admission_digest"],
        "credential_verifier_key_record_id": key["key_record"]["key_record_id"],
        "expected_credential_verifier_key_record_digest": key["key_record"]["key_record_digest"],
        "expected_credential_verifier_key_id": key["key_record"]["key_id"],
        "verifier_report_id": format!("{suffix}-report"),
        "verification_started_at": (now-Duration::minutes(2)).to_rfc3339_opts(SecondsFormat::Nanos,true),
        "verification_completed_at": (now-Duration::minutes(1)).to_rfc3339_opts(SecondsFormat::Nanos,true),
        "report_generated_at": now.to_rfc3339_opts(SecondsFormat::Nanos,true),
        "report_expires_at": (now+Duration::minutes(30)).to_rfc3339_opts(SecondsFormat::Nanos,true),
        "credential_resolution_outcome": "passed",
        "provider_authentication_outcome": "passed",
        "provider_response_evidence_digest": "8".repeat(64)
    })
}

pub(super) fn verification_path() -> &'static str {
    "/api/admin/compute/external-pool-adapter-credential-verifications"
}

fn assert_redacted(value: &Value, locator: &str) {
    let encoded = value.to_string();
    assert!(
        !encoded.contains(locator),
        "response exposed credential locator"
    );
    for key in [
        "credential_ref",
        "non_bearer_credential_ref",
        "credential_hint",
        "public_key_pem",
        "signature_base64",
        "idempotency_key",
        "idempotency_scope",
        "bearer",
        "token",
        "secret",
    ] {
        assert_forbidden(value, key);
    }
}

fn assert_forbidden(value: &Value, key: &str) {
    match value {
        Value::Object(map) => {
            assert!(!map.contains_key(key), "{key}: {value}");
            for nested in map.values() {
                assert_forbidden(nested, key);
            }
        }
        Value::Array(items) => {
            for nested in items {
                assert_forbidden(nested, key);
            }
        }
        _ => {}
    }
}
