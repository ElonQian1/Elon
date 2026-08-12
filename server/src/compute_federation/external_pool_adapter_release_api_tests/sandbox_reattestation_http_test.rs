use super::{sandbox_reattestation_test_support::*, *};

#[tokio::test]
async fn sandbox_reattestation_http_records_provider_neutral_head_replays_and_redacts() {
    let fixture = fixture();
    let roots =
        create_sandbox_reattestation_fixture(&fixture, "sandbox-reattest-genesis", "51.0.0").await;
    let path = collection_path(&roots);
    let body = challenge_body(&roots, "sandbox-reattest-genesis");
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
    assert_eq!(
        call(
            &fixture.router,
            Method::POST,
            &format!("{path}/challenge"),
            Some(&fixture.member_token),
            &body,
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );

    let challenge = issue_challenge(&fixture, &roots, "sandbox-reattest-genesis").await;
    assert_eq!(challenge["binding"]["sequence"], 1);
    assert!(challenge["binding"]["predecessor_receipt_id"].is_null());
    assert_eq!(
        challenge["binding"]["registry_release_digest"],
        roots.roots.registry["release"]["registry_release_digest"]
    );
    assert_eq!(
        challenge["binding"]["installation_content_digest"],
        roots.roots.registry["release"]["installation_content_digest"]
    );
    assert_eq!(
        challenge["binding"]["vulnerability_reattestation_receipt_id"],
        roots.vulnerability_reattestation["reattestation"]["reattestation_receipt_id"]
    );
    assert!(challenge["binding"]["nonce_base64"].as_str().is_some());
    assert!(challenge["signature_message_base64"].as_str().is_some());
    assert_challenge_redacted(&challenge);

    let mut invalid_signature = record_body(
        &roots,
        &challenge,
        "sandbox-reattest-genesis-invalid-signature",
    );
    invalid_signature["signature_base64"] = json!("AA==");
    assert_eq!(
        call(
            &fixture.router,
            Method::POST,
            &path,
            Some(&fixture.applier_token),
            &invalid_signature,
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    let (status, created) = record_challenge(
        &fixture,
        &roots,
        &challenge,
        "sandbox-reattest-genesis-record",
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{created}");
    assert_eq!(created["replayed"], false);
    assert_eq!(created["reattestation"]["sequence"], 1);
    assert_eq!(
        created["reattestation"]["sandbox_reattestation_effect"],
        "signed_sandbox_reattestation_verified_current"
    );
    for effect in [
        "adapter_effect",
        "provider_effect",
        "credential_effect",
        "route_effect",
        "execution_effect",
        "settlement_effect",
    ] {
        assert_eq!(created["reattestation"][effect], "none");
    }
    assert_response_redacted(&created);

    let (status, replay) = record_challenge(
        &fixture,
        &roots,
        &challenge,
        "sandbox-reattest-genesis-record",
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{replay}");
    assert_eq!(replay["replayed"], true);
    let (status, current) = call(
        &fixture.router,
        Method::GET,
        &format!("{path}/currentness"),
        Some(lifecycle_support::LOCAL_OWNER_TOKEN),
        &Value::Null,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{current}");
    assert_eq!(current["current_status"], "verified_current");
    assert_eq!(current["head_status"], "head");
    assert_response_redacted(&current);
    assert_no_activation_effects(&fixture, &roots);
    fixture.cleanup();
}

#[tokio::test]
async fn sandbox_reattestation_http_renews_revokes_and_classifies_failures() {
    let fixture = fixture();
    let roots =
        create_sandbox_reattestation_fixture(&fixture, "sandbox-reattest-renew", "51.1.0").await;
    let path = collection_path(&roots);
    assert_eq!(
        call(
            &fixture.router,
            Method::GET,
            &format!("{path}/currentness"),
            Some(&fixture.applier_token),
            &Value::Null,
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
    let missing = "/api/admin/compute/external-pool-adapter-registry-releases/missing-v252-release/sandbox-reattestations";
    assert_eq!(
        call(
            &fixture.router,
            Method::POST,
            &format!("{missing}/challenge"),
            Some(&fixture.applier_token),
            &challenge_body(&roots, "sandbox-reattest-missing"),
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
    let mut injected = challenge_body(&roots, "sandbox-reattest-injected");
    injected["recorded_by_admin_user_id"] = json!(fixture.applier.id);
    assert_eq!(
        call(
            &fixture.router,
            Method::POST,
            &format!("{path}/challenge"),
            Some(&fixture.applier_token),
            &injected,
        )
        .await
        .0,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    let mut invalid = challenge_body(&roots, "sandbox-reattest-invalid");
    invalid["expected_registry_release_digest"] = json!("not-a-digest");
    assert_eq!(
        call(
            &fixture.router,
            Method::POST,
            &format!("{path}/challenge"),
            Some(&fixture.applier_token),
            &invalid,
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    let mut mismatched_observation =
        challenge_body(&roots, "sandbox-reattest-mismatched-observation");
    mismatched_observation["observations"][0]["test_case_id"] = json!("reconcile-contract-r1-v1");
    assert_eq!(
        call(
            &fixture.router,
            Method::POST,
            &format!("{path}/challenge"),
            Some(&fixture.applier_token),
            &mismatched_observation,
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    assert_eq!(
        call(
            &fixture.router,
            Method::POST,
            &path,
            Some(&fixture.applier_token),
            &json!({
                "challenge_id":"missing-v252-challenge",
                "expected_signature_message_digest":"a".repeat(64),
                "signature_base64":"AA==",
                "idempotency_key":"sandbox-reattest-missing-challenge",
                "confirm_reattestation":true
            }),
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );

    let first = issue_challenge(&fixture, &roots, "sandbox-reattest-first").await;
    let stale = issue_challenge(&fixture, &roots, "sandbox-reattest-stale").await;
    let (_, first_receipt) =
        record_challenge(&fixture, &roots, &first, "sandbox-reattest-first-record").await;
    assert_eq!(
        record_challenge(&fixture, &roots, &stale, "sandbox-reattest-stale-record")
            .await
            .0,
        StatusCode::CONFLICT
    );
    let renewed = issue_challenge(&fixture, &roots, "sandbox-reattest-second").await;
    assert_eq!(renewed["binding"]["sequence"], 2);
    assert_eq!(
        renewed["binding"]["predecessor_receipt_id"],
        first_receipt["reattestation"]["reattestation_receipt_id"]
    );
    let (_, second) =
        record_challenge(&fixture, &roots, &renewed, "sandbox-reattest-second-record").await;
    let receipt_id = second["reattestation"]["reattestation_receipt_id"]
        .as_str()
        .unwrap();
    let receipt_digest = second["reattestation"]["reattestation_receipt_digest"]
        .as_str()
        .unwrap();
    let revoke_path = format!("{path}/{receipt_id}/revoke");
    let revoke = json!({
        "expected_reattestation_receipt_digest":receipt_digest,
        "reason":"fixture retires the renewable sandbox head",
        "idempotency_key":"sandbox-reattest-revoke",
        "confirm_revocation":true
    });
    let (status, revoked) = call(
        &fixture.router,
        Method::POST,
        &revoke_path,
        Some(&fixture.applier_token),
        &revoke,
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{revoked}");
    assert_eq!(
        revoked["revocation"]["revocation_effect"],
        "sandbox_reattestation_revoked"
    );
    assert_response_redacted(&revoked);
    assert_eq!(
        call(
            &fixture.router,
            Method::POST,
            &revoke_path,
            Some(&fixture.applier_token),
            &revoke,
        )
        .await
        .0,
        StatusCode::OK
    );
    assert_eq!(
        call(
            &fixture.router,
            Method::POST,
            &format!("{path}/missing-v252-receipt/revoke"),
            Some(&fixture.applier_token),
            &json!({
                "expected_reattestation_receipt_digest":"a".repeat(64),
                "reason":"fixture cannot revoke an absent sandbox receipt",
                "idempotency_key":"sandbox-reattest-missing-revoke",
                "confirm_revocation":true
            }),
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        call(
            &fixture.router,
            Method::GET,
            &format!("{path}/currentness"),
            Some(&fixture.applier_token),
            &Value::Null,
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    assert_no_activation_effects(&fixture, &roots);
    fixture.cleanup();
}
