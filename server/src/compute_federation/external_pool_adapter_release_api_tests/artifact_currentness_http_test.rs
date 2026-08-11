use super::{lifecycle_support::*, *};

#[tokio::test]
async fn artifact_receipt_stays_historical_while_terminal_blocks_every_put_before_body_poll() {
    let fixture = fixture();
    let artifact = b"external-pool-adapter-real-raw-artifact-v1";
    let artifact_digest = sha256(artifact);
    let staged = stage_release(&fixture, "artifact-currentness", "10.0.1", artifact).await;

    let (status, source_receipt) = raw_artifact_call(
        &fixture.router,
        &staged,
        Some(&fixture.applier_token),
        "artifact-source-exact",
        Body::from(artifact.to_vec()),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{source_receipt}");
    assert_eq!(source_receipt["replayed"], false);
    assert_eq!(source_receipt["admission_id"], staged["admission_id"]);
    assert_eq!(
        source_receipt["declared_implementation_sha256"],
        artifact_digest
    );
    assert_eq!(source_receipt["intake_sha256"], artifact_digest);
    assert_eq!(source_receipt["reopened_sha256"], artifact_digest);
    assert_eq!(source_receipt["custody_state"], "quarantined");
    assert!(source_receipt.get("content_address_digest").is_none());
    assert!(source_receipt.get("artifact_size_bytes").is_none());
    assert_release_material_redacted(&source_receipt, &fixture.applier_token);
    assert!(!source_receipt
        .to_string()
        .contains(std::str::from_utf8(artifact).unwrap()));

    let blob_path = artifact_blob_path(&fixture, &artifact_digest);
    assert_eq!(std::fs::read(&blob_path).unwrap(), artifact);

    let terminal = terminal_body(
        &staged,
        "artifact-currentness-terminal",
        "revoked",
        None,
        true,
    );
    let (status, terminal_receipt) = call(
        &fixture.router,
        Method::POST,
        &terminal_path(&staged),
        Some(&fixture.applier_token),
        &terminal,
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{terminal_receipt}");
    assert_eq!(
        terminal_receipt["terminal_receipt"]["terminal"]["existing_artifact_source_effect"],
        "historical_only"
    );

    let (status, historical_receipt) = call(
        &fixture.router,
        Method::GET,
        &artifact_source_path(&staged),
        Some(&fixture.applier_token),
        &json!({}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{historical_receipt}");
    assert_eq!(
        historical_receipt["source_receipt_id"],
        source_receipt["source_receipt_id"]
    );
    assert_release_material_redacted(&historical_receipt, &fixture.applier_token);
    assert!(!historical_receipt
        .to_string()
        .contains(std::str::from_utf8(artifact).unwrap()));

    let (exact_body, exact_polled) = tracked_body(artifact);
    let (status, exact_replay) = raw_artifact_call(
        &fixture.router,
        &staged,
        Some(&fixture.applier_token),
        "artifact-source-exact",
        exact_body,
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT, "{exact_replay}");
    assert!(
        !was_polled(&exact_polled),
        "terminal exact replay polled the raw request body"
    );

    let (fresh_body, fresh_polled) = tracked_body(b"fresh-body-must-remain-unpolled");
    let (status, fresh_put) = raw_artifact_call(
        &fixture.router,
        &staged,
        Some(&fixture.applier_token),
        "artifact-source-fresh-after-terminal",
        fresh_body,
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT, "{fresh_put}");
    assert!(
        !was_polled(&fresh_polled),
        "terminal fresh PUT polled the raw request body"
    );

    std::fs::write(&blob_path, b"external-pool-adapter-drifted-artifact").unwrap();
    let (status, drifted) = call(
        &fixture.router,
        Method::GET,
        &artifact_source_path(&staged),
        Some(&fixture.applier_token),
        &json!({}),
    )
    .await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR, "{drifted}");
    assert_release_material_redacted(&drifted, &fixture.applier_token);
    fixture.cleanup();
}
