use rsa::{
    pkcs8::{EncodePublicKey, LineEnding},
    rand_core::OsRng,
    RsaPrivateKey,
};

use super::*;

#[tokio::test]
async fn external_pool_adapter_artifact_signing_key_http_enforces_auth_four_eyes_and_redaction() {
    let fixture = fixture();
    let public_key_pem = RsaPrivateKey::new(&mut OsRng, 2_048)
        .unwrap()
        .to_public_key()
        .to_public_key_pem(LineEnding::LF)
        .unwrap();
    let register = json!({
        "source_operator":"http-fixture-pool",
        "algorithm":"rsa-pkcs1v15-sha256",
        "public_key_pem":public_key_pem,
        "idempotency_key":"register-http-key",
        "confirm_registration":true
    });

    assert_eq!(
        call(
            &fixture.router,
            Method::POST,
            signing_key_path(),
            None,
            &register,
        )
        .await
        .0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        call(
            &fixture.router,
            Method::POST,
            signing_key_path(),
            Some(&fixture.member_token),
            &register,
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );

    let mut unknown = register.clone();
    unknown["created_by_admin_user_id"] = json!(fixture.reviewer.id);
    assert_eq!(
        call(
            &fixture.router,
            Method::POST,
            signing_key_path(),
            Some(&fixture.submitter_token),
            &unknown,
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );

    let (status, created) = call(
        &fixture.router,
        Method::POST,
        signing_key_path(),
        Some(&fixture.submitter_token),
        &register,
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{created}");
    assert_eq!(created["replayed"], false);
    assert_eq!(
        created["key_record"]["created_by_admin_user_id"],
        fixture.submitter.id
    );
    assert_redacted(&created, &public_key_pem);

    let (status, replayed) = call(
        &fixture.router,
        Method::POST,
        signing_key_path(),
        Some(&fixture.submitter_token),
        &register,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{replayed}");
    assert_eq!(replayed["replayed"], true);
    assert_eq!(replayed["key_record"], created["key_record"]);

    let id = created["key_record"]["key_record_id"].as_str().unwrap();
    let digest = created["key_record"]["key_record_digest"].as_str().unwrap();
    let activate = json!({
        "expected_key_record_digest":digest,
        "idempotency_key":"activate-http-key",
        "confirm_activation":true
    });
    assert_eq!(
        call(
            &fixture.router,
            Method::POST,
            &format!("{}/{id}/activate", signing_key_path()),
            Some(&fixture.submitter_token),
            &activate,
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    let (status, activated) = call(
        &fixture.router,
        Method::POST,
        &format!("{}/{id}/activate", signing_key_path()),
        Some(&fixture.reviewer_token),
        &activate,
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{activated}");
    assert_redacted(&activated, &public_key_pem);

    let revoke = json!({
        "expected_key_record_digest":digest,
        "idempotency_key":"revoke-http-key",
        "reason":"HTTP fixture intentionally retires this trust root",
        "confirm_revocation":true
    });
    let (status, revoked) = call(
        &fixture.router,
        Method::POST,
        &format!("{}/{id}/revoke", signing_key_path()),
        Some(lifecycle_support::LOCAL_OWNER_TOKEN),
        &revoke,
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{revoked}");
    assert_redacted(&revoked, &public_key_pem);

    let (status, historical_activation) = call(
        &fixture.router,
        Method::POST,
        &format!("{}/{id}/activate", signing_key_path()),
        Some(&fixture.reviewer_token),
        &activate,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{historical_activation}");
    assert_eq!(historical_activation["replayed"], true);

    let (status, current) = call(
        &fixture.router,
        Method::GET,
        &format!("{}/{id}/currentness", signing_key_path()),
        Some(&fixture.applier_token),
        &Value::Null,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{current}");
    assert_eq!(current["current_status"], "revoked");
    assert_redacted(&current, &public_key_pem);

    assert_eq!(
        call(
            &fixture.router,
            Method::GET,
            &format!("{}/missing/currentness", signing_key_path()),
            Some(&fixture.applier_token),
            &Value::Null,
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
    fixture.cleanup();
}

fn signing_key_path() -> &'static str {
    "/api/admin/compute/external-pool-adapter-artifact-signing-keys"
}

fn assert_redacted(value: &Value, pem: &str) {
    let encoded = value.to_string();
    assert!(!encoded.contains(pem));
    assert_forbidden_key(value, "public_key_pem");
    assert_forbidden_key(value, "idempotency_key");
    assert_forbidden_key(value, "idempotency_scope");
}

fn assert_forbidden_key(value: &Value, forbidden: &str) {
    match value {
        Value::Object(object) => {
            assert!(!object.contains_key(forbidden), "{forbidden}: {value}");
            for nested in object.values() {
                assert_forbidden_key(nested, forbidden);
            }
        }
        Value::Array(values) => {
            for nested in values {
                assert_forbidden_key(nested, forbidden);
            }
        }
        _ => {}
    }
}
