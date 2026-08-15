const FACADE: &str = include_str!("../external_pool_adapter_task_protocol_production.rs");
const TYPES: &str = include_str!("../external_pool_adapter_task_protocol_production/types.rs");
const TYPE_COMMON: &str =
    include_str!("../external_pool_adapter_task_protocol_production/types/common.rs");
const TYPE_POLLS: &str =
    include_str!("../external_pool_adapter_task_protocol_production/types/polls.rs");
const CANONICAL: &str =
    include_str!("../external_pool_adapter_task_protocol_production/canonical.rs");
const LANE: &str = include_str!("../external_pool_adapter_task_protocol_production/lane.rs");
const VALIDATE_EXCHANGE: &str =
    include_str!("../external_pool_adapter_task_protocol_production/validation/exchange.rs");
const VALIDATE_POLLS: &str =
    include_str!("../external_pool_adapter_task_protocol_production/validation/polls.rs");
const VALIDATE_EVENTS: &str =
    include_str!("../external_pool_adapter_task_protocol_production/validation/events.rs");
const MIGRATION: &str =
    include_str!("../../store_migrations/compute_external_pool_adapter_task_delivery.rs");
const TABLE_INSTALLER: &str =
    include_str!("../../store_migrations/compute_external_pool_adapter_task_delivery/tables.rs");
const ATTEMPTS: &str = include_str!(
    "../../store_migrations/compute_external_pool_adapter_task_delivery/tables/exchange_attempts.sql"
);
const RECEIPTS: &str = include_str!(
    "../../store_migrations/compute_external_pool_adapter_task_delivery/tables/exchange_receipts.sql"
);
const POLLS: &str = include_str!(
    "../../store_migrations/compute_external_pool_adapter_task_delivery/tables/polls.sql"
);
const EVENTS: &str = include_str!(
    "../../store_migrations/compute_external_pool_adapter_task_delivery/tables/events.sql"
);
const INDEXES: &str = include_str!(
    "../../store_migrations/compute_external_pool_adapter_task_delivery/tables/indexes.sql"
);
const GUARD_INSTALLER: &str =
    include_str!("../../store_migrations/compute_external_pool_adapter_task_delivery/guards.rs");
const INTEGRITY: &str = include_str!(
    "../../store_migrations/compute_external_pool_adapter_task_delivery/receipt_integrity.rs"
);
const IMMUTABILITY: &str = include_str!(
    "../../store_migrations/compute_external_pool_adapter_task_delivery/guards/immutability.sql"
);
const NO_REPLACE: &str = include_str!(
    "../../store_migrations/compute_external_pool_adapter_task_delivery/guards/no_replace.sql"
);
const POLL_CLAIMS: &str = include_str!(
    "../../store_migrations/compute_external_pool_adapter_task_delivery/guards/poll_claims.sql"
);
const PROJECTION: &str = include_str!(
    "../../store_migrations/compute_external_pool_adapter_task_delivery/guards/projection.rs"
);
const PROJECTION_EXCHANGE: &str = include_str!(
    "../../store_migrations/compute_external_pool_adapter_task_delivery/guards/projection/exchange.rs"
);
const PROJECTION_POLLS: &str = include_str!(
    "../../store_migrations/compute_external_pool_adapter_task_delivery/guards/projection/polls.rs"
);
const PROJECTION_EVENTS: &str = include_str!(
    "../../store_migrations/compute_external_pool_adapter_task_delivery/guards/projection/events.rs"
);
const SOURCE_LINEAGE: &str = include_str!(
    "../../store_migrations/compute_external_pool_adapter_task_delivery/guards/source_lineage.rs"
);
const ROUTE_AUTHORITY: &str = include_str!(
    "../../store_migrations/compute_external_pool_adapter_task_delivery/guards/route_authority.rs"
);
const EVENT_LINEAGE: &str = include_str!(
    "../../store_migrations/compute_external_pool_adapter_task_delivery/guards/event_lineage.rs"
);
const STORE_MIGRATIONS: &str = include_str!("../../store_migrations.rs");
const STORE_SCHEMA: &str = include_str!("../../store_schema.rs");

const EFFECTS: &str = "effects_json TEXT NOT NULL CHECK(effects_json='{\"activation_effect\":\"none\",\"adapter_effect\":\"none\",\"credential_effect\":\"none\",\"execution_effect\":\"none\",\"market_effect\":\"none\",\"provider_effect\":\"none\",\"route_effect\":\"none\",\"settlement_effect\":\"none\",\"usage_effect\":\"none\"}')";
const READINESS: &str = "readiness_json TEXT NOT NULL CHECK(readiness_json='{\"activation_ready\":false,\"broker_connect_ready\":false,\"execution_ready\":false,\"ipc_session_ready\":false,\"process_spawn_ready\":false,\"route_ready\":false,\"runtime_launch_ready\":false,\"secret_delivery_ready\":false,\"upstream_probe_ready\":false}')";

#[test]
fn task_protocol_production_domain_freezes_six_evidence_envelopes() {
    for module in [
        "mod canonical;",
        "mod lane;",
        "mod session;",
        "mod types;",
        "mod validation;",
    ] {
        assert!(FACADE.contains(module), "Domain facade lost {module}");
    }
    for schema in [
        "compute_federation.external_pool_adapter_task_exchange_attempt.v1",
        "compute_federation.external_pool_adapter_task_exchange_receipt.v1",
        "compute_federation.external_pool_adapter_task_reconcile_poll.v1",
        "compute_federation.external_pool_adapter_task_event_poll.v1",
        "compute_federation.external_pool_adapter_task_event_batch.v1",
        "compute_federation.external_pool_adapter_task_event.v1",
    ] {
        assert_eq!(TYPES.matches(schema).count(), 1, "schema drifted: {schema}");
    }
    for domain in [
        "ELON-EXTERNAL-POOL-ADAPTER-TASK-EXCHANGE-ATTEMPT-V1",
        "ELON-EXTERNAL-POOL-ADAPTER-TASK-EXCHANGE-RECEIPT-V1",
        "ELON-EXTERNAL-POOL-ADAPTER-TASK-RECONCILE-POLL-V1",
        "ELON-EXTERNAL-POOL-ADAPTER-TASK-EVENT-POLL-V1",
        "ELON-EXTERNAL-POOL-ADAPTER-TASK-EVENT-BATCH-V1",
        "ELON-EXTERNAL-POOL-ADAPTER-TASK-EVENT-V1",
        "ELON-EXTERNAL-POOL-ADAPTER-TASK-REMOTE-IDENTITY-V1",
        "ELON-EXTERNAL-POOL-ADAPTER-TASK-EVENT-ROOT-V1",
        "ELON-EXTERNAL-POOL-ADAPTER-TASK-EVENT-BATCH-ROOT-V1",
    ] {
        assert_eq!(
            CANONICAL.matches(domain).count(),
            1,
            "domain drifted: {domain}"
        );
    }
    assert!(LANE.contains("ELON-EXTERNAL-POOL-PRODUCTION-LANE-SUBJECT-V1"));
    assert!(LANE.contains("This type deliberately has no executor/string conversion"));
    assert!(!LANE.contains("executor_id("));
    assert!(TYPES.contains("production_transport_evidence_no_v213_authority"));
    assert_eq!(
        TYPE_COMMON
            .matches("TASK_PRODUCTION_NO_EFFECT.into()")
            .count(),
        9
    );
    assert_eq!(TYPE_COMMON.matches(": false,").count(), 9);
    assert!(TYPE_COMMON.contains("#[serde(deny_unknown_fields)]"));
}

#[test]
fn task_protocol_production_domain_freezes_typed_unknown_and_event_semantics() {
    assert!(TYPE_POLLS.contains("pub authenticated_subject_sha256: Option<String>"));
    for marker in [
        "poll.remote.executor_binding_digest != poll.command.executor_binding_digest",
        "canonical_task_production_remote_subject_json_and_sha256",
        "task production reconcile remote subject was not authenticated",
    ] {
        assert!(
            VALIDATE_POLLS.contains(marker),
            "poll validation lost {marker}"
        );
    }
    for marker in [
        "batch.event_count == 0 && batch.replay_classification == \"empty\"",
        "batch.event_count > 0 && batch.replay_classification == \"new\"",
        "task_production_event_inventory_digest(&batch.event_roots)?",
        "canonical_task_production_authenticated_event_observation_json_and_sha256",
        "task_production_event_batch_root(batch)?",
        "validate_task_production_event_remote_state_transition",
        "task production event remote state transition is not monotonic",
        "batch.event_count > TASK_PRODUCTION_MAX_EVENTS_PER_BATCH",
        "event.event_ordinal > TASK_PRODUCTION_MAX_EVENTS_PER_BATCH",
    ] {
        assert!(
            VALIDATE_EVENTS.contains(marker),
            "event validation lost {marker}"
        );
    }
    assert!(CANONICAL.contains("executor_binding_digest: &'a str"));
    assert!(CANONICAL.contains("remote_identity_digest: &value.remote_identity_digest"));
    assert!(
        VALIDATE_EXCHANGE.contains("value.session_transcript_digest != value.session_roots_digest")
    );
    assert!(
        VALIDATE_EXCHANGE.contains("task production authenticated session transcript is not exact")
    );
    assert!(TYPES.contains("pub(crate) const TASK_PRODUCTION_MAX_EVENTS_PER_BATCH: u64 = 256;"));
    assert!(EVENTS.contains("event_count BETWEEN 0 AND 256"));
    assert!(EVENTS.contains("event_ordinal BETWEEN 1 AND 256"));
    assert!(ATTEMPTS.contains("session_transcript_digest=session_roots_digest"));
    assert!(RECEIPTS.contains("session_transcript_digest=session_roots_digest"));
}

#[test]
fn task_protocol_production_migration_freezes_six_tables_zero_views_and_udfs() {
    assert_eq!(
        table_columns(
            ATTEMPTS,
            "compute_external_pool_adapter_task_exchange_attempts"
        )
        .len(),
        52
    );
    assert_eq!(
        table_columns(
            RECEIPTS,
            "compute_external_pool_adapter_task_exchange_receipts"
        )
        .len(),
        65
    );
    assert_eq!(
        table_columns(POLLS, "compute_external_pool_adapter_task_reconcile_polls").len(),
        39
    );
    assert_eq!(
        table_columns(POLLS, "compute_external_pool_adapter_task_event_polls").len(),
        42
    );
    assert_eq!(
        table_columns(EVENTS, "compute_external_pool_adapter_task_event_batches").len(),
        35
    );
    assert_eq!(
        table_columns(EVENTS, "compute_external_pool_adapter_task_events").len(),
        21
    );
    let ddl = format!("{ATTEMPTS}{RECEIPTS}{POLLS}{EVENTS}{INDEXES}");
    assert_eq!(ddl.matches("CREATE TABLE").count(), 6);
    assert_eq!(ddl.matches("CREATE VIEW").count(), 0);
    assert_eq!(TABLE_INSTALLER.matches("include_str!(\"tables/").count(), 5);
    assert_ordered(
        TABLE_INSTALLER,
        &[
            "exchange_attempts.sql",
            "exchange_receipts.sql",
            "polls.sql",
            "events.sql",
            "indexes.sql",
        ],
    );
    assert_eq!(INTEGRITY.matches("conn.create_scalar_function").count(), 6);
    assert_eq!(INTEGRITY.matches("CREATE TRIGGER IF NOT EXISTS").count(), 6);
    assert!(MIGRATION.contains("TransactionBehavior::Immediate"));
    assert_ordered(
        MIGRATION,
        &[
            "tables::create(&transaction)?",
            "guards::install(&transaction)?",
            "transaction.commit()?",
        ],
    );
    assert_eq!(
        STORE_MIGRATIONS
            .matches("register_v273_receipt_integrity_functions")
            .count(),
        1
    );
    assert_eq!(
        STORE_MIGRATIONS
            .matches("compute_external_pool_adapter_task_delivery::migration_v273")
            .count(),
        1
    );
    assert_ordered(
        STORE_SCHEMA,
        &[
            "register_v272_receipt_integrity_functions(conn)?",
            "register_v273_receipt_integrity_functions(conn)?",
            "CREATE TABLE IF NOT EXISTS schema_migrations",
        ],
    );
    for table in [ATTEMPTS, RECEIPTS, POLLS, EVENTS] {
        let table_count = table.matches("CREATE TABLE").count();
        assert_eq!(table.matches(EFFECTS).count(), table_count);
        assert_eq!(table.matches(READINESS).count(), table_count);
    }
}

#[test]
fn task_protocol_production_migration_freezes_guards_and_recovery_boundaries() {
    assert_ordered(
        GUARD_INSTALLER,
        &[
            "guards/immutability.sql",
            "guards/no_replace.sql",
            "receipt_integrity::install(conn)?",
            "projection::install(conn)?",
            "source_lineage::install(conn)?",
            "route_authority::install(conn)?",
            "event_lineage::install(conn)?",
            "guards/poll_claims.sql",
        ],
    );
    let fixed_guards = format!(
        "{INTEGRITY}{IMMUTABILITY}{NO_REPLACE}{POLL_CLAIMS}{SOURCE_LINEAGE}{ROUTE_AUTHORITY}{EVENT_LINEAGE}"
    );
    let projection_calls = format!("{PROJECTION_EXCHANGE}{PROJECTION_POLLS}{PROJECTION_EVENTS}")
        .matches("install_projection(")
        .count();
    assert!(PROJECTION.contains("CREATE TRIGGER IF NOT EXISTS {trigger}"));
    assert_eq!(
        fixed_guards.matches("CREATE TRIGGER IF NOT EXISTS").count() + projection_calls,
        45
    );
    assert_eq!(
        NO_REPLACE.matches("CREATE TRIGGER IF NOT EXISTS").count(),
        6
    );
    for table in [
        "compute_external_pool_adapter_task_exchange_attempts",
        "compute_external_pool_adapter_task_exchange_receipts",
        "compute_external_pool_adapter_task_reconcile_polls",
        "compute_external_pool_adapter_task_event_polls",
        "compute_external_pool_adapter_task_event_batches",
        "compute_external_pool_adapter_task_events",
    ] {
        assert!(NO_REPLACE.contains(table), "no-replace lost {table}");
        assert!(IMMUTABILITY.contains(table), "immutability lost {table}");
    }
    for marker in [
        "NEW.started_at<poll.claim_expires_at",
        "NEW.claim_status='pending'",
        "NEW.claim_status='in_flight_unknown'",
        "NOT EXISTS(SELECT 1 FROM compute_external_pool_adapter_task_exchange_receipts",
        "NEW.claim_status='delivery_observed'",
    ] {
        assert!(
            POLL_CLAIMS.contains(marker) || SOURCE_LINEAGE.contains(marker),
            "recovery guard lost {marker}"
        );
    }
    for marker in [
        "compute_external_pool_adapter_credential_reattestation_receipts",
        "compute_external_pool_adapter_supervisor_session_policy_companion_current",
        "compute_external_pool_adapter_task_protocol_conformance_run_receipts",
        "NEW.operation_kind IN ('cancel_no_start','reconcile','authenticated_events')",
        "receipt.semantic_observation_sha256=NEW.authenticated_subject_sha256",
        "attempt.operation_kind IN ('prepare','idempotent_commit','cancel_no_start')",
        "attempt.operation_kind IN ('idempotent_commit','cancel_no_start')",
        "attempt.operation_kind='cancel_no_start'",
        "receipt.operation_kind='cancel_no_start'",
    ] {
        assert!(
            SOURCE_LINEAGE.contains(marker),
            "source authority lost {marker}"
        );
    }
    for marker in [
        "compute_route_adapters current_adapter",
        "compute_service_actor_authorizations actor_authority",
        "compute_route_authorization_seals seal",
        "compute_route_authorization_capabilities capability",
        "NEW.operation_kind IN ('cancel_no_start','reconcile','authenticated_events')",
    ] {
        assert!(
            ROUTE_AUTHORITY.contains(marker),
            "route authority lost {marker}"
        );
    }
    assert!(EVENT_LINEAGE.contains("replay_classification='empty'"));
    assert!(EVENT_LINEAGE.contains("replay_classification='new'"));
    assert!(EVENT_LINEAGE.contains("poll.remote_execution_state='committed'"));
    assert!(EVENT_LINEAGE
        .contains("NEW.remote_execution_state IN ('committed','running','terminal_after_run')"));
    assert!(EVENT_LINEAGE.contains("poll.remote_execution_state='running'"));
    assert!(
        EVENT_LINEAGE.contains("NEW.remote_execution_state IN ('running','terminal_after_run')")
    );
    assert!(EVENT_LINEAGE.contains("remote_event_id"));
    assert!(EVENT_LINEAGE.contains("predecessor_poll.poll_ordinal<later_poll.poll_ordinal"));
    assert!(EVENT_LINEAGE.contains("later_poll.poll_ordinal<current_poll.poll_ordinal"));
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

fn assert_ordered(source: &str, needles: &[&str]) {
    let mut cursor = 0;
    for needle in needles {
        let offset = source[cursor..]
            .find(needle)
            .unwrap_or_else(|| panic!("missing ordered source marker {needle}"));
        cursor += offset + needle.len();
    }
}
