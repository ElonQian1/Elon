use super::*;

#[tokio::test]
async fn every_action_authenticates_before_malformed_private_parameters() {
    let fixture = Fixture::new();
    let _disabled = override_configuration(SellbackConfiguration::Disabled);
    for (method, suffix, body) in [
        ("GET", "?limit=bad&limit=2", "null"),
        ("POST", "?user_id=private", "{\"schema\":1,\"schema\":2}"),
        ("GET", "/%FF?unknown=value", "null"),
        ("POST", "/bad-id/cancel?user_id=private", "{"),
        ("POST", "/lookup?idempotency_key=private", "{"),
    ] {
        for token in [
            None,
            Some("synthetic-static-owner-not-a-session"),
            Some(fixture.state.admin_token.as_str()),
        ] {
            let (status, error) =
                send_raw(&fixture, method, &format!("{BASE}{suffix}"), token, body).await;
            assert_eq!(status, StatusCode::UNAUTHORIZED, "{method} {suffix}");
            assert_eq!(error.as_object().unwrap().len(), 1);
            assert!(error.get("summary").is_none());
            assert!(!error.to_string().contains("private"));
        }
    }
    let (status, _) = send_raw(&fixture, "POST", BASE, None, &"x".repeat(20000)).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    fixture.cleanup();
}

#[tokio::test]
async fn exact_queries_and_json_reject_unknown_duplicate_and_numeric_amounts() {
    let fixture = Fixture::new();
    let _allocation_policy = enable_fixture_policy();
    credit(&fixture).await;
    let (_configuration, _) = configure(&fixture);
    let initial = page(&fixture).await;
    let valid = submit_body(&initial, "synthetic-key", "1");
    for suffix in [
        "?user_id=other",
        "?limit=0",
        "?limit=21",
        "?limit=1&limit=2",
        "?cursor=invalid",
        "?idempotency_key=private",
    ] {
        let (status, error) = send(
            &fixture,
            "GET",
            &format!("{BASE}{suffix}"),
            Some(&fixture.user_token),
            Value::Null,
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(
            error,
            json!({"error":"ESK_PLATFORM_SELLBACK_INVALID_INPUT"})
        );
    }
    let mut unknown = valid.clone();
    unknown["user_id"] = "never-echo-private-user".into();
    let mut number = valid.clone();
    number["amount_base_units"] = 1.into();
    let mut exponent = valid.clone();
    exponent["amount_base_units"] = "1e6".into();
    let mut overflow = valid.clone();
    overflow["amount_base_units"] = "9223372036854775808".into();
    for bad in [unknown, number, exponent, overflow] {
        let (status, error) = send(&fixture, "POST", BASE, Some(&fixture.user_token), bad).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(
            error,
            json!({"error":"ESK_PLATFORM_SELLBACK_INVALID_INPUT"})
        );
    }
    let duplicate = valid
        .to_string()
        .replacen("{", "{\"schema\":\"duplicate\",", 1);
    for raw in [duplicate, "{".into(), "x".repeat(20000)] {
        let (status, error) =
            send_raw(&fixture, "POST", BASE, Some(&fixture.user_token), &raw).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(
            error,
            json!({"error":"ESK_PLATFORM_SELLBACK_INVALID_INPUT"})
        );
    }
    for (suffix, body) in [
        ("?user_id=other", valid),
        (
            "/lookup?idempotency_key=never-in-url",
            json!({"schema":LOOKUP_SCHEMA,"idempotency_key":"key"}),
        ),
    ] {
        assert_eq!(
            send(
                &fixture,
                "POST",
                &format!("{BASE}{suffix}"),
                Some(&fixture.user_token),
                body
            )
            .await
            .0,
            StatusCode::BAD_REQUEST
        );
    }
    assert_summary(
        &page(&fixture).await["summary"],
        "25000000",
        "0",
        "25000000",
        "0",
    );
    fixture.cleanup();
}

#[tokio::test]
async fn private_lookup_and_detail_cannot_observe_or_cancel_another_users_request() {
    let fixture = Fixture::new();
    let _allocation_policy = enable_fixture_policy();
    credit(&fixture).await;
    let (_configuration, policy) = configure(&fixture);
    let initial = page(&fixture).await;
    let (status, accepted) = send(
        &fixture,
        "POST",
        BASE,
        Some(&fixture.user_token),
        submit_body(&initial, "one", "10"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let id = accepted["request"]["request_id"].as_str().unwrap();
    for (method, path, body) in [
        ("GET", format!("{BASE}/{id}"), Value::Null),
        ("POST", format!("{BASE}/{id}/cancel"), cancel_body()),
        (
            "POST",
            format!("{BASE}/lookup"),
            json!({"schema":LOOKUP_SCHEMA,"idempotency_key":"one"}),
        ),
    ] {
        let (status, error) = send(&fixture, method, &path, Some(&fixture.other_token), body).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(error, json!({"error":"ESK_PLATFORM_SELLBACK_NOT_FOUND"}));
    }
    let changes = || {
        fixture
            .state
            .store
            .conn()
            .unwrap()
            .query_row("SELECT total_changes()", [], |r| r.get::<_, i64>(0))
            .unwrap()
    };
    let before = changes();
    for _ in 0..3 {
        let result = fixture
            .state
            .store
            .lookup_esk_platform_sellback(
                &fixture.user_id,
                &fixture.user_token,
                "one",
                &SellbackConfiguration::Enabled(policy.clone()),
            )
            .unwrap();
        assert_eq!(result.request.request_id, id);
        assert!(result.replayed);
    }
    assert_eq!(before, changes());
    let (status, absent) = send(
        &fixture,
        "POST",
        &format!("{BASE}/lookup"),
        Some(&fixture.user_token),
        json!({"schema":LOOKUP_SCHEMA,"idempotency_key":"not-yet-seen"}),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(absent, json!({"error":"ESK_PLATFORM_SELLBACK_NOT_FOUND"}));
    // Absence is not proof a prior in-flight submit will never commit; no replacement is created.
    assert_eq!(page(&fixture).await["summary"], accepted["summary"]);
    fixture.cleanup();
}

#[tokio::test]
async fn all_actions_reject_revoked_real_sessions_without_revealing_or_releasing_requests() {
    let fixture = Fixture::new();
    let _allocation_policy = enable_fixture_policy();
    credit(&fixture).await;
    let (_configuration, _) = configure(&fixture);
    let initial = page(&fixture).await;
    let body = submit_body(&initial, "one", "10");
    let (_, accepted) = send(
        &fixture,
        "POST",
        BASE,
        Some(&fixture.user_token),
        body.clone(),
    )
    .await;
    let id = accepted["request"]["request_id"].as_str().unwrap();
    fixture
        .state
        .store
        .conn()
        .unwrap()
        .execute(
            "UPDATE sessions SET revoked_at='synthetic-revoked' WHERE user_id=?1",
            [&fixture.user_id],
        )
        .unwrap();
    for (method, path, body) in [
        ("GET", BASE.into(), Value::Null),
        ("POST", BASE.into(), body),
        ("GET", format!("{BASE}/{id}"), Value::Null),
        ("POST", format!("{BASE}/{id}/cancel"), cancel_body()),
        (
            "POST",
            format!("{BASE}/lookup"),
            json!({"schema":LOOKUP_SCHEMA,"idempotency_key":"one"}),
        ),
    ] {
        let (status, error) = send(&fixture, method, &path, Some(&fixture.user_token), body).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert_eq!(error.as_object().unwrap().len(), 1);
    }
    fixture.cleanup();
}
