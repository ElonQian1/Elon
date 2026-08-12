use super::{
    adapter_adoption_http_test::{adoption_body, create_adoption_roots, sign, AdoptionRoots},
    adapter_installation_http_test::{installation_body, installation_root},
    credential_verification_http_test::{
        create_onboarding_application, verification_body, verification_path,
    },
    *,
};

pub(super) const REGISTRY_PATH: &str = "/api/admin/compute/external-pool-adapter-registry-bindings";
const ADOPTION_PATH: &str = "/api/admin/compute/external-pool-adapter-adoptions";
const INSTALLATION_PATH: &str = "/api/admin/compute/external-pool-adapter-installations";

pub(super) struct InstalledRegistryFixture {
    pub roots: AdoptionRoots,
    pub adoption: Value,
    pub installation: Value,
}

pub(super) async fn create_installed_registry_fixture(
    fixture: &Fixture,
    suffix: &str,
    version: &str,
) -> InstalledRegistryFixture {
    let roots = create_adoption_roots(fixture, suffix, version).await;
    let (status, adoption) = call(
        &fixture.router,
        Method::POST,
        ADOPTION_PATH,
        Some(&fixture.applier_token),
        &adoption_body(&roots, &format!("{suffix}-adoption")),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{adoption}");
    let (status, installation) = call(
        &fixture.router,
        Method::POST,
        INSTALLATION_PATH,
        Some(&fixture.applier_token),
        &installation_body(fixture, &adoption, &format!("{suffix}-install"), true),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{installation}");
    InstalledRegistryFixture {
        roots,
        adoption,
        installation,
    }
}

pub(super) async fn create_second_provider_installation(
    fixture: &Fixture,
    first: &InstalledRegistryFixture,
    suffix: &str,
) -> (Value, Value) {
    let version = first.installation["installation"]["adapter_release_version"]
        .as_str()
        .unwrap();
    let application = create_onboarding_application(fixture, suffix, version);
    let verification = verification_body(
        &application,
        &first.roots.staged,
        &first.roots.credential_key,
        suffix,
    );
    let (_, challenge) = call(
        &fixture.router,
        Method::POST,
        &format!("{}/challenge", verification_path()),
        Some(&fixture.applier_token),
        &verification,
    )
    .await;
    let mut record = verification;
    record["expected_signature_message_digest"] = challenge["signature_message_digest"].clone();
    record["signature_base64"] = json!(sign(&challenge, first.roots.credential_private.clone()));
    record["idempotency_key"] = json!(format!("{suffix}-credential-record"));
    record["confirm_verification"] = json!(true);
    let (status, credential) = call(
        &fixture.router,
        Method::POST,
        verification_path(),
        Some(&fixture.applier_token),
        &record,
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{credential}");
    let roots = AdoptionRoots {
        staged: first.roots.staged.clone(),
        application,
        sandbox: first.roots.sandbox.clone(),
        credential,
        credential_key: first.roots.credential_key.clone(),
        credential_private: first.roots.credential_private.clone(),
    };
    let (status, adoption) = call(
        &fixture.router,
        Method::POST,
        ADOPTION_PATH,
        Some(&fixture.applier_token),
        &adoption_body(&roots, &format!("{suffix}-adoption")),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{adoption}");
    let (status, installation) = call(
        &fixture.router,
        Method::POST,
        INSTALLATION_PATH,
        Some(&fixture.applier_token),
        &installation_body(fixture, &adoption, &format!("{suffix}-install"), true),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{installation}");
    (adoption, installation)
}

pub(super) fn registry_body(installation: &Value, idempotency_key: &str, confirmed: bool) -> Value {
    json!({
        "installation_receipt_id":installation["installation"]["installation_receipt_id"],
        "expected_installation_receipt_digest":installation["installation"]["installation_receipt_digest"],
        "idempotency_key":idempotency_key,
        "confirm_registry_binding":confirmed
    })
}

pub(super) fn currentness_path(created: &Value) -> String {
    format!(
        "{REGISTRY_PATH}/{}/currentness",
        created["binding"]["provider_binding_id"].as_str().unwrap()
    )
}

pub(super) fn installed_entrypoint(fixture: &Fixture, installation: &Value) -> PathBuf {
    let digest = installation["installation"]["installation_content_digest"]
        .as_str()
        .unwrap();
    installation_root(fixture, digest).join("bin/adapter.sh")
}

pub(super) fn assert_registry_redacted(fixture: &Fixture, value: &Value) {
    let encoded = value.to_string();
    for forbidden in [
        "bin/adapter.sh",
        "entrypoint",
        "files",
        "credential_locator_commitment",
        "candidate_artifact_ref",
        "signature_base64",
        "bound_by_admin_user_id",
        "idempotency_key",
        "idempotency_scope",
        "confirmation",
        &fixture.data_dir.display().to_string(),
        &fixture.database_path.display().to_string(),
    ] {
        assert!(
            !encoded.contains(forbidden),
            "exposed {forbidden}: {encoded}"
        );
    }
}

pub(super) fn assert_no_registry_activation_effects(fixture: &Fixture, created: &Value) {
    let provider_id = created["binding"]["provider_id"].as_str().unwrap();
    let connection = fixture.state.store.conn().unwrap();
    let status: String = connection
        .query_row(
            "SELECT status FROM compute_providers WHERE provider_id=?1",
            [provider_id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(status, "registering");
    for table in [
        "compute_route_adapters",
        "compute_route_credentials",
        "compute_route_authorization_receipts",
        "compute_service_actor_authorizations",
        "compute_attempt_start_outbox",
        "compute_offers",
        "compute_jobs",
    ] {
        let count: i64 = connection
            .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(count, 0, "registry unexpectedly populated {table}");
    }
}
