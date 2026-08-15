use sha2::{Digest, Sha256};

const MIGRATION: &str = include_str!(
    "../../store_migrations/compute_external_pool_adapter_runtime_compatibility_verification.rs"
);
const TABLES: &str = concat!(
    include_str!("../../store_migrations/compute_external_pool_adapter_runtime_compatibility_verification/tables/challenges.sql"),
    include_str!("../../store_migrations/compute_external_pool_adapter_runtime_compatibility_verification/tables/run_observations.sql"),
    include_str!("../../store_migrations/compute_external_pool_adapter_runtime_compatibility_verification/tables/verifications.sql"),
    include_str!("../../store_migrations/compute_external_pool_adapter_runtime_compatibility_verification/tables/revocations.sql"),
    include_str!("../../store_migrations/compute_external_pool_adapter_runtime_compatibility_verification/tables/indexes.sql"),
);
const CHALLENGES: &str = include_str!(
    "../../store_migrations/compute_external_pool_adapter_runtime_compatibility_verification/tables/challenges.sql"
);
const OBSERVATIONS: &str = include_str!(
    "../../store_migrations/compute_external_pool_adapter_runtime_compatibility_verification/tables/run_observations.sql"
);
const VERIFICATIONS: &str = include_str!(
    "../../store_migrations/compute_external_pool_adapter_runtime_compatibility_verification/tables/verifications.sql"
);
const REVOCATIONS: &str = include_str!(
    "../../store_migrations/compute_external_pool_adapter_runtime_compatibility_verification/tables/revocations.sql"
);
const TABLE_INSTALLER: &str = include_str!(
    "../../store_migrations/compute_external_pool_adapter_runtime_compatibility_verification/tables.rs"
);
const GUARD_INSTALLER: &str = include_str!(
    "../../store_migrations/compute_external_pool_adapter_runtime_compatibility_verification/guards.rs"
);
const NO_REPLACE: &str = include_str!(
    "../../store_migrations/compute_external_pool_adapter_runtime_compatibility_verification/guards/no_replace.sql"
);
const IMMUTABILITY: &str = include_str!(
    "../../store_migrations/compute_external_pool_adapter_runtime_compatibility_verification/guards/immutability.sql"
);
const PROJECTION: &str = include_str!(
    "../../store_migrations/compute_external_pool_adapter_runtime_compatibility_verification/guards/projection.sql"
);
const LINEAGE: &str = include_str!(
    "../../store_migrations/compute_external_pool_adapter_runtime_compatibility_verification/guards/lineage.sql"
);
const RECEIPT_INTEGRITY: &str = include_str!(
    "../../store_migrations/compute_external_pool_adapter_runtime_compatibility_verification/receipt_integrity.rs"
);
const CURRENT_VIEW: &str = include_str!(
    "../../store_migrations/compute_external_pool_adapter_runtime_compatibility_verification/view.sql"
);
const V254_FENCES: &str = include_str!(
    "../../store_migrations/compute_external_pool_provider_activation_candidate/guards/fences.rs"
);
const STORE_CHALLENGE: &str = include_str!(
    "../../store/compute_external_pool_adapter_runtime_compatibility_verification/challenge.rs"
);
const STORE_RECORD: &str = include_str!(
    "../../store/compute_external_pool_adapter_runtime_compatibility_verification/record.rs"
);
const STORE_REVOCATION: &str = include_str!(
    "../../store/compute_external_pool_adapter_runtime_compatibility_verification/revocation.rs"
);
const STORE_CURRENT: &str = include_str!(
    "../../store/compute_external_pool_adapter_runtime_compatibility_verification/current.rs"
);
const STORE_READ: &str = include_str!(
    "../../store/compute_external_pool_adapter_runtime_compatibility_verification/read.rs"
);
const STORE_TYPES: &str = include_str!(
    "../../store/compute_external_pool_adapter_runtime_compatibility_verification/types.rs"
);
const STORE_ERROR: &str = include_str!(
    "../../store/compute_external_pool_adapter_runtime_compatibility_verification/error.rs"
);
const STORE_PERSISTENCE: &str = include_str!(
    "../../store/compute_external_pool_adapter_runtime_compatibility_verification/persistence.rs"
);

const FOUR_TABLES: &[&str] = &[
    "compute_external_pool_adapter_runtime_compatibility_verification_challenges",
    "compute_external_pool_adapter_runtime_compatibility_verification_run_observations",
    "compute_external_pool_adapter_runtime_compatibility_verification_receipts",
    "compute_external_pool_adapter_runtime_compatibility_verification_revocations",
];

#[test]
fn runtime_compatibility_migration_source_freezes_four_tables_and_one_diagnostic_view() {
    assert_eq!(TABLES.matches("CREATE TABLE IF NOT EXISTS").count(), 4);
    for table in FOUR_TABLES {
        assert!(TABLES.contains(table), "missing V268 table {table}");
    }
    assert_eq!(CURRENT_VIEW.matches("CREATE VIEW").count(), 1);
    assert!(CURRENT_VIEW.contains(
        "CREATE VIEW compute_external_pool_adapter_runtime_compatibility_verification_current"
    ));
    for required in [
        "TransactionBehavior::Immediate",
        "tables::create(&transaction)",
        "guards::install(&transaction)",
        "view::install(&transaction)",
        "transaction.commit()",
    ] {
        assert!(
            MIGRATION.contains(required),
            "migration order drifted: {required}"
        );
    }
}

#[test]
fn runtime_compatibility_migration_source_freezes_append_only_canonical_chain() {
    assert_eq!(IMMUTABILITY.matches("BEFORE UPDATE").count(), 4);
    assert_eq!(IMMUTABILITY.matches("BEFORE DELETE").count(), 4);
    for required in [
        "elon_v268_runtime_compatibility_challenge_is_exact",
        "elon_v268_runtime_compatibility_observation_is_exact",
        "elon_v268_runtime_compatibility_verification_is_exact",
        "elon_v268_runtime_compatibility_revocation_is_exact",
        "runtime_compatibility_signature_challenge(&challenge, &observation)",
        "verify_runtime_compatibility_signature(",
        "canonical == observation_json",
        "canonical == verification_json",
    ] {
        assert!(
            RECEIPT_INTEGRITY.contains(required),
            "missing canonical/signature guard {required}"
        );
    }
    assert!(!OBSERVATIONS.contains("signature_message_base64"));
    assert!(!OBSERVATIONS.contains("signature_message_digest"));
    assert!(VERIFICATIONS.contains("signature_message_digest TEXT NOT NULL"));
    assert!(VERIFICATIONS.contains("signature_base64 TEXT NOT NULL"));
    assert!(PROJECTION.contains("V268 observation scalar projection mismatch"));
    assert!(PROJECTION.contains("V268 verification scalar projection mismatch"));
}

#[test]
fn runtime_compatibility_migration_source_freezes_sql_abi_bounds_and_no_replace() {
    for (ddl, table, expected_count) in [
        (CHALLENGES, FOUR_TABLES[0], 43),
        (OBSERVATIONS, FOUR_TABLES[1], 40),
        (VERIFICATIONS, FOUR_TABLES[2], 46),
        (REVOCATIONS, FOUR_TABLES[3], 21),
    ] {
        let ddl_columns = ddl_columns(ddl, table);
        let insert_columns = insert_columns(STORE_PERSISTENCE, table);
        assert_eq!(
            ddl_columns.len(),
            expected_count,
            "DDL count drifted: {table}"
        );
        assert_eq!(
            insert_columns.len(),
            expected_count,
            "INSERT count drifted: {table}"
        );
        assert_eq!(
            ddl_columns, insert_columns,
            "ordered SQL ABI drifted: {table}"
        );
    }
    assert_eq!(TABLES.matches("__MAX_RECEIPT_JSON_BYTES_SQL__").count(), 5);
    assert!(TABLE_INSTALLER
        .contains("RUNTIME_COMPATIBILITY_VERIFICATION_MAX_RECEIPT_JSON_BYTES.to_string()"));
    assert!(TABLE_INSTALLER.contains(".replace(\"__MAX_RECEIPT_JSON_BYTES_SQL__\", &maximum)"));
    assert!(!TABLES.contains("2097152"));
    assert!(!TABLE_INSTALLER.contains("2097152"));
    assert_eq!(NO_REPLACE.matches("BEFORE INSERT ON").count(), 4);
    assert_eq!(NO_REPLACE.matches("replacement is forbidden").count(), 4);
    for table in FOUR_TABLES {
        assert!(
            NO_REPLACE.contains(table),
            "missing no-replace guard: {table}"
        );
    }
    assert!(GUARD_INSTALLER.contains("include_str!(\"guards/no_replace.sql\")"));
}

#[test]
fn runtime_compatibility_migration_source_freezes_release_lineage_and_currentness() {
    for required in [
        "CHECK((sequence=1)=(predecessor_verification_receipt_id IS NULL))",
        "UNIQUE(registry_release_id,sequence)",
        "UNIQUE(registry_release_id,predecessor_verification_receipt_id)",
        "challenge_id TEXT NOT NULL UNIQUE",
        "run_observation_id TEXT NOT NULL UNIQUE",
    ] {
        let source = format!("{CHALLENGES}{OBSERVATIONS}{VERIFICATIONS}");
        assert!(
            source.contains(required),
            "missing release lineage {required}"
        );
    }
    for required in [
        "v268_runtime_compatibility_challenge_current_authority",
        "v268_runtime_compatibility_observation_current_authority",
        "v268_runtime_compatibility_verification_current_authority",
        "v268_runtime_compatibility_revocation_head_only",
        "current_status='release_current'",
        "current_status='active'",
        "ORDER BY sequence DESC LIMIT 1",
    ] {
        assert!(
            LINEAGE.contains(required),
            "missing current lineage guard {required}"
        );
    }
    for required in [
        "current_signed_verifier_assertion",
        "historical_signed_verifier_assertion",
        "julianday(verification.expires_at)>julianday('now')",
        "revocation.revocation_receipt_id IS NULL",
        "signature_integrity",
    ] {
        assert!(
            CURRENT_VIEW.contains(required),
            "missing diagnostic view field {required}"
        );
    }
}

#[test]
fn runtime_compatibility_migration_source_cannot_upgrade_effect_or_readiness() {
    let none_effects = "{\"activation_effect\":\"none\",\"adapter_effect\":\"none\",\"credential_effect\":\"none\",\"execution_effect\":\"none\",\"market_effect\":\"none\",\"provider_effect\":\"none\",\"route_effect\":\"none\",\"settlement_effect\":\"none\",\"usage_effect\":\"none\"}";
    let false_readiness = "{\"activation_ready\":false,\"broker_connect_ready\":false,\"process_ready\":false,\"runtime_launch_ready\":false,\"secret_delivery_ready\":false,\"session_ready\":false,\"upstream_probe_ready\":false}";
    assert_eq!(TABLES.matches(none_effects).count(), 3);
    assert_eq!(TABLES.matches(false_readiness).count(), 3);
    for forbidden_write in [
        "INSERT INTO compute_external_pool_providers",
        "UPDATE compute_external_pool_providers",
        "compute_route_adapters",
        "compute_service_actor_authorizations",
        "compute_capacity_pools",
        "compute_offers",
        "compute_attempt_start_outbox",
        "compute_attempt_settlements",
        "compute_external_pool_adapter_upstream_transport_targets",
        "compute_external_pool_adapter_runtime_bundles",
    ] {
        let source = format!("{TABLES}{IMMUTABILITY}{PROJECTION}{LINEAGE}{CURRENT_VIEW}");
        assert!(
            !source.contains(forbidden_write),
            "V268 migration gained downstream authority: {forbidden_write}"
        );
    }
}

#[test]
fn runtime_compatibility_store_source_freezes_http_and_atomic_consumer_seams() {
    for (source, method, scope) in [
        (
            STORE_CHALLENGE,
            "issue_external_pool_adapter_runtime_compatibility_verification_challenge",
            "v268:runtime-compatibility-challenge:{admin_user_id}",
        ),
        (
            STORE_RECORD,
            "record_external_pool_adapter_runtime_compatibility_verification",
            "v268:runtime-compatibility-verify:{admin_user_id}",
        ),
        (
            STORE_REVOCATION,
            "revoke_external_pool_adapter_runtime_compatibility_verification",
            "v268:runtime-compatibility-revoke:{admin_user_id}",
        ),
    ] {
        assert!(source.contains(&format!("pub(crate) fn {method}")));
        assert!(source.contains("admin_user_id: &str"));
        assert!(source.contains(scope));
        assert!(source.contains("TransactionBehavior::Immediate"));
    }
    for method in [
        "external_pool_adapter_runtime_compatibility_verification_run_observation_exists",
        "external_pool_adapter_runtime_compatibility_verification_exists",
    ] {
        assert!(
            STORE_READ.contains(&format!("pub(crate) fn {method}")),
            "missing resource-existence seam {method}"
        );
    }
    for required in [
        "validate_runtime_compatibility_verification_receipt(",
        "runtime_compatibility_signature_challenge(&challenge.receipt, &observation.receipt)",
        "verify_runtime_compatibility_signature(",
        "runtime_compatibility_verification_receipt_json_and_digest(&stored.receipt)",
    ] {
        assert!(
            STORE_READ.contains(required),
            "durable verification read audit drifted: {required}"
        );
    }
    assert!(STORE_CURRENT.contains(
        "pub(crate) fn external_pool_adapter_runtime_compatibility_verification_currentness"
    ));
    assert!(
        STORE_CURRENT.contains("Option<ExternalPoolAdapterRuntimeCompatibilityCurrentnessSummary>")
    );
    for variant in [
        "Conflict(#[source] AnyError)",
        "Storage(#[source] AnyError)",
    ] {
        assert!(
            STORE_ERROR.contains(variant),
            "missing typed Store classification {variant}"
        );
    }

    let authority = STORE_TYPES
        .split_once(
            "pub(in crate::store) struct CurrentExternalPoolAdapterRuntimeCompatibilityVerificationAuthority",
        )
        .unwrap()
        .1
        .split_once("impl<'tx, 'conn>")
        .unwrap()
        .0;
    for root in [
        "verification",
        "run_observation",
        "release",
        "verifier_key",
        "checked_at",
    ] {
        assert!(
            authority.contains(root),
            "missing atomic consumer root {root}"
        );
    }
    assert!(!authority.contains("derive("));
    assert!(!authority.contains("Serialize"));
    assert!(STORE_CURRENT.contains(
        "pub(in crate::store) fn current_external_pool_adapter_runtime_compatibility_verification_authority_on"
    ));
    for required in [
        "verification_head_by_release_on(transaction",
        "revocation_by_verification_on(transaction",
        "current_external_pool_adapter_registry_release_authority_on(",
        "current_sandbox_verifier_key_authority_on(",
        "run_observation_by_id_on(transaction",
    ] {
        assert!(
            STORE_CURRENT.contains(required),
            "atomic consumer current-root check drifted: {required}"
        );
    }
}

#[test]
fn runtime_compatibility_source_preserves_v254_eighteen_fences_exactly() {
    assert_eq!(
        hex::encode(Sha256::digest(V254_FENCES.as_bytes())),
        "7d2971d0987e2c2939e0b212d4aedfa15a4b7cd3205e433eb7030f1371840de6"
    );
    assert_eq!(V254_TRIGGER_NAMES.len(), 18);
    for name in V254_TRIGGER_NAMES {
        assert!(V254_FENCES.contains(name), "missing V254 fence {name}");
    }
}

const V254_TRIGGER_NAMES: &[&str] = &[
    "v254_external_pool_provider_activation_fence",
    "v254_external_pool_provider_insert_active_fence",
    "v254_external_pool_provider_identity_update_fence",
    "v254_external_pool_provider_kind_update_fence",
    "v254_external_pool_provider_version_active_fence",
    "v254_external_pool_candidate_projection_adapter_fence",
    "v254_external_pool_candidate_projection_adapter_version_fence",
    "v254_external_pool_candidate_service_actor_fence",
    "v254_external_pool_route_credential_fence",
    "v254_external_pool_route_authorization_fence",
    "v254_external_pool_route_capability_fence",
    "v254_external_pool_route_seal_fence",
    "v254_external_pool_capacity_pool_insert_active_fence",
    "v254_external_pool_capacity_pool_update_active_fence",
    "v254_external_pool_capacity_pool_version_active_fence",
    "v254_external_pool_offer_insert_market_fence",
    "v254_external_pool_offer_update_market_fence",
    "v254_external_pool_offer_version_market_fence",
];

fn ddl_columns<'a>(source: &'a str, table: &str) -> Vec<&'a str> {
    source
        .split_once(&format!("CREATE TABLE IF NOT EXISTS {table} ("))
        .unwrap()
        .1
        .split_once(");")
        .unwrap()
        .0
        .lines()
        .map(str::trim)
        .filter(|line| {
            !line.is_empty()
                && !line.starts_with("UNIQUE(")
                && !line.starts_with("FOREIGN KEY(")
                && !line.starts_with("CHECK(")
        })
        .map(|line| {
            line.split_whitespace()
                .next()
                .unwrap()
                .trim_end_matches(',')
        })
        .collect()
}

fn insert_columns<'a>(source: &'a str, table: &str) -> Vec<&'a str> {
    source
        .split_once(&format!("INSERT INTO {table}("))
        .unwrap()
        .1
        .split_once(") VALUES")
        .unwrap()
        .0
        .split(',')
        .map(str::trim)
        .collect()
}
