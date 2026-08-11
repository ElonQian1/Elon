use super::{lifecycle_support::*, *};

#[tokio::test]
async fn artifact_put_creates_once_then_exact_replay_and_get_return_redacted_receipts() {
    let fixture = fixture();
    let artifact = b"artifact-http-exact-replay-v1";
    let artifact_digest = sha256(artifact);
    let staged = stage_release(&fixture, "artifact-http-replay", "11.4.1", artifact).await;

    let (status, created) = raw_artifact_call(
        &fixture.router,
        &staged,
        Some(&fixture.applier_token),
        "artifact-http-exact-key",
        Body::from(artifact.to_vec()),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{created}");
    assert_eq!(created["replayed"], false);
    assert_eq!(created["admission_id"], staged["admission_id"]);
    assert_eq!(created["declared_implementation_sha256"], artifact_digest);
    assert_eq!(created["intake_sha256"], artifact_digest);
    assert_eq!(created["reopened_sha256"], artifact_digest);
    assert_artifact_http_response_redacted(&fixture, &created, artifact, &fixture.applier_token);
    assert_eq!(
        std::fs::read(artifact_blob_path(&fixture, &artifact_digest)).unwrap(),
        artifact
    );

    let (status, replayed) = raw_artifact_call(
        &fixture.router,
        &staged,
        Some(&fixture.applier_token),
        "artifact-http-exact-key",
        Body::from(artifact.to_vec()),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{replayed}");
    assert_eq!(replayed["replayed"], true);
    assert_eq!(replayed["source_receipt_id"], created["source_receipt_id"]);
    assert_eq!(
        replayed["source_receipt_digest"],
        created["source_receipt_digest"]
    );
    assert_artifact_http_response_redacted(&fixture, &replayed, artifact, &fixture.applier_token);

    let (status, fetched) = call(
        &fixture.router,
        Method::GET,
        &artifact_source_path(&staged),
        Some(&fixture.applier_token),
        &json!({}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{fetched}");
    assert_eq!(fetched["replayed"], false);
    assert_eq!(fetched["source_receipt_id"], created["source_receipt_id"]);
    assert_artifact_http_response_redacted(&fixture, &fetched, artifact, &fixture.applier_token);

    assert_eq!(artifact_source_receipt_count(&fixture), 1);
    fixture.cleanup();
}

#[tokio::test]
async fn artifact_put_rejects_different_key_and_same_key_material_drift() {
    let fixture = fixture();
    let artifact_a = b"artifact-http-conflict-a-v1";
    let artifact_b = b"artifact-http-conflict-b-v1";
    let digest_a = sha256(artifact_a);
    let digest_b = sha256(artifact_b);
    let staged_a = stage_release(&fixture, "artifact-http-conflict-a", "11.5.1", artifact_a).await;
    let staged_b = stage_release(&fixture, "artifact-http-conflict-b", "11.5.2", artifact_b).await;

    let (status, created) = raw_artifact_call(
        &fixture.router,
        &staged_a,
        Some(&fixture.applier_token),
        "artifact-http-shared-key",
        Body::from(artifact_a.to_vec()),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{created}");

    let (status, different_key) = raw_artifact_call(
        &fixture.router,
        &staged_a,
        Some(&fixture.applier_token),
        "artifact-http-different-key",
        Body::from(artifact_a.to_vec()),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT, "{different_key}");
    assert_artifact_http_response_redacted(
        &fixture,
        &different_key,
        artifact_a,
        &fixture.applier_token,
    );

    let (status, material_drift) = raw_artifact_call(
        &fixture.router,
        &staged_b,
        Some(&fixture.applier_token),
        "artifact-http-shared-key",
        Body::from(artifact_b.to_vec()),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT, "{material_drift}");
    assert_artifact_http_response_redacted(
        &fixture,
        &material_drift,
        artifact_b,
        &fixture.applier_token,
    );

    let (status, missing_receipt) = call(
        &fixture.router,
        Method::GET,
        &artifact_source_path(&staged_b),
        Some(&fixture.applier_token),
        &json!({}),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{missing_receipt}");
    assert_eq!(artifact_source_receipt_count(&fixture), 1);
    assert_eq!(
        std::fs::read(artifact_blob_path(&fixture, &digest_a)).unwrap(),
        artifact_a
    );
    assert_eq!(
        std::fs::read(artifact_blob_path(&fixture, &digest_b)).unwrap(),
        artifact_b
    );
    fixture.cleanup();
}

#[tokio::test]
async fn artifact_get_and_exact_replay_fail_closed_when_receipt_blob_is_missing() {
    let fixture = fixture();
    let artifact = b"artifact-http-missing-blob-v1";
    let artifact_digest = sha256(artifact);
    let staged = stage_release(&fixture, "artifact-http-missing-blob", "11.6.1", artifact).await;

    let (status, created) = raw_artifact_call(
        &fixture.router,
        &staged,
        Some(&fixture.applier_token),
        "artifact-http-missing-blob-key",
        Body::from(artifact.to_vec()),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{created}");
    std::fs::remove_file(artifact_blob_path(&fixture, &artifact_digest)).unwrap();

    let (status, missing) = call(
        &fixture.router,
        Method::GET,
        &artifact_source_path(&staged),
        Some(&fixture.applier_token),
        &json!({}),
    )
    .await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR, "{missing}");
    assert_artifact_http_response_redacted(&fixture, &missing, artifact, &fixture.applier_token);

    let (replay_body, replay_polled) = tracked_body(artifact);
    let (status, replay) = raw_artifact_call(
        &fixture.router,
        &staged,
        Some(&fixture.applier_token),
        "artifact-http-missing-blob-key",
        replay_body,
    )
    .await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR, "{replay}");
    assert!(
        !was_polled(&replay_polled),
        "missing receipt blob replay polled the raw body"
    );
    assert_artifact_http_response_redacted(&fixture, &replay, artifact, &fixture.applier_token);

    assert_eq!(artifact_source_receipt_count(&fixture), 1);
    fixture.cleanup();
}
