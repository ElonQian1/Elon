use rusqlite::Connection;

use super::super::{
    create_schema_objects_v3, create_schema_objects_v4, create_schema_objects_v5, ensure_schema,
};

const APPLICATION_ID: i64 = 0x454c_4350;
const VERSION_V3: i64 = 3;
const VERSION_V4: i64 = 4;
const VERSION_V5: i64 = 5;
const VERSION_V6: i64 = 6;
const PLAN_APPLICATION_TRIGGER: &str = "plan_application_matches_authority";
const POLICY_BINDING_TABLE: &str = "sharing_policy_binding_receipts";
const POLICY_REVOCATION_TABLE: &str = "sharing_policy_binding_revocation_receipts";
const CATALOG_BINDING_TABLE: &str = "manifest_catalog_binding_receipts";

fn connection() -> Connection {
    let connection = Connection::open_in_memory().unwrap();
    connection
        .pragma_update(None, "foreign_keys", "ON")
        .unwrap();
    connection
        .pragma_update(None, "trusted_schema", "OFF")
        .unwrap();
    connection
}

fn install_legacy(connection: &Connection, version: i64) {
    match version {
        VERSION_V3 => create_schema_objects_v3(connection).unwrap(),
        VERSION_V4 => create_schema_objects_v4(connection).unwrap(),
        VERSION_V5 => create_schema_objects_v5(connection).unwrap(),
        other => panic!("unsupported legacy fixture version {other}"),
    }
    connection
        .pragma_update(None, "user_version", version)
        .unwrap();
    connection
        .pragma_update(None, "application_id", APPLICATION_ID)
        .unwrap();
}

fn pragma(connection: &Connection, name: &str) -> i64 {
    connection
        .pragma_query_value(None, name, |row| row.get(0))
        .unwrap()
}

fn object_count(connection: &Connection, object_type: &str, name: &str) -> i64 {
    connection
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = ?1 AND name = ?2",
            [object_type, name],
            |row| row.get(0),
        )
        .unwrap()
}

fn trigger_sql(connection: &Connection, name: &str) -> String {
    connection
        .query_row(
            "SELECT sql FROM sqlite_master WHERE type = 'trigger' AND name = ?1",
            [name],
            |row| row.get(0),
        )
        .unwrap()
}

fn assert_v6_shape(connection: &Connection) {
    assert_eq!(pragma(connection, "user_version"), VERSION_V6);
    assert_eq!(pragma(connection, "application_id"), APPLICATION_ID);
    assert_eq!(object_count(connection, "table", POLICY_BINDING_TABLE), 1);
    assert_eq!(
        object_count(connection, "table", POLICY_REVOCATION_TABLE),
        1
    );
    assert_eq!(object_count(connection, "table", CATALOG_BINDING_TABLE), 1);

    let binding_object_count: i64 = connection
        .query_row(
            r#"SELECT COUNT(*) FROM sqlite_master
               WHERE (type = 'table' AND name = 'sharing_policy_binding_receipts')
                  OR (type = 'trigger' AND name IN (
                      'sharing_policy_binding_revision_monotonic',
                      'sharing_policy_binding_insert_fenced',
                      'authority_sharing_policy_binding_receipt_required',
                      'sharing_policy_binding_apply_authority',
                      'sharing_policy_binding_receipt_update_forbidden',
                      'sharing_policy_binding_receipt_delete_forbidden'
                  ))"#,
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(binding_object_count, 7);

    let plan_gate = trigger_sql(connection, PLAN_APPLICATION_TRIGGER);
    assert!(plan_gate.contains("NEW.applied_at_ms > meta.trusted_time_high_water_ms"));
    assert!(!plan_gate.contains("NEW.applied_at_ms >= meta.trusted_time_high_water_ms"));
}

fn assert_migrates_to_v6(version: i64) {
    let mut connection = connection();
    install_legacy(&connection, version);
    if version == VERSION_V3 {
        assert!(trigger_sql(&connection, PLAN_APPLICATION_TRIGGER)
            .contains("NEW.applied_at_ms >= meta.trusted_time_high_water_ms"));
    }

    ensure_schema(&mut connection).unwrap();
    assert_v6_shape(&connection);
    ensure_schema(&mut connection).unwrap();
}

fn assert_drift_rejected_without_later_schema(
    version: i64,
    expected_policy: i64,
    expected_revocation: i64,
    expected_catalog: i64,
) {
    let mut connection = connection();
    install_legacy(&connection, version);
    connection
        .execute_batch("CREATE TABLE unexpected_legacy_object (id INTEGER PRIMARY KEY);")
        .unwrap();

    let error = ensure_schema(&mut connection).unwrap_err();

    assert!(format!("{error:#}").contains("COMPUTE_PLUGIN_AUTHORITY_SCHEMA_UNEXPECTED"));
    assert_eq!(pragma(&connection, "user_version"), version);
    assert_eq!(
        object_count(&connection, "table", POLICY_BINDING_TABLE),
        expected_policy
    );
    assert_eq!(
        object_count(&connection, "table", POLICY_REVOCATION_TABLE),
        expected_revocation
    );
    assert_eq!(
        object_count(&connection, "table", CATALOG_BINDING_TABLE),
        expected_catalog
    );
}

#[test]
fn fresh_v6_installs_exact_version_chain_and_reopens() {
    let mut connection = connection();

    ensure_schema(&mut connection).unwrap();
    assert_v6_shape(&connection);
    ensure_schema(&mut connection).unwrap();
    assert_v6_shape(&connection);
}

#[test]
fn exact_v3_migrates_atomically_to_v6() {
    assert_migrates_to_v6(VERSION_V3);
}

#[test]
fn exact_v4_migrates_atomically_to_v6() {
    assert_migrates_to_v6(VERSION_V4);
}

#[test]
fn exact_v5_migrates_atomically_to_v6() {
    assert_migrates_to_v6(VERSION_V5);
}

#[test]
fn drifted_v3_is_rejected_without_v4_v5_or_v6_ddl() {
    assert_drift_rejected_without_later_schema(VERSION_V3, 0, 0, 0);
}

#[test]
fn drifted_v4_is_rejected_without_v5_or_v6_ddl() {
    assert_drift_rejected_without_later_schema(VERSION_V4, 1, 0, 0);
}

#[test]
fn drifted_v5_is_rejected_without_v6_ddl() {
    assert_drift_rejected_without_later_schema(VERSION_V5, 1, 1, 0);
}

#[test]
fn drifted_v6_is_rejected_on_reopen() {
    let mut connection = connection();
    ensure_schema(&mut connection).unwrap();
    connection
        .execute_batch("DROP TRIGGER sharing_policy_binding_receipt_delete_forbidden;")
        .unwrap();

    let error = ensure_schema(&mut connection).unwrap_err();

    let message = format!("{error:#}");
    assert!(message.contains("COMPUTE_PLUGIN_AUTHORITY_SCHEMA_INCOMPLETE"));
    assert!(message.contains("sharing_policy_binding_receipt_delete_forbidden"));
    assert_eq!(pragma(&connection, "user_version"), VERSION_V6);
}
