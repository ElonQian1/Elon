use rsa::{
    pkcs8::{EncodePublicKey, LineEnding},
    rand_core::OsRng,
    RsaPrivateKey,
};

use super::*;

#[tokio::test]
async fn credential_verifier_key_http_enforces_exact_binding_revocation_and_redaction() {
    let fixture = fixture();
    let verifier =
        credential_verifier_http_test::create_active_credential_verifier(&fixture, "key-http")
            .await;
    let private = RsaPrivateKey::new(&mut OsRng, 2_048).unwrap();
    let pem = private
        .to_public_key()
        .to_public_key_pem(LineEnding::LF)
        .unwrap();
    let registration = json!({
        "verifier_record_id": verifier["verifier_record"]["verifier_record_id"],
        "expected_verifier_record_digest": verifier["verifier_record"]["verifier_record_digest"],
        "verification_kind": verifier["verifier_record"]["verification_kind"],
        "verifier_id": verifier["verifier_record"]["verifier_id"],
        "verifier_revision": verifier["verifier_record"]["verifier_revision"],
        "expected_verifier_digest": verifier["verifier_record"]["verifier_digest"],
        "algorithm": "rsa-pkcs1v15-sha256",
        "public_key_pem": pem,
        "idempotency_key": "credential-verifier-key-register",
        "confirm_registration": true
    });

    assert_eq!(
        call(&fixture.router, Method::POST, path(), None, &registration)
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
            &registration,
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        call(
            &fixture.router,
            Method::POST,
            path(),
            Some(&fixture.submitter_token),
            &registration,
        )
        .await
        .0,
        StatusCode::CONFLICT,
        "the verifier creator must not register its signing key"
    );

    let mut unknown = registration.clone();
    unknown["credential_ref"] = json!("vault-ref:must-not-be-accepted");
    assert_eq!(
        call(
            &fixture.router,
            Method::POST,
            path(),
            Some(&fixture.reviewer_token),
            &unknown,
        )
        .await
        .0,
        StatusCode::UNPROCESSABLE_ENTITY
    );

    let (status, created) = call(
        &fixture.router,
        Method::POST,
        path(),
        Some(&fixture.reviewer_token),
        &registration,
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{created}");
    assert_redacted(&created, &pem);
    let (status, replayed) = call(
        &fixture.router,
        Method::POST,
        path(),
        Some(&fixture.reviewer_token),
        &registration,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{replayed}");
    assert_eq!(replayed["replayed"], true);

    let id = created["key_record"]["key_record_id"].as_str().unwrap();
    let digest = created["key_record"]["key_record_digest"].as_str().unwrap();
    {
        let connection = fixture.state.store.conn().unwrap();
        assert!(connection
            .execute(
                "INSERT OR REPLACE INTO compute_external_pool_adapter_credential_verifier_keys
                 SELECT * FROM compute_external_pool_adapter_credential_verifier_keys
                 WHERE key_record_id=?1",
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
    assert_redacted(&current, &pem);

    let verifier_id = verifier["verifier_record"]["verifier_record_id"]
        .as_str()
        .unwrap();
    let verifier_digest = verifier["verifier_record"]["verifier_record_digest"]
        .as_str()
        .unwrap();
    let (status, revoked_verifier) = call(
        &fixture.router,
        Method::POST,
        &format!(
            "{}/{verifier_id}/revoke",
            credential_verifier_http_test::path()
        ),
        Some(&fixture.applier_token),
        &json!({
            "expected_verifier_record_digest": verifier_digest,
            "idempotency_key": "credential-verifier-key-parent-revoke",
            "reason": "fixture credential verifier implementation intentionally retired",
            "confirm_revocation": true
        }),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{revoked_verifier}");
    let (status, inherited) = call(
        &fixture.router,
        Method::GET,
        &format!("{}/{id}/currentness", path()),
        Some(&fixture.applier_token),
        &Value::Null,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{inherited}");
    assert_eq!(inherited["current_status"], "verifier_not_current");

    let revoke = json!({
        "expected_key_record_digest": digest,
        "idempotency_key": "credential-verifier-key-revoke",
        "reason": "fixture credential verifier signing key intentionally retired",
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
    assert_redacted(&revoked, &pem);
    let (status, replayed) = call(
        &fixture.router,
        Method::POST,
        &format!("{}/{id}/revoke", path()),
        Some(&fixture.applier_token),
        &revoke,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{replayed}");
    assert_eq!(replayed["replayed"], true);
    fixture.cleanup();
}

pub(super) fn path() -> &'static str {
    "/api/admin/compute/external-pool-adapter-credential-verifier-keys"
}

pub(super) async fn create_active_credential_verifier_key(
    fixture: &Fixture,
    verifier: &Value,
    suffix: &str,
) -> (RsaPrivateKey, Value) {
    let private = RsaPrivateKey::new(&mut OsRng, 2_048).unwrap();
    let pem = private
        .to_public_key()
        .to_public_key_pem(LineEnding::LF)
        .unwrap();
    let body = json!({
        "verifier_record_id": verifier["verifier_record"]["verifier_record_id"],
        "expected_verifier_record_digest": verifier["verifier_record"]["verifier_record_digest"],
        "verification_kind": verifier["verifier_record"]["verification_kind"],
        "verifier_id": verifier["verifier_record"]["verifier_id"],
        "verifier_revision": verifier["verifier_record"]["verifier_revision"],
        "expected_verifier_digest": verifier["verifier_record"]["verifier_digest"],
        "algorithm": "rsa-pkcs1v15-sha256",
        "public_key_pem": pem,
        "idempotency_key": format!("{suffix}-credential-verifier-key"),
        "confirm_registration": true
    });
    let (status, created) = call(
        &fixture.router,
        Method::POST,
        path(),
        Some(&fixture.reviewer_token),
        &body,
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{created}");
    (private, created)
}

fn assert_redacted(value: &Value, pem: &str) {
    let encoded = value.to_string();
    assert!(!encoded.contains(pem));
    for key in [
        "public_key_pem",
        "idempotency_key",
        "idempotency_scope",
        "credential",
        "credential_ref",
        "bearer",
        "token",
        "secret",
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
