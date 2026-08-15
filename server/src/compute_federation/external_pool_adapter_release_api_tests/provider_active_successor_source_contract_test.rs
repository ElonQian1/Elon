macro_rules! domain_source {
    ($path:literal) => {
        include_str!(concat!(
            "../external_pool_adapter_provider_active_successor/",
            $path
        ))
    };
}
macro_rules! migration_source {
    ($path:literal) => {
        include_str!(concat!(
            "../../store_migrations/compute_external_pool_adapter_provider_active_successor/",
            $path
        ))
    };
}
const DOMAIN_ROOT: &str = include_str!("../external_pool_adapter_provider_active_successor.rs");
const DOMAIN_TYPES: &str = domain_source!("types.rs");
const DOMAIN_ROOT_TYPES: &str = domain_source!("types/roots.rs");
const CANONICAL: &str = domain_source!("canonical.rs");
const POLICY: &str = domain_source!("policy.rs");
const VALIDATION_ROOTS: &str = domain_source!("validation/roots.rs");
const VALIDATION_RECEIPTS: &str = domain_source!("validation/receipts.rs");
const CARRIER_POLICY: &str =
    include_str!("../external_pool_adapter_task_protocol_production/carrier_policy.rs");
const MIGRATION: &str = include_str!(
    "../../store_migrations/compute_external_pool_adapter_provider_active_successor.rs"
);
const TABLES: &str = migration_source!("tables.rs");
const RECEIPTS: &str = migration_source!("tables/receipts.sql");
const REVOCATIONS: &str = migration_source!("tables/revocations.sql");
const VIEW: &str = migration_source!("view.sql");
const INTEGRITY: &str = migration_source!("receipt_integrity.rs");
const IMMUTABILITY: &str = migration_source!("guards/immutability.sql");
const NO_REPLACE: &str = migration_source!("guards/no_replace.sql");
const PROJECTION: &str = migration_source!("guards/projection.sql");
const LINEAGE: &str = migration_source!("guards/lineage.sql");
const STRUCTURAL_ROOTS: &str = migration_source!("guards/roots/structural.sql");
const PROVIDER_CREDENTIAL_ROOTS: &str = migration_source!("guards/roots/provider_credential.sql");
const TASK_PROTOCOL_ROOTS: &str = migration_source!("guards/roots/task_protocol.sql");
const V253_VIEW: &str = migration_source!("v253/view.sql");
const V253_CHALLENGE_ROOTS: &str = migration_source!("v253/challenge_roots.sql");
const V253_RECEIPT_ROOTS: &str = migration_source!("v253/receipt_current_roots.sql");
const STORE_MIGRATIONS: &str = include_str!("../../store_migrations.rs");
const STORE_SCHEMA: &str = include_str!("../../store_schema.rs");

const ROOT_FIELDS: &str = concat!(
    "provider_id,provider_owner_account_id,source_registering_provider_id,",
    "source_registering_provider_policy_revision,source_registering_provider_json,",
    "source_registering_provider_digest,initial_active_provider_id,",
    "initial_active_provider_policy_revision,initial_active_provider_json,",
    "initial_active_provider_digest,provider_binding_id,provider_binding_digest,",
    "registry_release_id,registry_release_digest,registry_release_material_digest,",
    "installation_receipt_id,installation_receipt_digest,installation_content_digest,",
    "candidate_id,candidate_digest,delegation_id,delegation_digest,service_actor_id,",
    "logical_adapter_id,logical_adapter_binding_digest,logical_projection_compatibility_digest,",
    "route_adapter_projection_id,profile_id,profile_digest,launch_policy_digest,",
    "target_id,target_digest,target_policy_digest,companion_id,companion_digest,",
    "supervisor_session_policy_digest,entrypoint_capsule_policy_digest,launch_image_sha256,",
    "task_protocol_profile_digest,lane_subject_digest,task_production_carrier_policy_digest"
);
const RECEIPT_COLUMNS: &str = concat!(
    "active_successor_receipt_id,active_successor_receipt_schema,receipt_digest,receipt_json,",
    "canonicalization,digest_algorithm,provider_binding_id,activation_root_digest,",
    "successor_sequence,predecessor_active_successor_receipt_id,",
    "predecessor_active_successor_receipt_digest,activation_root_json,provider_id,",
    "provider_owner_account_id,source_registering_provider_id,",
    "source_registering_provider_policy_revision,source_registering_provider_json,",
    "source_registering_provider_digest,initial_active_provider_id,",
    "initial_active_provider_policy_revision,initial_active_provider_json,",
    "initial_active_provider_digest,provider_binding_digest,registry_release_id,",
    "registry_release_digest,registry_release_material_digest,installation_receipt_id,",
    "installation_receipt_digest,installation_content_digest,candidate_id,candidate_digest,",
    "delegation_id,delegation_digest,service_actor_id,logical_adapter_id,",
    "logical_adapter_binding_digest,logical_projection_compatibility_digest,",
    "route_adapter_projection_id,profile_id,profile_digest,launch_policy_digest,target_id,",
    "target_digest,target_policy_digest,companion_id,companion_digest,",
    "supervisor_session_policy_digest,entrypoint_capsule_policy_digest,launch_image_sha256,",
    "task_protocol_profile_digest,lane_subject_digest,task_production_carrier_policy_digest,",
    "evidence_provider_id,evidence_provider_policy_revision,evidence_provider_json,",
    "evidence_provider_digest,reattestation_receipt_id,reattestation_receipt_digest,",
    "credential_observed_provider_id,credential_observed_provider_policy_revision,",
    "credential_observed_provider_json,credential_observed_provider_digest,",
    "runtime_observation_id,runtime_observation_digest,runtime_observed_provider_id,",
    "runtime_observed_provider_policy_revision,runtime_observed_provider_json,",
    "runtime_observed_provider_digest,observation_started_at,observation_completed_at,",
    "observation_expires_at,task_protocol_conformance_run_receipt_id,",
    "task_protocol_conformance_run_receipt_digest,task_protocol_conformance_expires_at,",
    "process_custody_epoch_digest,process_custody_nonce_digest,process_custody_seal_digest,",
    "activation_witness_id,activation_witness_digest,checked_at,created_at,effects_json,",
    "readiness_json,receipt_integrity_digest"
);
const REVOCATION_COLUMNS: &str = concat!(
    "active_successor_revocation_id,active_successor_revocation_schema,revocation_digest,",
    "revocation_json,canonicalization,digest_algorithm,target_active_successor_receipt_id,",
    "target_active_successor_receipt_digest,provider_binding_id,activation_root_digest,",
    "revoked_by_actor_kind,revoked_by_actor_user_id,reason_code,idempotency_scope,",
    "idempotency_key,idempotency_digest,confirmation,confirmation_digest,revoked_at,",
    "process_custody_epoch_digest,process_custody_nonce_digest,process_custody_seal_digest,",
    "effects_json,readiness_json,receipt_integrity_digest"
);
const EFFECTS: &str = "effects_json TEXT NOT NULL CHECK(effects_json='{\"activation_effect\":\"none\",\"adapter_effect\":\"none\",\"credential_effect\":\"none\",\"execution_effect\":\"none\",\"market_effect\":\"none\",\"provider_effect\":\"none\",\"route_effect\":\"none\",\"settlement_effect\":\"none\",\"usage_effect\":\"none\"}')";
const READINESS: &str = "readiness_json TEXT NOT NULL CHECK(readiness_json='{\"activation_ready\":false,\"broker_connect_ready\":false,\"execution_ready\":false,\"ipc_session_ready\":false,\"process_spawn_ready\":false,\"route_ready\":false,\"runtime_launch_ready\":false,\"secret_delivery_ready\":false,\"upstream_probe_ready\":false}')";

#[test]
fn provider_active_successor_domain_freezes_stable_root_and_projected_target() {
    for module in ["canonical", "policy", "types", "validation"] {
        assert!(DOMAIN_ROOT.contains(&format!("mod {module};")));
    }
    assert_eq!(
        rust_struct_fields(
            DOMAIN_ROOT_TYPES,
            "ExternalPoolAdapterProviderActiveSuccessorActivationRootEnvelope",
        ),
        csv(ROOT_FIELDS)
    );
    for marker in [
        "ELON-EXTERNAL-POOL-ADAPTER-PROVIDER-ACTIVE-SUCCESSOR-ACTIVATION-ROOT-V1",
        "let source_json = serde_json::to_string(source)?;",
        "let initial_json = serde_json::to_string(&initial)?;",
        "initial.status = PROVIDER_STATUS_ACTIVE.into();",
        "initial.updated_at = checked_at.into();",
        ".adapter_id = structural.route_adapter_projection_id.clone();",
    ] {
        assert!(CANONICAL.contains(marker), "canonical root lost {marker}");
    }
    for domain in [
        "ELON-EXTERNAL-POOL-ADAPTER-PROVIDER-ACTIVE-SUCCESSOR-RECEIPT-V1",
        "ELON-EXTERNAL-POOL-ADAPTER-PROVIDER-ACTIVE-SUCCESSOR-RUNTIME-OBSERVATION-V1",
        "ELON-EXTERNAL-POOL-ADAPTER-PROVIDER-ACTIVE-SUCCESSOR-REVOCATION-V1",
        "ELON-EXTERNAL-POOL-ADAPTER-PROVIDER-ACTIVE-SUCCESSOR-PRIVATE-INTEGRITY-V1",
    ] {
        assert_eq!(
            CANONICAL.matches(domain).count(),
            1,
            "domain drifted: {domain}"
        );
    }
    for marker in [
        "source.status != PROVIDER_STATUS_REGISTERING",
        "Some(root.logical_adapter_id.as_str())",
        "expected.status = PROVIDER_STATUS_ACTIVE.into();",
        ".adapter_id = root.route_adapter_projection_id.clone();",
        "provider.status != PROVIDER_STATUS_ACTIVE",
        "adapter.adapter_id != root.route_adapter_projection_id",
    ] {
        assert!(
            VALIDATION_ROOTS.contains(marker),
            "root validation lost {marker}"
        );
    }
    assert!(DOMAIN_TYPES
        .contains("pub(crate) const PROVIDER_ACTIVE_SUCCESSOR_MAX_OBSERVATION_SECONDS: i64 = 15;"));
    assert!(VALIDATION_RECEIPTS.contains("PROVIDER_ACTIVE_SUCCESSOR_MAX_OBSERVATION_SECONDS"));
    assert!(VALIDATION_RECEIPTS.contains("provider_active_successor_runtime_observation_digest"));
    assert_eq!(
        POLICY
            .matches("PROVIDER_ACTIVE_SUCCESSOR_NO_EFFECT.into()")
            .count(),
        9
    );
    assert_eq!(POLICY.matches(": false,").count(), 9);
    assert!(
        POLICY.contains("relationally_current_requires_process_custody_and_active_root_reproof")
    );
    assert!(
        CARRIER_POLICY.contains("0e2f1ee192d4701c09327a94a0a30de8fe9714c049231f8a89eeb0d4c896645b")
    );
    for marker in [
        "non_authoritative_carrier_only",
        "requires_v276_current_authority_reproof",
        "effects: \"none\"",
    ] {
        assert!(CARRIER_POLICY.contains(marker));
    }
}

#[test]
fn provider_active_successor_migration_freezes_exact_tables_views_and_guards() {
    assert_eq!(
        table_columns(
            RECEIPTS,
            "compute_external_pool_adapter_provider_active_successor_receipts",
        ),
        csv(RECEIPT_COLUMNS)
    );
    assert_eq!(
        table_columns(
            REVOCATIONS,
            "compute_external_pool_adapter_provider_active_successor_revocations",
        ),
        csv(REVOCATION_COLUMNS)
    );
    assert_eq!(csv(RECEIPT_COLUMNS).len(), 84);
    assert_eq!(csv(REVOCATION_COLUMNS).len(), 25);
    for table in [RECEIPTS, REVOCATIONS] {
        assert_eq!(table.matches(EFFECTS).count(), 1);
        assert_eq!(table.matches(READINESS).count(), 1);
    }
    let receipt_projection = source_block(
        PROJECTION,
        "v274_provider_active_successor_receipt_projection",
        "v274_provider_active_successor_revocation_projection",
    );
    let revocation_projection = source_tail(
        PROJECTION,
        "v274_provider_active_successor_revocation_projection",
    );
    let receipt_integrity = source_block(
        INTEGRITY,
        "v274_provider_active_successor_receipt_integrity",
        "v274_provider_active_successor_revocation_integrity",
    );
    let revocation_integrity = source_tail(
        INTEGRITY,
        "v274_provider_active_successor_revocation_integrity",
    );
    for (columns, projection, integrity) in [
        (RECEIPT_COLUMNS, receipt_projection, receipt_integrity),
        (
            REVOCATION_COLUMNS,
            revocation_projection,
            revocation_integrity,
        ),
    ] {
        for column in csv(columns) {
            let source =
                if column.starts_with("process_custody_") || column == "receipt_integrity_digest" {
                    integrity
                } else {
                    projection
                };
            assert!(
                source.contains(&format!("NEW.{column}")),
                "projection lost {column}"
            );
        }
    }
    assert_eq!(RECEIPTS.matches("CREATE TABLE").count(), 1);
    assert_eq!(REVOCATIONS.matches("CREATE TABLE").count(), 1);
    assert_eq!(VIEW.matches("CREATE VIEW").count(), 1);
    assert_eq!(V253_VIEW.matches("CREATE VIEW").count(), 1);
    assert!(VIEW.contains("compute_external_pool_adapter_provider_active_successor_current"));
    assert!(VIEW.contains("relationally_current_requires_process_custody_and_active_root_reproof"));
    for forbidden in [
        "registry_release_current",
        "credential_reattestation_current",
        "provider_runtime_readiness_current",
        "task_protocol_conformance_current",
    ] {
        assert!(
            !VIEW.contains(forbidden),
            "diagnostic view gained {forbidden}"
        );
    }
    for required in [
        "compute_external_pool_adapter_provider_active_successor_receipts",
        "compute_external_pool_adapter_provider_active_successor_revocations",
        "compute_providers",
        "compute_provider_versions",
    ] {
        assert!(VIEW.contains(required), "diagnostic view lost {required}");
    }
    assert_ordered(
        TABLES,
        &[
            "tables/receipts.sql",
            "tables/revocations.sql",
            "tables/indexes.sql",
        ],
    );
    assert_ordered(
        MIGRATION,
        &[
            "TransactionBehavior::Immediate",
            "tables::create(&transaction)?",
            "guards::install(&transaction)?",
            "view::install(&transaction)?",
            "v253_registering_bridge::install(&transaction)?",
            "transaction.commit()?",
        ],
    );
    let v274_guards = format!(
        "{INTEGRITY}{IMMUTABILITY}{NO_REPLACE}{PROJECTION}{LINEAGE}{STRUCTURAL_ROOTS}{PROVIDER_CREDENTIAL_ROOTS}{TASK_PROTOCOL_ROOTS}"
    );
    assert_eq!(
        v274_guards.matches("CREATE TRIGGER IF NOT EXISTS").count(),
        18
    );
    assert_eq!(
        IMMUTABILITY.matches("CREATE TRIGGER IF NOT EXISTS").count(),
        4
    );
    assert_eq!(
        NO_REPLACE.matches("CREATE TRIGGER IF NOT EXISTS").count(),
        2
    );
    let receipt_no_replace = source_block(
        NO_REPLACE,
        "v274_provider_active_successor_receipt_no_replace",
        "v274_provider_active_successor_revocation_no_replace",
    );
    for marker in [
        "old.active_successor_receipt_id=NEW.active_successor_receipt_id",
        "old.receipt_digest=NEW.receipt_digest",
        "old.provider_binding_id=NEW.provider_binding_id",
        "old.activation_root_digest=NEW.activation_root_digest",
        "old.successor_sequence=NEW.successor_sequence",
        "old.predecessor_active_successor_receipt_id=NEW.predecessor_active_successor_receipt_id",
        "old.runtime_observation_id=NEW.runtime_observation_id",
        "old.runtime_observation_digest=NEW.runtime_observation_digest",
        "old.process_custody_nonce_digest=NEW.process_custody_nonce_digest",
        "old.process_custody_seal_digest=NEW.process_custody_seal_digest",
        "old.receipt_integrity_digest=NEW.receipt_integrity_digest",
    ] {
        assert!(
            receipt_no_replace.contains(marker),
            "receipt no-replace lost {marker}"
        );
    }
    let revocation_no_replace = source_tail(
        NO_REPLACE,
        "v274_provider_active_successor_revocation_no_replace",
    );
    for marker in [
        "old.active_successor_revocation_id=NEW.active_successor_revocation_id",
        "old.revocation_digest=NEW.revocation_digest",
        "old.target_active_successor_receipt_id=NEW.target_active_successor_receipt_id",
        "old.idempotency_digest=NEW.idempotency_digest",
        "old.process_custody_nonce_digest=NEW.process_custody_nonce_digest",
        "old.process_custody_seal_digest=NEW.process_custody_seal_digest",
        "old.receipt_integrity_digest=NEW.receipt_integrity_digest",
    ] {
        assert!(
            revocation_no_replace.contains(marker),
            "revocation no-replace lost {marker}"
        );
    }
    for marker in [
        "BEFORE UPDATE ON compute_external_pool_adapter_provider_active_successor_receipts",
        "BEFORE DELETE ON compute_external_pool_adapter_provider_active_successor_receipts",
        "BEFORE UPDATE ON compute_external_pool_adapter_provider_active_successor_revocations",
        "BEFORE DELETE ON compute_external_pool_adapter_provider_active_successor_revocations",
    ] {
        assert!(IMMUTABILITY.contains(marker), "immutability lost {marker}");
    }
    assert_eq!(V253_CHALLENGE_ROOTS.matches("CREATE TRIGGER ").count(), 1);
    assert_eq!(V253_RECEIPT_ROOTS.matches("CREATE TRIGGER ").count(), 1);
    assert!(!format!("{MIGRATION}{TABLES}{RECEIPTS}{REVOCATIONS}{VIEW}")
        .contains("INSERT INTO compute_external_pool_adapter_provider_active_successor_"));
    assert!(STORE_MIGRATIONS.contains("register_v274_receipt_integrity_functions"));
    assert!(STORE_MIGRATIONS.contains("provider_active_successor::migration_v274"));
    assert_ordered(
        STORE_SCHEMA,
        &[
            "register_v273_receipt_integrity_functions(conn)?",
            "register_v274_receipt_integrity_functions(conn)?",
            "CREATE TABLE IF NOT EXISTS schema_migrations",
        ],
    );
}

fn csv(value: &str) -> Vec<&str> {
    value.split(',').collect()
}

fn table_columns<'a>(source: &'a str, table: &str) -> Vec<&'a str> {
    let start = format!("CREATE TABLE IF NOT EXISTS {table} (");
    source
        .split_once(&start)
        .unwrap()
        .1
        .split_once("\n);")
        .unwrap()
        .0
        .lines()
        .filter_map(|line| {
            let mut words = line.trim().trim_end_matches(',').split_whitespace();
            let name = words.next()?;
            match words.next()? {
                "TEXT" | "INTEGER" | "BLOB" | "REAL" => Some(name),
                _ => None,
            }
        })
        .collect()
}

fn rust_struct_fields<'a>(source: &'a str, name: &str) -> Vec<&'a str> {
    source
        .split_once(&format!("pub(crate) struct {name} {{"))
        .unwrap()
        .1
        .split_once("\n}")
        .unwrap()
        .0
        .lines()
        .filter_map(|line| {
            line.trim()
                .strip_prefix("pub ")
                .and_then(|line| line.split_once(':').map(|item| item.0))
        })
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

fn source_tail<'a>(source: &'a str, start: &str) -> &'a str {
    source.split_once(start).unwrap().1
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
