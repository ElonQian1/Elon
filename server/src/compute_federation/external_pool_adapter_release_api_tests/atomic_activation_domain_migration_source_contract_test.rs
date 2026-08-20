macro_rules! domain_source {
    ($path:literal) => {
        include_str!(concat!(
            "../external_pool_adapter_atomic_activation/",
            $path
        ))
    };
}

macro_rules! migration_source {
    ($path:literal) => {
        include_str!(concat!(
            "../../store_migrations/compute_external_pool_adapter_atomic_activation/",
            $path
        ))
    };
}

const DOMAIN_ROOT: &str = include_str!("../external_pool_adapter_atomic_activation.rs");
const DOMAIN_TYPES: &str = domain_source!("types.rs");
const DOMAIN_CANONICAL: &str = domain_source!("canonical.rs");
const DOMAIN_VALIDATION: &str = domain_source!("validation.rs");
const DOMAIN_PROJECTED: &str = domain_source!("projected_binding.rs");
const DOMAIN_CARRIER: &str = domain_source!("active_carrier.rs");
const MIGRATION_ROOT: &str =
    include_str!("../../store_migrations/compute_external_pool_adapter_atomic_activation.rs");
const TABLE: &str = migration_source!("tables/receipts.sql");
const PROJECTION: &str = migration_source!("guards/projection.sql");
const ROOTS: &str = migration_source!("guards/roots.sql");
const IMMUTABILITY: &str = migration_source!("guards/immutability.sql");
const INTEGRITY: &str = migration_source!("receipt_integrity.rs");
const PRECHECK: &str = migration_source!("precheck.rs");
const REBUILD: &str = migration_source!("v274_rebuild.rs");
const FENCES: &str = migration_source!("fences/replace_pending_plan_fences.sql");
const V253_VIEW: &str = migration_source!("v253_active_bridge/current_view.sql");
const V253_CHALLENGE: &str = migration_source!("v253_active_bridge/challenge_roots.sql");
const V253_RECEIPT: &str = migration_source!("v253_active_bridge/receipt_current_roots.sql");
const V271: &str = migration_source!("v271_projected_source.rs");
const STORE_RECEIPT: &str = include_str!(
    "../../store/compute_external_pool_adapter_provider_active_successor/atomic_activation/receipt.rs"
);
const STORE_MIGRATIONS: &str = include_str!("../../store_migrations.rs");
const STORE_SCHEMA: &str = include_str!("../../store_schema.rs");

#[test]
fn atomic_activation_domain_freezes_stable_executor_projection_and_dual_time() {
    for module in [
        "active_carrier",
        "canonical",
        "projected_binding",
        "types",
        "validation",
    ] {
        assert!(DOMAIN_ROOT.contains(&format!("mod {module};")));
    }
    for marker in [
        "ELON-EXTERNAL-POOL-ADAPTER-ATOMIC-ACTIVATION-RECEIPT-V1",
        "ELON-EXTERNAL-POOL-STABLE-EXECUTOR-ID-V1",
        "ELON-EXTERNAL-POOL-STABLE-EXECUTOR-BINDING-V1",
        "ELON-EXTERNAL-POOL-ADAPTER-CREDENTIAL-PROJECTED-ACTIVE-TRANSITION-PROOF-V1",
        "ELON-EXTERNAL-POOL-ADAPTER-TASK-PROTOCOL-ACTIVE-CARRIER-V1",
        "ELON-EXTERNAL-POOL-ADAPTER-ATOMIC-ACTIVATION-IDEMPOTENCY-V1",
        "ELON-EXTERNAL-POOL-ADAPTER-ATOMIC-ACTIVATION-CONFIRMATION-V1",
        "external_pool_executor_",
    ] {
        assert!(
            DOMAIN_CANONICAL.contains(marker),
            "canonical ABI lost {marker}"
        );
    }
    for marker in [
        "derive_external_pool_stable_executor",
        "derive_external_pool_projected_v211_adapter_binding",
        "canonical_external_pool_adapter_atomic_activation_route_capabilities_json",
        "canonical_adapter_binding_json_and_digest(&binding)",
        "COMPUTE_ATTEMPT_ADAPTER_BINDING_SCHEMA",
        "route_adapter_projection_id",
    ] {
        assert!(
            format!("{DOMAIN_CANONICAL}{DOMAIN_PROJECTED}").contains(marker),
            "projected binding lost {marker}"
        );
    }
    for marker in [
        "activation_target_updated_at",
        "evidence_checked_at",
        "source.updated_at",
        "observation_started_at",
        "observation_completed_at",
        "task_protocol_conformance_expires_at",
        "ATOMIC_ACTIVATION_ACTOR_KIND",
        "ATOMIC_ACTIVATION_IDEMPOTENCY_SCOPE",
        "ATOMIC_ACTIVATION_CONFIRMATION",
    ] {
        assert!(
            format!("{DOMAIN_TYPES}{DOMAIN_VALIDATION}").contains(marker),
            "domain validation lost {marker}"
        );
    }
    assert!(DOMAIN_CARRIER.contains("ExternalPoolAdapterTaskProtocolGenesisActiveCarrier"));
    assert!(DOMAIN_CARRIER.contains("ExternalPoolAdapterTaskProtocolRefreshActiveCarrier"));
}

#[test]
fn atomic_activation_migration_is_one_immutable_table_with_exact_projection() {
    let sql_columns = table_columns(
        TABLE,
        "compute_external_pool_adapter_atomic_activation_receipts",
    );
    let store_columns = store_receipt_columns();
    assert_eq!(sql_columns.len(), 79);
    assert_eq!(sql_columns, store_columns);
    assert!(STORE_RECEIPT
        .contains("canonical_external_pool_adapter_atomic_activation_route_capabilities_json"));
    assert!(PROJECTION.contains(
        "NEW.route_capabilities_json IS NOT json_extract(NEW.activation_receipt_json,'$.activation.route_closure.capabilities')"
    ));
    assert_eq!(TABLE.matches("CREATE TABLE").count(), 1);
    assert!(!format!("{TABLE}{MIGRATION_ROOT}").contains("CREATE VIEW"));
    assert!(!format!("{TABLE}{MIGRATION_ROOT}").contains("revocation"));
    for column in &sql_columns {
        if *column != "activation_receipt_json" {
            assert!(
                PROJECTION.contains(&format!("NEW.{column}")),
                "scalar projection lost {column}"
            );
        }
    }
    for marker in [
        "v277_atomic_activation_receipt_no_replace",
        "v277_atomic_activation_receipt_no_update",
        "v277_atomic_activation_receipt_no_delete",
    ] {
        assert!(IMMUTABILITY.contains(marker));
    }
    for marker in [
        "v277_atomic_activation_receipt_integrity",
        "v277_atomic_activation_receipt_pending_plan",
        "elon_v277_external_pool_adapter_atomic_activation_receipt_is_exact",
        "elon_v277_external_pool_adapter_atomic_activation_pending_plan_matches",
    ] {
        assert!(INTEGRITY.contains(marker));
    }
    assert!(INTEGRITY.contains("FunctionFlags::SQLITE_DETERMINISTIC"));
    assert!(!INTEGRITY.contains("create_scalar_function(PENDING_MATCHES"));
    assert!(ROOTS.contains("projected_v211_adapter_binding_digest"));
    assert!(!ROOTS.contains("logical_adapter_binding_digest=NEW.route_binding_digest"));
}

#[test]
fn atomic_activation_upgrade_keeps_v274_v253_v271_and_fences_fail_closed() {
    for marker in [
        "columns.len() == 85",
        "activation_target_updated_at",
        "evidence_checked_at",
        "V277 refuses to rebuild non-empty V274 authority tables",
        "activation_witness_id",
        "activation_receipt_id",
        "activation_witness_digest",
        "activation_receipt_digest",
        "activation_root_digest",
    ] {
        assert!(REBUILD.contains(marker), "V274 rebuild lost {marker}");
    }
    assert_eq!(FENCES.matches("CREATE TRIGGER ").count(), 9);
    assert_eq!(FENCES.matches("pending_plan_matches(").count(), 9);
    assert!(PRECHECK.contains("const PENDING_PERMITS: [&str; 9]"));
    assert!(PRECHECK.contains("const RETAINED_ABSOLUTE_DENIES: [&str; 9]"));
    assert!(PRECHECK.contains("namespace_count != 1"));
    assert!(PRECHECK.contains("migration must not seed activation receipts"));
    for source in [V253_VIEW, V253_CHALLENGE, V253_RECEIPT] {
        assert!(source.contains("compute_external_pool_adapter_atomic_activation_receipts"));
        assert!(source.contains("successor_sequence=1"));
        assert!(!source.contains("provider_active_successor_current"));
    }
    assert!(V253_VIEW.contains("witnessed_projected_active"));
    assert!(V271.contains("reinstall_exact_source_trigger_for_v277"));
    assert_ordered(
        MIGRATION_ROOT,
        &[
            "register_receipt_integrity_functions(conn)?",
            "TransactionBehavior::Immediate",
            "tables::create(&transaction)?",
            "v274_rebuild::rebuild_if_required(&transaction)?",
            "v253_active_bridge::install(&transaction)?",
            "v271_projected_source::install(&transaction)?",
            "fences::install(&transaction)?",
            "transaction.commit()?",
        ],
    );
    assert_ordered(
        STORE_MIGRATIONS,
        &[
            "rust_cache_fleet::migration_v275",
            "rust_cache_gc_approval::migration_v276",
            "compute_external_pool_adapter_atomic_activation::migration_v277",
        ],
    );
    assert!(STORE_SCHEMA.contains("register_v277_receipt_integrity_functions"));
}

fn store_receipt_columns() -> Vec<&'static str> {
    STORE_RECEIPT
        .split_once("pub(super) const RECEIPT_COLUMNS: &str = \"")
        .unwrap()
        .1
        .split_once("\";")
        .unwrap()
        .0
        .split(',')
        .collect()
}

fn table_columns<'a>(source: &'a str, table: &str) -> Vec<&'a str> {
    source
        .split_once(&format!("CREATE TABLE IF NOT EXISTS {table} ("))
        .unwrap()
        .1
        .split_once("\n);")
        .unwrap()
        .0
        .lines()
        .filter_map(|line| {
            let mut words = line.trim().trim_end_matches(',').split_whitespace();
            let name = words.next()?;
            matches!(words.next()?, "TEXT" | "INTEGER" | "BLOB" | "REAL").then_some(name)
        })
        .collect()
}

fn assert_ordered(source: &str, needles: &[&str]) {
    let mut cursor = 0;
    for needle in needles {
        let offset = source[cursor..]
            .find(needle)
            .unwrap_or_else(|| panic!("missing ordered marker {needle}"));
        cursor += offset + needle.len();
    }
}
