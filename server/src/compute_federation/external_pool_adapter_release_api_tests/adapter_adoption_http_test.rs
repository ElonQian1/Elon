use base64::{engine::general_purpose::STANDARD, Engine as _};
use rsa::{
    pkcs1v15::SigningKey,
    signature::{SignatureEncoding, Signer},
};
use sha2::Sha256;

use super::{
    artifact_sandbox_conformance_http_test::{conformance_body, conformance_path},
    artifact_vulnerability_report_http_test::create_vulnerability_report,
    credential_verification_http_test::{
        create_onboarding_application, create_release_credential_verifier, verification_body,
        verification_path,
    },
    credential_verifier_key_http_test::{self, create_active_credential_verifier_key},
    sandbox_verifier_key_http_test::create_active_sandbox_verifier_key,
    *,
};

#[tokio::test]
async fn Adapter_adoption_http_binds_current_roots_and_appends_revocation() {
    let fixture = fixture();
    let roots = create_adoption_roots(&fixture, "adoption", "44.0.0").await;
    let body = adoption_body(&roots, "adoption-create");

    assert_eq!(
        call(&fixture.router, Method::POST, path(), None, &body)
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
            &body,
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    let mut actor_injection = body.clone();
    actor_injection["adopted_by_admin_user_id"] = json!(fixture.applier.id);
    assert_eq!(
        call(
            &fixture.router,
            Method::POST,
            path(),
            Some(&fixture.applier_token),
            &actor_injection,
        )
        .await
        .0,
        StatusCode::UNPROCESSABLE_ENTITY
    );

    let (status, created) = call(
        &fixture.router,
        Method::POST,
        path(),
        Some(&fixture.applier_token),
        &body,
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{created}");
    assert_eq!(
        created["adoption"]["adoption_effect"],
        "adoption_authority_current"
    );
    assert_eq!(created["adoption"]["install_effect"], "authorization_only");
    for effect in [
        "provider_effect",
        "route_effect",
        "execution_effect",
        "settlement_effect",
    ] {
        assert_eq!(created["adoption"][effect], "none");
    }
    assert_redacted(&created);
    let (_, replay) = call(
        &fixture.router,
        Method::POST,
        path(),
        Some(&fixture.applier_token),
        &body,
    )
    .await;
    assert_eq!(replay["replayed"], true);

    let id = created["adoption"]["adoption_receipt_id"].as_str().unwrap();
    let digest = created["adoption"]["adoption_receipt_digest"]
        .as_str()
        .unwrap();
    let currentness_path = format!("{path}/{id}/currentness", path = path());
    let (status, current) = call(
        &fixture.router,
        Method::GET,
        &currentness_path,
        Some(&fixture.applier_token),
        &Value::Null,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{current}");
    assert_eq!(current["current_status"], "adopted_current");
    assert_eq!(current["sandbox_conformance_status"], "verified_current");
    assert_eq!(
        current["credential_verification_status"],
        "verified_current"
    );
    assert_eq!(current["terminal_status"], "none");

    {
        let connection = fixture.state.store.conn().unwrap();
        assert!(connection
            .execute(
                "INSERT OR REPLACE INTO compute_external_pool_adapter_adoption_receipts
                 SELECT * FROM compute_external_pool_adapter_adoption_receipts
                 WHERE adoption_receipt_id=?1",
                [id],
            )
            .is_err());
    }
    let revoke = json!({
        "expected_adoption_receipt_digest":digest,
        "reason":"operator intentionally withdraws adoption authority",
        "idempotency_key":"adoption-revoke",
        "confirm_revocation":true
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
    assert_eq!(
        revoked["terminal"]["adoption_effect"],
        "adoption_authority_revoked"
    );
    let (_, historical) = call(
        &fixture.router,
        Method::GET,
        &currentness_path,
        Some(&fixture.applier_token),
        &Value::Null,
    )
    .await;
    assert_eq!(historical["current_status"], "historical_only");
    assert_eq!(historical["terminal_status"], "revoked");
    fixture.cleanup();
}

#[tokio::test]
async fn Adapter_adoption_currentness_fails_closed_when_credential_root_is_revoked() {
    let fixture = fixture();
    let roots = create_adoption_roots(&fixture, "adoption-upstream", "44.1.0").await;
    let (_, created) = call(
        &fixture.router,
        Method::POST,
        path(),
        Some(&fixture.applier_token),
        &adoption_body(&roots, "adoption-upstream-create"),
    )
    .await;
    let id = created["adoption"]["adoption_receipt_id"].as_str().unwrap();
    let key_id = roots.credential_key["key_record"]["key_record_id"]
        .as_str()
        .unwrap();
    let key_digest = roots.credential_key["key_record"]["key_record_digest"]
        .as_str()
        .unwrap();
    let (status, revoked) = call(
        &fixture.router,
        Method::POST,
        &format!(
            "{}/{key_id}/revoke",
            credential_verifier_key_http_test::path()
        ),
        Some(&fixture.applier_token),
        &json!({
            "expected_key_record_digest":key_digest,
            "idempotency_key":"adoption-upstream-key-revoke",
            "reason":"fixture invalidates the adopted credential root",
            "confirm_revocation":true
        }),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{revoked}");
    let (_, historical) = call(
        &fixture.router,
        Method::GET,
        &format!("{}/{id}/currentness", path()),
        Some(&fixture.applier_token),
        &Value::Null,
    )
    .await;
    assert_eq!(historical["current_status"], "historical_only");
    assert_eq!(
        historical["credential_verification_status"],
        "historical_only"
    );
    assert_eq!(historical["terminal_status"], "none");
    fixture.cleanup();
}

pub(super) struct AdoptionRoots {
    pub(super) staged: Value,
    pub(super) application: crate::store::ExternalPoolOnboardingApplicationReceipt,
    pub(super) sandbox: Value,
    pub(super) credential: Value,
    pub(super) credential_key: Value,
    pub(super) credential_private: rsa::RsaPrivateKey,
}

pub(super) async fn create_adoption_roots(
    fixture: &Fixture,
    suffix: &str,
    version: &str,
) -> AdoptionRoots {
    let (staged, vulnerability) = create_vulnerability_report(fixture, suffix, version).await;
    let (sandbox_private, sandbox_key) = create_active_sandbox_verifier_key(fixture, suffix).await;
    let sandbox_path = conformance_path(&staged);
    let sandbox_body = conformance_body(&vulnerability, &sandbox_key, suffix);
    let (_, challenge) = call(
        &fixture.router,
        Method::POST,
        &format!("{sandbox_path}/challenge"),
        Some(&fixture.applier_token),
        &sandbox_body,
    )
    .await;
    let mut sandbox_record = sandbox_body;
    sandbox_record["expected_signature_message_digest"] =
        challenge["signature_message_digest"].clone();
    sandbox_record["signature_base64"] = json!(sign(&challenge, sandbox_private));
    sandbox_record["idempotency_key"] = json!(format!("{suffix}-sandbox-record"));
    sandbox_record["confirm_conformance"] = json!(true);
    let (status, sandbox) = call(
        &fixture.router,
        Method::POST,
        &sandbox_path,
        Some(&fixture.applier_token),
        &sandbox_record,
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{sandbox}");

    let verifier = create_release_credential_verifier(fixture, suffix).await;
    let (credential_private, credential_key) =
        create_active_credential_verifier_key(fixture, &verifier, suffix).await;
    let application = create_onboarding_application(fixture, suffix, version);
    let credential_body = verification_body(&application, &staged, &credential_key, suffix);
    let (_, challenge) = call(
        &fixture.router,
        Method::POST,
        &format!("{}/challenge", verification_path()),
        Some(&fixture.applier_token),
        &credential_body,
    )
    .await;
    let mut credential_record = credential_body;
    credential_record["expected_signature_message_digest"] =
        challenge["signature_message_digest"].clone();
    credential_record["signature_base64"] = json!(sign(&challenge, credential_private.clone()));
    credential_record["idempotency_key"] = json!(format!("{suffix}-credential-record"));
    credential_record["confirm_verification"] = json!(true);
    let (status, credential) = call(
        &fixture.router,
        Method::POST,
        verification_path(),
        Some(&fixture.applier_token),
        &credential_record,
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{credential}");
    AdoptionRoots {
        staged,
        application,
        sandbox,
        credential,
        credential_key,
        credential_private,
    }
}

pub(super) async fn create_current_adoption(
    fixture: &Fixture,
    suffix: &str,
    version: &str,
) -> Value {
    let roots = create_adoption_roots(fixture, suffix, version).await;
    let (status, created) = call(
        &fixture.router,
        Method::POST,
        path(),
        Some(&fixture.applier_token),
        &adoption_body(&roots, &format!("{suffix}-adoption")),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{created}");
    created
}

pub(super) fn adoption_body(roots: &AdoptionRoots, idempotency_key: &str) -> Value {
    json!({
        "application_id":roots.application.application_id,
        "expected_application_digest":roots.application.application_digest,
        "admission_id":roots.staged["admission_id"],
        "expected_admission_digest":roots.staged["admission_digest"],
        "expected_sandbox_conformance_receipt_digest":roots.sandbox["sandbox_conformance"]["sandbox_conformance_receipt_digest"],
        "credential_verification_receipt_id":roots.credential["credential_verification"]["credential_verification_receipt_id"],
        "expected_credential_verification_receipt_digest":roots.credential["credential_verification"]["credential_verification_receipt_digest"],
        "idempotency_key":idempotency_key,
        "confirm_adoption":true
    })
}

pub(super) fn sign(challenge: &Value, private: rsa::RsaPrivateKey) -> String {
    let message = STANDARD
        .decode(challenge["signature_message_base64"].as_str().unwrap())
        .unwrap();
    STANDARD.encode(SigningKey::<Sha256>::new(private).sign(&message).to_vec())
}

fn path() -> &'static str {
    "/api/admin/compute/external-pool-adapter-adoptions"
}

fn assert_redacted(value: &Value) {
    for forbidden in [
        "credential_ref",
        "non_bearer_credential_ref",
        "signature_base64",
        "idempotency_key",
        "idempotency_scope",
    ] {
        assert_forbidden_key(value, forbidden);
    }
}

fn assert_forbidden_key(value: &Value, forbidden: &str) {
    match value {
        Value::Object(map) => {
            assert!(!map.contains_key(forbidden), "exposed {forbidden}: {value}");
            for child in map.values() {
                assert_forbidden_key(child, forbidden);
            }
        }
        Value::Array(items) => {
            for child in items {
                assert_forbidden_key(child, forbidden);
            }
        }
        _ => {}
    }
}
