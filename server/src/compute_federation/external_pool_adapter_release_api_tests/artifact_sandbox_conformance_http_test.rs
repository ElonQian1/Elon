use base64::{engine::general_purpose::STANDARD, Engine as _};
use chrono::{Duration, SecondsFormat, Utc};
use rsa::{
    pkcs1v15::SigningKey,
    signature::{SignatureEncoding, Signer},
};
use sha2::Sha256;

use super::{
    artifact_vulnerability_report_http_test::create_vulnerability_report,
    sandbox_verifier_key_http_test::{create_active_sandbox_verifier_key, path as verifier_path},
    *,
};

#[tokio::test]
async fn sandbox_conformance_http_binds_exact_six_capability_plan_and_currentness() {
    let fixture = fixture();
    let (staged, vulnerability) =
        create_vulnerability_report(&fixture, "sandbox-conformance", "11.0.0").await;
    let (private, verifier) =
        create_active_sandbox_verifier_key(&fixture, "sandbox-conformance").await;
    let path = conformance_path(&staged);
    let body = conformance_body(&vulnerability, &verifier, "sandbox-conformance");

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
    let (status, challenge) = call(
        &fixture.router,
        Method::POST,
        &format!("{path}/challenge"),
        Some(&fixture.applier_token),
        &body,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{challenge}");
    assert_eq!(
        challenge["binding"]["test_plan"].as_array().unwrap().len(),
        6
    );
    assert_eq!(
        challenge["binding"]["test_plan"][0]["capability_id"],
        "authenticated_ack"
    );
    assert_eq!(challenge["binding"]["policy_violation_count"], 0);
    assert_eq!(
        challenge["binding"]["vulnerability_report_receipt_digest"],
        vulnerability["vulnerability_report"]["vulnerability_report_receipt_digest"]
    );

    let message = STANDARD
        .decode(challenge["signature_message_base64"].as_str().unwrap())
        .unwrap();
    let signature = SigningKey::<Sha256>::new(private).sign(&message).to_vec();
    let mut record = body.clone();
    record["expected_signature_message_digest"] = challenge["signature_message_digest"].clone();
    record["signature_base64"] = json!(STANDARD.encode(signature));
    record["idempotency_key"] = json!("sandbox-conformance-record");
    record["confirm_conformance"] = json!(true);
    let (status, created) = call(
        &fixture.router,
        Method::POST,
        &path,
        Some(&fixture.applier_token),
        &record,
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{created}");
    assert_eq!(
        created["sandbox_conformance"]["conformance_effect"],
        "signed_sandbox_report_verified_current"
    );
    assert_eq!(created["sandbox_conformance"]["credential_effect"], "none");
    assert_eq!(created["sandbox_conformance"]["adapter_effect"], "none");
    assert_eq!(created["sandbox_conformance"]["route_effect"], "none");
    assert_redacted(&created);

    let (status, replay) = call(
        &fixture.router,
        Method::POST,
        &path,
        Some(&fixture.applier_token),
        &record,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{replay}");
    assert_eq!(replay["replayed"], true);
    let (_, current) = call(
        &fixture.router,
        Method::GET,
        &path,
        Some(lifecycle_support::LOCAL_OWNER_TOKEN),
        &Value::Null,
    )
    .await;
    assert_eq!(current["current_status"], "verified_current");
    assert_eq!(current["vulnerability_report_status"], "verified_current");
    assert_eq!(current["sandbox_verifier_key_status"], "active");
    assert_redacted(&current);

    let id = verifier["key_record"]["key_record_id"].as_str().unwrap();
    let digest = verifier["key_record"]["key_record_digest"]
        .as_str()
        .unwrap();
    let (status, revoked) = call(
        &fixture.router,
        Method::POST,
        &format!("{}/{id}/revoke", verifier_path()),
        Some(&fixture.applier_token),
        &json!({
            "expected_key_record_digest":digest,
            "idempotency_key":"sandbox-conformance-revoke",
            "reason":"fixture dynamic conformance must become historical",
            "confirm_revocation":true
        }),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{revoked}");
    let (_, historical) = call(
        &fixture.router,
        Method::GET,
        &path,
        Some(lifecycle_support::LOCAL_OWNER_TOKEN),
        &Value::Null,
    )
    .await;
    assert_eq!(historical["current_status"], "historical_only");
    assert_eq!(historical["sandbox_verifier_key_status"], "revoked");
    fixture.cleanup();
}

#[tokio::test]
async fn sandbox_conformance_http_rejects_missing_capability_and_policy_violation() {
    let fixture = fixture();
    let (staged, vulnerability) =
        create_vulnerability_report(&fixture, "sandbox-invalid", "11.1.0").await;
    let (_, verifier) = create_active_sandbox_verifier_key(&fixture, "sandbox-invalid").await;
    let path = conformance_path(&staged);
    let mut missing = conformance_body(&vulnerability, &verifier, "sandbox-invalid");
    missing["observations"].as_array_mut().unwrap().pop();
    assert_eq!(
        call(
            &fixture.router,
            Method::POST,
            &format!("{path}/challenge"),
            Some(&fixture.applier_token),
            &missing,
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    let mut violation = conformance_body(&vulnerability, &verifier, "sandbox-invalid-violation");
    violation["observations"][0]["policy_violation_count"] = json!(1);
    assert_eq!(
        call(
            &fixture.router,
            Method::POST,
            &format!("{path}/challenge"),
            Some(&fixture.applier_token),
            &violation,
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    fixture.cleanup();
}

fn conformance_body(vulnerability: &Value, verifier: &Value, suffix: &str) -> Value {
    let now = Utc::now();
    let vulnerability_verified = chrono::DateTime::parse_from_rfc3339(
        vulnerability["vulnerability_report"]["verified_at"]
            .as_str()
            .unwrap(),
    )
    .unwrap()
    .with_timezone(&Utc);
    let intelligence_expires = chrono::DateTime::parse_from_rfc3339(
        vulnerability["vulnerability_report"]["intelligence_expires_at"]
            .as_str()
            .unwrap(),
    )
    .unwrap()
    .with_timezone(&Utc);
    let run_started = vulnerability_verified + Duration::nanoseconds(1);
    let run_completed = run_started + Duration::nanoseconds(1);
    let report_generated = std::cmp::max(now, run_completed);
    let report_expires = intelligence_expires - Duration::nanoseconds(1);
    let capabilities = [
        "authenticated_ack",
        "authenticated_events",
        "cancel_no_start",
        "idempotent_commit",
        "prepare",
        "reconcile",
    ];
    let observations = capabilities
        .into_iter()
        .map(|capability_id| {
            json!({
                "capability_id":capability_id,
                "capability_revision":1,
                "test_case_id":format!("{capability_id}-contract-r1-v1"),
                "outcome":"passed",
                "output_transcript_digest":"8".repeat(64),
                "duration_ms":100,
                "policy_violation_count":0
            })
        })
        .collect::<Vec<_>>();
    json!({
        "expected_vulnerability_report_receipt_digest":vulnerability["vulnerability_report"]["vulnerability_report_receipt_digest"],
        "sandbox_verifier_key_record_id":verifier["key_record"]["key_record_id"],
        "expected_sandbox_verifier_key_record_digest":verifier["key_record"]["key_record_digest"],
        "expected_sandbox_verifier_key_id":verifier["key_record"]["key_id"],
        "verifier_report_id":format!("{suffix}-verifier-report"),
        "sandbox_runtime_id":"fixture-firecracker-v1",
        "runtime_image_digest":"9".repeat(64),
        "isolation_profile_id":"offline_readonly_ephemeral_no_child_process_v1",
        "run_started_at":run_started.to_rfc3339_opts(SecondsFormat::Nanos,true),
        "run_completed_at":run_completed.to_rfc3339_opts(SecondsFormat::Nanos,true),
        "report_generated_at":report_generated.to_rfc3339_opts(SecondsFormat::Nanos,true),
        "report_expires_at":report_expires.to_rfc3339_opts(SecondsFormat::Nanos,true),
        "external_network_attempt_count":0,
        "write_outside_ephemeral_count":0,
        "child_process_attempt_count":0,
        "peak_memory_bytes":67_108_864,
        "cpu_time_ms":600,
        "observations":observations
    })
}

fn conformance_path(staged: &Value) -> String {
    format!(
        "/api/admin/compute/external-pool-adapter-release-admissions/{}/sandbox-conformance",
        staged["admission_id"].as_str().unwrap()
    )
}

fn assert_redacted(value: &Value) {
    for forbidden in [
        "signature_base64",
        "observations",
        "test_plan",
        "expected_credential_verifier",
        "idempotency_key",
        "idempotency_scope",
    ] {
        assert_forbidden_key(value, forbidden);
    }
}

fn assert_forbidden_key(value: &Value, forbidden: &str) {
    match value {
        Value::Object(map) => {
            assert!(!map.contains_key(forbidden), "exposed {forbidden}: {value}");
            for child in map.values() {
                assert_forbidden_key(child, forbidden);
            }
        }
        Value::Array(items) => {
            for child in items {
                assert_forbidden_key(child, forbidden);
            }
        }
        _ => {}
    }
}
