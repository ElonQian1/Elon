use super::{upstream_transport_target_test_support::*, *};

#[tokio::test]
async fn upstream_transport_target_owner_surface_is_minimal_redacted_and_inert() {
    let fixture = fixture();
    let roots = create_upstream_transport_target_fixture(&fixture, "v258-owner").await;
    let policy_path = owner_policy_path(&roots);

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
    assert_eq!(roots.policy["broker_connect_ready"], false);
    assert_transport_public(&roots.policy);
    assert_transport_inert_effects(&fixture, &roots, 0, 0);

    let path = owner_collection_path(&roots);
    let body = target_body(&roots, "v258-owner-create", None);
    assert_eq!(
        malformed_call(
            &fixture.router,
            &path,
            &fixture.member_token,
            "{\"expected_profile_digest\":"
        )
        .await,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    let mut nested_unknown = body.clone();
    nested_unknown["draft"]["resolved_ip"] = json!("203.0.113.1");
    assert_eq!(
        call(
            &fixture.router,
            Method::POST,
            &path,
            Some(&fixture.member_token),
            &nested_unknown,
        )
        .await
        .0,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    assert_eq!(
        call(&fixture.router, Method::POST, &path, None, &body)
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        call(
            &fixture.router,
            Method::POST,
            &path,
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
        "target_policy",
        "tls_server_name",
        "dns_answers",
        "resolved_ip",
        "upstream_probe_observed",
        "runtime_launch_ready",
    ] {
        let mut injected = body.clone();
        injected[forbidden] = json!("caller-selected-authority");
        assert_eq!(
            call(
                &fixture.router,
                Method::POST,
                &path,
                Some(&fixture.member_token),
                &injected,
            )
            .await
            .0,
            StatusCode::UNPROCESSABLE_ENTITY,
            "accepted forbidden field {forbidden}"
        );
    }
    let mut invalid_host = body.clone();
    invalid_host["draft"]["dns_hostname"] = json!("127.0.0.1");
    assert_eq!(
        call(
            &fixture.router,
            Method::POST,
            &path,
            Some(&fixture.member_token),
            &invalid_host,
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    let mut invalid_port = body.clone();
    invalid_port["draft"]["port"] = json!(0);
    assert_eq!(
        call(
            &fixture.router,
            Method::POST,
            &path,
            Some(&fixture.member_token),
            &invalid_port,
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    let mut invalid_pin = body.clone();
    invalid_pin["draft"]["expected_tls_leaf_spki_sha256"] = json!("not-a-digest");
    assert_eq!(
        call(
            &fixture.router,
            Method::POST,
            &path,
            Some(&fixture.member_token),
            &invalid_pin,
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    let mut wrong_policy = body.clone();
    wrong_policy["expected_target_policy_digest"] = json!("f".repeat(64));
    assert_eq!(
        call(
            &fixture.router,
            Method::POST,
            &path,
            Some(&fixture.member_token),
            &wrong_policy,
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    let mut unconfirmed = body.clone();
    unconfirmed["confirm_upstream_transport_target"] = json!(false);
    assert_eq!(
        call(
            &fixture.router,
            Method::POST,
            &path,
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
        &path,
        Some(&fixture.member_token),
        &body,
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{created}");
    assert_eq!(created["replayed"], false);
    assert_eq!(
        created["target"]["target_status"],
        "upstream_transport_target_current_inert"
    );
    assert_transport_public(&created);
    assert_transport_inert_effects(&fixture, &roots, 1, 0);

    let (status, replayed) = call(
        &fixture.router,
        Method::POST,
        &path,
        Some(&fixture.member_token),
        &body,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{replayed}");
    assert_eq!(replayed["replayed"], true);
    assert_eq!(
        replayed["target"]["target_id"],
        created["target"]["target_id"]
    );
    assert_transport_public(&replayed);

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
        "upstream_transport_target_current_inert"
    );
    assert_eq!(current["upstream_probe_observed"], false);
    assert_transport_public(&current);
    assert_transport_inert_effects(&fixture, &roots, 1, 0);

    for (needle, replacement) in [
        (
            roots.profile["profile"]["provider_binding_id"]
                .as_str()
                .unwrap(),
            "different-provider-binding",
        ),
        (
            roots.profile["profile"]["candidate_id"].as_str().unwrap(),
            "different-candidate",
        ),
        (
            roots.profile["profile"]["profile_id"].as_str().unwrap(),
            "different-profile",
        ),
        (
            created["target"]["target_id"].as_str().unwrap(),
            "different-target",
        ),
    ] {
        let wrong_path = owner_currentness_path(&roots, &created).replacen(needle, replacement, 1);
        let status = call(
            &fixture.router,
            Method::GET,
            &wrong_path,
            Some(&fixture.member_token),
            &Value::Null,
        )
        .await
        .0;
        assert!(
            matches!(status, StatusCode::NOT_FOUND | StatusCode::CONFLICT),
            "unexpected exact-path status {status} for {wrong_path}"
        );
    }

    std::fs::write(
        installed_entrypoint(&fixture, &roots),
        b"v258 drift after durable target",
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

    let revoke = revoke_body(&created, "v258-owner-revoke");
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
    assert_transport_public(&revoked);
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
    assert_transport_public(&replayed_revocation);
    assert_transport_inert_effects(&fixture, &roots, 1, 1);
    fixture.cleanup();
}

async fn malformed_call(router: &Router, path: &str, token: &str, body: &str) -> StatusCode {
    router
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(path)
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap()
        .status()
}
