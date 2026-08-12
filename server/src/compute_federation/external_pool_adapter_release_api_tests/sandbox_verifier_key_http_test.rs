use rsa::{
    pkcs8::{EncodePublicKey, LineEnding},
    rand_core::OsRng,
    RsaPrivateKey,
};

use super::*;

#[tokio::test]
async fn sandbox_verifier_key_http_enforces_four_eyes_redaction_and_role_separation() {
    let fixture = fixture();
    let pem = RsaPrivateKey::new(&mut OsRng, 2048)
        .unwrap()
        .to_public_key()
        .to_public_key_pem(LineEnding::LF)
        .unwrap();
    let register = registration(&pem, "sandbox-register");
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
            &register
        )
        .await
        .0,
        StatusCode::FORBIDDEN
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
    assert_redacted(&created, &pem);
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

    let id = created["key_record"]["key_record_id"].as_str().unwrap();
    let digest = created["key_record"]["key_record_digest"].as_str().unwrap();
    let activate = json!({
        "expected_key_record_digest": digest,
        "idempotency_key": "sandbox-activate",
        "confirm_activation": true
    });
    assert_eq!(
        call(
            &fixture.router,
            Method::POST,
            &format!("{}/{id}/activate", path()),
            Some(&fixture.submitter_token),
            &activate
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    let supplier_collision = json!({
        "source_operator": "fixture-pool-operator",
        "algorithm": "rsa-pkcs1v15-sha256",
        "public_key_pem": pem,
        "idempotency_key": "supplier-collision",
        "confirm_registration": true
    });
    assert_eq!(
        call(
            &fixture.router,
            Method::POST,
            "/api/admin/compute/external-pool-adapter-artifact-signing-keys",
            Some(&fixture.submitter_token),
            &supplier_collision
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
    assert_redacted(&active, &pem);

    let scanner_collision = json!({
        "scanner_operator": "fixture-security-lab",
        "scanner_product": "fixture-scanner-v1",
        "algorithm": "rsa-pkcs1v15-sha256",
        "public_key_pem": pem,
        "idempotency_key": "scanner-collision",
        "confirm_registration": true
    });
    assert_eq!(
        call(
            &fixture.router,
            Method::POST,
            scanner_key_http_test::path(),
            Some(&fixture.submitter_token),
            &scanner_collision
        )
        .await
        .0,
        StatusCode::CONFLICT
    );

    let revoke = json!({
        "expected_key_record_digest": digest,
        "idempotency_key": "sandbox-revoke",
        "reason": "fixture sandbox verifier trust root intentionally retired",
        "confirm_revocation": true
    });
    assert_eq!(
        call(
            &fixture.router,
            Method::POST,
            &format!("{}/{id}/revoke", path()),
            Some(&fixture.applier_token),
            &revoke
        )
        .await
        .0,
        StatusCode::CREATED
    );
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
    assert_redacted(&current, &pem);
    fixture.cleanup();
}

fn registration(pem: &str, key: &str) -> Value {
    json!({
        "verifier_operator": "fixture-sandbox-lab",
        "verifier_product": "fixture-sandbox-verifier-v1",
        "algorithm": "rsa-pkcs1v15-sha256",
        "public_key_pem": pem,
        "idempotency_key": key,
        "confirm_registration": true
    })
}

fn path() -> &'static str {
    "/api/admin/compute/external-pool-adapter-sandbox-verifier-keys"
}

fn assert_redacted(value: &Value, pem: &str) {
    let encoded = value.to_string();
    assert!(!encoded.contains(pem));
    for key in ["public_key_pem", "idempotency_key", "idempotency_scope"] {
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
