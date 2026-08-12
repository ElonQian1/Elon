use rusqlite::Connection;

use super::*;

#[test]
fn migration_registers_complete_append_only_v252_authority() {
    let connection = Connection::open_in_memory().unwrap();
    create_v251_contract_schema(&connection);
    migration_v252(&connection).unwrap();
    migration_v252(&connection).unwrap();

    for object in [
        "compute_external_pool_adapter_sandbox_reattestation_challenges",
        "compute_external_pool_adapter_sandbox_reattestation_receipts",
        "compute_external_pool_adapter_sandbox_reattestation_revocations",
        "compute_external_pool_adapter_sandbox_reattestation_current",
        "external_pool_adapter_sandbox_reattestation_challenge_exact_roots",
        "external_pool_adapter_sandbox_reattestation_receipt_exact_challenge",
        "external_pool_adapter_sandbox_reattestation_receipt_current_roots",
        "external_pool_adapter_sandbox_reattestation_revocation_exact_target",
        "external_pool_adapter_sandbox_reattestation_receipt_lineage",
        "external_pool_adapter_sandbox_reattestation_challenge_time_bounds",
        "external_pool_adapter_sandbox_reattestation_revocation_time_order",
    ] {
        assert_eq!(
            object_count(&connection, object),
            1,
            "missing V252 object {object}"
        );
    }
}

#[test]
fn tables_have_single_use_chain_full_roots_and_no_mutable_consumption() {
    let connection = Connection::open_in_memory().unwrap();
    create_v251_contract_schema(&connection);
    migration_v252(&connection).unwrap();
    let challenge_columns = columns(
        &connection,
        "compute_external_pool_adapter_sandbox_reattestation_challenges",
    );
    assert_eq!(challenge_columns.len(), 20);
    assert!(!challenge_columns
        .iter()
        .any(|column| column.starts_with("consumed")));
    for column in [
        "challenge_nonce_base64",
        "challenge_json",
        "registry_release_material_digest",
        "vulnerability_reattestation_material_digest",
        "predecessor_receipt_digest",
    ] {
        assert!(
            challenge_columns.contains(&column.to_string()),
            "missing challenge column {column}"
        );
    }
    let receipt = object_sql(
        &connection,
        "table",
        "compute_external_pool_adapter_sandbox_reattestation_receipts",
    );
    assert_eq!(
        columns(
            &connection,
            "compute_external_pool_adapter_sandbox_reattestation_receipts",
        )
        .len(),
        91
    );
    assert_eq!(
        columns(
            &connection,
            "compute_external_pool_adapter_sandbox_reattestation_revocations",
        )
        .len(),
        22
    );
    for fence in [
        "challenge_id TEXT NOT NULL UNIQUE",
        "UNIQUE(registry_release_id,sequence)",
        "UNIQUE(predecessor_receipt_id)",
        "verifier_report_id TEXT NOT NULL UNIQUE",
        "passed_capability_count=6",
        "policy_violation_count=0",
        "supported_provider_kinds_json='[\"external_pool\"]'",
    ] {
        assert!(receipt.contains(fence), "missing receipt fence {fence}");
    }
    let genesis = object_sql(
        &connection,
        "index",
        "uq_external_pool_adapter_sandbox_reattestation_genesis",
    );
    assert!(genesis.contains("WHERE predecessor_receipt_id IS NULL"));
}

#[test]
fn store_insert_columns_match_migration_columns_in_exact_order() {
    let connection = Connection::open_in_memory().unwrap();
    create_v251_contract_schema(&connection);
    migration_v252(&connection).unwrap();
    let persistence = include_str!(
        "../../store/compute_external_pool_adapter_sandbox_reattestation/persistence.rs"
    );
    let revocation = include_str!(
        "../../store/compute_external_pool_adapter_sandbox_reattestation/revocation.rs"
    );
    for (table, source) in [
        (
            "compute_external_pool_adapter_sandbox_reattestation_challenges",
            persistence,
        ),
        (
            "compute_external_pool_adapter_sandbox_reattestation_receipts",
            persistence,
        ),
        (
            "compute_external_pool_adapter_sandbox_reattestation_revocations",
            revocation,
        ),
    ] {
        assert_eq!(columns(&connection, table), insert_columns(source, table));
    }
}

#[test]
fn canonical_projection_counts_are_frozen() {
    assert_eq!(guards::projection_counts(), (32, 95, 24));
}

#[test]
fn root_projection_and_time_guards_cover_frozen_security_contract() {
    let connection = Connection::open_in_memory().unwrap();
    create_v251_contract_schema(&connection);
    migration_v252(&connection).unwrap();
    let challenge = object_sql(
        &connection,
        "trigger",
        "external_pool_adapter_sandbox_reattestation_challenge_projection",
    );
    for path in [
        "$.binding.registry_release_material_digest",
        "$.binding.vulnerability_reattestation_material_digest",
        "$.binding.sandbox_verifier_key_record_digest",
        "$.binding.predecessor_receipt_digest",
        "$.binding.sandbox_policy_id",
    ] {
        assert!(
            challenge.contains(path),
            "missing challenge projection {path}"
        );
    }
    let receipt = object_sql(
        &connection,
        "trigger",
        "external_pool_adapter_sandbox_reattestation_receipt_projection",
    );
    for path in [
        "$.reattestation.binding.supported_capabilities",
        "$.reattestation.binding.installation_content_digest",
        "$.reattestation.binding.vulnerability_intelligence_expires_at",
        "$.reattestation.binding.sandbox_verifier_operator",
        "$.reattestation.binding.isolation_profile_id",
        "$.reattestation.binding.test_plan",
        "$.reattestation.binding.observations",
        "$.reattestation.signature_digest",
        "$.reattestation.settlement_effect",
    ] {
        assert!(receipt.contains(path), "missing receipt projection {path}");
    }
    let roots = object_sql(
        &connection,
        "trigger",
        "external_pool_adapter_sandbox_reattestation_receipt_current_roots",
    );
    for root in [
        "compute_external_pool_adapter_registry_release_current",
        "compute_external_pool_adapter_vulnerability_reattestation_current",
        "compute_external_pool_adapter_sandbox_verifier_key_current",
        "supported_capabilities_json=NEW.supported_capabilities_json",
        "dependency_inventory_digest=NEW.dependency_inventory_digest",
    ] {
        assert!(roots.contains(root), "missing exact root fence {root}");
    }
    let time = object_sql(
        &connection,
        "trigger",
        "external_pool_adapter_sandbox_reattestation_receipt_time_bounds",
    );
    assert!(time.contains("NEW.verified_at>=NEW.report_expires_at"));
    assert!(time.contains("NEW.report_expires_at>NEW.vulnerability_intelligence_expires_at"));
    assert!(time.contains("NEW.run_completed_at<NEW.run_started_at"));
    assert!(time.contains("NEW.report_expires_at<=NEW.report_generated_at"));
    assert!(time.contains("substr(NEW.run_started_at,20)"));
}

#[test]
fn source_contract_is_isolated_and_complete() {
    let source = concat!(
        include_str!("tables.sql"),
        include_str!("view.sql"),
        include_str!("guards/immutability.rs"),
        include_str!("guards/lineage.rs"),
        include_str!("guards/projection.rs"),
        include_str!("guards/roots.rs"),
    );
    for forbidden in [
        "compute_external_pool_adapter_sandbox_conformance_reports",
        "compute_external_pool_adapter_vulnerability_reports",
        concat!("TO", "DO"),
        concat!("place", "holder"),
        concat!("renew", "able_", "v1"),
    ] {
        assert!(
            !source.contains(forbidden),
            "V252 source contains forbidden dependency {forbidden}"
        );
    }
    assert!(source.contains("external_pool_adapter_six_capability_offline_sandbox_v1"));
}

#[test]
fn view_is_display_only_and_lineage_allows_terminal_head_recovery() {
    let connection = Connection::open_in_memory().unwrap();
    create_v251_contract_schema(&connection);
    migration_v252(&connection).unwrap();
    let view = object_sql(
        &connection,
        "view",
        "compute_external_pool_adapter_sandbox_reattestation_current",
    );
    for status in [
        "head_status",
        "registry_release_status",
        "vulnerability_reattestation_status",
        "sandbox_verifier_key_status",
        "report_validity_status",
        "revocation_status",
        "verified_current",
        "historical_only",
    ] {
        assert!(view.contains(status), "missing currentness status {status}");
    }
    assert!(view.contains("julianday('now')"));
    assert!(view.contains("julianday(receipt.verified_at)<=julianday('now')"));
    let lineage = object_sql(
        &connection,
        "trigger",
        "external_pool_adapter_sandbox_reattestation_challenge_lineage",
    );
    assert!(lineage.contains("NOT EXISTS"));
    assert!(!lineage.contains("revocation"));
    assert!(!lineage.contains("report_expires_at"));
    assert!(lineage.contains("existing.registry_release_id=NEW.registry_release_id"));
}

fn create_v251_contract_schema(connection: &Connection) {
    connection
        .execute_batch(include_str!("tests/upstream_fixture.sql"))
        .unwrap();
}

fn object_count(connection: &Connection, name: &str) -> i64 {
    connection
        .query_row(
            "SELECT count(*) FROM sqlite_master WHERE name=?1",
            [name],
            |row| row.get(0),
        )
        .unwrap()
}

fn object_sql(connection: &Connection, kind: &str, name: &str) -> String {
    connection
        .query_row(
            "SELECT sql FROM sqlite_master WHERE type=?1 AND name=?2",
            [kind, name],
            |row| row.get(0),
        )
        .unwrap()
}

fn columns(connection: &Connection, table: &str) -> Vec<String> {
    let mut statement = connection
        .prepare(&format!("PRAGMA table_info({table})"))
        .unwrap();
    statement
        .query_map([], |row| row.get(1))
        .unwrap()
        .map(|item| item.unwrap())
        .collect()
}

fn insert_columns(source: &str, table: &str) -> Vec<String> {
    let marker = format!("INSERT INTO {table}(");
    let columns = source
        .split_once(&marker)
        .unwrap_or_else(|| panic!("missing Store INSERT for {table}"))
        .1
        .split_once(") VALUES (")
        .unwrap_or_else(|| panic!("missing Store VALUES for {table}"))
        .0;
    columns
        .split(',')
        .map(str::trim)
        .map(str::to_owned)
        .collect()
}
