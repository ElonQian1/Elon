const STORE_ROOT: &str = include_str!("../../store.rs");
const STORE_SCHEMA: &str = include_str!("../../store_schema.rs");
const STORE_MIGRATIONS: &str = include_str!("../../store_migrations.rs");
const STORE_WRITE: &str =
    include_str!("../../store/compute_external_pool_adapter_task_protocol_conformance/write.rs");
const STORE_WRITE_REPLAY: &str = include_str!(
    "../../store/compute_external_pool_adapter_task_protocol_conformance/write/replay.rs"
);
const STORE_CURRENT: &str =
    include_str!("../../store/compute_external_pool_adapter_task_protocol_conformance/current.rs");
const STORE_TYPES: &str =
    include_str!("../../store/compute_external_pool_adapter_task_protocol_conformance/types.rs");
const STORE_PERSISTENCE: &str = include_str!(
    "../../store/compute_external_pool_adapter_task_protocol_conformance/persistence.rs"
);
const STORE_AUDIT: &str =
    include_str!("../../store/compute_external_pool_adapter_task_protocol_conformance/audit.rs");
const STORE_ROOTS: &str =
    include_str!("../../store/compute_external_pool_adapter_task_protocol_conformance/roots.rs");
const STORE_PROJECTION: &str = include_str!(
    "../../store/compute_external_pool_adapter_task_protocol_conformance/roots/projection.rs"
);
const TASK_CUSTODY: &str = include_str!(
    "../../store/compute_external_pool_adapter_runtime_bundle/runtime/custody/task_protocol_conformance.rs"
);
const MIGRATION_ROOT: &str = include_str!(
    "../../store_migrations/compute_external_pool_adapter_task_protocol_conformance.rs"
);
const TABLES_INSTALLER: &str = include_str!(
    "../../store_migrations/compute_external_pool_adapter_task_protocol_conformance/tables.rs"
);
const VIEW_INSTALLER: &str = include_str!(
    "../../store_migrations/compute_external_pool_adapter_task_protocol_conformance/view.rs"
);
const MIGRATION_GUARDS: &str = include_str!(
    "../../store_migrations/compute_external_pool_adapter_task_protocol_conformance/guards.rs"
);
const RECEIPT_INTEGRITY: &str = include_str!(
    "../../store_migrations/compute_external_pool_adapter_task_protocol_conformance/receipt_integrity.rs"
);
const RUN_TABLE: &str = include_str!(
    "../../store_migrations/compute_external_pool_adapter_task_protocol_conformance/tables/run_receipts.sql"
);
const REVOCATION_TABLE: &str = include_str!(
    "../../store_migrations/compute_external_pool_adapter_task_protocol_conformance/tables/revocations.sql"
);
const INDEXES: &str = include_str!(
    "../../store_migrations/compute_external_pool_adapter_task_protocol_conformance/tables/indexes.sql"
);
const VIEW: &str = include_str!(
    "../../store_migrations/compute_external_pool_adapter_task_protocol_conformance/view.sql"
);
const IMMUTABILITY: &str = include_str!(
    "../../store_migrations/compute_external_pool_adapter_task_protocol_conformance/guards/immutability.sql"
);
const NO_REPLACE: &str = include_str!(
    "../../store_migrations/compute_external_pool_adapter_task_protocol_conformance/guards/no_replace.sql"
);
const PROJECTION: &str = include_str!(
    "../../store_migrations/compute_external_pool_adapter_task_protocol_conformance/guards/projection.sql"
);
const LINEAGE: &str = include_str!(
    "../../store_migrations/compute_external_pool_adapter_task_protocol_conformance/guards/lineage.sql"
);
const ROOT_GUARDS: &str = include_str!(
    "../../store_migrations/compute_external_pool_adapter_task_protocol_conformance/guards/roots.rs"
);
const ROOT_RELEASE_SECURITY: &str = include_str!(
    "../../store_migrations/compute_external_pool_adapter_task_protocol_conformance/guards/roots/release_security.sql"
);
const ROOT_RUNTIME: &str = include_str!(
    "../../store_migrations/compute_external_pool_adapter_task_protocol_conformance/guards/roots/runtime_compatibility.sql"
);
const EFFECTS_CHECK: &str = "effects_json TEXT NOT NULL CHECK(effects_json='{\"activation_effect\":\"none\",\"adapter_effect\":\"none\",\"credential_effect\":\"none\",\"execution_effect\":\"none\",\"market_effect\":\"none\",\"provider_effect\":\"none\",\"route_effect\":\"none\",\"settlement_effect\":\"none\",\"usage_effect\":\"none\"}')";
const READINESS_CHECK: &str = "readiness_json TEXT NOT NULL CHECK(readiness_json='{\"activation_ready\":false,\"broker_connect_ready\":false,\"execution_ready\":false,\"ipc_session_ready\":false,\"process_spawn_ready\":false,\"route_ready\":false,\"runtime_launch_ready\":false,\"secret_delivery_ready\":false,\"upstream_probe_ready\":false}')";

#[test]
fn task_protocol_conformance_store_freezes_ordered_schema_persistence_and_audit() {
    let run_columns = ddl_columns(RUN_TABLE);
    let revocation_columns = ddl_columns(REVOCATION_TABLE);
    assert_eq!(run_columns.len(), 105);
    assert_eq!(revocation_columns.len(), 21);
    assert_eq!(
        run_columns,
        insert_columns(
            STORE_PERSISTENCE,
            "compute_external_pool_adapter_task_protocol_conformance_run_receipts"
        )
    );
    assert_eq!(
        revocation_columns,
        insert_columns(
            STORE_PERSISTENCE,
            "compute_external_pool_adapter_task_protocol_conformance_revocations"
        )
    );
    for column in run_columns.iter().chain(&revocation_columns) {
        assert!(STORE_AUDIT.contains(*column), "audit lost {column}");
    }
    assert_eq!(RUN_TABLE.matches("CREATE TABLE IF NOT EXISTS").count(), 1);
    assert_eq!(
        REVOCATION_TABLE
            .matches("CREATE TABLE IF NOT EXISTS")
            .count(),
        1
    );
    assert_eq!(VIEW.matches("CREATE VIEW").count(), 1);
    let migration_sql = format!(
        "{RUN_TABLE}{REVOCATION_TABLE}{INDEXES}{VIEW}{IMMUTABILITY}{NO_REPLACE}{PROJECTION}{LINEAGE}{ROOT_RELEASE_SECURITY}{ROOT_RUNTIME}"
    );
    assert_eq!(migration_sql.matches("CREATE TABLE").count(), 2);
    assert_eq!(migration_sql.matches("CREATE VIEW").count(), 1);
    assert_eq!(
        TABLES_INSTALLER.matches("include_str!(\"tables/").count(),
        3
    );
    for source in [
        "tables/run_receipts.sql",
        "tables/revocations.sql",
        "tables/indexes.sql",
    ] {
        assert_eq!(
            TABLES_INSTALLER.matches(source).count(),
            1,
            "table installer drifted for {source}"
        );
    }
    assert_eq!(
        VIEW_INSTALLER.matches("include_str!(\"view.sql\")").count(),
        1
    );
    assert_eq!(VIEW_INSTALLER.matches("include_str!(").count(), 1);
    assert!(RUN_TABLE.contains("runtime_compatibility_public_fixture_delivery_root"));
    assert!(RUN_TABLE.contains("public_fixture_delivery_root"));
    assert!(STORE_PERSISTENCE.contains(":runtime_public_delivery_root"));
    assert!(STORE_PERSISTENCE.contains(":public_delivery_root"));
    for table in [RUN_TABLE, REVOCATION_TABLE] {
        assert_eq!(table.matches(EFFECTS_CHECK).count(), 1);
        assert_eq!(table.matches(READINESS_CHECK).count(), 1);
    }
}

#[test]
fn task_protocol_conformance_store_freezes_guards_and_per_connection_udfs() {
    assert_ordered(
        MIGRATION_GUARDS,
        &[
            "guards/immutability.sql",
            "guards/no_replace.sql",
            "receipt_integrity::install(conn)",
            "guards/projection.sql",
            "guards/lineage.sql",
            "roots::install(conn)",
        ],
    );
    assert_eq!(
        IMMUTABILITY.matches("CREATE TRIGGER IF NOT EXISTS").count(),
        4
    );
    assert_eq!(
        NO_REPLACE.matches("CREATE TRIGGER IF NOT EXISTS").count(),
        2
    );
    assert_eq!(
        PROJECTION.matches("CREATE TRIGGER IF NOT EXISTS").count(),
        2
    );
    assert_eq!(LINEAGE.matches("CREATE TRIGGER IF NOT EXISTS").count(), 4);
    assert_eq!(
        RECEIPT_INTEGRITY
            .matches("conn.create_scalar_function")
            .count(),
        3
    );
    for udf in [
        "elon_v272_task_protocol_conformance_run_receipt_is_exact",
        "elon_v272_task_protocol_conformance_revocation_receipt_is_exact",
        "elon_v272_task_protocol_conformance_receipt_integrity_is_exact",
    ] {
        assert!(RECEIPT_INTEGRITY.contains(udf), "UDF lost {udf}");
    }
    assert!(MIGRATION_ROOT.contains("register_receipt_integrity_functions(conn)?"));
    assert!(STORE_MIGRATIONS.contains("register_v272_receipt_integrity_functions"));
    assert_ordered(
        STORE_SCHEMA,
        &[
            "register_v270_receipt_integrity_functions(conn)?",
            "register_v272_receipt_integrity_functions(conn)?",
            "CREATE TABLE IF NOT EXISTS schema_migrations",
        ],
    );
    assert!(STORE_ROOT.contains("mod compute_external_pool_adapter_task_protocol_conformance;"));
    assert!(STORE_ROOT.contains(
        "pub(crate) use compute_external_pool_adapter_task_protocol_conformance::api::*;"
    ));
}

#[test]
fn task_protocol_conformance_store_freezes_two_reopens_transactions_and_seals() {
    assert_eq!(STORE_WRITE.matches("reopen_prepared()").count(), 2);
    assert_eq!(
        STORE_WRITE
            .matches("transaction_with_behavior(TransactionBehavior::Immediate)")
            .count(),
        2
    );
    assert_ordered(
        STORE_WRITE,
        &[
            "let preflight_prepared = reopen_prepared()",
            "TransactionBehavior::Immediate",
            "current_roots_for_create_on(&tx, &input, preflight_prepared",
            "tx.commit()",
            "execute_external_pool_adapter_task_protocol_conformance(execution_input, runtime)",
            "let final_prepared = reopen_prepared()",
            "TransactionBehavior::Immediate",
            "current_roots_for_create_on(&tx, &input, final_prepared",
            "remember_pending_task_protocol_conformance_seal(",
            "insert_run(",
            "ensure_fresh_readback(",
            "tx.commit()",
            "promote_task_protocol_conformance_seal(",
        ],
    );
    let preflight_replay = source_block(
        STORE_WRITE,
        "let preflight_prepared = reopen_prepared()",
        "let evidence =",
    );
    assert_ordered(
        preflight_replay,
        &[
            "run_by_idempotency_on(",
            "current_roots_for_create_on(&tx, &input, preflight_prepared",
            "tx.commit()",
            "promote_exact_pending_replay(",
        ],
    );
    assert!(!STORE_WRITE_REPLAY.contains("remember_pending_task_protocol_conformance_seal"));
    assert!(!STORE_WRITE_REPLAY.contains("seal_task_protocol_conformance"));
    assert!(TASK_CUSTODY.contains("committed: false"));
    assert!(TASK_CUSTODY.contains("seal.committed = true"));
    assert!(TASK_CUSTODY.contains("seal.committed"));
}

#[test]
fn task_protocol_conformance_store_freezes_current_authority_and_private_carrier() {
    assert!(STORE_CURRENT.contains("current_roots_for_receipt_on("));
    assert!(STORE_CURRENT.contains("attests_task_protocol_conformance_seal("));
    assert!(STORE_CURRENT.contains("PreparedExternalPoolAdapterInstallation"));
    assert!(STORE_PROJECTION.contains("checked_add_signed("));
    assert!(VIEW.contains("relationally_current_requires_process_custody_and_prepared_reproof"));
    assert!(VIEW.contains("requires_same_process_committed_seal_reproof"));
    assert!(VIEW.contains("requires_fresh_prepared_execution_carrier_reproof"));
    let authority = source_block(
        STORE_TYPES,
        "/// Same-transaction authority.",
        "impl<'tx, 'conn> CurrentExternalPoolAdapterTaskProtocolConformanceAuthority",
    );
    for marker in [
        "CurrentExternalPoolAdapterTaskProtocolConformanceAuthority<'tx, 'conn>",
        "carrier: CurrentExternalPoolAdapterRegistryProviderBindingAuthority",
        "vulnerability: CurrentExternalPoolAdapterVulnerabilityReattestationAuthority",
        "sandbox: CurrentExternalPoolAdapterSandboxReattestationAuthority",
        "runtime_compatibility:",
        "CurrentExternalPoolAdapterRuntimeCompatibilityVerificationAuthority<",
        "PhantomData<&'tx Transaction<'conn>>",
    ] {
        assert!(
            authority.contains(marker),
            "private authority lost {marker}"
        );
    }
    assert!(!authority.contains("#[derive"));
    for forbidden in [
        "impl Clone for CurrentExternalPoolAdapterTaskProtocolConformanceAuthority",
        "impl Debug for CurrentExternalPoolAdapterTaskProtocolConformanceAuthority",
        "Serialize for CurrentExternalPoolAdapterTaskProtocolConformanceAuthority",
        "Deserialize for CurrentExternalPoolAdapterTaskProtocolConformanceAuthority",
    ] {
        assert!(
            !STORE_TYPES.contains(forbidden),
            "private authority gained {forbidden}"
        );
    }
    for private in [
        "provider_binding_id",
        "provider_binding_digest",
        "installation_receipt_id",
        "installation_receipt_digest",
    ] {
        let durable = format!("{RUN_TABLE}{REVOCATION_TABLE}{VIEW}{TASK_CUSTODY}");
        assert!(
            !durable.contains(private),
            "durable evidence gained carrier {private}"
        );
    }
    for required in [
        "current_vulnerability.current_status='verified_current'",
        "current_sandbox.current_status='verified_current'",
        "current_verification.currentness_status='current_signed_verifier_assertion'",
        "sandbox_verifier_operator",
        "sandbox_verifier_product",
        "source_capsule_size_bytes",
        "launch_image_size_bytes",
    ] {
        let roots = format!("{ROOT_GUARDS}{ROOT_RELEASE_SECURITY}{ROOT_RUNTIME}");
        assert!(roots.contains(required), "root guard lost {required}");
    }

    let expiry = source_block(
        STORE_PROJECTION,
        "pub(in super::super) fn task_protocol_conformance_expires_at(",
        "pub(in super::super) fn canonical_time(",
    );
    assert_ordered(
        expiry,
        &[
            "Duration::seconds(TASK_PROTOCOL_CONFORMANCE_EXPIRY_SECONDS)",
            "roots.vulnerability_reattestation.intelligence_expires_at",
            "roots.sandbox_reattestation.report_expires_at",
            "roots.runtime_compatibility.expires_at",
            ".min()",
        ],
    );
    assert_ordered(
        ROOT_RUNTIME,
        &[
            "AND NEW.expires_at=min(",
            "NEW.post_cleanup_checked_at,'+15 seconds'",
            "SELECT upstream.intelligence_expires_at",
            "upstream.reattestation_receipt_id=NEW.vulnerability_reattestation_receipt_id",
            "SELECT upstream.report_expires_at",
            "upstream.reattestation_receipt_id=NEW.sandbox_reattestation_receipt_id",
            "verification.expires_at)",
        ],
    );

    let seal = source_block(
        TASK_CUSTODY,
        "pub(in crate::store) fn seal_task_protocol_conformance(",
        "/// Remembers an exact pending tuple.",
    );
    assert!(TASK_CUSTODY
        .contains("ELON-EXTERNAL-POOL-ADAPTER-TASK-PROTOCOL-CONFORMANCE-PROCESS-SEAL-V1"));
    assert_ordered(
        seal,
        &[
            "input.run_receipt_digest.as_bytes()",
            "input.task_observation_root.as_bytes()",
            "input.session_roots_digest.as_bytes()",
            "input.session_transcript_digest.as_bytes()",
            "input.delivery_inventory_digest.as_bytes()",
            "input.exchange_inventory_digest.as_bytes()",
            "input.post_cleanup_checked_at.as_bytes()",
            "input.expires_at.as_bytes()",
            "self.custody_epoch_digest().as_bytes()",
        ],
    );
}

fn ddl_columns(source: &str) -> Vec<&str> {
    source
        .lines()
        .map(str::trim)
        .skip(1)
        .take_while(|line| *line != ");")
        .filter(|line| {
            !line.is_empty()
                && !line.starts_with("CHECK(")
                && !line.starts_with("UNIQUE(")
                && !line.starts_with("FOREIGN KEY(")
        })
        .map(|line| line.split_whitespace().next().unwrap())
        .collect()
}

fn insert_columns<'a>(source: &'a str, table: &str) -> Vec<&'a str> {
    let marker = format!("INSERT INTO {table}(");
    source
        .split_once(marker.as_str())
        .unwrap()
        .1
        .split_once(") VALUES (")
        .unwrap()
        .0
        .split(',')
        .map(str::trim)
        .collect()
}

fn source_block<'a>(source: &'a str, start: &str, end: &str) -> &'a str {
    source
        .split_once(start)
        .unwrap()
        .1
        .split_once(end)
        .unwrap()
        .0
}

fn assert_ordered(source: &str, needles: &[&str]) {
    let mut cursor = 0;
    for needle in needles {
        let offset = source[cursor..]
            .find(needle)
            .unwrap_or_else(|| panic!("missing ordered source marker {needle}"));
        cursor += offset + needle.len();
    }
}
