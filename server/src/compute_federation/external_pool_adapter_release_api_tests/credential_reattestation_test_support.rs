use base64::{engine::general_purpose::STANDARD, Engine as _};
use chrono::{Duration, SecondsFormat, Utc};
use rsa::{
    pkcs1v15::SigningKey,
    signature::{SignatureEncoding, Signer},
};
use sha2::{Digest, Sha256};

use super::{
    adapter_registry_test_support::{
        assert_no_registry_activation_effects, create_installed_registry_fixture, registry_body,
        REGISTRY_PATH,
    },
    credential_verifier_key_http_test, *,
};

pub(super) struct CredentialReattestationFixture {
    pub registry: Value,
    pub private: rsa::RsaPrivateKey,
    pub key: Value,
}

pub(super) async fn create_credential_reattestation_fixture(
    fixture: &Fixture,
    suffix: &str,
    version: &str,
) -> CredentialReattestationFixture {
    let installed = create_installed_registry_fixture(fixture, suffix, version).await;
    let private = installed.roots.credential_private.clone();
    let key = installed.roots.credential_key.clone();
    let (status, registry) = call(
        &fixture.router,
        Method::POST,
        REGISTRY_PATH,
        Some(&fixture.applier_token),
        &registry_body(&installed.installation, &format!("{suffix}-registry"), true),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{registry}");
    assert_no_registry_activation_effects(fixture, &registry);
    CredentialReattestationFixture {
        registry,
        private,
        key,
    }
}

pub(super) fn challenge_body(roots: &CredentialReattestationFixture, suffix: &str) -> Value {
    let now = Utc::now();
    json!({
        "expected_provider_binding_digest":roots.registry["binding"]["provider_binding_digest"],
        "expected_registry_release_digest":roots.registry["release"]["registry_release_digest"],
        "credential_verifier_key_record_id":roots.key["key_record"]["key_record_id"],
        "expected_credential_verifier_key_record_digest":roots.key["key_record"]["key_record_digest"],
        "expected_credential_verifier_key_id":roots.key["key_record"]["key_id"],
        "verifier_report_id":format!("{suffix}-credential-reattest-report"),
        "verification_started_at":(now-Duration::seconds(4)).to_rfc3339_opts(SecondsFormat::Nanos,true),
        "verification_completed_at":(now-Duration::seconds(3)).to_rfc3339_opts(SecondsFormat::Nanos,true),
        "report_generated_at":(now-Duration::seconds(2)).to_rfc3339_opts(SecondsFormat::Nanos,true),
        "report_expires_at":(now+Duration::minutes(30)).to_rfc3339_opts(SecondsFormat::Nanos,true),
        "credential_resolution_outcome":"passed",
        "provider_authentication_outcome":"passed",
        "provider_response_evidence_digest":"c".repeat(64)
    })
}

pub(super) fn collection_path(roots: &CredentialReattestationFixture) -> String {
    let binding_id = roots.registry["binding"]["provider_binding_id"]
        .as_str()
        .unwrap();
    format!(
        "/api/admin/compute/external-pool-adapter-registry-provider-bindings/{binding_id}/credential-reattestations"
    )
}

pub(super) async fn issue_challenge(
    fixture: &Fixture,
    roots: &CredentialReattestationFixture,
    suffix: &str,
) -> Value {
    let (status, challenge) = call(
        &fixture.router,
        Method::POST,
        &format!("{}/challenge", collection_path(roots)),
        Some(&fixture.applier_token),
        &challenge_body(roots, suffix),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{challenge}");
    challenge
}

pub(super) fn record_body(
    roots: &CredentialReattestationFixture,
    challenge: &Value,
    idempotency_key: &str,
) -> Value {
    let message = STANDARD
        .decode(challenge["signature_message_base64"].as_str().unwrap())
        .unwrap();
    let signature = SigningKey::<Sha256>::new(roots.private.clone())
        .sign(&message)
        .to_vec();
    json!({
        "challenge_id":challenge["binding"]["challenge_id"],
        "expected_signature_message_digest":challenge["signature_message_digest"],
        "signature_base64":STANDARD.encode(signature),
        "idempotency_key":idempotency_key,
        "confirm_reattestation":true
    })
}

pub(super) async fn record_challenge(
    fixture: &Fixture,
    roots: &CredentialReattestationFixture,
    challenge: &Value,
    idempotency_key: &str,
) -> (StatusCode, Value) {
    call(
        &fixture.router,
        Method::POST,
        &collection_path(roots),
        Some(&fixture.applier_token),
        &record_body(roots, challenge, idempotency_key),
    )
    .await
}

pub(super) fn assert_challenge_redacted(value: &Value) {
    for forbidden in [
        "challenge_nonce_base64",
        "challenge_nonce_digest",
        "credential_ref",
        "non_bearer_credential_ref",
        "credential_ref_scheme",
        "credential_locator_commitment",
        "public_key_pem",
        "idempotency_key",
    ] {
        assert_forbidden_key(value, forbidden);
    }
    let encoded = value.to_string();
    assert!(!encoded.contains("vault-ref:"));
    assert!(!encoded.contains("gateway-ref:"));
    assert!(value["binding"]["observed_provider_digest"].is_string());
    assert!(value["binding"]["legacy_credential_verification_receipt_digest"].is_string());
    assert!(value["binding"]["provider_response_evidence_digest"].is_string());
    let decoded = STANDARD
        .decode(value["signature_message_base64"].as_str().unwrap())
        .unwrap();
    let decoded = String::from_utf8(decoded).unwrap();
    assert!(decoded.contains("credential_locator_commitment"));
    assert!(!decoded.contains("vault-ref:"));
    assert!(!decoded.contains("gateway-ref:"));
}

pub(super) fn assert_response_redacted(value: &Value) {
    for forbidden in [
        "challenge_nonce_base64",
        "challenge_nonce_digest",
        "nonce_base64",
        "nonce_digest",
        "signature_message_base64",
        "signature_message_digest",
        "signature_base64",
        "signature_digest",
        "credential_ref",
        "non_bearer_credential_ref",
        "credential_ref_scheme",
        "credential_locator_commitment",
        "public_key_pem",
        "recorded_by_admin_user_id",
        "revoked_by_admin_user_id",
        "idempotency_scope",
        "idempotency_key",
        "confirmation",
        "receipt_json",
        "installation_path",
        "entrypoint_path",
    ] {
        assert_forbidden_key(value, forbidden);
    }
    let encoded = value.to_string();
    assert!(!encoded.contains("vault-ref:"));
    assert!(!encoded.contains("gateway-ref:"));
    assert!(value["reattestation"]["observed_provider_digest"].is_string());
    assert!(value["reattestation"]["legacy_credential_verification_receipt_digest"].is_string());
}

pub(super) fn assert_no_effects(fixture: &Fixture, roots: &CredentialReattestationFixture) {
    let connection = fixture.state.store.conn().unwrap();
    for table in [
        "compute_route_adapter_versions",
        "compute_route_credential_versions",
        "compute_route_authorization_capabilities",
        "compute_service_actor_authorizations",
        "compute_attempt_start_outbox",
        "compute_offers",
        "compute_jobs",
    ] {
        let count: i64 = connection
            .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(count, 0, "V253 unexpectedly populated {table}");
    }
    let effects: (i64, i64, i64) = connection
        .query_row(
            "SELECT
               (SELECT COUNT(*) FROM compute_external_pool_adapter_credential_reattestation_receipts
                 WHERE credential_reattestation_effect='signed_provider_credential_reattestation_verified_current'
                   AND adapter_effect='none' AND provider_effect='none' AND route_effect='none'
                   AND execution_effect='none' AND usage_effect='none' AND settlement_effect='none'),
               (SELECT COUNT(*) FROM compute_external_pool_adapter_credential_reattestation_receipts
                 WHERE adapter_effect<>'none' OR provider_effect<>'none' OR route_effect<>'none'
                    OR execution_effect<>'none' OR usage_effect<>'none' OR settlement_effect<>'none'),
               (SELECT COUNT(*) FROM compute_external_pool_adapter_credential_reattestation_revocations
                 WHERE adapter_effect<>'none' OR provider_effect<>'none' OR route_effect<>'none'
                    OR execution_effect<>'none' OR usage_effect<>'none' OR settlement_effect<>'none')",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .unwrap();
    assert!(effects.0 > 0);
    assert_eq!((effects.1, effects.2), (0, 0));
}

pub(super) fn advance_provider_to_active_revision(
    fixture: &Fixture,
    roots: &CredentialReattestationFixture,
    target_revision: i64,
) {
    let provider_id = roots.registry["binding"]["provider_id"].as_str().unwrap();
    loop {
        let current = fixture.state.store.compute_provider(provider_id).unwrap();
        if current.provider.policy_revision == target_revision {
            break;
        }
        assert!(current.provider.policy_revision < target_revision);
        let mut provider = current.provider;
        provider.policy_revision += 1;
        provider.status = "active".to_string();
        provider.updated_at = Utc::now().to_rfc3339_opts(SecondsFormat::Nanos, true);
        crate::store::validate_compute_provider_contract(&provider).unwrap();
        let provider_json = serde_json::to_string(&provider).unwrap();
        let provider_digest = hex::encode(Sha256::digest(provider_json.as_bytes()));
        let provider_id = provider.provider_id.clone();
        let provider_status = provider.status.clone();
        let updated_at = provider.updated_at.clone();
        let policy_revision = provider.policy_revision;
        let connection = fixture.state.store.conn().unwrap();
        connection
            .execute(
                "INSERT INTO compute_provider_versions(
                   provider_id,policy_revision,provider_digest,provider_json,created_at
                 ) VALUES(?1,?2,?3,?4,?5)",
                rusqlite::params![
                    &provider_id,
                    policy_revision,
                    &provider_digest,
                    &provider_json,
                    Utc::now().to_rfc3339_opts(SecondsFormat::Nanos, true),
                ],
            )
            .unwrap();
        let updated = connection
            .execute(
                "UPDATE compute_providers
                    SET status=?1,current_policy_revision=?2,current_provider_digest=?3,updated_at=?4
                  WHERE provider_id=?5 AND current_policy_revision=?6 AND current_provider_digest=?7",
                rusqlite::params![
                    &provider_status,
                    policy_revision,
                    &provider_digest,
                    &updated_at,
                    &provider_id,
                    policy_revision - 1,
                    &current.provider_digest,
                ],
            )
            .unwrap();
        assert_eq!(updated, 1);
    }
}

pub(super) async fn revoke_verifier_key(
    fixture: &Fixture,
    roots: &CredentialReattestationFixture,
    suffix: &str,
) {
    let id = roots.key["key_record"]["key_record_id"].as_str().unwrap();
    let digest = roots.key["key_record"]["key_record_digest"]
        .as_str()
        .unwrap();
    let (status, revoked) = call(
        &fixture.router,
        Method::POST,
        &format!("{}/{id}/revoke", credential_verifier_key_http_test::path()),
        Some(&fixture.applier_token),
        &json!({
            "expected_key_record_digest":digest,
            "idempotency_key":format!("{suffix}-key-revoke"),
            "reason":"credential re-attestation fixture intentionally retires verifier key",
            "confirm_revocation":true
        }),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{revoked}");
}

fn assert_forbidden_key(value: &Value, forbidden: &str) {
    match value {
        Value::Object(map) => {
            assert!(!map.contains_key(forbidden), "exposed {forbidden}: {value}");
            map.values()
                .for_each(|child| assert_forbidden_key(child, forbidden));
        }
        Value::Array(items) => items
            .iter()
            .for_each(|child| assert_forbidden_key(child, forbidden)),
        _ => {}
    }
}
