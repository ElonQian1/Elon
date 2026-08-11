use axum::http::{HeaderMap, HeaderName, HeaderValue};

use super::{lifecycle_support::*, *};

const BODY_THAT_MUST_NOT_BE_POLLED: &[u8] = b"artifact-intake-precondition-body-v1";

#[tokio::test]
async fn artifact_put_authenticates_admin_before_polling_the_raw_body() {
    let fixture = fixture();
    let staged = stage_release(
        &fixture,
        "artifact-auth-before-body",
        "11.0.1",
        BODY_THAT_MUST_NOT_BE_POLLED,
    )
    .await;

    assert_unpolled_rejection(
        &fixture,
        &staged,
        None,
        artifact_source_headers(&staged, "artifact-auth-unauthenticated"),
        StatusCode::UNAUTHORIZED,
        "unauthenticated PUT",
    )
    .await;
    assert_unpolled_rejection(
        &fixture,
        &staged,
        Some(&fixture.member_token),
        artifact_source_headers(&staged, "artifact-auth-non-admin"),
        StatusCode::FORBIDDEN,
        "non-administrator PUT",
    )
    .await;

    assert_eq!(artifact_source_receipt_count(&fixture), 0);
    assert!(!artifact_blob_path(&fixture, &sha256(BODY_THAT_MUST_NOT_BE_POLLED)).exists());
    fixture.cleanup();
}

#[tokio::test]
async fn artifact_put_rejects_media_type_parameters_encoding_and_content_type_cardinality() {
    let fixture = fixture();
    let staged = stage_release(
        &fixture,
        "artifact-media-before-body",
        "11.0.2",
        BODY_THAT_MUST_NOT_BE_POLLED,
    )
    .await;
    let base = artifact_source_headers(&staged, "artifact-media");

    let mut parameterized = base.clone();
    parameterized.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/octet-stream; charset=binary"),
    );
    let mut encoded = base.clone();
    encoded.insert(
        header::CONTENT_ENCODING,
        HeaderValue::from_static("identity"),
    );
    let mut missing = base.clone();
    missing.remove(header::CONTENT_TYPE);
    let mut duplicate = base;
    duplicate.append(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/octet-stream"),
    );

    for (label, headers) in [
        ("parameterized Content-Type", parameterized),
        ("encoded body", encoded),
        ("missing Content-Type", missing),
        ("duplicate Content-Type", duplicate),
    ] {
        assert_unpolled_rejection(
            &fixture,
            &staged,
            Some(&fixture.applier_token),
            headers,
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            label,
        )
        .await;
    }

    assert_eq!(artifact_source_receipt_count(&fixture), 0);
    assert!(!artifact_blob_path(&fixture, &sha256(BODY_THAT_MUST_NOT_BE_POLLED)).exists());
    fixture.cleanup();
}

#[tokio::test]
async fn artifact_put_rejects_missing_duplicate_and_invalid_control_headers_before_body() {
    let fixture = fixture();
    let staged = stage_release(
        &fixture,
        "artifact-headers-before-body",
        "11.0.3",
        BODY_THAT_MUST_NOT_BE_POLLED,
    )
    .await;
    let base = artifact_source_headers(&staged, "artifact-header-matrix");

    let mut cases = Vec::new();

    let mut missing_key = base.clone();
    missing_key.remove(idempotency_key_header());
    cases.push((
        "missing Idempotency-Key",
        missing_key,
        StatusCode::BAD_REQUEST,
    ));

    let mut duplicate_key = base.clone();
    duplicate_key.append(
        idempotency_key_header(),
        HeaderValue::from_static("artifact-header-matrix-duplicate"),
    );
    cases.push((
        "duplicate Idempotency-Key",
        duplicate_key,
        StatusCode::BAD_REQUEST,
    ));

    let mut invalid_key = base.clone();
    let overlong_key = "k".repeat(161);
    invalid_key.insert(
        idempotency_key_header(),
        HeaderValue::from_bytes(overlong_key.as_bytes()).unwrap(),
    );
    cases.push((
        "invalid Idempotency-Key",
        invalid_key,
        StatusCode::BAD_REQUEST,
    ));

    let mut missing_digest = base.clone();
    missing_digest.remove(expected_digest_header());
    cases.push((
        "missing expected digest",
        missing_digest,
        StatusCode::CONFLICT,
    ));

    let mut duplicate_digest = base.clone();
    duplicate_digest.append(
        expected_digest_header(),
        HeaderValue::from_static(
            "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
        ),
    );
    cases.push((
        "duplicate expected digest",
        duplicate_digest,
        StatusCode::CONFLICT,
    ));

    let mut invalid_digest = base.clone();
    invalid_digest.insert(
        expected_digest_header(),
        HeaderValue::from_static(
            "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
        ),
    );
    cases.push((
        "invalid expected digest",
        invalid_digest,
        StatusCode::CONFLICT,
    ));

    let mut missing_confirmation = base.clone();
    missing_confirmation.remove(confirmation_header());
    cases.push((
        "missing confirmation",
        missing_confirmation,
        StatusCode::CONFLICT,
    ));

    let mut duplicate_confirmation = base.clone();
    duplicate_confirmation.append(
        confirmation_header(),
        HeaderValue::from_static("confirm_external_pool_adapter_artifact_source_intake"),
    );
    cases.push((
        "duplicate confirmation",
        duplicate_confirmation,
        StatusCode::CONFLICT,
    ));

    let mut invalid_confirmation = base;
    invalid_confirmation.insert(
        confirmation_header(),
        HeaderValue::from_static("confirm_external_pool_adapter_artifact_source_intake_v2"),
    );
    cases.push((
        "invalid confirmation",
        invalid_confirmation,
        StatusCode::CONFLICT,
    ));

    for (label, headers, expected_status) in cases {
        assert_unpolled_rejection(
            &fixture,
            &staged,
            Some(&fixture.applier_token),
            headers,
            expected_status,
            label,
        )
        .await;
    }

    assert_eq!(artifact_source_receipt_count(&fixture), 0);
    assert!(!artifact_blob_path(&fixture, &sha256(BODY_THAT_MUST_NOT_BE_POLLED)).exists());
    fixture.cleanup();
}

async fn assert_unpolled_rejection(
    fixture: &Fixture,
    staged: &Value,
    bearer: Option<&str>,
    headers: HeaderMap,
    expected_status: StatusCode,
    label: &str,
) {
    let (body, polled) = tracked_body(BODY_THAT_MUST_NOT_BE_POLLED);
    let (status, response) = raw_artifact_call_with_headers(
        &fixture.router,
        staged["admission_id"].as_str().unwrap(),
        bearer,
        headers,
        body,
    )
    .await;
    assert_eq!(status, expected_status, "{label}: {response}");
    assert!(!was_polled(&polled), "{label} polled the raw body");
    assert_artifact_http_response_redacted(
        fixture,
        &response,
        BODY_THAT_MUST_NOT_BE_POLLED,
        bearer.unwrap_or(&fixture.applier_token),
    );
}

fn idempotency_key_header() -> HeaderName {
    HeaderName::from_static("idempotency-key")
}

fn expected_digest_header() -> HeaderName {
    HeaderName::from_static("x-elon-expected-admission-digest")
}

fn confirmation_header() -> HeaderName {
    HeaderName::from_static("x-elon-artifact-source-confirmation")
}
