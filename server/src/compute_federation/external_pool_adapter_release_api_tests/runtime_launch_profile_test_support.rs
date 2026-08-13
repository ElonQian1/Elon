use super::{
    activation_candidate_test_support::{
        assert_zero_effects, candidate_body, create_activation_candidate_fixture, owner_collection,
        ActivationCandidateFixture,
    },
    *,
};

pub(super) struct RuntimeLaunchProfileFixture {
    pub roots: ActivationCandidateFixture,
    pub candidate: Value,
    pub policy: Value,
}

pub(super) async fn create_runtime_launch_profile_fixture(
    fixture: &Fixture,
    suffix: &str,
) -> RuntimeLaunchProfileFixture {
    let roots = create_activation_candidate_fixture(fixture, suffix).await;
    let (status, candidate) = call(
        &fixture.router,
        Method::POST,
        &owner_collection(&roots),
        Some(&fixture.member_token),
        &candidate_body(&roots, &format!("{suffix}-v254-candidate")),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{candidate}");
    let (status, policy) = call(
        &fixture.router,
        Method::GET,
        &owner_policy_path_from(&roots, &candidate),
        Some(&fixture.member_token),
        &Value::Null,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{policy}");
    RuntimeLaunchProfileFixture {
        roots,
        candidate,
        policy,
    }
}

pub(super) fn owner_policy_path(roots: &RuntimeLaunchProfileFixture) -> String {
    owner_policy_path_from(&roots.roots, &roots.candidate)
}

pub(super) fn admin_policy_path(roots: &RuntimeLaunchProfileFixture) -> String {
    format!(
        "/api/admin/compute/external-pool-provider-bindings/{}/activation-candidates/{}/runtime-launch-policy",
        binding_id(roots),
        candidate_id(roots)
    )
}

pub(super) fn owner_collection_path(roots: &RuntimeLaunchProfileFixture) -> String {
    format!(
        "/api/me/compute/external-pool-provider-bindings/{}/activation-candidates/{}/runtime-launch-profiles",
        binding_id(roots),
        candidate_id(roots)
    )
}

pub(super) fn admin_collection_path(roots: &RuntimeLaunchProfileFixture) -> String {
    format!(
        "/api/admin/compute/external-pool-provider-bindings/{}/activation-candidates/{}/runtime-launch-profiles",
        binding_id(roots),
        candidate_id(roots)
    )
}

pub(super) fn profile_body(
    roots: &RuntimeLaunchProfileFixture,
    key: &str,
    predecessor: Option<&Value>,
) -> Value {
    let expected_predecessor = predecessor.map_or(Value::Null, |profile| {
        json!({
            "profile_id":profile["profile"]["profile_id"],
            "profile_digest":profile["profile"]["profile_digest"]
        })
    });
    json!({
        "expected_candidate_digest":roots.candidate["candidate"]["candidate_digest"],
        "expected_provider_binding_digest":roots.candidate["candidate"]["provider_binding_digest"],
        "expected_launch_policy_digest":roots.policy["policy_digest"],
        "expected_predecessor":expected_predecessor,
        "idempotency_key":key,
        "confirm_runtime_launch_profile":true
    })
}

pub(super) fn owner_currentness_path(
    roots: &RuntimeLaunchProfileFixture,
    profile: &Value,
) -> String {
    profile_path(owner_collection_path(roots), profile, "currentness")
}

pub(super) fn admin_currentness_path(
    roots: &RuntimeLaunchProfileFixture,
    profile: &Value,
) -> String {
    profile_path(admin_collection_path(roots), profile, "currentness")
}

pub(super) fn owner_revocation_path(
    roots: &RuntimeLaunchProfileFixture,
    profile: &Value,
) -> String {
    profile_path(owner_collection_path(roots), profile, "revocation")
}

pub(super) fn admin_revocation_path(
    roots: &RuntimeLaunchProfileFixture,
    profile: &Value,
) -> String {
    profile_path(admin_collection_path(roots), profile, "revocation")
}

pub(super) fn revoke_body(profile: &Value, key: &str) -> Value {
    json!({
        "expected_profile_digest":profile["profile"]["profile_digest"],
        "expected_candidate_digest":profile["profile"]["candidate_digest"],
        "reason":"authorized actor withdraws the inert runtime launch profile",
        "idempotency_key":key,
        "confirm_revocation":true
    })
}

pub(super) fn installed_entrypoint(
    fixture: &Fixture,
    roots: &RuntimeLaunchProfileFixture,
) -> PathBuf {
    let digest = roots.candidate["candidate"]["installation_content_digest"]
        .as_str()
        .unwrap();
    super::adapter_installation_http_test::installation_root(fixture, digest).join("bin/adapter.sh")
}

pub(super) fn assert_runtime_public(value: &Value) {
    for forbidden in [
        "provider_owner_account_id",
        "service_actor_id",
        "route_adapter_projection_id",
        "recorded_by_actor_kind",
        "recorded_by_actor_user_id",
        "revoked_by_actor_kind",
        "revoked_by_actor_user_id",
        "idempotency_scope",
        "idempotency_key",
        "confirmation",
        "credential_ref",
        "credential_locator",
        "credential_locator_commitment",
        "resolver_backend_policy_digest",
        "resolver_backend_root",
        "installation_path",
        "installation_root",
        "entrypoint_path",
        "entrypoint_relative_path",
        "receipt_json",
    ] {
        assert_forbidden_key(value, forbidden);
    }
    assert_public_none_effects_recursive(value);
}

pub(super) fn assert_public_none_effects(value: &Value) {
    assert_eq!(value["adapter_effect"], "none", "{value}");
    assert_eq!(value["runtime_effect"], "none", "{value}");
    assert_eq!(value["usage_effect"], "none", "{value}");
}

pub(super) fn assert_inert_effects(
    fixture: &Fixture,
    roots: &RuntimeLaunchProfileFixture,
    profiles: i64,
    revocations: i64,
) {
    assert_zero_effects(fixture, &roots.roots);
    let connection = fixture.state.store.conn().unwrap();
    for (table, expected) in [
        (
            "compute_external_pool_adapter_runtime_launch_profiles",
            profiles,
        ),
        (
            "compute_external_pool_adapter_runtime_launch_profile_revocations",
            revocations,
        ),
    ] {
        let count: i64 = connection
            .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(count, expected, "unexpected rows in {table}");
    }
}

fn owner_policy_path_from(roots: &ActivationCandidateFixture, candidate: &Value) -> String {
    format!(
        "/api/me/compute/external-pool-provider-bindings/{}/activation-candidates/{}/runtime-launch-policy",
        roots.upstream.roots.registry["binding"]["provider_binding_id"]
            .as_str()
            .unwrap(),
        candidate["candidate"]["candidate_id"].as_str().unwrap()
    )
}

fn binding_id(roots: &RuntimeLaunchProfileFixture) -> &str {
    roots.roots.upstream.roots.registry["binding"]["provider_binding_id"]
        .as_str()
        .unwrap()
}

fn candidate_id(roots: &RuntimeLaunchProfileFixture) -> &str {
    roots.candidate["candidate"]["candidate_id"]
        .as_str()
        .unwrap()
}

fn profile_path(collection: String, profile: &Value, suffix: &str) -> String {
    format!(
        "{}/{}/{}",
        collection,
        profile["profile"]["profile_id"].as_str().unwrap(),
        suffix
    )
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

fn assert_public_none_effects_recursive(value: &Value) {
    match value {
        Value::Object(map) => {
            if map.contains_key("runtime_effect") {
                assert_public_none_effects(value);
            }
            map.values().for_each(assert_public_none_effects_recursive);
        }
        Value::Array(items) => items.iter().for_each(assert_public_none_effects_recursive),
        _ => {}
    }
}
