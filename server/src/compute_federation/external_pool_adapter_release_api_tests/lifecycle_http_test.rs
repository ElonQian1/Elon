use super::{lifecycle_support::*, *};

#[tokio::test]
async fn lifecycle_http_enforces_auth_confirmation_idempotency_and_currentness() {
    let fixture = fixture();
    let staged = stage_release(
        &fixture,
        "lifecycle-admin",
        "9.0.1",
        b"external-pool-adapter-lifecycle-admin",
    )
    .await;
    let terminal_path = terminal_path(&staged);
    let currentness_path = currentness_path(&staged);
    let terminal = terminal_body(&staged, "lifecycle-admin-terminal", "revoked", None, true);
    let empty = json!({});

    assert_eq!(
        call(
            &fixture.router,
            Method::POST,
            &terminal_path,
            None,
            &terminal,
        )
        .await
        .0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        call(
            &fixture.router,
            Method::POST,
            &terminal_path,
            Some(&fixture.member_token),
            &terminal,
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        call(
            &fixture.router,
            Method::GET,
            &currentness_path,
            None,
            &empty,
        )
        .await
        .0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        call(
            &fixture.router,
            Method::GET,
            &currentness_path,
            Some(&fixture.member_token),
            &empty,
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );

    let (status, staged_currentness) = call(
        &fixture.router,
        Method::GET,
        &currentness_path,
        Some(&fixture.applier_token),
        &empty,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{staged_currentness}");
    assert_eq!(staged_currentness["admission_status"], "staged");
    assert_eq!(staged_currentness["current_status"], "staged");
    assert!(staged_currentness["terminal_receipt_id"].is_null());

    let mut unknown_field = terminal.clone();
    unknown_field["actor_id"] = json!(fixture.applier.id);
    assert_eq!(
        call(
            &fixture.router,
            Method::POST,
            &terminal_path,
            Some(&fixture.applier_token),
            &unknown_field,
        )
        .await
        .0,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    let unconfirmed = terminal_body(
        &staged,
        "lifecycle-admin-unconfirmed",
        "revoked",
        None,
        false,
    );
    assert_eq!(
        call(
            &fixture.router,
            Method::POST,
            &terminal_path,
            Some(&fixture.applier_token),
            &unconfirmed,
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );

    let (status, created) = call(
        &fixture.router,
        Method::POST,
        &terminal_path,
        Some(&fixture.applier_token),
        &terminal,
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{created}");
    assert_eq!(created["replayed"], false);
    assert_eq!(
        created["terminal_receipt"]["terminal"]["admission"]["admission_id"],
        staged["admission_id"]
    );
    assert_eq!(
        created["terminal_receipt"]["terminal"]["actor_id"],
        fixture.applier.id
    );
    assert_eq!(
        created["terminal_receipt"]["terminal"]["confirmation"],
        "confirm_external_pool_adapter_release_admission_revocation"
    );
    assert_eq!(
        created["terminal_receipt"]["terminal"]["currentness_effect"],
        "admission_terminal"
    );
    assert_eq!(
        created["terminal_receipt"]["terminal"]["artifact_intake_effect"],
        "blocked"
    );
    assert_release_material_redacted(&created, &fixture.applier_token);

    let (status, replayed) = call(
        &fixture.router,
        Method::POST,
        &terminal_path,
        Some(&fixture.applier_token),
        &terminal,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{replayed}");
    assert_eq!(replayed["replayed"], true);
    assert_eq!(
        replayed["terminal_receipt"]["terminal_receipt_id"],
        created["terminal_receipt"]["terminal_receipt_id"]
    );

    let mut changed_replay = terminal.clone();
    changed_replay["reason"] = json!("revoked with different immutable replay material");
    assert_eq!(
        call(
            &fixture.router,
            Method::POST,
            &terminal_path,
            Some(&fixture.applier_token),
            &changed_replay,
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    let second_terminal = terminal_body(
        &staged,
        "lifecycle-admin-second-terminal",
        "withdrawn",
        None,
        true,
    );
    assert_eq!(
        call(
            &fixture.router,
            Method::POST,
            &terminal_path,
            Some(&fixture.applier_token),
            &second_terminal,
        )
        .await
        .0,
        StatusCode::CONFLICT
    );

    let (status, terminal_currentness) = call(
        &fixture.router,
        Method::GET,
        &currentness_path,
        Some(&fixture.applier_token),
        &empty,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{terminal_currentness}");
    assert_eq!(terminal_currentness["admission_status"], "staged");
    assert_eq!(terminal_currentness["current_status"], "revoked");
    assert_eq!(
        terminal_currentness["terminal_receipt_id"],
        created["terminal_receipt"]["terminal_receipt_id"]
    );
    assert_release_material_redacted(&terminal_currentness, &fixture.applier_token);

    let unknown = format!(
        "/api/admin/compute/external-pool-adapter-release-admissions/{}/currentness",
        Uuid::new_v4()
    );
    assert_eq!(
        call(
            &fixture.router,
            Method::GET,
            &unknown,
            Some(&fixture.applier_token),
            &empty,
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
    fixture.cleanup();
}

#[tokio::test]
async fn persisted_owner_and_local_owner_can_write_exact_terminal_kinds() {
    let fixture = fixture();
    let persisted_owner = user(&fixture.state.store, "release-owner", None);
    fixture
        .state
        .store
        .conn()
        .unwrap()
        .execute(
            "UPDATE users SET role='owner' WHERE id=?1",
            [&persisted_owner.id],
        )
        .unwrap();
    let persisted_owner_token = session(&fixture.state.store, &persisted_owner.id);

    let withdrawn = stage_release(
        &fixture,
        "lifecycle-owner",
        "9.0.2",
        b"external-pool-adapter-lifecycle-owner",
    )
    .await;
    let withdrawn_body = terminal_body(
        &withdrawn,
        "lifecycle-owner-terminal",
        "withdrawn",
        None,
        true,
    );
    let (status, owner_receipt) = call(
        &fixture.router,
        Method::POST,
        &terminal_path(&withdrawn),
        Some(&persisted_owner_token),
        &withdrawn_body,
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{owner_receipt}");
    assert_eq!(
        owner_receipt["terminal_receipt"]["terminal"]["actor_id"],
        persisted_owner.id
    );
    assert_eq!(
        owner_receipt["terminal_receipt"]["terminal"]["confirmation"],
        "confirm_external_pool_adapter_release_admission_withdrawal"
    );

    let superseded = stage_release(
        &fixture,
        "lifecycle-local-owner-base",
        "9.0.3",
        b"external-pool-adapter-lifecycle-local-owner-base",
    )
    .await;
    let successor = stage_release(
        &fixture,
        "lifecycle-local-owner-successor",
        "9.0.4",
        b"external-pool-adapter-lifecycle-local-owner-successor",
    )
    .await;
    let superseded_body = terminal_body(
        &superseded,
        "lifecycle-local-owner-terminal",
        "superseded",
        Some(&successor),
        true,
    );
    let (status, local_owner_receipt) = call(
        &fixture.router,
        Method::POST,
        &terminal_path(&superseded),
        Some(LOCAL_OWNER_TOKEN),
        &superseded_body,
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{local_owner_receipt}");
    assert_eq!(
        local_owner_receipt["terminal_receipt"]["terminal"]["actor_id"],
        "local-owner"
    );
    assert_eq!(
        local_owner_receipt["terminal_receipt"]["terminal"]["successor_admission"]["admission_id"],
        successor["admission_id"]
    );
    assert_eq!(
        local_owner_receipt["terminal_receipt"]["terminal"]["confirmation"],
        "confirm_external_pool_adapter_release_admission_supersession"
    );
    assert_release_material_redacted(&local_owner_receipt, LOCAL_OWNER_TOKEN);

    let (status, local_owner_currentness) = call(
        &fixture.router,
        Method::GET,
        &currentness_path(&superseded),
        Some(LOCAL_OWNER_TOKEN),
        &json!({}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{local_owner_currentness}");
    assert_eq!(local_owner_currentness["current_status"], "superseded");
    assert_eq!(
        local_owner_currentness["successor_admission"]["admission_digest"],
        successor["admission_digest"]
    );
    fixture.cleanup();
}
