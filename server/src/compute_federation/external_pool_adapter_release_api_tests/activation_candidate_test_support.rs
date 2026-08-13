use super::{
    credential_reattestation_test_support::{
        challenge_body as credential_challenge_body, record_body as credential_record_body,
    },
    sandbox_reattestation_test_support::{
        create_sandbox_reattestation_fixture, issue_challenge as issue_sandbox_challenge,
        record_challenge as record_sandbox_challenge, SandboxReattestationFixture,
    },
    *,
};

pub(super) struct ActivationCandidateFixture {
    pub upstream: SandboxReattestationFixture,
    pub sandbox: Value,
    pub credential: Value,
}

pub(super) async fn create_activation_candidate_fixture(
    fixture: &Fixture,
    suffix: &str,
) -> ActivationCandidateFixture {
    let upstream = create_sandbox_reattestation_fixture(fixture, suffix, "254.0.0").await;
    let sandbox_challenge = issue_sandbox_challenge(fixture, &upstream, suffix).await;
    let (status, sandbox) = record_sandbox_challenge(
        fixture,
        &upstream,
        &sandbox_challenge,
        &format!("{suffix}-v252-record"),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{sandbox}");
    let binding_id = provider_binding_id_from(&upstream);
    let credential_collection = format!(
        "/api/admin/compute/external-pool-adapter-registry-provider-bindings/{binding_id}/credential-reattestations"
    );
    let roots = credential_roots(&upstream);
    let (status, challenge) = call(
        &fixture.router,
        Method::POST,
        &format!("{credential_collection}/challenge"),
        Some(&fixture.applier_token),
        &credential_challenge_body(&roots, suffix),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{challenge}");
    let (status, credential) = call(
        &fixture.router,
        Method::POST,
        &credential_collection,
        Some(&fixture.applier_token),
        &credential_record_body(&roots, &challenge, &format!("{suffix}-v253-record")),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{credential}");
    ActivationCandidateFixture {
        upstream,
        sandbox,
        credential,
    }
}

pub(super) fn owner_collection(roots: &ActivationCandidateFixture) -> String {
    format!(
        "/api/me/compute/external-pool-provider-bindings/{}/activation-candidates",
        provider_binding_id(roots)
    )
}

pub(super) fn candidate_body(roots: &ActivationCandidateFixture, key: &str) -> Value {
    json!({
        "expected_provider_binding_digest":roots.upstream.roots.registry["binding"]["provider_binding_digest"],
        "expected_registry_release_digest":roots.upstream.roots.registry["release"]["registry_release_digest"],
        "idempotency_key":key,
        "confirm_activation_candidate":true
    })
}

pub(super) fn owner_currentness_path(
    roots: &ActivationCandidateFixture,
    created: &Value,
) -> String {
    format!(
        "{}/{}/currentness",
        owner_collection(roots),
        created["candidate"]["candidate_id"].as_str().unwrap()
    )
}

pub(super) fn admin_currentness_path(
    roots: &ActivationCandidateFixture,
    created: &Value,
) -> String {
    format!(
        "/api/admin/compute/external-pool-provider-bindings/{}/activation-candidates/{}/currentness",
        provider_binding_id(roots),
        created["candidate"]["candidate_id"].as_str().unwrap()
    )
}

pub(super) fn owner_preflight_path(roots: &ActivationCandidateFixture, created: &Value) -> String {
    format!(
        "{}/{}/preflight?{}",
        owner_collection(roots),
        created["candidate"]["candidate_id"].as_str().unwrap(),
        preflight_query(roots, created)
    )
}

pub(super) fn admin_preflight_path(roots: &ActivationCandidateFixture, created: &Value) -> String {
    format!(
        "/api/admin/compute/external-pool-provider-bindings/{}/activation-candidates/{}/preflight?{}",
        provider_binding_id(roots),
        created["candidate"]["candidate_id"].as_str().unwrap(),
        preflight_query(roots, created)
    )
}

pub(super) fn revoke_path(roots: &ActivationCandidateFixture, created: &Value) -> String {
    format!(
        "/api/me/compute/external-pool-provider-bindings/{}/activation-delegations/{}/revocation",
        provider_binding_id(roots),
        created["delegation"]["delegation_id"].as_str().unwrap()
    )
}

pub(super) fn revoke_body(created: &Value, key: &str) -> Value {
    json!({
        "expected_delegation_digest":created["delegation"]["delegation_digest"],
        "expected_candidate_digest":created["candidate"]["candidate_digest"],
        "reason":"owner explicitly withdraws the inert activation delegation",
        "idempotency_key":key,
        "confirm_revocation":true
    })
}

pub(super) fn assert_public_redaction(value: &Value) {
    for forbidden in [
        "service_actor_id",
        "route_adapter_projection_id",
        "provider_owner_account_id",
        "issued_by_owner_user_id",
        "revoked_by_owner_user_id",
        "idempotency_scope",
        "idempotency_key",
        "confirmation",
        "credential_ref",
        "non_bearer_credential_ref",
        "credential_locator_commitment",
        "installation_path",
        "entrypoint_path",
        "receipt_json",
    ] {
        assert_forbidden_key(value, forbidden);
    }
}

pub(super) fn assert_zero_effects(fixture: &Fixture, roots: &ActivationCandidateFixture) {
    let provider_id = roots.upstream.roots.registry["binding"]["provider_id"]
        .as_str()
        .unwrap();
    let connection = fixture.state.store.conn().unwrap();
    let provider: (String, i64, String) = connection
        .query_row(
            "SELECT status,current_policy_revision,current_provider_digest
               FROM compute_providers WHERE provider_id=?1",
            [provider_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .unwrap();
    assert_eq!(provider.0, "registering");
    assert_eq!(provider.1, 1);
    assert_eq!(
        provider.2.as_str(),
        roots.upstream.roots.registry["binding"]["provider_digest"]
            .as_str()
            .unwrap()
    );
    for table in [
        "compute_capacity_pools",
        "compute_capacity_pool_versions",
        "compute_offers",
        "compute_offer_versions",
        "compute_price_snapshots",
        "compute_jobs",
        "compute_job_versions",
        "compute_reservations",
        "compute_reservation_versions",
        "compute_attempt_activations",
        "compute_attempt_execution_plans",
        "compute_attempt_lease_states",
        "compute_attempt_dispatch_commands",
        "compute_attempt_dispatch_acks",
        "compute_attempt_dispatch_applications",
        "compute_route_adapters",
        "compute_route_adapter_versions",
        "compute_route_credentials",
        "compute_route_credential_versions",
        "compute_route_credential_revocations",
        "compute_route_authorization_receipts",
        "compute_route_authorization_capabilities",
        "compute_route_authorization_seals",
        "compute_service_actor_authorizations",
        "compute_attempt_dispatch_actor_receipts",
        "compute_attempt_lease_authority_bindings",
        "compute_attempt_start_outbox",
        "compute_attempt_start_send_attempts",
        "compute_attempt_start_remote_observations",
        "compute_attempt_usage_declarations",
        "compute_attempt_execution_receipts",
        "compute_attempt_settlements",
        "compute_settlement_postings",
        "compute_settlement_ledger_legs",
        "compute_settlement_account_balances",
        "compute_settlement_releases",
        "compute_settlement_release_postings",
        "compute_settlement_release_ledger_legs",
        "compute_settlement_withdrawal_requests",
        "compute_settlement_withdrawal_request_postings",
        "compute_settlement_withdrawal_request_ledger_legs",
    ] {
        let count: i64 = connection
            .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(count, 0, "V254 candidate unexpectedly populated {table}");
    }
}

fn credential_roots(
    upstream: &SandboxReattestationFixture,
) -> super::credential_reattestation_test_support::CredentialReattestationFixture {
    super::credential_reattestation_test_support::CredentialReattestationFixture {
        registry: upstream.roots.registry.clone(),
        private: upstream.roots.credential_private.clone(),
        key: upstream.roots.credential_key.clone(),
    }
}

fn provider_binding_id(roots: &ActivationCandidateFixture) -> &str {
    provider_binding_id_from(&roots.upstream)
}

fn provider_binding_id_from(roots: &SandboxReattestationFixture) -> &str {
    roots.roots.registry["binding"]["provider_binding_id"]
        .as_str()
        .unwrap()
}

fn preflight_query(roots: &ActivationCandidateFixture, created: &Value) -> String {
    let vulnerability = &roots.upstream.vulnerability_reattestation["reattestation"];
    let sandbox = &roots.sandbox["reattestation"];
    let credential = &roots.credential["reattestation"];
    [
        (
            "expected_candidate_digest",
            created["candidate"]["candidate_digest"].as_str().unwrap(),
        ),
        (
            "vulnerability_reattestation_receipt_id",
            vulnerability["reattestation_receipt_id"].as_str().unwrap(),
        ),
        (
            "expected_vulnerability_reattestation_receipt_digest",
            vulnerability["reattestation_receipt_digest"]
                .as_str()
                .unwrap(),
        ),
        (
            "sandbox_reattestation_receipt_id",
            sandbox["reattestation_receipt_id"].as_str().unwrap(),
        ),
        (
            "expected_sandbox_reattestation_receipt_digest",
            sandbox["reattestation_receipt_digest"].as_str().unwrap(),
        ),
        (
            "credential_reattestation_receipt_id",
            credential["reattestation_receipt_id"].as_str().unwrap(),
        ),
        (
            "expected_credential_reattestation_receipt_digest",
            credential["reattestation_receipt_digest"].as_str().unwrap(),
        ),
    ]
    .into_iter()
    .map(|(key, value)| format!("{key}={value}"))
    .collect::<Vec<_>>()
    .join("&")
}

fn assert_forbidden_key(value: &Value, forbidden: &str) {
    match value {
        Value::Object(map) => {
            assert!(!map.contains_key(forbidden), "exposed {forbidden}: {value}");
            map.values()
                .for_each(|child| assert_forbidden_key(child, forbidden));
        }
        Value::Array(items) => items
            .iter()
            .for_each(|child| assert_forbidden_key(child, forbidden)),
        _ => {}
    }
}
