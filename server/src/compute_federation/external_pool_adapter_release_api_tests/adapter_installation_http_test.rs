use super::{adapter_adoption_http_test::create_current_adoption, *};

const INSTALLATIONS_PATH: &str = "/api/admin/compute/external-pool-adapter-installations";

#[tokio::test]
async fn Adapter_installation_http_installs_exact_bytes_replays_and_stays_inert() {
    let fixture = fixture();
    let adoption = create_current_adoption(&fixture, "installation-http", "46.0.0").await;
    let body = installation_body(&fixture, &adoption, "installation-create", true);

    assert_eq!(
        call(
            &fixture.router,
            Method::POST,
            INSTALLATIONS_PATH,
            None,
            &body
        )
        .await
        .0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        call(
            &fixture.router,
            Method::POST,
            INSTALLATIONS_PATH,
            Some(&fixture.member_token),
            &body,
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );

    let (status, created) = call(
        &fixture.router,
        Method::POST,
        INSTALLATIONS_PATH,
        Some(&fixture.applier_token),
        &body,
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{created}");
    assert_eq!(created["replayed"], false);
    assert_eq!(
        created["installation"]["installation_effect"],
        "adapter_bytes_installed_inert"
    );
    for effect in [
        "credential_effect",
        "provider_effect",
        "route_effect",
        "execution_effect",
        "settlement_effect",
    ] {
        assert_eq!(created["installation"][effect], "none");
    }
    assert_installation_redacted(&fixture, &created);

    let (status, replay) = call(
        &fixture.router,
        Method::POST,
        INSTALLATIONS_PATH,
        Some(&fixture.applier_token),
        &body,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{replay}");
    assert_eq!(replay["replayed"], true);
    assert_eq!(
        replay["installation"]["installation_receipt_id"],
        created["installation"]["installation_receipt_id"]
    );
    assert_installation_redacted(&fixture, &replay);

    let receipt_id = created["installation"]["installation_receipt_id"]
        .as_str()
        .unwrap();
    let (status, current) = call(
        &fixture.router,
        Method::GET,
        &format!("{INSTALLATIONS_PATH}/{receipt_id}/currentness"),
        Some(&fixture.applier_token),
        &Value::Null,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{current}");
    assert_eq!(current["current_status"], "installed_upstreams_current");
    assert_eq!(current["adoption_status"], "adopted_current");
    assert_eq!(current["package_status"], "verified_current");
    assert_eq!(current["source_status"], "exact");
    assert_eq!(current["file_inventory_status"], "exact");
    assert_installation_redacted(&fixture, &current);
    assert_installation_has_no_activation_effects(&fixture, &adoption);
    fixture.cleanup();
}

#[tokio::test]
async fn Adapter_installation_http_classifies_json_confirmation_missing_and_drift() {
    let fixture = fixture();
    let adoption = create_current_adoption(&fixture, "installation-failure", "46.1.0").await;
    let body = installation_body(&fixture, &adoption, "installation-failure", true);

    let mut unknown = body.clone();
    unknown["installed_by_admin_user_id"] = json!(fixture.applier.id);
    assert_eq!(
        call(
            &fixture.router,
            Method::POST,
            INSTALLATIONS_PATH,
            Some(&fixture.applier_token),
            &unknown,
        )
        .await
        .0,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    let mut unconfirmed = body.clone();
    unconfirmed["confirm_installed_inert"] = json!(false);
    assert_eq!(
        call(
            &fixture.router,
            Method::POST,
            INSTALLATIONS_PATH,
            Some(&fixture.applier_token),
            &unconfirmed,
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    let missing = json!({
        "adoption_receipt_id":"missing-installation-adoption",
        "expected_adoption_receipt_digest":"a".repeat(64),
        "expected_package_receipt_digest":"b".repeat(64),
        "expected_source_receipt_digest":"c".repeat(64),
        "idempotency_key":"missing-installation",
        "confirm_installed_inert":true
    });
    assert_eq!(
        call(
            &fixture.router,
            Method::POST,
            INSTALLATIONS_PATH,
            Some(&fixture.applier_token),
            &missing,
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );

    let (status, created) = call(
        &fixture.router,
        Method::POST,
        INSTALLATIONS_PATH,
        Some(&fixture.applier_token),
        &body,
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{created}");
    let content_digest = created["installation"]["installation_content_digest"]
        .as_str()
        .unwrap();
    let entrypoint = installation_root(&fixture, content_digest).join("bin/adapter.sh");
    std::fs::write(&entrypoint, b"drifted inert bytes").unwrap();
    let receipt_id = created["installation"]["installation_receipt_id"]
        .as_str()
        .unwrap();
    assert_eq!(
        call(
            &fixture.router,
            Method::GET,
            &format!("{INSTALLATIONS_PATH}/{receipt_id}/currentness"),
            Some(&fixture.applier_token),
            &Value::Null,
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    let mut changed = body;
    changed["expected_package_receipt_digest"] = json!("d".repeat(64));
    assert_eq!(
        call(
            &fixture.router,
            Method::POST,
            INSTALLATIONS_PATH,
            Some(&fixture.applier_token),
            &changed,
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    fixture.cleanup();
}

fn installation_body(
    fixture: &Fixture,
    adoption: &Value,
    idempotency_key: &str,
    confirmed: bool,
) -> Value {
    let admission_id = adoption["adoption"]["admission_id"].as_str().unwrap();
    let connection = fixture.state.store.conn().unwrap();
    let (package_digest, source_digest): (String, String) = connection
        .query_row(
            "SELECT package_receipt_digest,source_receipt_digest
               FROM compute_external_pool_adapter_artifact_package_receipts
              WHERE admission_id=?1",
            [admission_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    json!({
        "adoption_receipt_id":adoption["adoption"]["adoption_receipt_id"],
        "expected_adoption_receipt_digest":adoption["adoption"]["adoption_receipt_digest"],
        "expected_package_receipt_digest":package_digest,
        "expected_source_receipt_digest":source_digest,
        "idempotency_key":idempotency_key,
        "confirm_installed_inert":confirmed
    })
}

fn installation_root(fixture: &Fixture, digest: &str) -> PathBuf {
    fixture
        .data_dir
        .join("compute-federation/external-pool-adapter-artifacts/v1/installed-inert/sha256")
        .join(&digest[..2])
        .join(digest)
}

fn assert_installation_redacted(fixture: &Fixture, value: &Value) {
    let encoded = value.to_string();
    for forbidden in [
        "bin/adapter.sh",
        "credential_locator_commitment",
        "installed_files",
        "idempotency_key",
        "idempotency_scope",
        "confirmation",
        "signature_base64",
        &fixture.data_dir.display().to_string(),
        &fixture.database_path.display().to_string(),
    ] {
        assert!(
            !encoded.contains(forbidden),
            "exposed {forbidden}: {encoded}"
        );
    }
}

fn assert_installation_has_no_activation_effects(fixture: &Fixture, adoption: &Value) {
    let provider_id = adoption["adoption"]["provider_id"].as_str().unwrap();
    let connection = fixture.state.store.conn().unwrap();
    let status: String = connection
        .query_row(
            "SELECT status FROM compute_providers WHERE provider_id=?1",
            [provider_id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(status, "registering");
    for table in [
        "compute_route_adapters",
        "compute_route_credentials",
        "compute_route_authorization_receipts",
        "compute_service_actor_authorizations",
        "compute_attempt_start_outbox",
    ] {
        let count: i64 = connection
            .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(count, 0, "installation unexpectedly populated {table}");
    }
}
