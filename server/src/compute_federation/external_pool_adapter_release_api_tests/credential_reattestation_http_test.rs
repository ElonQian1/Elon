use base64::{engine::general_purpose::STANDARD, Engine as _};

use super::{credential_reattestation_test_support::*, *};

#[tokio::test]
async fn credential_reattestation_http_records_genesis_replays_renews_and_redacts() {
    let fixture = fixture();
    let roots =
        create_credential_reattestation_fixture(&fixture, "credential-reattest", "52.0.0").await;
    let body = challenge_body(&roots, "credential-reattest-genesis");

    assert_eq!(
        call(
            &fixture.router,
            Method::POST,
            &format!("{}/challenge", collection_path(&roots)),
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
            &format!("{}/challenge", collection_path(&roots)),
            Some(&fixture.member_token),
            &body,
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    let mut unknown = body;
    unknown["credential_locator_commitment"] = json!("d".repeat(64));
    assert_eq!(
        call(
            &fixture.router,
            Method::POST,
            &format!("{}/challenge", collection_path(&roots)),
            Some(&fixture.applier_token),
            &unknown,
        )
        .await
        .0,
        StatusCode::UNPROCESSABLE_ENTITY
    );

    let challenge = issue_challenge(&fixture, &roots, "credential-reattest-genesis").await;
    assert_eq!(challenge["binding"]["sequence"], 1);
    assert!(challenge["binding"]["predecessor_receipt_id"].is_null());
    assert_challenge_redacted(&challenge);
    let record = record_body(&roots, &challenge, "credential-reattest-record-1");
    let (status, created) = call(
        &fixture.router,
        Method::POST,
        &collection_path(&roots),
        Some(&fixture.applier_token),
        &record,
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{created}");
    assert_eq!(created["replayed"], false);
    assert_eq!(
        created["reattestation"]["credential_reattestation_effect"],
        "signed_provider_credential_reattestation_verified_current"
    );
    for effect in [
        "adapter_effect",
        "provider_effect",
        "route_effect",
        "execution_effect",
        "usage_effect",
        "settlement_effect",
    ] {
        assert_eq!(created["reattestation"][effect], "none");
    }
    assert_response_redacted(&created);
    let (status, replayed) = call(
        &fixture.router,
        Method::POST,
        &collection_path(&roots),
        Some(&fixture.applier_token),
        &record,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{replayed}");
    assert_eq!(replayed["replayed"], true);
    assert_eq!(
        replayed["reattestation"]["reattestation_receipt_id"],
        created["reattestation"]["reattestation_receipt_id"]
    );

    let (status, current) = call(
        &fixture.router,
        Method::GET,
        &format!("{}/currentness", collection_path(&roots)),
        Some(&fixture.applier_token),
        &Value::Null,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{current}");
    assert_eq!(current["current_status"], "verified_current");
    assert_eq!(current["head_status"], "head");
    assert_response_redacted(&current);

    let successor = issue_challenge(&fixture, &roots, "credential-reattest-successor").await;
    assert_eq!(successor["binding"]["sequence"], 2);
    assert_eq!(
        successor["binding"]["predecessor_receipt_id"],
        created["reattestation"]["reattestation_receipt_id"]
    );
    let (status, renewed) =
        record_challenge(&fixture, &roots, &successor, "credential-reattest-record-2").await;
    assert_eq!(status, StatusCode::CREATED, "{renewed}");
    assert_eq!(renewed["reattestation"]["sequence"], 2);
    assert_no_effects(&fixture, &roots);
    fixture.cleanup();
}

#[tokio::test]
async fn credential_reattestation_http_revokes_head_and_classifies_failures() {
    let fixture = fixture();
    let roots =
        create_credential_reattestation_fixture(&fixture, "credential-revoke", "52.1.0").await;
    let challenge = issue_challenge(&fixture, &roots, "credential-revoke").await;
    let (status, created) =
        record_challenge(&fixture, &roots, &challenge, "credential-revoke-record").await;
    assert_eq!(status, StatusCode::CREATED, "{created}");

    let mut bad = challenge_body(&roots, "credential-bad");
    bad["provider_authentication_outcome"] = json!("failed");
    assert_eq!(
        call(
            &fixture.router,
            Method::POST,
            &format!("{}/challenge", collection_path(&roots)),
            Some(&fixture.applier_token),
            &bad,
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        call(
            &fixture.router,
            Method::POST,
            "/api/admin/compute/external-pool-adapter-registry-provider-bindings/missing-binding/credential-reattestations/challenge",
            Some(&fixture.applier_token),
            &challenge_body(&roots, "missing-binding"),
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
    let mut bad_signature = record_body(&roots, &challenge, "credential-bad-signature");
    bad_signature["signature_base64"] = json!(STANDARD.encode(vec![7_u8; 256]));
    assert_eq!(
        call(
            &fixture.router,
            Method::POST,
            &collection_path(&roots),
            Some(&fixture.applier_token),
            &bad_signature,
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    let mut wrong_root = challenge_body(&roots, "credential-wrong-root");
    wrong_root["expected_provider_binding_digest"] = json!("e".repeat(64));
    assert_eq!(
        call(
            &fixture.router,
            Method::POST,
            &format!("{}/challenge", collection_path(&roots)),
            Some(&fixture.applier_token),
            &wrong_root,
        )
        .await
        .0,
        StatusCode::CONFLICT
    );

    let receipt_id = created["reattestation"]["reattestation_receipt_id"]
        .as_str()
        .unwrap();
    let receipt_digest = created["reattestation"]["reattestation_receipt_digest"]
        .as_str()
        .unwrap();
    let revoke = json!({
        "expected_reattestation_receipt_digest":receipt_digest,
        "reason":"credential authority fixture intentionally revokes the current head",
        "idempotency_key":"credential-revoke-head",
        "confirm_revocation":true
    });
    let revoke_path = format!("{}/{receipt_id}/revoke", collection_path(&roots));
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
        "credential_reattestation_revoked"
    );
    assert_response_redacted(&revoked);
    let (status, replay) = call(
        &fixture.router,
        Method::POST,
        &revoke_path,
        Some(&fixture.applier_token),
        &revoke,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{replay}");
    assert_eq!(replay["replayed"], true);
    assert_eq!(
        call(
            &fixture.router,
            Method::GET,
            &format!("{}/currentness", collection_path(&roots)),
            Some(&fixture.applier_token),
            &Value::Null,
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    revoke_verifier_key(&fixture, &roots, "credential-revoke").await;
    assert_eq!(
        call(
            &fixture.router,
            Method::POST,
            &format!("{}/challenge", collection_path(&roots)),
            Some(&fixture.applier_token),
            &challenge_body(&roots, "credential-revoked-key"),
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    assert_no_effects(&fixture, &roots);
    fixture.cleanup();
}

#[tokio::test]
async fn credential_reattestation_http_is_registering_only_before_v277() {
    let fixture = fixture();
    let roots =
        create_credential_reattestation_fixture(&fixture, "credential-active", "52.2.0").await;
    let registering = issue_challenge(&fixture, &roots, "credential-active-registering").await;
    let (status, first) =
        record_challenge(&fixture, &roots, &registering, "credential-active-record-1").await;
    assert_eq!(status, StatusCode::CREATED, "{first}");
    assert_eq!(
        first["reattestation"]["observed_provider_status"],
        "registering"
    );

    advance_provider_to_active_revision(&fixture, &roots, 2);
    let (status, historical) = call(
        &fixture.router,
        Method::GET,
        &format!("{}/currentness", collection_path(&roots)),
        Some(&fixture.applier_token),
        &Value::Null,
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT, "{historical}");
    assert!(error(&historical).contains("not current"));
    let (status, rejected) = call(
        &fixture.router,
        Method::POST,
        &format!("{}/challenge", collection_path(&roots)),
        Some(&fixture.applier_token),
        &challenge_body(&roots, "credential-active-rejected"),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT, "{rejected}");
    assert!(error(&rejected).contains("exact registering observation"));

    let provider_binding_id = roots.registry["binding"]["provider_binding_id"]
        .as_str()
        .unwrap();
    let connection = fixture.state.store.conn().unwrap();
    let (receipt_count, current_status): (i64, String) = connection
        .query_row(
            "SELECT
               (SELECT COUNT(*)
                  FROM compute_external_pool_adapter_credential_reattestation_receipts
                 WHERE provider_binding_id=?1),
               current_status
               FROM compute_external_pool_adapter_credential_reattestation_current
              WHERE provider_binding_id=?1",
            [provider_binding_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(receipt_count, 1, "pre-V277 must not mint an active receipt");
    assert_eq!(current_status, "historical_only");
    drop(connection);
    assert_no_effects(&fixture, &roots);
    fixture.cleanup();
}
