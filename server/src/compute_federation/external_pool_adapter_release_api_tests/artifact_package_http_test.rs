use std::io::{Cursor, Write};

use base64::{engine::general_purpose::STANDARD, Engine as _};
use rsa::{
    pkcs1v15::SigningKey,
    pkcs8::{EncodePublicKey, LineEnding},
    rand_core::OsRng,
    signature::{SignatureEncoding, Signer},
    RsaPrivateKey,
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use zip::{write::SimpleFileOptions, CompressionMethod, ZipWriter};

use crate::{
    compute_federation::{
        external_pool_adapter_artifact_package::{
            ExternalPoolAdapterArtifactManifest, ExternalPoolAdapterArtifactManifestFile,
            ExternalPoolAdapterArtifactRuntime, ARTIFACT_PACKAGE_FORMAT,
            ARTIFACT_PACKAGE_MANIFEST_PATH, ARTIFACT_PACKAGE_MANIFEST_SCHEMA,
            ARTIFACT_PACKAGE_RUNTIME_KIND,
        },
        external_pool_adapter_release::{
            canonical_external_pool_adapter_release_capability_set_digest,
            ComputeExternalPoolAdapterReleaseCapability,
            ComputeExternalPoolAdapterReleaseVerifierIntent,
        },
    },
    compute_plugin_sharing_directive::canonical_compute_plugin_ijson_and_sha256,
};

use super::*;

#[tokio::test]
async fn artifact_package_http_records_exact_static_format_and_rejects_unsafe_archives() {
    let fixture = fixture();
    let valid = package_bytes("community-external-pool", "8.0.0", None);
    let staged = lifecycle_support::stage_release(&fixture, "package-http", "8.0.0", &valid).await;
    let (status, source) = lifecycle_support::raw_artifact_call(
        &fixture.router,
        &staged,
        Some(&fixture.applier_token),
        "package-http-source",
        Body::from(valid.clone()),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{source}");
    let provenance = create_signed_provenance(&fixture, &staged, &source, "package-http").await;
    let body = json!({
        "expected_admission_digest":staged["admission_digest"],
        "expected_source_receipt_digest":source["source_receipt_digest"],
        "expected_provenance_receipt_digest":provenance["provenance"]["provenance_receipt_digest"],
        "idempotency_key":"package-http-inspect",
        "confirm_package_inspection":true
    });
    let path = package_path(&staged);
    assert_eq!(
        call(&fixture.router, Method::POST, &path, None, &body)
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        call(
            &fixture.router,
            Method::POST,
            &path,
            Some(&fixture.member_token),
            &body
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    let (status, created) = call(
        &fixture.router,
        Method::POST,
        &path,
        Some(&fixture.applier_token),
        &body,
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{created}");
    assert_eq!(created["replayed"], false);
    assert_eq!(
        created["package"]["artifact_format_effect"],
        "static_format_verified"
    );
    assert_eq!(created["package"]["adapter_effect"], "none");
    assert_redacted(&created);
    let (status, replay) = call(
        &fixture.router,
        Method::POST,
        &path,
        Some(&fixture.applier_token),
        &body,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{replay}");
    assert_eq!(replay["replayed"], true);
    let (status, current) = call(
        &fixture.router,
        Method::GET,
        &path,
        Some(lifecycle_support::LOCAL_OWNER_TOKEN),
        &Value::Null,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{current}");
    assert_eq!(current["current_status"], "verified_current");
    assert_redacted(&current);

    let (status, terminal) = call(
        &fixture.router,
        Method::POST,
        &lifecycle_support::terminal_path(&staged),
        Some(&fixture.reviewer_token),
        &lifecycle_support::terminal_body(&staged, "package-http-terminal", "revoked", None, true),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{terminal}");
    let (status, historical) = call(
        &fixture.router,
        Method::GET,
        &path,
        Some(lifecycle_support::LOCAL_OWNER_TOKEN),
        &Value::Null,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{historical}");
    assert_eq!(historical["current_status"], "historical_only");

    std::fs::remove_file(lifecycle_support::artifact_blob_path(
        &fixture,
        created["package"]["archive_sha256"].as_str().unwrap(),
    ))
    .unwrap();
    assert_eq!(
        call(
            &fixture.router,
            Method::GET,
            &path,
            Some(lifecycle_support::LOCAL_OWNER_TOKEN),
            &Value::Null
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    fixture.cleanup();
}

#[tokio::test]
async fn artifact_package_http_rejects_path_traversal_and_manifest_drift() {
    for (suffix, version, mutation) in [
        ("package-traversal", "8.1.0", PackageMutation::Traversal),
        (
            "package-manifest-drift",
            "8.2.0",
            PackageMutation::ManifestAdapterDrift,
        ),
        (
            "package-case-conflict",
            "8.3.0",
            PackageMutation::CaseConflict,
        ),
        (
            "package-compression-bomb",
            "8.4.0",
            PackageMutation::CompressionBomb,
        ),
    ] {
        let fixture = fixture();
        let bytes = package_bytes("community-external-pool", version, Some(mutation));
        let staged = lifecycle_support::stage_release(&fixture, suffix, version, &bytes).await;
        let (status, source) = lifecycle_support::raw_artifact_call(
            &fixture.router,
            &staged,
            Some(&fixture.applier_token),
            &format!("{suffix}-source"),
            Body::from(bytes),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED, "{source}");
        let provenance = create_signed_provenance(&fixture, &staged, &source, suffix).await;
        let body = json!({
            "expected_admission_digest":staged["admission_digest"],
            "expected_source_receipt_digest":source["source_receipt_digest"],
            "expected_provenance_receipt_digest":provenance["provenance"]["provenance_receipt_digest"],
            "idempotency_key":format!("{suffix}-inspect"),
            "confirm_package_inspection":true
        });
        let (status, error) = call(
            &fixture.router,
            Method::POST,
            &package_path(&staged),
            Some(&fixture.applier_token),
            &body,
        )
        .await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{error}");
        fixture.cleanup();
    }
}

#[derive(Clone, Copy)]
enum PackageMutation {
    Traversal,
    ManifestAdapterDrift,
    CaseConflict,
    CompressionBomb,
}

fn package_bytes(adapter_id: &str, version: &str, mutation: Option<PackageMutation>) -> Vec<u8> {
    let capabilities = [
        "authenticated_ack",
        "authenticated_events",
        "cancel_no_start",
        "idempotent_commit",
        "prepare",
        "reconcile",
    ]
    .into_iter()
    .map(
        |capability_id| ComputeExternalPoolAdapterReleaseCapability {
            capability_id: capability_id.to_string(),
            capability_revision: 1,
        },
    )
    .collect::<Vec<_>>();
    let verifier = ComputeExternalPoolAdapterReleaseVerifierIntent {
        verification_kind: "signed_challenge".to_string(),
        verifier_id: "community-pool-verifier".to_string(),
        verifier_revision: 1,
        verifier_digest: "2".repeat(64),
    };
    let entrypoint = if matches!(mutation, Some(PackageMutation::CompressionBomb)) {
        vec![0_u8; 2 * 1024 * 1024]
    } else {
        b"#!/bin/sh\nexit 0\n".to_vec()
    };
    let path = if matches!(mutation, Some(PackageMutation::Traversal)) {
        "../adapter.sh"
    } else {
        "bin/adapter.sh"
    };
    let manifest = ExternalPoolAdapterArtifactManifest {
        schema: ARTIFACT_PACKAGE_MANIFEST_SCHEMA.to_string(),
        adapter_id: if matches!(mutation, Some(PackageMutation::ManifestAdapterDrift)) {
            "different-adapter".to_string()
        } else {
            adapter_id.to_string()
        },
        release_version: version.to_string(),
        package_format: ARTIFACT_PACKAGE_FORMAT.to_string(),
        runtime: ExternalPoolAdapterArtifactRuntime {
            kind: ARTIFACT_PACKAGE_RUNTIME_KIND.to_string(),
            entrypoint: path.to_string(),
        },
        supported_capabilities: capabilities.clone(),
        capability_set_digest: canonical_external_pool_adapter_release_capability_set_digest(
            &capabilities,
        )
        .unwrap(),
        credential_verifier: verifier,
        files: vec![ExternalPoolAdapterArtifactManifestFile {
            path: path.to_string(),
            sha256: hex::encode(Sha256::digest(&entrypoint)),
            size_bytes: entrypoint.len() as u64,
            role: "entrypoint".to_string(),
        }],
    };
    let manifest_json = canonical(&manifest);
    let cursor = Cursor::new(Vec::new());
    let mut zip = ZipWriter::new(cursor);
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
    zip.start_file(ARTIFACT_PACKAGE_MANIFEST_PATH, options)
        .unwrap();
    zip.write_all(manifest_json.as_bytes()).unwrap();
    let content_options = SimpleFileOptions::default().compression_method(
        if matches!(mutation, Some(PackageMutation::CompressionBomb)) {
            CompressionMethod::Deflated
        } else {
            CompressionMethod::Stored
        },
    );
    zip.start_file(path, content_options).unwrap();
    zip.write_all(&entrypoint).unwrap();
    if matches!(mutation, Some(PackageMutation::CaseConflict)) {
        zip.start_file("BIN/adapter.sh", options).unwrap();
        zip.write_all(&entrypoint).unwrap();
    }
    zip.finish().unwrap().into_inner()
}

fn canonical<T: Serialize>(value: &T) -> String {
    canonical_compute_plugin_ijson_and_sha256(value, 512 * 1024)
        .unwrap()
        .0
}

async fn create_signed_provenance(
    fixture: &Fixture,
    staged: &Value,
    source: &Value,
    suffix: &str,
) -> Value {
    let private = RsaPrivateKey::new(&mut OsRng, 2_048).unwrap();
    let pem = private
        .to_public_key()
        .to_public_key_pem(LineEnding::LF)
        .unwrap();
    let (status, key) = call(&fixture.router, Method::POST, "/api/admin/compute/external-pool-adapter-artifact-signing-keys", Some(&fixture.submitter_token), &json!({
        "source_operator":format!("{suffix}-pool"), "algorithm":"rsa-pkcs1v15-sha256", "public_key_pem":pem,
        "idempotency_key":format!("{suffix}-register"), "confirm_registration":true
    })).await;
    assert_eq!(status, StatusCode::CREATED, "{key}");
    let key_record_id = key["key_record"]["key_record_id"].as_str().unwrap();
    let key_record_digest = key["key_record"]["key_record_digest"].as_str().unwrap();
    let key_id = key["key_record"]["key_id"].as_str().unwrap();
    assert_eq!(call(&fixture.router, Method::POST, &format!("/api/admin/compute/external-pool-adapter-artifact-signing-keys/{key_record_id}/activate"), Some(&fixture.reviewer_token), &json!({
        "expected_key_record_digest":key_record_digest, "idempotency_key":format!("{suffix}-activate"), "confirm_activation":true
    })).await.0, StatusCode::CREATED);
    let provenance_path = format!(
        "/api/admin/compute/external-pool-adapter-release-admissions/{}/artifact-signed-provenance",
        staged["admission_id"].as_str().unwrap()
    );
    let challenge_body = json!({
        "expected_admission_digest":staged["admission_digest"], "expected_source_receipt_digest":source["source_receipt_digest"],
        "key_record_id":key_record_id, "expected_key_record_digest":key_record_digest, "expected_key_id":key_id
    });
    let (status, challenge) = call(
        &fixture.router,
        Method::POST,
        &format!("{provenance_path}/challenge"),
        Some(&fixture.applier_token),
        &challenge_body,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{challenge}");
    let message = STANDARD
        .decode(challenge["signature_message_base64"].as_str().unwrap())
        .unwrap();
    let signature = STANDARD.encode(
        SigningKey::<sha2::Sha256>::new(private)
            .sign(&message)
            .to_bytes(),
    );
    let (status, provenance) = call(&fixture.router, Method::POST, &provenance_path, Some(&fixture.applier_token), &json!({
        "expected_admission_digest":staged["admission_digest"], "expected_source_receipt_digest":source["source_receipt_digest"],
        "key_record_id":key_record_id, "expected_key_record_digest":key_record_digest, "expected_key_id":key_id,
        "expected_signature_message_digest":challenge["signature_message_digest"], "signature_base64":signature,
        "idempotency_key":format!("{suffix}-provenance"), "confirm_provenance":true
    })).await;
    assert_eq!(status, StatusCode::CREATED, "{provenance}");
    provenance
}

fn package_path(staged: &Value) -> String {
    format!(
        "/api/admin/compute/external-pool-adapter-release-admissions/{}/artifact-package",
        staged["admission_id"].as_str().unwrap()
    )
}

fn assert_redacted(value: &Value) {
    let encoded = value.to_string();
    for forbidden in [
        "\"manifest_canonical_json\":",
        "\"runtime_entrypoint\":",
        "\"supported_capabilities\":",
        "\"credential_verifier\":",
        "\"idempotency_key\":",
        "\"idempotency_scope\":",
        "\"signature_base64\":",
        "artifact-ref:",
    ] {
        assert!(
            !encoded.contains(forbidden),
            "exposed {forbidden}: {encoded}"
        );
    }
}
