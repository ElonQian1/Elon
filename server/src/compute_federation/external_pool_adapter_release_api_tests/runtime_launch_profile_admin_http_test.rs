use super::{runtime_launch_profile_test_support::*, *};

#[tokio::test]
async fn runtime_launch_profile_admin_can_repair_with_a_linear_successor() {
    let fixture = fixture();
    let roots = create_runtime_launch_profile_fixture(&fixture, "v255-admin").await;

    let (status, policy) = call(
        &fixture.router,
        Method::GET,
        &admin_policy_path(&roots),
        Some(&fixture.applier_token),
        &Value::Null,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{policy}");
    assert_eq!(policy["policy_digest"], roots.policy["policy_digest"]);
    assert_public_none_effects(&policy);
    assert_runtime_public(&policy);

    let admin_collection = admin_collection_path(&roots);
    assert_eq!(
        call(
            &fixture.router,
            Method::POST,
            &admin_collection,
            Some(&fixture.member_token),
            &profile_body(&roots, "v255-admin-first", None),
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    let (status, first) = call(
        &fixture.router,
        Method::POST,
        &admin_collection,
        Some(&fixture.applier_token),
        &profile_body(&roots, "v255-admin-first", None),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{first}");
    assert_runtime_public(&first);
    assert_inert_effects(&fixture, &roots, 1, 0);

    let missing_predecessor = profile_body(&roots, "v255-admin-missing-predecessor", None);
    assert_eq!(
        call(
            &fixture.router,
            Method::POST,
            &admin_collection,
            Some(&fixture.applier_token),
            &missing_predecessor,
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    let mut partial_predecessor =
        profile_body(&roots, "v255-admin-partial-predecessor", Some(&first));
    partial_predecessor["expected_predecessor"]
        .as_object_mut()
        .unwrap()
        .remove("profile_digest");
    assert_eq!(
        call(
            &fixture.router,
            Method::POST,
            &admin_collection,
            Some(&fixture.applier_token),
            &partial_predecessor,
        )
        .await
        .0,
        StatusCode::UNPROCESSABLE_ENTITY
    );

    let revoke_path = admin_revocation_path(&roots, &first);
    let revoke = revoke_body(&first, "v255-admin-revoke-first");
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
        "runtime_launch_profile_revoked"
    );
    assert_public_none_effects(&revoked["profile"]);
    assert_public_none_effects(&revoked["revocation"]);
    assert_runtime_public(&revoked);
    assert_inert_effects(&fixture, &roots, 1, 1);
    assert_eq!(
        call(
            &fixture.router,
            Method::GET,
            &admin_currentness_path(&roots, &first),
            Some(&fixture.applier_token),
            &Value::Null,
        )
        .await
        .0,
        StatusCode::CONFLICT
    );

    let successor_body = profile_body(&roots, "v255-admin-successor", Some(&first));
    let (status, successor) = call(
        &fixture.router,
        Method::POST,
        &admin_collection,
        Some(&fixture.applier_token),
        &successor_body,
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{successor}");
    assert_eq!(successor["profile"]["sequence"], 2);
    assert_eq!(
        successor["profile"]["predecessor_profile_id"],
        first["profile"]["profile_id"]
    );
    assert_runtime_public(&successor);
    assert_inert_effects(&fixture, &roots, 2, 1);

    let (status, current) = call(
        &fixture.router,
        Method::GET,
        &owner_currentness_path(&roots, &successor),
        Some(&fixture.member_token),
        &Value::Null,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{current}");
    assert_eq!(current["runtime_launch_ready"], false);
    assert_public_none_effects(&current["profile"]);
    assert_runtime_public(&current);
    assert_inert_effects(&fixture, &roots, 2, 1);
    fixture.cleanup();
}

#[tokio::test]
async fn revoke_remains_available_after_filesystem_drift_and_replays_exactly() {
    let fixture = fixture();
    let roots = create_runtime_launch_profile_fixture(&fixture, "v255-drift-revoke").await;
    let body = profile_body(&roots, "v255-drift-create", None);
    let (_, created) = call(
        &fixture.router,
        Method::POST,
        &owner_collection_path(&roots),
        Some(&fixture.member_token),
        &body,
    )
    .await;
    std::fs::write(
        installed_entrypoint(&fixture, &roots),
        b"drifted inert bytes",
    )
    .unwrap();

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
    let revoke = revoke_body(&created, "v255-drift-revoke");
    let path = owner_revocation_path(&roots, &created);
    let (status, revoked) = call(
        &fixture.router,
        Method::POST,
        &path,
        Some(&fixture.member_token),
        &revoke,
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{revoked}");
    assert_runtime_public(&revoked);
    let (status, replayed) = call(
        &fixture.router,
        Method::POST,
        &path,
        Some(&fixture.member_token),
        &revoke,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{replayed}");
    assert_eq!(replayed["replayed"], true);
    assert_eq!(
        replayed["revocation"]["revocation_id"],
        revoked["revocation"]["revocation_id"]
    );
    assert_public_none_effects(&replayed["revocation"]);
    assert_runtime_public(&replayed);
    assert_inert_effects(&fixture, &roots, 1, 1);
    fixture.cleanup();
}
