use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::Connection;

use super::*;

#[path = "tests/support.rs"]
mod support;

use support::*;

#[test]
fn migration_is_repeatable_after_reopen_and_preserves_v247_history() {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!(
        "elon-external-pool-adapter-registry-{}-{nonce}.sqlite",
        std::process::id()
    ));
    {
        let connection = Connection::open(&path).unwrap();
        create_v247_fixture(&connection);
        migration_v249(&connection).unwrap();
        migration_v249(&connection).unwrap();
    }
    {
        let connection = Connection::open(&path).unwrap();
        migration_v249(&connection).unwrap();
        for object in [
            "compute_external_pool_adapter_registry_releases",
            "compute_external_pool_adapter_registry_provider_bindings",
            "compute_external_pool_adapter_registry_release_current",
            "compute_external_pool_adapter_registry_provider_binding_current",
            "external_pool_adapter_registry_release_exact_roots",
            "external_pool_adapter_registry_provider_binding_exact_roots",
            "external_pool_adapter_registry_release_json_projection",
            "external_pool_adapter_registry_provider_binding_json_projection",
        ] {
            assert_eq!(object_count(&connection, object), 1, "missing V249 object");
        }
        let marker: String = connection
            .query_row("SELECT marker FROM legacy_v247_marker", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(marker, "untouched");
    }
    std::fs::remove_file(path).unwrap();
}

#[test]
fn one_neutral_release_supports_two_independent_provider_companions() {
    let connection = Connection::open_in_memory().unwrap();
    create_v247_fixture(&connection);
    migration_v249(&connection).unwrap();
    insert_release(&connection, &digest('6')).unwrap();
    insert_binding(
        &connection,
        1,
        "confirm_external_pool_adapter_registry_binding",
    )
    .unwrap();
    insert_binding(
        &connection,
        2,
        "confirm_external_pool_adapter_registry_binding",
    )
    .unwrap();

    assert_eq!(
        connection
            .query_row(
                "SELECT count(*) FROM compute_external_pool_adapter_registry_releases",
                [],
                |row| row.get::<_, i64>(0),
            )
            .unwrap(),
        1
    );
    let current: i64 = connection
        .query_row(
            "SELECT count(*) FROM compute_external_pool_adapter_registry_provider_binding_current
              WHERE current_status='binding_current'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(current, 2);
}

#[test]
fn projection_drift_reserved_collision_and_raw_sql_replace_fail_closed() {
    let connection = Connection::open_in_memory().unwrap();
    create_v247_fixture(&connection);
    migration_v249(&connection).unwrap();
    assert!(insert_release(&connection, &digest('7')).is_err());
    insert_release(&connection, &digest('6')).unwrap();
    assert!(insert_binding(&connection, 1, "tampered-confirmation").is_err());
    connection
        .execute(
            "INSERT INTO compute_route_adapters VALUES('projection-1')",
            [],
        )
        .unwrap();
    assert!(insert_binding(
        &connection,
        1,
        "confirm_external_pool_adapter_registry_binding"
    )
    .is_err());
    connection
        .execute("DELETE FROM compute_route_adapters", [])
        .unwrap();
    insert_binding(
        &connection,
        1,
        "confirm_external_pool_adapter_registry_binding",
    )
    .unwrap();

    assert!(connection
        .execute(
            "UPDATE compute_external_pool_adapter_registry_releases
                SET release_version='2.0.0'",
            [],
        )
        .is_err());
    assert!(connection
        .execute(
            "DELETE FROM compute_external_pool_adapter_registry_provider_bindings",
            [],
        )
        .is_err());
    assert!(connection
        .execute(
            "INSERT OR REPLACE INTO compute_external_pool_adapter_registry_releases
             SELECT * FROM compute_external_pool_adapter_registry_releases",
            [],
        )
        .is_err());
}

#[test]
fn explicit_terminals_and_provider_or_release_status_downgrade_views_only() {
    let connection = Connection::open_in_memory().unwrap();
    create_v247_fixture(&connection);
    migration_v249(&connection).unwrap();
    insert_release(&connection, &digest('6')).unwrap();
    insert_binding(
        &connection,
        1,
        "confirm_external_pool_adapter_registry_binding",
    )
    .unwrap();
    insert_binding(
        &connection,
        2,
        "confirm_external_pool_adapter_registry_binding",
    )
    .unwrap();

    connection
        .execute(
            "INSERT INTO compute_external_pool_adapter_installation_terminal_receipts
         VALUES('terminal-1','installation-1',?1)",
            [digest('9')],
        )
        .unwrap();
    assert_status(&connection, "binding-1", "historical_only");
    assert_status(&connection, "binding-2", "binding_current");

    connection
        .execute(
            "UPDATE compute_providers SET status='disabled' WHERE provider_id='provider-2'",
            [],
        )
        .unwrap();
    assert_status(&connection, "binding-2", "historical_only");
    let release: String = connection
        .query_row(
            "SELECT current_status FROM compute_external_pool_adapter_registry_release_current",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(release, "release_current");

    connection
        .execute(
            "UPDATE compute_external_pool_adapter_release_admission_current
                SET current_status='revoked'",
            [],
        )
        .unwrap();
    let release: String = connection
        .query_row(
            "SELECT current_status FROM compute_external_pool_adapter_registry_release_current",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(release, "historical_only");
    assert_eq!(
        connection
            .query_row("SELECT marker FROM legacy_v247_marker", [], |row| row
                .get::<_, String>(0),)
            .unwrap(),
        "untouched"
    );
}

#[test]
fn schema_guards_cover_every_signed_field_and_forbid_wallclock_expiry_in_views() {
    let connection = Connection::open_in_memory().unwrap();
    create_v247_fixture(&connection);
    migration_v249(&connection).unwrap();
    let release = object_sql(
        &connection,
        "trigger",
        "external_pool_adapter_registry_release_json_projection",
    );
    for path in [
        "$.release.installation_content_digest",
        "$.release.credential_verifier",
        "$.release.manifest",
        "$.release.registry_effect",
        "$.release.settlement_effect",
    ] {
        assert!(release.contains(path), "missing neutral projection {path}");
    }
    let binding = object_sql(
        &connection,
        "trigger",
        "external_pool_adapter_registry_provider_binding_json_projection",
    );
    for path in [
        "$.binding.route_adapter_projection_id",
        "$.binding.installation_content_digest",
        "$.binding.bound_by_admin_user_id",
        "$.binding.confirmation",
        "$.binding.checked_at",
        "$.binding.settlement_effect",
    ] {
        assert!(
            binding.contains(path),
            "missing companion projection {path}"
        );
    }
    let current = object_sql(
        &connection,
        "view",
        "compute_external_pool_adapter_registry_provider_binding_current",
    );
    assert!(!current.contains("julianday('now')"));
    assert!(!current.contains("sandbox_conformance_current"));
    assert!(!current.contains("credential_verification_current"));
    assert!(current.contains("installation_terminal_receipts"));
    assert!(current.contains("adoption_terminal_receipts"));
}

#[test]
fn v251_replaces_an_applied_v249_release_guard_and_is_repeatable() {
    let connection = Connection::open_in_memory().unwrap();
    create_v247_fixture(&connection);
    migration_v249(&connection).unwrap();
    connection
        .execute_batch(
            "DROP TRIGGER external_pool_adapter_registry_release_exact_roots;
             CREATE TRIGGER external_pool_adapter_registry_release_exact_roots
             BEFORE INSERT ON compute_external_pool_adapter_registry_releases
             BEGIN SELECT RAISE(ABORT,'legacy v249 registry guard'); END;",
        )
        .unwrap();

    migration_v251(&connection).unwrap();
    migration_v251(&connection).unwrap();

    let guard = object_sql(
        &connection,
        "trigger",
        "external_pool_adapter_registry_release_exact_roots",
    );
    assert!(guard.contains("json_extract(package.credential_verifier_json"));
    assert!(!guard.contains("legacy v249 registry guard"));
    insert_release(&connection, &digest('6')).unwrap();
}

fn assert_status(connection: &Connection, binding_id: &str, expected: &str) {
    let status: String = connection
        .query_row(
            "SELECT current_status
               FROM compute_external_pool_adapter_registry_provider_binding_current
              WHERE provider_binding_id=?1",
            [binding_id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(status, expected);
}
