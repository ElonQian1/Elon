use super::artifact_package_http_test::{
    create_signed_provenance, package_bytes, package_path, PackageMutation,
};
use super::*;

#[tokio::test]
async fn artifact_security_http_records_exact_local_policy_and_preserves_redaction() {
    let fixture = fixture();
    let bytes = package_bytes("community-external-pool", "9.0.0", None);
    let staged = lifecycle_support::stage_release(&fixture, "security-http", "9.0.0", &bytes).await;
    let (status, source) = lifecycle_support::raw_artifact_call(
        &fixture.router,
        &staged,
        Some(&fixture.applier_token),
        "security-http-source",
        Body::from(bytes),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{source}");
    let provenance = create_signed_provenance(&fixture, &staged, &source, "security-http").await;
    let (_, package) = call(
        &fixture.router, Method::POST, &package_path(&staged), Some(&fixture.applier_token),
        &json!({
            "expected_admission_digest":staged["admission_digest"],
            "expected_source_receipt_digest":source["source_receipt_digest"],
            "expected_provenance_receipt_digest":provenance["provenance"]["provenance_receipt_digest"],
            "idempotency_key":"security-http-package", "confirm_package_inspection":true
        }),
    ).await;
    let path = security_path(&staged);
    let body = json!({
        "expected_admission_digest":staged["admission_digest"],
        "expected_source_receipt_digest":source["source_receipt_digest"],
        "expected_provenance_receipt_digest":provenance["provenance"]["provenance_receipt_digest"],
        "expected_package_receipt_digest":package["package"]["package_receipt_digest"],
        "idempotency_key":"security-http-scan", "confirm_static_security_scan":true
    });
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
        created["security"]["artifact_security_effect"],
        "static_policy_verified"
    );
    assert_eq!(
        created["security"]["vulnerability_intelligence_effect"],
        "none"
    );
    assert_eq!(created["security"]["adapter_effect"], "none");
    assert_eq!(created["security"]["finding_count"], 0);
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
        &lifecycle_support::terminal_body(&staged, "security-http-terminal", "revoked", None, true),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{terminal}");
    let (_, historical) = call(
        &fixture.router,
        Method::GET,
        &path,
        Some(lifecycle_support::LOCAL_OWNER_TOKEN),
        &Value::Null,
    )
    .await;
    assert_eq!(historical["current_status"], "historical_only");
    fixture.cleanup();
}

#[tokio::test]
async fn artifact_security_http_rejects_license_gap_and_embedded_secret() {
    for (suffix, version, mutation) in [
        (
            "security-license",
            "9.1.0",
            PackageMutation::ForbiddenLicense,
        ),
        (
            "security-coverage",
            "9.2.0",
            PackageMutation::SbomCoverageGap,
        ),
        (
            "security-secret",
            "9.3.0",
            PackageMutation::EmbeddedPrivateKey,
        ),
    ] {
        let fixture = fixture();
        let bytes = package_bytes("community-external-pool", version, Some(mutation));
        let staged = lifecycle_support::stage_release(&fixture, suffix, version, &bytes).await;
        let (_, source) = lifecycle_support::raw_artifact_call(
            &fixture.router,
            &staged,
            Some(&fixture.applier_token),
            &format!("{suffix}-source"),
            Body::from(bytes),
        )
        .await;
        let provenance = create_signed_provenance(&fixture, &staged, &source, suffix).await;
        let (_, package) = call(
            &fixture.router, Method::POST, &package_path(&staged), Some(&fixture.applier_token),
            &json!({
                "expected_admission_digest":staged["admission_digest"],
                "expected_source_receipt_digest":source["source_receipt_digest"],
                "expected_provenance_receipt_digest":provenance["provenance"]["provenance_receipt_digest"],
                "idempotency_key":format!("{suffix}-package"), "confirm_package_inspection":true
            }),
        ).await;
        let (status, error) = call(
            &fixture.router, Method::POST, &security_path(&staged), Some(&fixture.applier_token),
            &json!({
                "expected_admission_digest":staged["admission_digest"],
                "expected_source_receipt_digest":source["source_receipt_digest"],
                "expected_provenance_receipt_digest":provenance["provenance"]["provenance_receipt_digest"],
                "expected_package_receipt_digest":package["package"]["package_receipt_digest"],
                "idempotency_key":format!("{suffix}-scan"), "confirm_static_security_scan":true
            }),
        ).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{error}");
        fixture.cleanup();
    }
}

pub(super) async fn create_artifact_security(
    fixture: &Fixture,
    suffix: &str,
    version: &str,
) -> (Value, Value) {
    let bytes = package_bytes("community-external-pool", version, None);
    let staged = lifecycle_support::stage_release(fixture, suffix, version, &bytes).await;
    let (status, source) = lifecycle_support::raw_artifact_call(
        &fixture.router,
        &staged,
        Some(&fixture.applier_token),
        &format!("{suffix}-source"),
        Body::from(bytes),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{source}");
    let provenance = create_signed_provenance(fixture, &staged, &source, suffix).await;
    let (status, package) = call(
        &fixture.router,
        Method::POST,
        &package_path(&staged),
        Some(&fixture.applier_token),
        &json!({
            "expected_admission_digest":staged["admission_digest"],
            "expected_source_receipt_digest":source["source_receipt_digest"],
            "expected_provenance_receipt_digest":provenance["provenance"]["provenance_receipt_digest"],
            "idempotency_key":format!("{suffix}-package"),
            "confirm_package_inspection":true
        }),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{package}");
    let (status, security) = call(
        &fixture.router,
        Method::POST,
        &security_path(&staged),
        Some(&fixture.applier_token),
        &json!({
            "expected_admission_digest":staged["admission_digest"],
            "expected_source_receipt_digest":source["source_receipt_digest"],
            "expected_provenance_receipt_digest":provenance["provenance"]["provenance_receipt_digest"],
            "expected_package_receipt_digest":package["package"]["package_receipt_digest"],
            "idempotency_key":format!("{suffix}-security"),
            "confirm_static_security_scan":true
        }),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{security}");
    (staged, security)
}

pub(super) fn security_path(staged: &Value) -> String {
    format!(
        "/api/admin/compute/external-pool-adapter-release-admissions/{}/artifact-security",
        staged["admission_id"].as_str().unwrap()
    )
}

fn assert_redacted(value: &Value) {
    let encoded = value.to_string();
    for forbidden in [
        "sbom_canonical_json",
        "file_paths",
        "package_url",
        "license_spdx_id",
        "idempotency_key",
        "idempotency_scope",
        "signature_base64",
        "runtime_entrypoint",
    ] {
        assert!(
            !encoded.contains(forbidden),
            "exposed {forbidden}: {encoded}"
        );
    }
}
