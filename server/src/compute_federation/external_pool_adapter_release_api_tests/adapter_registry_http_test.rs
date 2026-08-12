use super::{adapter_registry_test_support::*, *};

#[tokio::test]
async fn adapter_registry_http_registers_neutral_release_replays_and_stays_inert() {
    let fixture = fixture();
    let installed = create_installed_registry_fixture(&fixture, "registry-http", "48.0.0").await;
    let body = registry_body(&installed.installation, "registry-http-bind", true);

    assert_eq!(
        call(&fixture.router, Method::POST, REGISTRY_PATH, None, &body)
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        call(
            &fixture.router,
            Method::POST,
            REGISTRY_PATH,
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
        REGISTRY_PATH,
        Some(&fixture.applier_token),
        &body,
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{created}");
    assert_eq!(created["replayed"], false);
    assert_eq!(
        created["release"]["registry_effect"],
        "provider_neutral_release_registered"
    );
    assert_eq!(
        created["binding"]["registry_effect"],
        "installed_instance_companion_recorded"
    );
    assert!(created["binding"].get("bound_by_admin_user_id").is_none());
    for side in ["release", "binding"] {
        for effect in [
            "provider_effect",
            "credential_effect",
            "route_effect",
            "execution_effect",
            "settlement_effect",
        ] {
            assert_eq!(created[side][effect], "none");
        }
    }
    assert_registry_redacted(&fixture, &created);

    let (status, replayed) = call(
        &fixture.router,
        Method::POST,
        REGISTRY_PATH,
        Some(&fixture.applier_token),
        &body,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{replayed}");
    assert_eq!(replayed["replayed"], true);
    assert_eq!(
        replayed["binding"]["provider_binding_id"],
        created["binding"]["provider_binding_id"]
    );

    let (status, current) = call(
        &fixture.router,
        Method::GET,
        &currentness_path(&created),
        Some(&fixture.applier_token),
        &Value::Null,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{current}");
    assert_eq!(current["current_status"], "binding_current");
    assert_eq!(current["release_status"], "release_current");
    assert_eq!(current["adoption_terminal_status"], "none");
    assert_eq!(current["installation_terminal_status"], "none");
    assert_eq!(current["provider_status"], "exact_registering");
    assert_eq!(current["file_inventory_status"], "reopened_rehashed_exact");
    assert_eq!(current["route_projection_status"], "reserved");
    assert_registry_redacted(&fixture, &current);
    assert_no_registry_activation_effects(&fixture, &created);
    fixture.cleanup();
}

#[tokio::test]
async fn adapter_registry_http_rejects_bad_input_missing_authority_and_filesystem_drift() {
    let fixture = fixture();
    let installed = create_installed_registry_fixture(&fixture, "registry-errors", "48.1.0").await;
    let body = registry_body(&installed.installation, "registry-errors-bind", true);

    let mut actor_injection = body.clone();
    actor_injection["bound_by_admin_user_id"] = json!(fixture.applier.id);
    assert_eq!(
        call(
            &fixture.router,
            Method::POST,
            REGISTRY_PATH,
            Some(&fixture.applier_token),
            &actor_injection,
        )
        .await
        .0,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    let mut unconfirmed = body.clone();
    unconfirmed["confirm_registry_binding"] = json!(false);
    assert_eq!(
        call(
            &fixture.router,
            Method::POST,
            REGISTRY_PATH,
            Some(&fixture.applier_token),
            &unconfirmed,
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    let missing = json!({
        "installation_receipt_id":"missing-registry-installation",
        "expected_installation_receipt_digest":"a".repeat(64),
        "idempotency_key":"registry-missing",
        "confirm_registry_binding":true
    });
    assert_eq!(
        call(
            &fixture.router,
            Method::POST,
            REGISTRY_PATH,
            Some(&fixture.applier_token),
            &missing,
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        call(
            &fixture.router,
            Method::GET,
            &format!("{REGISTRY_PATH}/missing-binding/currentness"),
            Some(&fixture.applier_token),
            &Value::Null,
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );

    let (status, created) = call(
        &fixture.router,
        Method::POST,
        REGISTRY_PATH,
        Some(&fixture.applier_token),
        &body,
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{created}");
    std::fs::write(
        installed_entrypoint(&fixture, &installed.installation),
        b"drifted registry bytes",
    )
    .unwrap();
    assert_eq!(
        call(
            &fixture.router,
            Method::GET,
            &currentness_path(&created),
            Some(&fixture.applier_token),
            &Value::Null,
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    let mut changed = body;
    changed["expected_installation_receipt_digest"] = json!("f".repeat(64));
    assert_eq!(
        call(
            &fixture.router,
            Method::POST,
            REGISTRY_PATH,
            Some(&fixture.applier_token),
            &changed,
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    fixture.cleanup();
}

#[tokio::test]
async fn adapter_registry_http_reuses_neutral_release_with_independent_provider_companions() {
    let fixture = fixture();
    let first = create_installed_registry_fixture(&fixture, "registry-first", "48.2.0").await;
    let (_, second_installation) =
        create_second_provider_installation(&fixture, &first, "registry-second").await;
    let (_, first_binding) = call(
        &fixture.router,
        Method::POST,
        REGISTRY_PATH,
        Some(&fixture.applier_token),
        &registry_body(&first.installation, "registry-first-bind", true),
    )
    .await;
    let (status, second_binding) = call(
        &fixture.router,
        Method::POST,
        REGISTRY_PATH,
        Some(&fixture.applier_token),
        &registry_body(&second_installation, "registry-second-bind", true),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{second_binding}");
    assert_eq!(
        first_binding["release"]["registry_release_id"],
        second_binding["release"]["registry_release_id"]
    );
    assert_ne!(
        first_binding["binding"]["provider_binding_id"],
        second_binding["binding"]["provider_binding_id"]
    );
    assert_ne!(
        first_binding["binding"]["provider_id"],
        second_binding["binding"]["provider_id"]
    );
    let connection = fixture.state.store.conn().unwrap();
    for (table, expected) in [
        ("compute_external_pool_adapter_registry_releases", 1_i64),
        (
            "compute_external_pool_adapter_registry_provider_bindings",
            2_i64,
        ),
    ] {
        let count: i64 = connection
            .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(count, expected, "unexpected {table} count");
    }
    assert_no_registry_activation_effects(&fixture, &first_binding);
    assert_no_registry_activation_effects(&fixture, &second_binding);
    drop(connection);
    fixture.cleanup();
}

#[tokio::test]
async fn adapter_registry_currentness_fails_closed_after_installation_terminal() {
    let fixture = fixture();
    let installed =
        create_installed_registry_fixture(&fixture, "registry-terminal", "48.3.0").await;
    let (_, created) = call(
        &fixture.router,
        Method::POST,
        REGISTRY_PATH,
        Some(&fixture.applier_token),
        &registry_body(&installed.installation, "registry-terminal-bind", true),
    )
    .await;
    let installation_id = installed.installation["installation"]["installation_receipt_id"]
        .as_str()
        .unwrap();
    let installation_digest = installed.installation["installation"]["installation_receipt_digest"]
        .as_str()
        .unwrap();
    let revoke = json!({
        "expected_installation_receipt_digest":installation_digest,
        "reason":"registry fixture explicitly retires the installed instance",
        "idempotency_key":"registry-terminal-revoke",
        "confirm_revocation":true
    });
    let (status, terminal) = call(
        &fixture.router,
        Method::POST,
        &format!("/api/admin/compute/external-pool-adapter-installations/{installation_id}/revoke"),
        Some(&fixture.applier_token),
        &revoke,
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{terminal}");
    assert_eq!(
        call(
            &fixture.router,
            Method::GET,
            &currentness_path(&created),
            Some(&fixture.applier_token),
            &Value::Null,
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    assert_no_registry_activation_effects(&fixture, &created);
    fixture.cleanup();
}
