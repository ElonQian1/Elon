use base64::{engine::general_purpose::STANDARD, Engine as _};
use chrono::{Duration, SecondsFormat, Utc};
use rsa::{
    pkcs1v15::SigningKey,
    signature::{SignatureEncoding, Signer},
    RsaPrivateKey,
};
use sha2::Sha256;

use super::{
    sandbox_verifier_key_http_test::create_active_sandbox_verifier_key,
    vulnerability_reattestation_test_support::{
        assert_no_activation_effects as assert_no_v250_activation_effects,
        create_reattestation_fixture, issue_challenge as issue_v250_challenge,
        record_challenge as record_v250_challenge, ReattestationFixture,
    },
    *,
};

pub(super) struct SandboxReattestationFixture {
    pub roots: ReattestationFixture,
    pub vulnerability_reattestation: Value,
    pub sandbox_verifier: Value,
    pub sandbox_private: RsaPrivateKey,
}

pub(super) async fn create_sandbox_reattestation_fixture(
    fixture: &Fixture,
    suffix: &str,
    version: &str,
) -> SandboxReattestationFixture {
    let roots = create_reattestation_fixture(fixture, suffix, version).await;
    let challenge = issue_v250_challenge(fixture, &roots, suffix, Duration::hours(6)).await;
    let (status, vulnerability_reattestation) = record_v250_challenge(
        fixture,
        &roots,
        &challenge,
        &format!("{suffix}-v250-record"),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{vulnerability_reattestation}");
    let verifier_suffix = format!("{suffix}-reattestation");
    let (sandbox_private, sandbox_verifier) =
        create_active_sandbox_verifier_key(fixture, &verifier_suffix).await;
    SandboxReattestationFixture {
        roots,
        vulnerability_reattestation,
        sandbox_verifier,
        sandbox_private,
    }
}

pub(super) fn challenge_body(roots: &SandboxReattestationFixture, suffix: &str) -> Value {
    let reattestation = &roots.vulnerability_reattestation["reattestation"];
    let vulnerability_verified =
        chrono::DateTime::parse_from_rfc3339(reattestation["verified_at"].as_str().unwrap())
            .unwrap()
            .with_timezone(&Utc);
    let intelligence_expires = chrono::DateTime::parse_from_rfc3339(
        reattestation["intelligence_expires_at"].as_str().unwrap(),
    )
    .unwrap()
    .with_timezone(&Utc);
    let run_started = vulnerability_verified + Duration::nanoseconds(1);
    let run_completed = run_started + Duration::nanoseconds(1);
    let generated = std::cmp::max(Utc::now(), run_completed);
    let report_expires = std::cmp::min(
        generated + Duration::hours(1),
        intelligence_expires - Duration::nanoseconds(1),
    );
    let observations = [
        "authenticated_ack",
        "authenticated_events",
        "cancel_no_start",
        "idempotent_commit",
        "prepare",
        "reconcile",
    ]
    .into_iter()
    .map(observation)
    .collect::<Vec<_>>();
    json!({
        "expected_registry_release_digest":roots.roots.registry["release"]["registry_release_digest"],
        "vulnerability_reattestation_receipt_id":reattestation["reattestation_receipt_id"],
        "expected_vulnerability_reattestation_receipt_digest":reattestation["reattestation_receipt_digest"],
        "sandbox_verifier_key_record_id":roots.sandbox_verifier["key_record"]["key_record_id"],
        "expected_sandbox_verifier_key_record_digest":roots.sandbox_verifier["key_record"]["key_record_digest"],
        "expected_sandbox_verifier_key_id":roots.sandbox_verifier["key_record"]["key_id"],
        "verifier_report_id":format!("{suffix}-sandbox-reattestation-report"),
        "sandbox_runtime_id":"fixture-firecracker-v1",
        "runtime_image_digest":"9".repeat(64),
        "isolation_profile_id":"offline_readonly_ephemeral_no_child_process_v1",
        "run_started_at":run_started.to_rfc3339_opts(SecondsFormat::Nanos,true),
        "run_completed_at":run_completed.to_rfc3339_opts(SecondsFormat::Nanos,true),
        "report_generated_at":generated.to_rfc3339_opts(SecondsFormat::Nanos,true),
        "report_expires_at":report_expires.to_rfc3339_opts(SecondsFormat::Nanos,true),
        "external_network_attempt_count":0,
        "write_outside_ephemeral_count":0,
        "child_process_attempt_count":0,
        "peak_memory_bytes":67_108_864,
        "cpu_time_ms":600,
        "observations":observations
    })
}

pub(super) fn collection_path(roots: &SandboxReattestationFixture) -> String {
    let release_id = roots.roots.registry["release"]["registry_release_id"]
        .as_str()
        .unwrap();
    format!(
        "/api/admin/compute/external-pool-adapter-registry-releases/{release_id}/sandbox-reattestations"
    )
}

pub(super) async fn issue_challenge(
    fixture: &Fixture,
    roots: &SandboxReattestationFixture,
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
    roots: &SandboxReattestationFixture,
    challenge: &Value,
    idempotency_key: &str,
) -> Value {
    let message = STANDARD
        .decode(challenge["signature_message_base64"].as_str().unwrap())
        .unwrap();
    let signature = SigningKey::<Sha256>::new(roots.sandbox_private.clone())
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
    roots: &SandboxReattestationFixture,
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
        "test_plan",
        "observations",
        "output_transcript_digest",
        "sandbox_verifier_operator",
        "sandbox_verifier_product",
        "provider_id",
        "provider_binding_id",
        "installation_receipt_id",
        "public_key_pem",
        "idempotency_key",
    ] {
        assert_forbidden_key(value, forbidden);
    }
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
        "public_key_pem",
        "test_plan",
        "observations",
        "output_transcript_digest",
        "sandbox_verifier_operator",
        "sandbox_verifier_product",
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
}

pub(super) fn assert_no_activation_effects(fixture: &Fixture, roots: &SandboxReattestationFixture) {
    assert_no_v250_activation_effects(fixture, &roots.roots);
    let connection = fixture.state.store.conn().unwrap();
    for table in [
        "compute_route_adapter_versions",
        "compute_route_credential_versions",
        "compute_route_credential_revocations",
        "compute_route_authorization_capabilities",
        "compute_route_authorization_seals",
        "compute_attempt_start_send_attempts",
        "compute_attempt_start_remote_observations",
    ] {
        let count: i64 = connection
            .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(count, 0, "V252 unexpectedly populated v213 table {table}");
    }
}

fn observation(capability_id: &str) -> Value {
    json!({
        "capability_id":capability_id,
        "capability_revision":1,
        "test_case_id":format!("{capability_id}-contract-r1-v1"),
        "outcome":"passed",
        "output_transcript_digest":"8".repeat(64),
        "duration_ms":100,
        "policy_violation_count":0
    })
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
