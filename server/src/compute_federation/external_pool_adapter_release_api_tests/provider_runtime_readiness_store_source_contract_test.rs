use sha2::{Digest, Sha256};

const PERSISTENCE: &str = include_str!(
    "../../store/compute_external_pool_adapter_provider_runtime_readiness/persistence.rs"
);
const READBACK: &str =
    include_str!("../../store/compute_external_pool_adapter_provider_runtime_readiness/read.rs");
const AUDIT: &str =
    include_str!("../../store/compute_external_pool_adapter_provider_runtime_readiness/audit.rs");
const STORE_ROOTS: &str =
    include_str!("../../store/compute_external_pool_adapter_provider_runtime_readiness/roots.rs");
const STORE_FILE: &str = include_str!("../../store.rs");
const STORE_SCHEMA: &str = include_str!("../../store_schema.rs");
const STORE_MIGRATIONS: &str = include_str!("../../store_migrations.rs");
const MIGRATION_V270: &str = include_str!(
    "../../store_migrations/compute_external_pool_adapter_provider_runtime_readiness.rs"
);
const RECEIPT_INTEGRITY: &str = include_str!(
    "../../store_migrations/compute_external_pool_adapter_provider_runtime_readiness/receipt_integrity.rs"
);
const GUARDS_INSTALL: &str = include_str!(
    "../../store_migrations/compute_external_pool_adapter_provider_runtime_readiness/guards.rs"
);
const RECEIPTS_SQL: &str = include_str!(
    "../../store_migrations/compute_external_pool_adapter_provider_runtime_readiness/tables/receipts.sql"
);
const REVOCATIONS_SQL: &str = include_str!(
    "../../store_migrations/compute_external_pool_adapter_provider_runtime_readiness/tables/revocations.sql"
);
const VIEW_SQL: &str = include_str!(
    "../../store_migrations/compute_external_pool_adapter_provider_runtime_readiness/view.sql"
);
const NO_REPLACE: &str = include_str!(
    "../../store_migrations/compute_external_pool_adapter_provider_runtime_readiness/guards/no_replace.sql"
);
const IMMUTABILITY: &str = include_str!(
    "../../store_migrations/compute_external_pool_adapter_provider_runtime_readiness/guards/immutability.sql"
);
const PROJECTION: &str = include_str!(
    "../../store_migrations/compute_external_pool_adapter_provider_runtime_readiness/guards/projection.sql"
);
const LINEAGE: &str = include_str!(
    "../../store_migrations/compute_external_pool_adapter_provider_runtime_readiness/guards/lineage.sql"
);
const MIGRATION_ROOTS: &str = include_str!(
    "../../store_migrations/compute_external_pool_adapter_provider_runtime_readiness/guards/roots.rs"
);
const V254_FENCES: &str = include_str!(
    "../../store_migrations/compute_external_pool_provider_activation_candidate/guards/fences.rs"
);

#[test]
fn provider_runtime_readiness_store_source_freezes_ordered_persistence_and_readback() {
    let receipt_columns = ddl_columns(RECEIPTS_SQL);
    let revocation_columns = ddl_columns(REVOCATIONS_SQL);
    assert_eq!(receipt_columns.len(), 71);
    assert_eq!(revocation_columns.len(), 31);
    assert_eq!(
        receipt_columns,
        insert_columns(
            PERSISTENCE,
            "compute_external_pool_adapter_provider_runtime_readiness_receipts"
        )
    );
    assert_eq!(
        revocation_columns,
        insert_columns(
            PERSISTENCE,
            "compute_external_pool_adapter_provider_runtime_readiness_revocations"
        )
    );
    for column in receipt_columns.iter().chain(&revocation_columns) {
        assert!(AUDIT.contains(*column), "readback audit lost {column}");
    }
    for required in [
        "SELECT readiness_receipt_json",
        "SELECT revocation_receipt_json",
        "validate_provider_runtime_readiness_receipt",
        "validate_provider_runtime_readiness_revocation_receipt",
        "canonical_provider_runtime_readiness_receipt_json_and_digest",
        "canonical_provider_runtime_readiness_revocation_json_and_digest",
        "audit_readiness_projection(conn",
        "audit_revocation_projection(conn",
    ] {
        assert!(READBACK.contains(required), "readback lost {required}");
    }
    assert_eq!(AUDIT.matches("SELECT EXISTS(SELECT 1").count(), 2);
    assert!(!READBACK.contains("SELECT *"));
}

#[test]
fn provider_runtime_readiness_store_source_freezes_guards_and_per_connection_udfs() {
    assert_ordered(
        GUARDS_INSTALL,
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
    for required in [
        "BEFORE UPDATE ON compute_external_pool_adapter_provider_runtime_readiness_receipts",
        "BEFORE DELETE ON compute_external_pool_adapter_provider_runtime_readiness_revocations",
        "V270 readiness receipt replacement is forbidden",
        "V270 readiness revocation replacement is forbidden",
        "json_extract(NEW.readiness_receipt_json",
        "json_extract(NEW.revocation_receipt_json",
        "exact structural binding head",
        "exact unrevoked structural binding head",
    ] {
        let guards = format!("{IMMUTABILITY}{NO_REPLACE}{PROJECTION}{LINEAGE}");
        assert!(guards.contains(required), "guard lost {required}");
    }

    let open = source_block(STORE_FILE, "pub fn open(", "pub fn ensure_device_user(");
    assert_ordered(
        open,
        &["Connection::open(path)?", "apply_migrations(&conn)?"],
    );
    assert_ordered(
        STORE_SCHEMA,
        &[
            "pub(crate) fn apply_migrations(",
            "register_v270_receipt_integrity_functions(conn)?",
            "CREATE TABLE IF NOT EXISTS schema_migrations",
        ],
    );
    assert!(STORE_MIGRATIONS.contains(
        "compute_external_pool_adapter_provider_runtime_readiness::register_receipt_integrity_functions("
    ));
    assert!(MIGRATION_V270.contains("register_receipt_integrity_functions(conn)?"));
    assert_eq!(
        RECEIPT_INTEGRITY
            .matches("conn.create_scalar_function")
            .count(),
        2
    );
    for udf in [
        "elon_v270_provider_runtime_readiness_receipt_is_exact",
        "elon_v270_provider_runtime_readiness_revocation_is_exact",
    ] {
        assert!(RECEIPT_INTEGRITY.contains(udf), "missing V270 UDF {udf}");
    }
}

#[test]
fn provider_runtime_readiness_store_source_freezes_private_view_and_exact_roots() {
    for required in [
        "runtime_custody_epoch_digest",
        "runtime_bundle_identity_commitment",
        "post_cleanup_observation_commitment",
    ] {
        assert!(RECEIPTS_SQL.contains(required), "schema lost {required}");
        assert!(STORE_ROOTS.contains(required), "material lost {required}");
    }
    let private_sources = format!("{RECEIPTS_SQL}{VIEW_SQL}{STORE_ROOTS}");
    for old in [
        "hmac_key_epoch_id",
        "runtime_bundle_material_hmac",
        "secret_delivery_root_hmac",
        "no_work_probe_root_hmac",
    ] {
        assert!(!private_sources.contains(old), "obsolete seal {old}");
    }
    for private in [
        "runtime_custody_epoch_digest",
        "runtime_bundle_identity_commitment",
        "post_cleanup_observation_commitment",
        "probe_execution_id",
        "request_bytes",
        "response_bytes",
        "recorded_by_actor_user_id",
        "idempotency_key",
    ] {
        assert!(!VIEW_SQL.contains(private), "view exposed {private}");
    }
    assert!(VIEW_SQL.contains("relationally_current_requires_process_custody_reproof"));
    assert!(!VIEW_SQL
        .lines()
        .any(|line| line.trim() == "receipt.readiness_receipt_json,"));
    for root in [
        "candidate.candidate_status='candidate_current_not_activation_ready'",
        "candidate.activation_closure_status='activation_closure_not_implemented'",
        "provider.status='registering'",
        "current_vulnerability.current_status='verified_current'",
        "current_sandbox.current_status='verified_current'",
        "current_credential.current_status='verified_current'",
        "current_compatibility.currentness_status='current_signed_verifier_assertion'",
        "NEW.expires_at=min(",
    ] {
        assert!(MIGRATION_ROOTS.contains(root), "root guard lost {root}");
    }
    assert!(!MIGRATION_ROOTS.contains("signing_handoff"));
}

#[test]
fn provider_runtime_readiness_store_source_preserves_v254_fences_exactly() {
    assert_eq!(
        V254_FENCES.matches("CREATE TRIGGER IF NOT EXISTS").count(),
        18
    );
    assert_eq!(
        hex::encode(Sha256::digest(V254_FENCES.as_bytes())),
        "7d2971d0987e2c2939e0b212d4aedfa15a4b7cd3205e433eb7030f1371840de6"
    );
}

fn ddl_columns(source: &str) -> Vec<&str> {
    source
        .lines()
        .map(str::trim)
        .filter(|line| line.contains(" TEXT") || line.contains(" INTEGER"))
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
