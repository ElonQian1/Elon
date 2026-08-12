use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::Connection;

use super::*;

#[path = "tests/support.rs"]
mod support;

use support::*;

#[test]
fn migration_upgrades_v246_and_is_repeatable_after_reopen() {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!(
        "elon-external-pool-installation-terminal-{}-{nonce}.sqlite",
        std::process::id()
    ));
    {
        let connection = Connection::open(&path).unwrap();
        for (version, _, apply) in crate::store_migrations::MIGRATIONS {
            if *version > 246 {
                break;
            }
            apply(&connection).unwrap();
        }
    }
    {
        let connection = Connection::open(&path).unwrap();
        migration_v247(&connection).unwrap();
    }
    {
        let connection = Connection::open(&path).unwrap();
        migration_v247(&connection).unwrap();
        for object in [
            "compute_external_pool_adapter_installation_terminal_receipts",
            "compute_external_pool_adapter_installation_current",
            "external_pool_adapter_installation_terminal_no_update",
            "external_pool_adapter_installation_terminal_no_delete",
            "external_pool_adapter_installation_terminal_no_replace",
            "external_pool_adapter_installation_terminal_exact_root",
            "external_pool_adapter_installation_terminal_json_projection",
        ] {
            assert_eq!(object_count(&connection, object), 1, "missing V247 object");
        }
    }
    std::fs::remove_file(path).unwrap();
}

#[test]
fn terminal_projection_covers_every_signed_field_and_effect() {
    let connection = Connection::open_in_memory().unwrap();
    create_v246_fixture_schema(&connection);
    migration_v247(&connection).unwrap();
    let terminal_columns = object_columns(
        &connection,
        "compute_external_pool_adapter_installation_terminal_receipts",
    );
    assert_eq!(
        terminal_columns,
        [
            "terminal_receipt_id",
            "terminal_receipt_digest",
            "terminal_receipt_schema",
            "receipt_json",
            "terminal_material_digest",
            "canonicalization",
            "digest_algorithm",
            "installation_receipt_id",
            "installation_receipt_digest",
            "terminal_kind",
            "revoked_by_admin_user_id",
            "reason",
            "confirmation",
            "idempotency_scope",
            "idempotency_key",
            "revoked_at",
            "recorded_at",
            "installation_effect",
            "credential_effect",
            "provider_effect",
            "route_effect",
            "execution_effect",
            "settlement_effect",
        ]
    );
    let projection = object_sql(
        &connection,
        "trigger",
        "external_pool_adapter_installation_terminal_json_projection",
    );
    let signed_fields = [
        "$.schema",
        "$.terminal_receipt_id",
        "$.terminal_receipt_digest",
        "$.terminal_material_digest",
        "$.canonicalization",
        "$.digest_algorithm",
        "$.terminal.installation_receipt_id",
        "$.terminal.installation_receipt_digest",
        "$.terminal.terminal_kind",
        "$.terminal.revoked_by_admin_user_id",
        "$.terminal.reason",
        "$.terminal.confirmation",
        "$.terminal.idempotency_scope",
        "$.terminal.idempotency_key",
        "$.terminal.revoked_at",
        "$.terminal.recorded_at",
        "$.terminal.installation_effect",
        "$.terminal.credential_effect",
        "$.terminal.provider_effect",
        "$.terminal.route_effect",
        "$.terminal.execution_effect",
        "$.terminal.settlement_effect",
    ];
    assert_eq!(signed_fields.len(), 22);
    for signed in signed_fields {
        assert!(projection.contains(signed), "missing signed path {signed}");
    }

    let no_replace = object_sql(
        &connection,
        "trigger",
        "external_pool_adapter_installation_terminal_no_replace",
    );
    for fence in [
        "old.terminal_receipt_id=NEW.terminal_receipt_id",
        "old.terminal_receipt_digest=NEW.terminal_receipt_digest",
        "old.installation_receipt_id=NEW.installation_receipt_id",
        "old.idempotency_scope=NEW.idempotency_scope",
        "old.idempotency_key=NEW.idempotency_key",
    ] {
        assert!(
            no_replace.contains(fence),
            "missing no-replace fence {fence}"
        );
    }

    let exact_root = object_sql(
        &connection,
        "trigger",
        "external_pool_adapter_installation_terminal_exact_root",
    );
    for fence in [
        "installation.installation_receipt_id=NEW.installation_receipt_id",
        "installation.installation_receipt_digest=NEW.installation_receipt_digest",
        "installation.installed_at<=NEW.revoked_at",
    ] {
        assert!(
            exact_root.contains(fence),
            "missing exact-root fence {fence}"
        );
    }

    let view = object_sql(
        &connection,
        "view",
        "compute_external_pool_adapter_installation_current",
    );
    for currentness in [
        "terminal.terminal_receipt_id IS NULL",
        "installed_upstreams_current",
        "historical_only",
        "terminal_status",
        "revoked",
    ] {
        assert!(
            view.contains(currentness),
            "missing terminal currentness fence {currentness}"
        );
    }
    assert_eq!(
        object_columns(
            &connection,
            "compute_external_pool_adapter_installation_current"
        ),
        [
            "installation_receipt_id",
            "installation_receipt_digest",
            "current_status",
            "adoption_status",
            "package_status",
            "source_status",
            "file_inventory_status",
            "terminal_status",
        ]
    );
}

#[test]
fn exact_terminal_is_immutable_and_downgrades_currentness() {
    let connection = Connection::open_in_memory().unwrap();
    create_v246_fixture_schema(&connection);
    migration_v247(&connection).unwrap();
    seed_current_installation(&connection);

    let before: (String, String) = connection
        .query_row(
            "SELECT current_status,terminal_status
               FROM compute_external_pool_adapter_installation_current
              WHERE installation_receipt_id='installation-1'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(
        before,
        ("installed_upstreams_current".into(), "none".into())
    );

    let root_digest = digest('a');
    let wrong_digest = digest('b');
    assert!(insert_terminal(
        &connection,
        "terminal-wrong-root",
        &digest('c'),
        &wrong_digest,
        &wrong_digest,
        "wrong-root",
        REVOKED_AT,
        "revoked",
    )
    .is_err());
    assert!(insert_terminal(
        &connection,
        "terminal-wrong-projection",
        &digest('d'),
        &root_digest,
        &root_digest,
        "wrong-projection",
        REVOKED_AT,
        "tampered",
    )
    .is_err());
    assert!(insert_terminal(
        &connection,
        "terminal-before-install",
        &digest('e'),
        &root_digest,
        &root_digest,
        "before-install",
        "2026-08-12T23:59:59.999999999Z",
        "revoked",
    )
    .is_err());
    insert_terminal(
        &connection,
        "terminal-1",
        &digest('f'),
        &root_digest,
        &root_digest,
        "revoke-installation",
        REVOKED_AT,
        "revoked",
    )
    .unwrap();

    let after: (String, String) = connection
        .query_row(
            "SELECT current_status,terminal_status
               FROM compute_external_pool_adapter_installation_current
              WHERE installation_receipt_id='installation-1'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(after, ("historical_only".into(), "revoked".into()));
    assert!(connection
        .execute(
            "UPDATE compute_external_pool_adapter_installation_terminal_receipts
                SET reason='changed' WHERE terminal_receipt_id='terminal-1'",
            [],
        )
        .is_err());
    assert!(connection
        .execute(
            "DELETE FROM compute_external_pool_adapter_installation_terminal_receipts
                  WHERE terminal_receipt_id='terminal-1'",
            [],
        )
        .is_err());
    assert!(insert_terminal(
        &connection,
        "terminal-2",
        &digest('9'),
        &root_digest,
        &root_digest,
        "another-key",
        REVOKED_AT,
        "revoked",
    )
    .is_err());
}
