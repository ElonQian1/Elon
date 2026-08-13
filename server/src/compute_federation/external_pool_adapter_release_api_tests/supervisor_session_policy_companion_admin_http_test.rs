use super::{supervisor_session_policy_companion_test_support::*, *};

#[tokio::test]
async fn supervisor_session_policy_companion_admin_records_linear_recovery_and_revocation() {
    let fixture = fixture();
    let roots = create_supervisor_session_policy_companion_fixture(&fixture, "v259-admin").await;

    let (status, policy) = call(
        &fixture.router,
        Method::GET,
        &admin_policy_path(&roots.roots, &roots.target),
        Some(&fixture.applier_token),
        &Value::Null,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{policy}");
    assert_eq!(policy["policy_digest"], roots.policy["policy_digest"]);
    assert_eq!(
        policy["policy"]["linux_confinement"]["seccomp"]["unknown_syscall_action"],
        "kill_process"
    );
    assert_eq!(
        policy["policy"]["linux_confinement"]["seccomp"]["audit_arch_policy"],
        "x86_64_only_kill_other_arch"
    );
    assert_companion_public_and_inert(&policy);

    let collection = admin_collection_path(&roots);
    assert_eq!(
        call(
            &fixture.router,
            Method::POST,
            &collection,
            Some(&fixture.member_token),
            &companion_body(&roots, "v259-admin-first", None),
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    let (status, first) = call(
        &fixture.router,
        Method::POST,
        &collection,
        Some(&fixture.applier_token),
        &companion_body(&roots, "v259-admin-first", None),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{first}");
    assert_eq!(first["companion"]["sequence"], 1);
    assert_companion_public_and_inert(&first);
    assert_companion_rows(&fixture, 1, 0);

    assert_eq!(
        call(
            &fixture.router,
            Method::POST,
            &collection,
            Some(&fixture.applier_token),
            &companion_body(&roots, "v259-admin-missing-predecessor", None),
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    let mut partial = companion_body(&roots, "v259-admin-partial", Some(&first));
    partial["expected_predecessor"]
        .as_object_mut()
        .unwrap()
        .remove("companion_digest");
    assert_eq!(
        call(
            &fixture.router,
            Method::POST,
            &collection,
            Some(&fixture.applier_token),
            &partial,
        )
        .await
        .0,
        StatusCode::UNPROCESSABLE_ENTITY
    );

    let revoke = revoke_body(&first, "v259-admin-revoke-first");
    let revocation_path = admin_revocation_path(&roots, &first);
    let (status, revoked) = call(
        &fixture.router,
        Method::POST,
        &revocation_path,
        Some(&fixture.applier_token),
        &revoke,
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{revoked}");
    assert_eq!(
        revoked["revocation"]["revocation_effect"],
        "supervisor_session_policy_companion_revoked"
    );
    assert_companion_public_and_inert(&revoked);
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

    let (status, successor) = call(
        &fixture.router,
        Method::POST,
        &collection,
        Some(&fixture.applier_token),
        &companion_body(&roots, "v259-admin-successor", Some(&first)),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{successor}");
    assert_eq!(successor["companion"]["sequence"], 2);
    assert_eq!(
        successor["companion"]["predecessor_companion_id"],
        first["companion"]["companion_id"]
    );
    assert_companion_public_and_inert(&successor);
    assert_companion_rows(&fixture, 2, 1);
    fixture.cleanup();
}
