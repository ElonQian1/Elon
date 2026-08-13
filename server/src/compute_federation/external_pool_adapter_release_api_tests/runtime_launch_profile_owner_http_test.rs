use super::{runtime_launch_profile_test_support::*, *};

#[tokio::test]
async fn runtime_launch_profile_owner_surface_is_minimal_redacted_and_inert() {
    let fixture = fixture();
    let roots = create_runtime_launch_profile_fixture(&fixture, "v255-owner").await;
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
    assert_public_none_effects(&roots.policy);
    assert_runtime_public(&roots.policy);
    assert_inert_effects(&fixture, &roots, 0, 0);

    let path = owner_collection_path(&roots);
    let body = profile_body(&roots, "v255-owner-create", None);
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
        "launch_policy",
        "runtime_kind",
        "entrypoint_path",
        "credential_locator",
        "resolver_backend_root",
        "recorded_by_actor_kind",
        "recorded_by_actor_user_id",
        "provider_id",
        "recorded_at",
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
    assert_eq!(
        malformed_call(
            &fixture.router,
            &path,
            &fixture.member_token,
            "{\"expected_candidate_digest\":"
        )
        .await,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    let mut unconfirmed = body.clone();
    unconfirmed["confirm_runtime_launch_profile"] = json!(false);
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
    let mut drifted = body.clone();
    drifted["expected_launch_policy_digest"] = json!("f".repeat(64));
    assert_eq!(
        call(
            &fixture.router,
            Method::POST,
            &path,
            Some(&fixture.member_token),
            &drifted,
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    assert_inert_effects(&fixture, &roots, 0, 0);

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
        created["profile"]["profile_status"],
        "launch_profile_current_inert"
    );
    assert_public_none_effects(&created["profile"]);
    assert_eq!(created["profile"]["credential_ref_scheme"], "vault_ref");
    assert_runtime_public(&created);
    assert_inert_effects(&fixture, &roots, 1, 0);

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
        replayed["profile"]["profile_id"],
        created["profile"]["profile_id"]
    );
    assert_runtime_public(&replayed);
    assert_inert_effects(&fixture, &roots, 1, 0);

    let current_path = owner_currentness_path(&roots, &created);
    let (status, current) = call(
        &fixture.router,
        Method::GET,
        &current_path,
        Some(&fixture.member_token),
        &Value::Null,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{current}");
    assert_eq!(current["current_status"], "launch_profile_current_inert");
    assert_eq!(current["runtime_launch_ready"], false);
    assert_eq!(current["provider_status"], "registering");
    assert_public_none_effects(&current["profile"]);
    assert_runtime_public(&current);
    assert_inert_effects(&fixture, &roots, 1, 0);

    let wrong_binding = current_path.replacen(
        roots.candidate["candidate"]["provider_binding_id"]
            .as_str()
            .unwrap(),
        "different-provider-binding",
        1,
    );
    assert_eq!(
        call(
            &fixture.router,
            Method::GET,
            &wrong_binding,
            Some(&fixture.member_token),
            &Value::Null,
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    let wrong_candidate = current_path.replacen(
        roots.candidate["candidate"]["candidate_id"]
            .as_str()
            .unwrap(),
        "different-candidate",
        1,
    );
    assert_eq!(
        call(
            &fixture.router,
            Method::GET,
            &wrong_candidate,
            Some(&fixture.member_token),
            &Value::Null,
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
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
