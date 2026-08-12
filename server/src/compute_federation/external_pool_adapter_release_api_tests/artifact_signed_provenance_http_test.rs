use base64::{engine::general_purpose::STANDARD, Engine as _};
use rsa::{
    pkcs1v15::SigningKey,
    pkcs8::{EncodePublicKey, LineEnding},
    rand_core::OsRng,
    signature::{SignatureEncoding, Signer},
    RsaPrivateKey,
};

use super::*;

#[tokio::test]
async fn artifact_signed_provenance_http_verifies_and_redacts_exact_binding() {
    let fixture = fixture();
    let artifact = b"signed provenance HTTP fixture";
    let staged = lifecycle_support::stage_release(&fixture, "signed-http", "7.0.0", artifact).await;
    let (status, source) = lifecycle_support::raw_artifact_call(
        &fixture.router,
        &staged,
        Some(&fixture.applier_token),
        "signed-http-source",
        Body::from(artifact.as_slice()),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{source}");

    let private = RsaPrivateKey::new(&mut OsRng, 2_048).unwrap();
    let pem = private
        .to_public_key()
        .to_public_key_pem(LineEnding::LF)
        .unwrap();
    let (status, key) = call(
        &fixture.router,
        Method::POST,
        signing_key_path(),
        Some(&fixture.submitter_token),
        &json!({
            "source_operator":"signed-http-pool",
            "algorithm":"rsa-pkcs1v15-sha256",
            "public_key_pem":pem,
            "idempotency_key":"signed-http-register",
            "confirm_registration":true
        }),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{key}");
    let key_id = key["key_record"]["key_id"].as_str().unwrap();
    let key_record_id = key["key_record"]["key_record_id"].as_str().unwrap();
    let key_record_digest = key["key_record"]["key_record_digest"].as_str().unwrap();
    let (status, activated) = call(
        &fixture.router,
        Method::POST,
        &format!("{}/{key_record_id}/activate", signing_key_path()),
        Some(&fixture.reviewer_token),
        &json!({
            "expected_key_record_digest":key_record_digest,
            "idempotency_key":"signed-http-activate",
            "confirm_activation":true
        }),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{activated}");

    let challenge_body = json!({
        "expected_admission_digest":staged["admission_digest"],
        "expected_source_receipt_digest":source["source_receipt_digest"],
        "key_record_id":key_record_id,
        "expected_key_record_digest":key_record_digest,
        "expected_key_id":key_id
    });
    let challenge_path = format!("{}/challenge", provenance_path(&staged));
    assert_eq!(
        call(
            &fixture.router,
            Method::POST,
            &challenge_path,
            None,
            &challenge_body
        )
        .await
        .0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        call(
            &fixture.router,
            Method::POST,
            &challenge_path,
            Some(&fixture.member_token),
            &challenge_body,
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    let (status, challenge) = call(
        &fixture.router,
        Method::POST,
        &challenge_path,
        Some(&fixture.applier_token),
        &challenge_body,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{challenge}");
    assert_eq!(challenge["binding"]["artifact_size_bytes"], artifact.len());
    assert_redacted(&challenge, &pem);
    let signature = sign(
        &private,
        challenge["signature_message_base64"].as_str().unwrap(),
    );
    let record_body = json!({
        "expected_admission_digest":staged["admission_digest"],
        "expected_source_receipt_digest":source["source_receipt_digest"],
        "key_record_id":key_record_id,
        "expected_key_record_digest":key_record_digest,
        "expected_key_id":key_id,
        "expected_signature_message_digest":challenge["signature_message_digest"],
        "signature_base64":signature,
        "idempotency_key":"signed-http-record",
        "confirm_provenance":true
    });
    let (status, created) = call(
        &fixture.router,
        Method::POST,
        &provenance_path(&staged),
        Some(&fixture.applier_token),
        &record_body,
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{created}");
    assert_eq!(created["replayed"], false);
    assert_redacted(&created, &pem);

    let (status, replay) = call(
        &fixture.router,
        Method::POST,
        &provenance_path(&staged),
        Some(&fixture.applier_token),
        &record_body,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{replay}");
    assert_eq!(replay["replayed"], true);

    let (status, current) = call(
        &fixture.router,
        Method::GET,
        &provenance_path(&staged),
        Some(lifecycle_support::LOCAL_OWNER_TOKEN),
        &Value::Null,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{current}");
    assert_eq!(current["current_status"], "verified_current");
    assert_redacted(&current, &pem);

    std::fs::remove_file(quarantined_blob_path(
        &fixture.data_dir,
        challenge["binding"]["artifact_sha256"].as_str().unwrap(),
    ))
    .unwrap();
    let (status, drifted) = call(
        &fixture.router,
        Method::GET,
        &provenance_path(&staged),
        Some(lifecycle_support::LOCAL_OWNER_TOKEN),
        &Value::Null,
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT, "{drifted}");
    fixture.cleanup();
}

fn quarantined_blob_path(data_dir: &std::path::Path, digest: &str) -> std::path::PathBuf {
    data_dir
        .join("compute-federation")
        .join("external-pool-adapter-artifacts")
        .join("v1")
        .join("quarantine")
        .join("blobs")
        .join("sha256")
        .join(&digest[..2])
        .join(format!("{digest}.blob"))
}

fn provenance_path(staged: &Value) -> String {
    format!(
        "/api/admin/compute/external-pool-adapter-release-admissions/{}/artifact-signed-provenance",
        staged["admission_id"].as_str().unwrap()
    )
}

fn signing_key_path() -> &'static str {
    "/api/admin/compute/external-pool-adapter-artifact-signing-keys"
}

fn sign(private: &RsaPrivateKey, message_base64: &str) -> String {
    let message = STANDARD.decode(message_base64).unwrap();
    STANDARD.encode(
        SigningKey::<sha2::Sha256>::new(private.clone())
            .sign(&message)
            .to_bytes(),
    )
}

fn assert_redacted(value: &Value, pem: &str) {
    let encoded = value.to_string();
    for forbidden in [
        pem,
        "artifact-ref:sensitive-signed-http",
        "public_key_pem",
        "signature_base64",
        "idempotency_key",
        "idempotency_scope",
    ] {
        assert!(
            !encoded.contains(forbidden),
            "exposed {forbidden}: {encoded}"
        );
    }
}
