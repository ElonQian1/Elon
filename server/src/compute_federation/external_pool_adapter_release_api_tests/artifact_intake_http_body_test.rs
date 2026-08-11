use axum::http::{HeaderName, HeaderValue};
use futures::stream;

use super::{lifecycle_support::*, *};

const MAX_ARTIFACT_BYTES: usize = 32 * 1024 * 1024;

#[tokio::test]
async fn artifact_put_rejects_lineage_mismatch_and_missing_admission_before_body_poll() {
    let fixture = fixture();
    let artifact_a = b"artifact-lineage-a-v1";
    let artifact_b = b"artifact-lineage-b-v1";
    let staged_a = stage_release(&fixture, "artifact-lineage-a", "11.1.1", artifact_a).await;
    let staged_b = stage_release(&fixture, "artifact-lineage-b", "11.1.2", artifact_b).await;

    let mut mismatched_headers = artifact_source_headers(&staged_a, "artifact-lineage-mismatch");
    mismatched_headers.insert(
        expected_digest_header(),
        HeaderValue::from_bytes(staged_b["admission_digest"].as_str().unwrap().as_bytes()).unwrap(),
    );
    let (mismatch_body, mismatch_polled) = tracked_body(artifact_a);
    let (status, mismatch) = raw_artifact_call_with_headers(
        &fixture.router,
        staged_a["admission_id"].as_str().unwrap(),
        Some(&fixture.applier_token),
        mismatched_headers,
        mismatch_body,
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT, "{mismatch}");
    assert!(
        !was_polled(&mismatch_polled),
        "lineage mismatch polled the raw body"
    );
    assert_artifact_http_response_redacted(&fixture, &mismatch, artifact_a, &fixture.applier_token);

    let (missing_body, missing_polled) = tracked_body(artifact_a);
    let (status, missing) = raw_artifact_call_with_headers(
        &fixture.router,
        "missing-external-pool-adapter-release-admission",
        Some(&fixture.applier_token),
        artifact_source_headers(&staged_a, "artifact-missing-admission"),
        missing_body,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{missing}");
    assert!(
        !was_polled(&missing_polled),
        "missing admission polled the raw body"
    );
    assert_artifact_http_response_redacted(&fixture, &missing, artifact_a, &fixture.applier_token);

    assert_eq!(artifact_source_receipt_count(&fixture), 0);
    assert!(!artifact_blob_path(&fixture, &sha256(artifact_a)).exists());
    assert!(!artifact_blob_path(&fixture, &sha256(artifact_b)).exists());
    fixture.cleanup();
}

#[tokio::test]
async fn artifact_put_rejects_empty_and_hash_mismatched_bodies_without_receipts() {
    let fixture = fixture();
    let empty: &[u8] = b"";
    let expected = b"artifact-hash-expected-v1";
    let mismatched = b"artifact-hash-mismatched-v1";
    let empty_staged = stage_release(&fixture, "artifact-empty", "11.2.1", empty).await;
    let hash_staged = stage_release(&fixture, "artifact-hash", "11.2.2", expected).await;

    let (status, empty_response) = raw_artifact_call(
        &fixture.router,
        &empty_staged,
        Some(&fixture.applier_token),
        "artifact-empty-body",
        Body::empty(),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{empty_response}");
    assert_artifact_http_response_redacted(
        &fixture,
        &empty_response,
        empty,
        &fixture.applier_token,
    );

    let (status, mismatch_response) = raw_artifact_call(
        &fixture.router,
        &hash_staged,
        Some(&fixture.applier_token),
        "artifact-hash-mismatch",
        Body::from(mismatched.to_vec()),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "{mismatch_response}"
    );
    assert_artifact_http_response_redacted(
        &fixture,
        &mismatch_response,
        mismatched,
        &fixture.applier_token,
    );

    assert_eq!(artifact_source_receipt_count(&fixture), 0);
    assert!(!artifact_blob_path(&fixture, &sha256(empty)).exists());
    assert!(!artifact_blob_path(&fixture, &sha256(expected)).exists());
    fixture.cleanup();
}

#[tokio::test]
async fn artifact_put_cleans_partial_file_when_the_body_stream_fails() {
    let fixture = fixture();
    let artifact = b"artifact-body-stream-error-complete-v1";
    let artifact_digest = sha256(artifact);
    let staged = stage_release(&fixture, "artifact-body-stream-error", "11.2.3", artifact).await;
    let body = Body::from_stream(stream::iter([
        Ok::<Vec<u8>, std::io::Error>(b"artifact-body-stream-error-prefix".to_vec()),
        Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "synthetic artifact request body failure",
        )),
    ]));

    let (status, response) = raw_artifact_call(
        &fixture.router,
        &staged,
        Some(&fixture.applier_token),
        "artifact-body-stream-error-key",
        body,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{response}");
    assert_artifact_http_response_redacted(
        &fixture,
        &response,
        b"artifact-body-stream-error-prefix",
        &fixture.applier_token,
    );

    let blob_path = artifact_blob_path(&fixture, &artifact_digest);
    assert!(!blob_path.exists());
    let part_files = std::fs::read_dir(blob_path.parent().unwrap())
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().and_then(|extension| extension.to_str()) == Some("part"))
        .collect::<Vec<_>>();
    assert!(part_files.is_empty(), "orphan part files: {part_files:?}");
    assert_eq!(artifact_source_receipt_count(&fixture), 0);
    fixture.cleanup();
}

#[tokio::test]
async fn artifact_put_rejects_more_than_thirty_two_mib_without_a_receipt_or_blob() {
    let fixture = fixture();
    let oversized = vec![b'z'; MAX_ARTIFACT_BYTES + 1];
    let oversized_digest = sha256(&oversized);
    let staged = stage_release(&fixture, "artifact-oversized", "11.3.1", &oversized).await;

    let (status, response) = raw_artifact_call(
        &fixture.router,
        &staged,
        Some(&fixture.applier_token),
        "artifact-oversized-body",
        Body::from(oversized),
    )
    .await;
    assert_eq!(status, StatusCode::PAYLOAD_TOO_LARGE, "{response}");
    assert_artifact_http_response_redacted(
        &fixture,
        &response,
        b"zzzzzzzzzzzzzzzz",
        &fixture.applier_token,
    );

    assert_eq!(artifact_source_receipt_count(&fixture), 0);
    assert!(!artifact_blob_path(&fixture, &oversized_digest).exists());
    fixture.cleanup();
}

fn expected_digest_header() -> HeaderName {
    HeaderName::from_static("x-elon-expected-admission-digest")
}
