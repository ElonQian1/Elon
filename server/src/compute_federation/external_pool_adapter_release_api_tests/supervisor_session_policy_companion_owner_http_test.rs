use super::{supervisor_session_policy_companion_test_support::*, *};

#[tokio::test]
async fn supervisor_session_policy_companion_owner_surface_is_exact_redacted_and_inert() {
    let fixture = fixture();
    let roots = create_supervisor_session_policy_companion_fixture(&fixture, "v259-owner").await;
    let policy_path = owner_policy_path(&roots.roots, &roots.target);

    assert_eq!(
        call(
            &fixture.router,
            Method::GET,
            &policy_path,
            None,
            &Value::Null
        )
        .await
        .0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        call(
            &fixture.router,
            Method::GET,
            &policy_path,
            Some(&fixture.applier_token),
            &Value::Null,
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        roots.policy["policy"]["linux_confinement"]["descriptors"]["child_ipc_fd"],
        3
    );
    assert_eq!(
        roots.policy["policy"]["linux_confinement"]["descriptors"]["capsule_fd"],
        4
    );
    assert_eq!(
        roots.policy["policy"]["linux_confinement"]["descriptors"]["seed_fd"],
        5
    );
    assert_eq!(
        roots.policy["policy"]["linux_confinement"]["descriptors"]["seed_fd_cloexec"],
        false
    );
    assert_eq!(
        roots.policy["policy"]["linux_confinement"]["descriptors"]["seed_fd_read_phase"],
        "post_exec_before_hello_v1"
    );
    assert_eq!(
        roots.policy["policy"]["linux_confinement"]["descriptors"]["post_exec_open_fds"],
        json!([0, 1, 2, 3, 5])
    );
    assert_eq!(
        roots.policy["policy"]["linux_confinement"]["descriptors"]["post_seed_open_fds"],
        json!([0, 1, 2, 3])
    );
    assert_companion_public_and_inert(&roots.policy);
    assert_companion_rows(&fixture, 0, 0);

    let collection = owner_collection_path(&roots);
    let body = companion_body(&roots, "v259-owner-create", None);
    assert_eq!(
        malformed_json_status(&fixture.router, &collection, &fixture.member_token).await,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    assert_eq!(
        call(&fixture.router, Method::POST, &collection, None, &body)
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        call(
            &fixture.router,
            Method::POST,
            &collection,
            Some(&fixture.applier_token),
            &body,
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    for forbidden in [
        "recorded_by_actor_kind",
        "recorded_at",
        "supervisor_session_policy",
        "session_key",
        "host_nonce",
        "dns_hostname",
        "process_spawn_ready",
        "ipc_session_ready",
        "activation_ready",
    ] {
        let mut injected = body.clone();
        injected[forbidden] = json!("caller-selected-authority");
        assert_eq!(
            call(
                &fixture.router,
                Method::POST,
                &collection,
                Some(&fixture.member_token),
                &injected,
            )
            .await
            .0,
            StatusCode::UNPROCESSABLE_ENTITY,
            "accepted forbidden field {forbidden}"
        );
    }
    let mut invalid_digest = body.clone();
    invalid_digest["expected_target_digest"] = json!("not-a-digest");
    assert_eq!(
        call(
            &fixture.router,
            Method::POST,
            &collection,
            Some(&fixture.member_token),
            &invalid_digest,
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    let mut wrong_policy = body.clone();
    wrong_policy["expected_supervisor_session_policy_digest"] = json!("f".repeat(64));
    assert_eq!(
        call(
            &fixture.router,
            Method::POST,
            &collection,
            Some(&fixture.member_token),
            &wrong_policy,
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    let mut unconfirmed = body.clone();
    unconfirmed["confirm_supervisor_session_policy_companion"] = json!(false);
    assert_eq!(
        call(
            &fixture.router,
            Method::POST,
            &collection,
            Some(&fixture.member_token),
            &unconfirmed,
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );

    let (status, created) = call(
        &fixture.router,
        Method::POST,
        &collection,
        Some(&fixture.member_token),
        &body,
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{created}");
    assert_eq!(created["replayed"], false);
    assert_eq!(
        created["companion"]["companion_status"],
        "supervisor_session_policy_companion_current_inert"
    );
    assert_companion_public_and_inert(&created);
    assert_companion_rows(&fixture, 1, 0);

    let (status, replayed) = call(
        &fixture.router,
        Method::POST,
        &collection,
        Some(&fixture.member_token),
        &body,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{replayed}");
    assert_eq!(replayed["replayed"], true);
    assert_eq!(
        replayed["companion"]["companion_id"],
        created["companion"]["companion_id"]
    );
    assert_companion_public_and_inert(&replayed);

    let missing_path = format!("{collection}/missing-companion/currentness");
    assert_eq!(
        call(
            &fixture.router,
            Method::GET,
            &missing_path,
            Some(&fixture.member_token),
            &Value::Null,
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );

    let (status, current) = call(
        &fixture.router,
        Method::GET,
        &owner_currentness_path(&roots, &created),
        Some(&fixture.member_token),
        &Value::Null,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{current}");
    assert_eq!(
        current["current_status"],
        "supervisor_session_policy_companion_current_inert"
    );
    assert_companion_public_and_inert(&current);

    drift_installed_entrypoint(&fixture, &roots);
    assert_eq!(
        call(
            &fixture.router,
            Method::GET,
            &owner_currentness_path(&roots, &created),
            Some(&fixture.member_token),
            &Value::Null,
        )
        .await
        .0,
        StatusCode::CONFLICT
    );

    let revoke = revoke_body(&created, "v259-owner-revoke");
    let revoke_path = owner_revocation_path(&roots, &created);
    let (status, revoked) = call(
        &fixture.router,
        Method::POST,
        &revoke_path,
        Some(&fixture.member_token),
        &revoke,
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{revoked}");
    assert_companion_public_and_inert(&revoked);
    let (status, replayed_revocation) = call(
        &fixture.router,
        Method::POST,
        &revoke_path,
        Some(&fixture.member_token),
        &revoke,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{replayed_revocation}");
    assert_eq!(replayed_revocation["replayed"], true);
    assert_companion_public_and_inert(&replayed_revocation);
    assert_companion_rows(&fixture, 1, 1);
    fixture.cleanup();
}
