use super::{
    runtime_launch_profile_test_support::{
        assert_inert_effects, create_runtime_launch_profile_fixture,
        owner_collection_path as runtime_profile_owner_collection_path, profile_body,
        RuntimeLaunchProfileFixture,
    },
    *,
};

pub(super) struct UpstreamTransportTargetFixture {
    pub roots: RuntimeLaunchProfileFixture,
    pub profile: Value,
    pub policy: Value,
}

pub(super) async fn create_upstream_transport_target_fixture(
    fixture: &Fixture,
    suffix: &str,
) -> UpstreamTransportTargetFixture {
    let roots = create_runtime_launch_profile_fixture(fixture, suffix).await;
    let (status, profile) = call(
        &fixture.router,
        Method::POST,
        &runtime_profile_owner_collection_path(&roots),
        Some(&fixture.member_token),
        &profile_body(&roots, &format!("{suffix}-profile"), None),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{profile}");
    let (status, policy) = call(
        &fixture.router,
        Method::GET,
        &owner_policy_path_from(&roots, &profile),
        Some(&fixture.member_token),
        &Value::Null,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{policy}");
    UpstreamTransportTargetFixture {
        roots,
        profile,
        policy,
    }
}

pub(super) fn owner_policy_path(roots: &UpstreamTransportTargetFixture) -> String {
    owner_policy_path_from(&roots.roots, &roots.profile)
}

pub(super) fn admin_policy_path(roots: &UpstreamTransportTargetFixture) -> String {
    format!(
        "{}/upstream-transport-policy",
        profile_root(roots, "/api/admin/compute/external-pool-provider-bindings")
    )
}

pub(super) fn owner_collection_path(roots: &UpstreamTransportTargetFixture) -> String {
    format!(
        "{}/upstream-transport-targets",
        profile_root(roots, "/api/me/compute/external-pool-provider-bindings")
    )
}

pub(super) fn admin_collection_path(roots: &UpstreamTransportTargetFixture) -> String {
    format!(
        "{}/upstream-transport-targets",
        profile_root(roots, "/api/admin/compute/external-pool-provider-bindings")
    )
}

pub(super) fn target_body(
    roots: &UpstreamTransportTargetFixture,
    key: &str,
    predecessor: Option<&Value>,
) -> Value {
    let expected_predecessor = predecessor.map_or(Value::Null, |target| {
        json!({
            "target_id":target["target"]["target_id"],
            "target_digest":target["target"]["target_digest"]
        })
    });
    json!({
        "expected_profile_digest":roots.profile["profile"]["profile_digest"],
        "expected_candidate_digest":roots.profile["profile"]["candidate_digest"],
        "expected_provider_binding_digest":roots.profile["profile"]["provider_binding_digest"],
        "expected_target_policy_digest":roots.policy["policy_digest"],
        "draft":{
            "dns_hostname":"pool.example.test",
            "port":443,
            "expected_tls_leaf_spki_sha256":"a".repeat(64)
        },
        "expected_predecessor":expected_predecessor,
        "idempotency_key":key,
        "confirm_upstream_transport_target":true
    })
}

pub(super) fn owner_currentness_path(
    roots: &UpstreamTransportTargetFixture,
    target: &Value,
) -> String {
    target_path(owner_collection_path(roots), target, "currentness")
}

pub(super) fn admin_currentness_path(
    roots: &UpstreamTransportTargetFixture,
    target: &Value,
) -> String {
    target_path(admin_collection_path(roots), target, "currentness")
}

pub(super) fn owner_revocation_path(
    roots: &UpstreamTransportTargetFixture,
    target: &Value,
) -> String {
    target_path(owner_collection_path(roots), target, "revocation")
}

pub(super) fn admin_revocation_path(
    roots: &UpstreamTransportTargetFixture,
    target: &Value,
) -> String {
    target_path(admin_collection_path(roots), target, "revocation")
}

pub(super) fn revoke_body(target: &Value, key: &str) -> Value {
    json!({
        "expected_target_digest":target["target"]["target_digest"],
        "expected_profile_digest":target["target"]["profile_digest"],
        "reason":"authorized actor withdraws this inert upstream transport target",
        "idempotency_key":key,
        "confirm_revocation":true
    })
}

pub(super) fn installed_entrypoint(
    fixture: &Fixture,
    roots: &UpstreamTransportTargetFixture,
) -> PathBuf {
    let digest = roots.profile["profile"]["installation_content_digest"]
        .as_str()
        .unwrap();
    super::adapter_installation_http_test::installation_root(fixture, digest).join("bin/adapter.sh")
}

pub(super) fn assert_transport_public(value: &Value) {
    for forbidden in [
        "dns_hostname",
        "port",
        "tls_server_name",
        "expected_tls_leaf_spki_sha256",
        "provider_owner_account_id",
        "recorded_by_actor_kind",
        "recorded_by_actor_user_id",
        "revoked_by_actor_kind",
        "revoked_by_actor_user_id",
        "idempotency_scope",
        "idempotency_key",
        "confirmation",
        "route_adapter_projection_id",
        "service_actor_id",
        "receipt_json",
    ] {
        assert_forbidden_key(value, forbidden);
    }
    assert_false_readiness_recursive(value);
}

pub(super) fn assert_transport_inert_effects(
    fixture: &Fixture,
    roots: &UpstreamTransportTargetFixture,
    targets: i64,
    revocations: i64,
) {
    assert_inert_effects(fixture, &roots.roots, 1, 0);
    let connection = fixture.state.store.conn().unwrap();
    for (table, expected) in [
        (
            "compute_external_pool_adapter_upstream_transport_targets",
            targets,
        ),
        (
            "compute_external_pool_adapter_upstream_transport_target_revocations",
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

fn owner_policy_path_from(roots: &RuntimeLaunchProfileFixture, profile: &Value) -> String {
    format!(
        "{}/upstream-transport-policy",
        profile_root_from(
            roots,
            profile,
            "/api/me/compute/external-pool-provider-bindings"
        )
    )
}

fn profile_root(roots: &UpstreamTransportTargetFixture, prefix: &str) -> String {
    profile_root_from(&roots.roots, &roots.profile, prefix)
}

fn profile_root_from(roots: &RuntimeLaunchProfileFixture, profile: &Value, prefix: &str) -> String {
    format!(
        "{prefix}/{}/activation-candidates/{}/runtime-launch-profiles/{}",
        profile["profile"]["provider_binding_id"].as_str().unwrap(),
        roots.candidate["candidate"]["candidate_id"]
            .as_str()
            .unwrap(),
        profile["profile"]["profile_id"].as_str().unwrap()
    )
}

fn target_path(collection: String, target: &Value, suffix: &str) -> String {
    format!(
        "{}/{}/{}",
        collection,
        target["target"]["target_id"].as_str().unwrap(),
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

fn assert_false_readiness_recursive(value: &Value) {
    match value {
        Value::Object(map) => {
            for key in [
                "broker_connect_ready",
                "upstream_probe_observed",
                "runtime_launch_ready",
                "activation_ready",
            ] {
                if let Some(actual) = map.get(key) {
                    assert_eq!(
                        actual.as_bool(),
                        Some(false),
                        "unexpected readiness {key}: {value}"
                    );
                }
            }
            map.values().for_each(assert_false_readiness_recursive);
        }
        Value::Array(items) => items.iter().for_each(assert_false_readiness_recursive),
        _ => {}
    }
}
