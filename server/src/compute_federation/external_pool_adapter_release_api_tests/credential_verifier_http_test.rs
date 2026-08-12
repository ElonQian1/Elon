use super::*;

const VERIFIER_DIGEST: &str = "2222222222222222222222222222222222222222222222222222222222222222";

#[tokio::test]
async fn credential_verifier_http_enforces_identity_four_eyes_revocation_and_redaction() {
    let fixture = fixture();
    let register = registration("credential-register", VERIFIER_DIGEST);
    assert_eq!(
        call(&fixture.router, Method::POST, path(), None, &register)
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        call(
            &fixture.router,
            Method::POST,
            path(),
            Some(&fixture.member_token),
            &register,
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    let mut unknown_field = register.clone();
    unknown_field["credential"] = json!("must-not-be-accepted");
    assert_eq!(
        call(
            &fixture.router,
            Method::POST,
            path(),
            Some(&fixture.submitter_token),
            &unknown_field,
        )
        .await
        .0,
        StatusCode::UNPROCESSABLE_ENTITY
    );

    let (status, created) = call(
        &fixture.router,
        Method::POST,
        path(),
        Some(&fixture.submitter_token),
        &register,
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{created}");
    assert_eq!(
        created["verifier_record"]["verification_kind"],
        "signed_challenge"
    );
    assert_eq!(
        created["verifier_record"]["verifier_id"],
        "fixture-verifier"
    );
    assert_eq!(created["verifier_record"]["verifier_revision"], 1);
    assert_eq!(
        created["verifier_record"]["verifier_digest"],
        VERIFIER_DIGEST
    );
    assert_redacted(&created);

    let (status, replayed) = call(
        &fixture.router,
        Method::POST,
        path(),
        Some(&fixture.submitter_token),
        &register,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{replayed}");
    assert_eq!(replayed["replayed"], true);
    assert_eq!(replayed["verifier_record"], created["verifier_record"]);

    let idempotency_collision = registration(
        "credential-register",
        "3333333333333333333333333333333333333333333333333333333333333333",
    );
    assert_eq!(
        call(
            &fixture.router,
            Method::POST,
            path(),
            Some(&fixture.submitter_token),
            &idempotency_collision,
        )
        .await
        .0,
        StatusCode::CONFLICT
    );

    let replacement = registration(
        "credential-replacement",
        "3333333333333333333333333333333333333333333333333333333333333333",
    );
    assert_eq!(
        call(
            &fixture.router,
            Method::POST,
            path(),
            Some(&fixture.submitter_token),
            &replacement,
        )
        .await
        .0,
        StatusCode::CONFLICT
    );

    let id = created["verifier_record"]["verifier_record_id"]
        .as_str()
        .unwrap();
    let digest = created["verifier_record"]["verifier_record_digest"]
        .as_str()
        .unwrap();
    {
        let connection = fixture.state.store.conn().unwrap();
        assert!(connection
            .execute(
                "INSERT OR REPLACE INTO compute_external_pool_adapter_credential_verifiers
                 SELECT * FROM compute_external_pool_adapter_credential_verifiers
                 WHERE verifier_record_id=?1",
                [id],
            )
            .is_err());
    }
    let activate = json!({
        "expected_verifier_record_digest": digest,
        "idempotency_key": "credential-activate",
        "confirm_activation": true
    });
    assert_eq!(
        call(
            &fixture.router,
            Method::POST,
            &format!("{}/{id}/activate", path()),
            Some(&fixture.submitter_token),
            &activate,
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    let (status, active) = call(
        &fixture.router,
        Method::POST,
        &format!("{}/{id}/activate", path()),
        Some(&fixture.reviewer_token),
        &activate,
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{active}");
    assert_redacted(&active);
    let (status, active_replay) = call(
        &fixture.router,
        Method::POST,
        &format!("{}/{id}/activate", path()),
        Some(&fixture.reviewer_token),
        &activate,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{active_replay}");
    assert_eq!(active_replay["replayed"], true);
    {
        let connection = fixture.state.store.conn().unwrap();
        assert!(connection
            .execute(
                "INSERT OR REPLACE INTO compute_external_pool_adapter_credential_verifier_transitions
                 SELECT * FROM compute_external_pool_adapter_credential_verifier_transitions
                 WHERE verifier_record_id=?1 AND transition_kind='activation'",
                [id],
            )
            .is_err());
    }

    let (status, current) = call(
        &fixture.router,
        Method::GET,
        &format!("{}/{id}/currentness", path()),
        Some(&fixture.applier_token),
        &Value::Null,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{current}");
    assert_eq!(current["current_status"], "active");
    assert_redacted(&current);

    let revoke = json!({
        "expected_verifier_record_digest": digest,
        "idempotency_key": "credential-revoke",
        "reason": "fixture credential verifier implementation intentionally retired",
        "confirm_revocation": true
    });
    let (status, revoked) = call(
        &fixture.router,
        Method::POST,
        &format!("{}/{id}/revoke", path()),
        Some(&fixture.applier_token),
        &revoke,
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{revoked}");
    assert_redacted(&revoked);
    let (status, revoked_replay) = call(
        &fixture.router,
        Method::POST,
        &format!("{}/{id}/revoke", path()),
        Some(&fixture.applier_token),
        &revoke,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{revoked_replay}");
    assert_eq!(revoked_replay["replayed"], true);
    let (status, current) = call(
        &fixture.router,
        Method::GET,
        &format!("{}/{id}/currentness", path()),
        Some(&fixture.applier_token),
        &Value::Null,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{current}");
    assert_eq!(current["current_status"], "revoked");
    fixture.cleanup();
}

fn registration(idempotency_key: &str, digest: &str) -> Value {
    json!({
        "verifier_operator": "fixture-verification-lab",
        "verifier_product": "fixture-credential-verifier-v1",
        "verification_kind": "signed_challenge",
        "verifier_id": "fixture-verifier",
        "verifier_revision": 1,
        "verifier_digest": digest,
        "idempotency_key": idempotency_key,
        "confirm_registration": true
    })
}

pub(super) fn path() -> &'static str {
    "/api/admin/compute/external-pool-adapter-credential-verifiers"
}

fn assert_redacted(value: &Value) {
    for key in [
        "idempotency_key",
        "idempotency_scope",
        "credential",
        "credential_ref",
        "bearer",
        "token",
        "secret",
        "public_key_pem",
        "verification_receipt",
    ] {
        assert_forbidden(value, key);
    }
}

fn assert_forbidden(value: &Value, key: &str) {
    match value {
        Value::Object(map) => {
            assert!(!map.contains_key(key), "{key}: {value}");
            for item in map.values() {
                assert_forbidden(item, key);
            }
        }
        Value::Array(items) => {
            for item in items {
                assert_forbidden(item, key);
            }
        }
        _ => {}
    }
}
