use super::{
    adapter_adoption_http_test::create_current_adoption,
    adapter_installation_http_test::{
        assert_installation_has_no_activation_effects, assert_installation_redacted,
        installation_body, installation_root,
    },
    *,
};

const INSTALLATIONS_PATH: &str = "/api/admin/compute/external-pool-adapter-installations";

#[tokio::test]
async fn Adapter_installation_revocation_http_appends_terminal_and_preserves_inert_bytes() {
    let fixture = fixture();
    let adoption = create_current_adoption(&fixture, "installation-revoke", "47.0.0").await;
    let installation_body =
        installation_body(&fixture, &adoption, "installation-revoke-create", true);
    let (status, created) = call(
        &fixture.router,
        Method::POST,
        INSTALLATIONS_PATH,
        Some(&fixture.applier_token),
        &installation_body,
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{created}");

    let installation_id = created["installation"]["installation_receipt_id"]
        .as_str()
        .unwrap();
    let installation_digest = created["installation"]["installation_receipt_digest"]
        .as_str()
        .unwrap();
    let content_digest = created["installation"]["installation_content_digest"]
        .as_str()
        .unwrap();
    let installed_root = installation_root(&fixture, content_digest);
    assert!(installed_root.is_dir());
    let revoke_path = format!("{INSTALLATIONS_PATH}/{installation_id}/revoke");
    let revoke = json!({
        "expected_installation_receipt_digest":installation_digest,
        "reason":"operator intentionally retires this inert installed instance",
        "idempotency_key":"installation-revoke-terminal",
        "confirm_revocation":true
    });

    assert_eq!(
        call(&fixture.router, Method::POST, &revoke_path, None, &revoke)
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        call(
            &fixture.router,
            Method::POST,
            &revoke_path,
            Some(&fixture.member_token),
            &revoke,
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    let mut actor_injection = revoke.clone();
    actor_injection["revoked_by_admin_user_id"] = json!(fixture.applier.id);
    assert_eq!(
        call(
            &fixture.router,
            Method::POST,
            &revoke_path,
            Some(&fixture.applier_token),
            &actor_injection,
        )
        .await
        .0,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    let mut unconfirmed = revoke.clone();
    unconfirmed["confirm_revocation"] = json!(false);
    assert_eq!(
        call(
            &fixture.router,
            Method::POST,
            &revoke_path,
            Some(&fixture.applier_token),
            &unconfirmed,
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    let mut wrong_digest = revoke.clone();
    wrong_digest["expected_installation_receipt_digest"] = json!("f".repeat(64));
    wrong_digest["idempotency_key"] = json!("installation-revoke-wrong-digest");
    assert_eq!(
        call(
            &fixture.router,
            Method::POST,
            &revoke_path,
            Some(&fixture.applier_token),
            &wrong_digest,
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    let missing_path = format!("{INSTALLATIONS_PATH}/missing-installation/revoke");
    let missing = json!({
        "expected_installation_receipt_digest":"a".repeat(64),
        "reason":"operator retires a missing fixture installation",
        "idempotency_key":"installation-revoke-missing",
        "confirm_revocation":true
    });
    assert_eq!(
        call(
            &fixture.router,
            Method::POST,
            &missing_path,
            Some(&fixture.applier_token),
            &missing,
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );

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
        revoked["installation"]["installation_receipt_id"],
        installation_id
    );
    assert_eq!(revoked["terminal"]["terminal_kind"], "revoked");
    assert_eq!(
        revoked["terminal"]["installation_receipt_digest"],
        installation_digest
    );
    assert_eq!(
        revoked["terminal"]["revoked_by_admin_user_id"],
        fixture.applier.id
    );
    assert_eq!(
        revoked["terminal"]["installation_effect"],
        "installed_instance_revoked"
    );
    for effect in [
        "credential_effect",
        "provider_effect",
        "route_effect",
        "execution_effect",
        "settlement_effect",
    ] {
        assert_eq!(revoked["terminal"][effect], "none");
    }
    assert_eq!(revoked["replayed"], false);
    assert_installation_redacted(&fixture, &revoked);
    assert!(
        installed_root.is_dir(),
        "revocation must not delete CAS bytes"
    );

    let (status, replayed) = call(
        &fixture.router,
        Method::POST,
        &revoke_path,
        Some(&fixture.applier_token),
        &revoke,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{replayed}");
    assert_eq!(replayed["replayed"], true);
    assert_eq!(
        replayed["terminal"]["terminal_receipt_id"],
        revoked["terminal"]["terminal_receipt_id"]
    );
    let mut changed_replay = revoke;
    changed_replay["reason"] = json!("changed terminal history is forbidden");
    assert_eq!(
        call(
            &fixture.router,
            Method::POST,
            &revoke_path,
            Some(&fixture.applier_token),
            &changed_replay,
        )
        .await
        .0,
        StatusCode::CONFLICT
    );

    let (status, historical) = call(
        &fixture.router,
        Method::GET,
        &format!("{INSTALLATIONS_PATH}/{installation_id}/currentness"),
        Some(&fixture.applier_token),
        &Value::Null,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{historical}");
    assert_eq!(historical["current_status"], "historical_only");
    assert_eq!(historical["terminal_status"], "revoked");
    assert_eq!(
        historical["terminal"]["terminal_receipt_id"],
        revoked["terminal"]["terminal_receipt_id"]
    );
    assert_installation_redacted(&fixture, &historical);
    assert_installation_has_no_activation_effects(&fixture, &adoption);

    let terminal_count: i64 = fixture
        .state
        .store
        .conn()
        .unwrap()
        .query_row(
            "SELECT COUNT(*) FROM compute_external_pool_adapter_installation_terminal_receipts",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(terminal_count, 1);
    fixture.cleanup();
}
