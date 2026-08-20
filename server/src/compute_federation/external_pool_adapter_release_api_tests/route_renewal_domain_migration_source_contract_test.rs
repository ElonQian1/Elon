macro_rules! route_domain_source {
    ($path:literal) => {
        include_str!(concat!("../external_pool_adapter_route_renewal/", $path))
    };
}

macro_rules! route_migration_source {
    ($path:literal) => {
        include_str!(concat!(
            "../../store_migrations/compute_external_pool_adapter_route_renewal/",
            $path
        ))
    };
}

const DOMAIN_ROOT: &str = include_str!("../external_pool_adapter_route_renewal.rs");
const DOMAIN_TYPES: &str = route_domain_source!("types.rs");
const DOMAIN_CANONICAL: &str = route_domain_source!("canonical.rs");
const DOMAIN_VALIDATION: &str = route_domain_source!("validation.rs");
const MIGRATION_ROOT: &str =
    include_str!("../../store_migrations/compute_external_pool_adapter_route_renewal.rs");
const TABLE: &str = route_migration_source!("tables/receipts.sql");
const RECEIPT_GUARDS: &str = route_migration_source!("guards/receipts.sql");
const INTEGRITY: &str = route_migration_source!("receipt_integrity.rs");
const FENCE_UNION: &str = route_migration_source!("fences/v254_route_union.sql");
const V271: &str =
    include_str!("../../store_migrations/compute_external_pool_adapter_route_source_projection.rs");
const V274_REFRESH: &str = route_migration_source!("v274_refresh.rs");
const STORE_MIGRATIONS: &str = include_str!("../../store_migrations.rs");
const STORE_RECEIPT: &str =
    include_str!("../../store/compute_external_pool_adapter_route_renewal/receipt.rs");
const STORE_PLAN: &str =
    include_str!("../../store/compute_external_pool_adapter_route_renewal/pending.rs");

#[test]
fn route_renewal_domain_is_canonical_bounded_and_historical_only() {
    for module in ["canonical", "types", "validation"] {
        assert!(DOMAIN_ROOT.contains(&format!("mod {module};")));
    }
    for marker in [
        "ELON-EXTERNAL-POOL-ADAPTER-ROUTE-RENEWAL-RECEIPT-V1",
        "ELON-EXTERNAL-POOL-ADAPTER-ROUTE-RENEWAL-ID-V1",
        "ELON-EXTERNAL-POOL-ADAPTER-ROUTE-RENEWAL-POLICY-V1",
        "ELON-EXTERNAL-POOL-ADAPTER-ROUTE-RENEWAL-IDEMPOTENCY-V1",
        "canonical_compute_plugin_ijson_and_sha256",
        "derive_external_pool_adapter_route_renewal_receipt_id",
    ] {
        assert!(
            DOMAIN_CANONICAL.contains(marker),
            "canonical ABI lost {marker}"
        );
    }
    for marker in [
        "ROUTE_RENEWAL_MAX_JSON_BYTES: usize = 2 * 1024 * 1024",
        "ROUTE_RENEWAL_RENEW_BEFORE_SECONDS: i64 = 60",
        "ROUTE_RENEWAL_FRESH_MAX_SECONDS: i64 = 300",
        "ROUTE_RENEWAL_CLEANUP_MAX_SECONDS: i64 = 1_800",
    ] {
        assert!(DOMAIN_TYPES.contains(marker), "route policy lost {marker}");
    }
    for marker in [
        "COMPUTE_ROUTE_REQUIRED_CAPABILITY_COUNT",
        "canonical_route_capability_set_digest",
        "timing.created_at == timing.evidence_checked_at",
        "renew_at < expires",
        "expires < cleanup",
        "canonical_external_pool_adapter_route_renewal_idempotency_json_and_digest",
    ] {
        assert!(
            DOMAIN_VALIDATION.contains(marker),
            "validation lost {marker}"
        );
    }
    let domain = format!("{DOMAIN_ROOT}{DOMAIN_TYPES}{DOMAIN_CANONICAL}{DOMAIN_VALIDATION}");
    for forbidden in [
        "current authority",
        "impl Store",
        "pub struct Create",
        "revocation",
    ] {
        assert!(!domain.contains(forbidden), "domain gained {forbidden}");
    }
}

#[test]
fn route_renewal_migration_is_one_immutable_77_column_table() {
    assert_eq!(TABLE.matches("CREATE TABLE").count(), 1);
    assert_eq!(TABLE.matches("CREATE VIEW").count(), 0);
    assert_eq!(TABLE.matches("revocation").count(), 0);
    let ddl_columns = TABLE
        .lines()
        .filter(|line| {
            let mut words = line.split_whitespace();
            line.starts_with("  ") && matches!(words.nth(1), Some("TEXT") | Some("INTEGER"))
        })
        .count();
    assert_eq!(ddl_columns, 77);
    let columns = STORE_RECEIPT
        .split("pub(crate) const RECEIPT_COLUMNS: &str = \"")
        .nth(1)
        .and_then(|tail| tail.split('"').next())
        .expect("V278 receipt columns");
    assert_eq!(columns.split(',').count(), 77);
    for marker in [
        "v278_route_renewal_receipt_lineage",
        "v278_route_renewal_receipt_no_replace",
        "v278_route_renewal_receipt_no_update",
        "v278_route_renewal_receipt_no_delete",
        "v278_route_credential_root_cas",
        "compute_external_pool_adapter_atomic_activation_receipts",
        "activation_genesis_successor_receipt_id",
        "credential_reattestation_receipt_id",
    ] {
        assert!(RECEIPT_GUARDS.contains(marker), "guard lost {marker}");
    }
    assert!(INTEGRITY.contains("receipt_columns.len() == 77"));
    assert!(INTEGRITY.contains("FunctionFlags::SQLITE_DETERMINISTIC"));
    assert!(INTEGRITY.contains("FunctionFlags::SQLITE_INNOCUOUS"));
    assert!(STORE_PLAN.contains("FunctionFlags::SQLITE_UTF8 | FunctionFlags::SQLITE_INNOCUOUS"));
    assert!(!STORE_PLAN.contains("SQLITE_DETERMINISTIC"));
}

#[test]
fn route_renewal_reuses_v254_v271_and_v274_guard_names() {
    let union_names = [
        "v254_external_pool_candidate_service_actor_fence",
        "v254_external_pool_route_credential_fence",
        "v254_external_pool_route_authorization_fence",
        "v254_external_pool_route_capability_fence",
        "v254_external_pool_route_seal_fence",
    ];
    assert_eq!(FENCE_UNION.matches("CREATE TRIGGER").count(), 5);
    for name in union_names {
        assert!(FENCE_UNION.contains(name));
    }
    assert_eq!(
        FENCE_UNION
            .matches("elon_v278_external_pool_adapter_route_renewal_pending_plan_matches")
            .count(),
        5
    );
    assert_eq!(
        FENCE_UNION
            .matches("elon_v277_external_pool_adapter_atomic_activation_pending_plan_matches")
            .count(),
        5
    );
    assert!(V271.contains("reinstall_exact_source_trigger_for_v278"));
    assert!(V271.contains("provider.provider_kind='external_pool' AND provider.status='active'"));
    assert!(V271
        .contains("activation.projected_v211_adapter_binding_digest=NEW.adapter_binding_digest"));
    assert!(V274_REFRESH
        .contains("DROP TRIGGER IF EXISTS v274_provider_active_successor_receipt_pending_seal"));
    assert!(V274_REFRESH.contains("NEW.successor_sequence > 1 AND"));
    assert_eq!(
        V274_REFRESH
            .matches("elon_v278_external_pool_adapter_provider_active_successor_refresh_pending_plan_matches")
            .count(),
        1
    );
    assert!(MIGRATION_ROOT.contains("TransactionBehavior::Immediate"));
    assert_ordered(
        MIGRATION_ROOT,
        &[
            "register_receipt_integrity_functions(connection)?",
            "tables::create(&transaction)?",
            "receipt_integrity::install(&transaction)?",
            "fences::install(&transaction)?",
            "v271_active_source::install(&transaction)?",
            "v274_refresh::install(&transaction)?",
            "install_v278_reachability_guards",
            "transaction.commit()?",
        ],
    );
    assert!(STORE_MIGRATIONS.contains(
        "(278, \"外部矿池 Adapter immutable route renewal authority\", compute_external_pool_adapter_route_renewal::migration_v278)"
    ));
}

fn assert_ordered(source: &str, markers: &[&str]) {
    let mut cursor = 0;
    for marker in markers {
        let offset = source[cursor..]
            .find(marker)
            .unwrap_or_else(|| panic!("missing ordered marker {marker}"));
        cursor += offset + marker.len();
    }
}
