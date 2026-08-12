use rusqlite::Connection;

use super::*;

#[test]
fn migration_is_repeatable_and_defines_exact_append_only_authority() {
    let connection = Connection::open_in_memory().unwrap();
    crate::store_schema::apply_migrations(&connection).unwrap();
    migration_v246(&connection).unwrap();
    migration_v246(&connection).unwrap();

    for object in [
        "compute_external_pool_adapter_installation_receipts",
        "compute_external_pool_adapter_installation_files",
        "compute_external_pool_adapter_installation_current",
        "external_pool_adapter_installation_no_update",
        "external_pool_adapter_installation_no_delete",
        "external_pool_adapter_installation_no_replace",
        "external_pool_adapter_installation_exact_roots",
        "external_pool_adapter_installation_exact_files",
        "external_pool_adapter_installation_json_projection",
    ] {
        let count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE name=?1",
                [object],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1, "missing V246 object {object}");
    }
}

#[test]
fn migration_projection_covers_every_signed_root_and_inert_effect() {
    let connection = Connection::open_in_memory().unwrap();
    crate::store_schema::apply_migrations(&connection).unwrap();
    let projection = object_sql(
        &connection,
        "trigger",
        "external_pool_adapter_installation_json_projection",
    );
    for signed in [
        "$.installation.binding.adoption_receipt_digest",
        "$.installation.binding.package_receipt_digest",
        "$.installation.binding.source_receipt_digest",
        "$.installation.binding.archive_sha256",
        "$.installation.binding.manifest_digest",
        "$.installation.binding.entry_inventory_digest",
        "$.installation.binding.installation_content_digest",
        "$.installation.binding.credential_locator_commitment",
        "$.installation.installation_effect",
        "$.installation.credential_effect",
        "$.installation.provider_effect",
        "$.installation.route_effect",
        "$.installation.execution_effect",
        "$.installation.settlement_effect",
    ] {
        assert!(projection.contains(signed), "missing signed path {signed}");
    }
    let roots = object_sql(
        &connection,
        "trigger",
        "external_pool_adapter_installation_exact_roots",
    );
    for root in [
        "compute_external_pool_adapter_adoption_receipts",
        "compute_external_pool_adapter_artifact_package_receipts",
        "compute_external_pool_adapter_artifact_source_receipts",
    ] {
        assert!(roots.contains(root), "missing exact source root {root}");
    }
    let view = object_sql(
        &connection,
        "view",
        "compute_external_pool_adapter_installation_current",
    );
    for authority in [
        "compute_external_pool_adapter_adoption_current",
        "compute_external_pool_adapter_artifact_package_current",
        "compute_external_pool_adapter_artifact_source_receipts",
        "compute_external_pool_adapter_installation_files",
        "installed_upstreams_current",
        "historical_only",
    ] {
        assert!(
            view.contains(authority),
            "missing currentness root {authority}"
        );
    }
}

#[test]
fn migration_prevents_replace_and_inventory_mutation() {
    let connection = Connection::open_in_memory().unwrap();
    crate::store_schema::apply_migrations(&connection).unwrap();
    let no_replace = object_sql(
        &connection,
        "trigger",
        "external_pool_adapter_installation_no_replace",
    );
    assert!(no_replace.contains("old.adoption_receipt_id=NEW.adoption_receipt_id"));
    assert!(no_replace.contains("old.idempotency_scope=NEW.idempotency_scope"));
    assert!(no_replace.contains("old.idempotency_key=NEW.idempotency_key"));
    let files = object_sql(
        &connection,
        "trigger",
        "external_pool_adapter_installation_files_no_replace",
    );
    assert!(files.contains("compute_external_pool_adapter_installation_receipts sealed"));
    assert!(files.contains("old.ordinal=NEW.ordinal"));
    assert!(files.contains("old.path=NEW.path"));
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
