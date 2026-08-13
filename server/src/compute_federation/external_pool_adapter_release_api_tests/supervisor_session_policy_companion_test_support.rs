use super::{
    upstream_transport_target_test_support::{
        admin_collection_path as target_admin_collection_path,
        create_upstream_transport_target_fixture, installed_entrypoint,
        owner_collection_path as target_owner_collection_path, target_body,
        UpstreamTransportTargetFixture,
    },
    *,
};

pub(super) struct SupervisorSessionPolicyCompanionFixture {
    pub roots: UpstreamTransportTargetFixture,
    pub target: Value,
    pub policy: Value,
}

pub(super) async fn create_supervisor_session_policy_companion_fixture(
    fixture: &Fixture,
    suffix: &str,
) -> SupervisorSessionPolicyCompanionFixture {
    let roots = create_upstream_transport_target_fixture(fixture, suffix).await;
    let (status, target) = call(
        &fixture.router,
        Method::POST,
        &target_owner_collection_path(&roots),
        Some(&fixture.member_token),
        &target_body(&roots, &format!("{suffix}-target"), None),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{target}");
    let (status, policy) = call(
        &fixture.router,
        Method::GET,
        &owner_policy_path(&roots, &target),
        Some(&fixture.member_token),
        &Value::Null,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{policy}");
    SupervisorSessionPolicyCompanionFixture {
        roots,
        target,
        policy,
    }
}

pub(super) fn owner_policy_path(
    fixture: &UpstreamTransportTargetFixture,
    target: &Value,
) -> String {
    format!(
        "{}/{}/supervisor-session-policy",
        target_owner_collection_path(fixture),
        target["target"]["target_id"].as_str().unwrap()
    )
}

pub(super) fn admin_policy_path(
    fixture: &UpstreamTransportTargetFixture,
    target: &Value,
) -> String {
    format!(
        "{}/{}/supervisor-session-policy",
        target_admin_collection_path(fixture),
        target["target"]["target_id"].as_str().unwrap()
    )
}

pub(super) fn owner_collection_path(fixture: &SupervisorSessionPolicyCompanionFixture) -> String {
    companion_collection(
        target_owner_collection_path(&fixture.roots),
        &fixture.target,
    )
}

pub(super) fn admin_collection_path(fixture: &SupervisorSessionPolicyCompanionFixture) -> String {
    companion_collection(
        target_admin_collection_path(&fixture.roots),
        &fixture.target,
    )
}

pub(super) fn companion_body(
    fixture: &SupervisorSessionPolicyCompanionFixture,
    key: &str,
    predecessor: Option<&Value>,
) -> Value {
    let expected_predecessor = predecessor.map_or(Value::Null, |receipt| {
        json!({
            "companion_id":receipt["companion"]["companion_id"],
            "companion_digest":receipt["companion"]["companion_digest"]
        })
    });
    json!({
        "expected_target_digest":fixture.target["target"]["target_digest"],
        "expected_profile_digest":fixture.target["target"]["profile_digest"],
        "expected_candidate_digest":fixture.target["target"]["candidate_digest"],
        "expected_provider_binding_digest":fixture.target["target"]["provider_binding_digest"],
        "expected_supervisor_session_policy_digest":fixture.policy["policy_digest"],
        "expected_predecessor":expected_predecessor,
        "idempotency_key":key,
        "confirm_supervisor_session_policy_companion":true
    })
}

pub(super) fn owner_currentness_path(
    fixture: &SupervisorSessionPolicyCompanionFixture,
    companion: &Value,
) -> String {
    companion_path(owner_collection_path(fixture), companion, "currentness")
}

pub(super) fn admin_currentness_path(
    fixture: &SupervisorSessionPolicyCompanionFixture,
    companion: &Value,
) -> String {
    companion_path(admin_collection_path(fixture), companion, "currentness")
}

pub(super) fn owner_revocation_path(
    fixture: &SupervisorSessionPolicyCompanionFixture,
    companion: &Value,
) -> String {
    companion_path(owner_collection_path(fixture), companion, "revocation")
}

pub(super) fn admin_revocation_path(
    fixture: &SupervisorSessionPolicyCompanionFixture,
    companion: &Value,
) -> String {
    companion_path(admin_collection_path(fixture), companion, "revocation")
}

pub(super) fn revoke_body(companion: &Value, key: &str) -> Value {
    json!({
        "expected_companion_digest":companion["companion"]["companion_digest"],
        "expected_target_digest":companion["companion"]["target_digest"],
        "expected_profile_digest":companion["companion"]["profile_digest"],
        "reason":"authorized actor withdraws this inert supervisor session companion",
        "idempotency_key":key,
        "confirm_revocation":true
    })
}

pub(super) fn assert_companion_public_and_inert(value: &Value) {
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
        "credential_locator",
        "credential_sha256",
        "config_locator",
        "config_sha256",
        "entrypoint_path",
        "session_key",
        "host_nonce",
        "child_nonce",
        "transcript_digest",
        "pid",
        "pidfd",
        "cgroup_path",
        "receipt_json",
    ] {
        assert_forbidden_key(value, forbidden);
    }
    assert_inert_recursive(value);
}

pub(super) fn assert_companion_rows(fixture: &Fixture, companions: i64, revocations: i64) {
    let connection = fixture.state.store.conn().unwrap();
    for (table, expected) in [
        (
            "compute_external_pool_adapter_supervisor_session_policy_companions",
            companions,
        ),
        (
            "compute_external_pool_adapter_supervisor_session_policy_companion_revocations",
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

pub(super) fn drift_installed_entrypoint(
    fixture: &Fixture,
    roots: &SupervisorSessionPolicyCompanionFixture,
) {
    std::fs::write(
        installed_entrypoint(fixture, &roots.roots),
        b"v259 drift after durable companion",
    )
    .unwrap();
}

pub(super) async fn malformed_json_status(router: &Router, path: &str, token: &str) -> StatusCode {
    router
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(path)
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::from("{\"expected_target_digest\":"))
                .unwrap(),
        )
        .await
        .unwrap()
        .status()
}

fn companion_collection(collection: String, target: &Value) -> String {
    format!(
        "{}/{}/supervisor-session-policy-companions",
        collection,
        target["target"]["target_id"].as_str().unwrap()
    )
}

fn companion_path(collection: String, companion: &Value, suffix: &str) -> String {
    format!(
        "{}/{}/{}",
        collection,
        companion["companion"]["companion_id"].as_str().unwrap(),
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

fn assert_inert_recursive(value: &Value) {
    match value {
        Value::Object(map) => {
            for key in [
                "adapter_effect",
                "runtime_effect",
                "provider_effect",
                "credential_effect",
                "route_effect",
                "execution_effect",
                "usage_effect",
                "market_effect",
                "settlement_effect",
            ] {
                if let Some(actual) = map.get(key) {
                    assert_eq!(actual, "none", "unexpected effect {key}: {value}");
                }
            }
            for key in [
                "process_spawn_ready",
                "ipc_session_ready",
                "secret_delivery_ready",
                "broker_connect_ready",
                "upstream_probe_observed",
                "runtime_launch_ready",
                "activation_ready",
            ] {
                if let Some(actual) = map.get(key) {
                    assert_eq!(actual.as_bool(), Some(false), "unexpected {key}: {value}");
                }
            }
            map.values().for_each(assert_inert_recursive);
        }
        Value::Array(items) => items.iter().for_each(assert_inert_recursive),
        _ => {}
    }
}
