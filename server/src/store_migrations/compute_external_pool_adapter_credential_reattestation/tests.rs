use rusqlite::Connection;

use super::*;

#[test]
fn migration_registers_complete_append_only_v253_authority() {
    let connection = Connection::open_in_memory().unwrap();
    create_upstream_contract_schema(&connection);
    migration_v253(&connection).unwrap();
    migration_v253(&connection).unwrap();
    for object in [
        "compute_external_pool_adapter_credential_reattestation_challenges",
        "compute_external_pool_adapter_credential_reattestation_receipts",
        "compute_external_pool_adapter_credential_reattestation_revocations",
        "compute_external_pool_adapter_credential_reattestation_current",
        "external_pool_adapter_credential_reattestation_challenge_exact_roots",
        "external_pool_adapter_credential_reattestation_receipt_exact_challenge",
        "external_pool_adapter_credential_reattestation_receipt_historical_roots",
        "external_pool_adapter_credential_reattestation_receipt_current_roots",
        "external_pool_adapter_credential_reattestation_challenge_lineage",
        "external_pool_adapter_credential_reattestation_receipt_lineage",
        "external_pool_adapter_credential_reattestation_revocation_time_order",
    ] {
        assert_eq!(
            object_count(&connection, object),
            1,
            "missing V253 object {object}"
        );
    }
}

#[test]
fn store_insert_columns_match_migration_columns_in_exact_order() {
    let connection = Connection::open_in_memory().unwrap();
    create_upstream_contract_schema(&connection);
    migration_v253(&connection).unwrap();
    let source = include_str!(
        "../../store/compute_external_pool_adapter_credential_reattestation/persistence.rs"
    );
    for (table, expected) in [
        (
            "compute_external_pool_adapter_credential_reattestation_challenges",
            23,
        ),
        (
            "compute_external_pool_adapter_credential_reattestation_receipts",
            72,
        ),
        (
            "compute_external_pool_adapter_credential_reattestation_revocations",
            22,
        ),
    ] {
        let migration = columns(&connection, table);
        assert_eq!(migration.len(), expected);
        assert_eq!(migration, insert_columns(source, table));
    }
}

#[test]
fn canonical_projection_counts_are_frozen_and_complete() {
    assert_eq!(guards::projection_counts(), (29, 77, 24));

    // The receipt keeps 72 relational columns. Challenge issue/expiry timestamps
    // remain inside the canonical binding and are fenced by exact JSON equality
    // against the durable challenge instead of duplicate receipt columns.
    let roots = include_str!("guards/roots.rs");
    assert!(roots.contains("json(json_extract(challenge.challenge_json,'$.binding'))="));
    assert!(roots.contains("json(json_extract(NEW.receipt_json,'$.reattestation.binding'))"));
}

#[test]
fn source_contract_is_isolated_fail_closed_and_provider_aware() {
    let source = concat!(
        include_str!("tables.sql"),
        include_str!("view.sql"),
        include_str!("guards/immutability.rs"),
        include_str!("guards/lineage.rs"),
        include_str!("guards/projection.rs"),
        include_str!("guards/roots.rs"),
    );
    for required in [
        "challenge_id TEXT NOT NULL UNIQUE", "UNIQUE(provider_binding_id,sequence)",
        "UNIQUE(predecessor_receipt_id)", "provider.current_policy_revision>binding.provider_policy_revision",
        "provider.current_policy_revision=head.observed_provider_policy_revision+1",
        "provider.current_provider_digest=head.observed_provider_digest",
        "compute_external_pool_adapter_installation_terminal_receipts",
        "compute_external_pool_adapter_adoption_terminal_receipts",
        "compute_external_pool_adapter_credential_verifier_key_current",
        "release_root.registry_release_material_digest=NEW.registry_release_material_digest",
        "release_root.credential_verifier_digest=verifier.verifier_digest",
        "json_extract(release_root.credential_verifier_json,'$.verifier_revision')=verifier.verifier_revision",
        "json(json_extract(challenge.challenge_json,'$.binding'))",
        "json_extract(NEW.challenge_json,'$.binding.credential_resolution_outcome') IS NOT 'passed'",
        "$.signature_algorithm",
        "json_type(NEW.challenge_json,'$.binding.verifier_report_id') IS NOT 'text'",
        "json_type(NEW.challenge_json,'$.binding.provider_response_evidence_digest') IS NOT 'text'",
        "json_extract(NEW.challenge_json,'$.binding.credential_locator_commitment')=binding.credential_locator_commitment",
        "credential_reattestation_effect='signed_provider_credential_reattestation_verified_current'",
        "usage_effect TEXT NOT NULL CHECK(usage_effect='none')",
    ] {
        assert!(source.contains(required), "missing V253 contract fence {required}");
    }
    for forbidden in [
        "compute_external_pool_adapter_registry_provider_binding_current",
        "compute_external_pool_adapter_vulnerability_reattestation",
        "compute_external_pool_adapter_sandbox_reattestation",
        "compute_route_adapters",
        "compute_federation_activation",
    ] {
        assert!(
            !source.contains(forbidden),
            "forbidden V253 dependency {forbidden}"
        );
    }
}

#[test]
fn view_uses_display_only_two_layer_provider_rules() {
    let connection = Connection::open_in_memory().unwrap();
    create_upstream_contract_schema(&connection);
    migration_v253(&connection).unwrap();
    let view = object_sql(
        &connection,
        "view",
        "compute_external_pool_adapter_credential_reattestation_current",
    );
    for status in [
        "binding_exact",
        "historical_exact",
        "release_current",
        "historical_only",
        "subject_exact",
        "drifted",
        "exact_registering",
        "adjacent_active",
        "exact_active",
        "verified_current",
        "current",
        "expired",
        "active",
        "revoked",
    ] {
        assert!(
            view.contains(status),
            "missing V253 display status {status}"
        );
    }
    for field in [
        "$.created_at",
        "$.adapter.adapter_id",
        "$.adapter.adapter_version",
        "$.adapter.config_revision",
        "$.adapter.config_digest",
    ] {
        assert!(
            view.contains(field),
            "missing stable Provider subject field {field}"
        );
    }
    assert!(view.contains("julianday('now')"));
}

fn create_upstream_contract_schema(connection: &Connection) {
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
    source
        .split_once(&marker)
        .unwrap_or_else(|| panic!("missing Store INSERT for {table}"))
        .1
        .split_once(") VALUES (")
        .unwrap_or_else(|| panic!("missing Store VALUES for {table}"))
        .0
        .split(',')
        .map(str::trim)
        .map(str::to_owned)
        .collect()
}
