use rsa::{
    pkcs8::{EncodePublicKey, LineEnding},
    rand_core::OsRng,
    RsaPrivateKey,
};

use super::*;

#[tokio::test]
async fn scanner_key_http_enforces_four_eyes_replay_and_redaction() {
    let fixture = fixture();
    let pem = RsaPrivateKey::new(&mut OsRng, 2048)
        .unwrap()
        .to_public_key()
        .to_public_key_pem(LineEnding::LF)
        .unwrap();
    let register = json!({"scanner_operator":"fixture-security-lab","scanner_product":"fixture-scanner-v1","algorithm":"rsa-pkcs1v15-sha256","public_key_pem":pem,"idempotency_key":"scanner-register","confirm_registration":true});
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
    let activate = json!({"expected_key_record_digest":digest,"idempotency_key":"scanner-activate","confirm_activation":true});
    assert_eq!(
        call(
            &fixture.router,
            Method::POST,
            &format!("{path}/{id}/activate", path = path()),
            Some(&fixture.submitter_token),
            &activate
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    let (status, active) = call(
        &fixture.router,
        Method::POST,
        &format!("{path}/{id}/activate", path = path()),
        Some(&fixture.reviewer_token),
        &activate,
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{active}");
    assert_redacted(&active, &pem);
    let revoke = json!({"expected_key_record_digest":digest,"idempotency_key":"scanner-revoke","reason":"fixture scanner trust root is intentionally retired","confirm_revocation":true});
    assert_eq!(
        call(
            &fixture.router,
            Method::POST,
            &format!("{path}/{id}/revoke", path = path()),
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
        &format!("{path}/{id}/currentness", path = path()),
        Some(&fixture.applier_token),
        &Value::Null,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(current["current_status"], "revoked");
    assert_redacted(&current, &pem);
    fixture.cleanup();
}

fn path() -> &'static str {
    "/api/admin/compute/external-pool-adapter-scanner-keys"
}
fn assert_redacted(value: &Value, pem: &str) {
    let encoded = value.to_string();
    assert!(!encoded.contains(pem));
    for key in ["public_key_pem", "idempotency_key", "idempotency_scope"] {
        assert_forbidden(value, key)
    }
}
fn assert_forbidden(value: &Value, key: &str) {
    match value {
        Value::Object(map) => {
            assert!(!map.contains_key(key), "{key}: {value}");
            for item in map.values() {
                assert_forbidden(item, key)
            }
        }
        Value::Array(items) => {
            for item in items {
                assert_forbidden(item, key)
            }
        }
        _ => {}
    }
}
