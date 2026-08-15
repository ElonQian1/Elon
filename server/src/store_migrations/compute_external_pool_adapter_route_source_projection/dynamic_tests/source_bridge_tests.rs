use super::source_bridge_fixture::{
    insert_route, source_bridge_fixture, RouteAttempt, LOGICAL_ADAPTER_ID, OTHER_DIGEST,
};

#[test]
fn v271_exact_source_bridge_accepts_only_the_unique_projection_id() {
    let connection = source_bridge_fixture();
    let exact = RouteAttempt::exact("route-exact");
    assert_eq!(
        insert_route(&connection, &exact).expect("exact projected route should pass"),
        1
    );

    let mut logical = RouteAttempt::exact("route-logical-id");
    logical.adapter_id = LOGICAL_ADAPTER_ID.to_owned();
    assert_source_rejected(&connection, &logical, "logical Adapter ID");
    assert_eq!(route_count(&connection), 1);
}

#[test]
fn v271_source_bridge_rejects_each_cross_root_drift() {
    let connection = source_bridge_fixture();
    let mut cases = Vec::new();

    let mut source = RouteAttempt::exact("route-drift-source");
    source.source_digest = OTHER_DIGEST.to_owned();
    cases.push(("source digest", source));

    let mut provider = RouteAttempt::exact("route-drift-provider");
    provider.provider_id = "provider-2".to_owned();
    cases.push(("provider", provider));

    let mut owner = RouteAttempt::exact("route-drift-owner");
    owner.owner_id = "owner-2".to_owned();
    cases.push(("provider owner", owner));

    let mut binding = RouteAttempt::exact("route-drift-binding");
    binding.route_binding_digest = OTHER_DIGEST.to_owned();
    cases.push(("logical binding", binding));

    let mut release = RouteAttempt::exact("route-drift-release");
    release.release_version = "2.0.0".to_owned();
    cases.push(("release", release));

    let mut implementation = RouteAttempt::exact("route-drift-implementation");
    implementation.implementation_digest = OTHER_DIGEST.to_owned();
    cases.push(("implementation", implementation));

    let mut config = RouteAttempt::exact("route-drift-config");
    config.config_digest = OTHER_DIGEST.to_owned();
    cases.push(("config", config));

    let mut actor = RouteAttempt::exact("route-drift-actor");
    actor.service_actor_id = "service-actor-2".to_owned();
    cases.push(("service actor", actor));

    for (label, attempt) in cases {
        assert_source_rejected(&connection, &attempt, label);
    }
    assert_eq!(route_count(&connection), 0);
}

fn assert_source_rejected(connection: &rusqlite::Connection, attempt: &RouteAttempt, label: &str) {
    let error = insert_route(connection, attempt)
        .expect_err(&format!("{label} drift unexpectedly inserted a route"));
    assert!(
        format!("{error:#}").contains("compute route authorization lacks exact source"),
        "unexpected {label} rejection: {error:#}"
    );
}

fn route_count(connection: &rusqlite::Connection) -> i64 {
    connection
        .query_row(
            "SELECT COUNT(*) FROM compute_route_authorization_receipts",
            [],
            |row| row.get(0),
        )
        .expect("route count should read")
}
