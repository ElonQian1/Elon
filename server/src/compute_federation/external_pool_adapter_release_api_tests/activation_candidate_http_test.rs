use super::{activation_candidate_test_support::*, *};

#[tokio::test]
async fn activation_candidate_http_is_owner_bound_redacted_and_inert() {
    let fixture = fixture();
    let roots = create_activation_candidate_fixture(&fixture, "v254-owner").await;
    let path = owner_collection(&roots);
    let body = candidate_body(&roots, "v254-owner-create");

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

    let mut unknown = body.clone();
    unknown["service_actor_id"] = json!("caller-selected-actor");
    assert_eq!(
        call(
            &fixture.router,
            Method::POST,
            &path,
            Some(&fixture.member_token),
            &unknown,
        )
        .await
        .0,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    assert_eq!(
        malformed_call(
            &fixture.router,
            &path,
            &fixture.member_token,
            "{\"expected_provider_binding_digest\":"
        )
        .await,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    for forbidden in [
        "route_adapter_projection_id",
        "logical_adapter_binding_digest",
        "issued_at",
        "checked_at",
        "provider_id",
        "actor_phase",
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

    let mut unconfirmed = body.clone();
    unconfirmed["confirm_activation_candidate"] = json!(false);
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
    drifted["expected_provider_binding_digest"] = json!("f".repeat(64));
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
    let missing_path =
        "/api/me/compute/external-pool-provider-bindings/missing-binding/activation-candidates";
    assert_eq!(
        call(
            &fixture.router,
            Method::POST,
            missing_path,
            Some(&fixture.member_token),
            &body,
        )
        .await
        .0,
        StatusCode::NOT_FOUND
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
        created["candidate"]["candidate_status"],
        "candidate_current_not_activation_ready"
    );
    assert_eq!(
        created["candidate"]["activation_closure_status"],
        "activation_closure_not_implemented"
    );
    assert_eq!(created["candidate"]["provider_status"], "registering");
    assert_public_redaction(&created);
    assert_zero_effects(&fixture, &roots);

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
        replayed["candidate"]["candidate_id"],
        created["candidate"]["candidate_id"]
    );
    assert_public_redaction(&replayed);
    assert_zero_effects(&fixture, &roots);

    let wrong_binding = owner_currentness_path(&roots, &created).replace(
        roots.upstream.roots.registry["binding"]["provider_binding_id"]
            .as_str()
            .unwrap(),
        "different-provider-binding",
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

    for (path, token) in [
        (
            owner_currentness_path(&roots, &created),
            &fixture.member_token,
        ),
        (
            admin_currentness_path(&roots, &created),
            &fixture.applier_token,
        ),
    ] {
        let (status, current) = call(
            &fixture.router,
            Method::GET,
            &path,
            Some(token),
            &Value::Null,
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{current}");
        assert_eq!(
            current["current_status"],
            "candidate_current_not_activation_ready"
        );
        assert_eq!(current["activation_ready"], false);
        assert_eq!(
            current["activation_closure_status"],
            "activation_closure_not_implemented"
        );
        assert_public_redaction(&current);
        assert_zero_effects(&fixture, &roots);
    }

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

#[tokio::test]
async fn activation_preflight_is_dynamic_and_revocation_fails_closed() {
    let fixture = fixture();
    let roots = create_activation_candidate_fixture(&fixture, "v254-preflight").await;
    let body = candidate_body(&roots, "v254-preflight-create");
    let (status, created) = call(
        &fixture.router,
        Method::POST,
        &owner_collection(&roots),
        Some(&fixture.member_token),
        &body,
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{created}");

    let owner_preflight = owner_preflight_path(&roots, &created);
    assert_eq!(
        call(
            &fixture.router,
            Method::GET,
            &owner_preflight,
            None,
            &Value::Null,
        )
        .await
        .0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        call(
            &fixture.router,
            Method::GET,
            &owner_preflight,
            Some(&fixture.applier_token),
            &Value::Null,
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    let missing = owner_preflight.replace(
        created["candidate"]["candidate_id"].as_str().unwrap(),
        "missing-candidate",
    );
    assert_eq!(
        call(
            &fixture.router,
            Method::GET,
            &missing,
            Some(&fixture.member_token),
            &Value::Null,
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
    let drifted = owner_preflight.replace(
        created["candidate"]["candidate_digest"].as_str().unwrap(),
        &"e".repeat(64),
    );
    assert_eq!(
        call(
            &fixture.router,
            Method::GET,
            &drifted,
            Some(&fixture.member_token),
            &Value::Null,
        )
        .await
        .0,
        StatusCode::CONFLICT
    );

    for (path, token) in [
        (owner_preflight, &fixture.member_token),
        (
            admin_preflight_path(&roots, &created),
            &fixture.applier_token,
        ),
    ] {
        let (status, preflight) = call(
            &fixture.router,
            Method::GET,
            &path,
            Some(token),
            &Value::Null,
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{preflight}");
        assert_eq!(preflight["inputs_status"], "inputs_current");
        assert_eq!(
            preflight["activation_closure_status"],
            "activation_closure_not_implemented"
        );
        assert_eq!(preflight["activation_ready"], false);
        assert_public_redaction(&preflight);
        assert_zero_effects(&fixture, &roots);
    }

    let revoke_path = revoke_path(&roots, &created);
    let revoke_body = revoke_body(&created, "v254-preflight-revoke");
    assert_eq!(
        call(
            &fixture.router,
            Method::POST,
            &revoke_path,
            Some(&fixture.applier_token),
            &revoke_body,
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    let (status, revoked) = call(
        &fixture.router,
        Method::POST,
        &revoke_path,
        Some(&fixture.member_token),
        &revoke_body,
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{revoked}");
    assert_eq!(revoked["replayed"], false);
    assert_eq!(
        revoked["revocation"]["revocation_effect"],
        "owner_delegation_revoked"
    );
    assert_public_redaction(&revoked);
    assert_zero_effects(&fixture, &roots);
    let (status, replayed) = call(
        &fixture.router,
        Method::POST,
        &revoke_path,
        Some(&fixture.member_token),
        &revoke_body,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{replayed}");
    assert_eq!(replayed["replayed"], true);
    assert_zero_effects(&fixture, &roots);
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
    fixture.cleanup();
}
