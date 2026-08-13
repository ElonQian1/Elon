use super::{upstream_transport_target_test_support::*, *};

#[tokio::test]
async fn upstream_transport_target_admin_can_revoke_and_record_a_linear_successor() {
    let fixture = fixture();
    let roots = create_upstream_transport_target_fixture(&fixture, "v258-admin").await;

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
    assert_transport_public(&policy);

    let collection = admin_collection_path(&roots);
    assert_eq!(
        call(
            &fixture.router,
            Method::POST,
            &collection,
            Some(&fixture.member_token),
            &target_body(&roots, "v258-admin-first", None),
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
        &target_body(&roots, "v258-admin-first", None),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{first}");
    assert_transport_public(&first);
    assert_transport_inert_effects(&fixture, &roots, 1, 0);

    assert_eq!(
        call(
            &fixture.router,
            Method::POST,
            &collection,
            Some(&fixture.applier_token),
            &target_body(&roots, "v258-missing-predecessor", None),
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    let mut partial = target_body(&roots, "v258-partial-predecessor", Some(&first));
    partial["expected_predecessor"]
        .as_object_mut()
        .unwrap()
        .remove("target_digest");
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

    let revoke = revoke_body(&first, "v258-admin-revoke-first");
    let path = admin_revocation_path(&roots, &first);
    let (status, revoked) = call(
        &fixture.router,
        Method::POST,
        &path,
        Some(&fixture.applier_token),
        &revoke,
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{revoked}");
    assert_eq!(
        revoked["revocation"]["revocation_effect"],
        "upstream_transport_target_revoked"
    );
    assert_transport_public(&revoked);
    assert_transport_inert_effects(&fixture, &roots, 1, 1);
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

    let successor_body = target_body(&roots, "v258-admin-successor", Some(&first));
    let (status, successor) = call(
        &fixture.router,
        Method::POST,
        &collection,
        Some(&fixture.applier_token),
        &successor_body,
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{successor}");
    assert_eq!(successor["target"]["sequence"], 2);
    assert_eq!(
        successor["target"]["predecessor_target_id"],
        first["target"]["target_id"]
    );
    assert_transport_public(&successor);
    assert_transport_inert_effects(&fixture, &roots, 2, 1);
    fixture.cleanup();
}
