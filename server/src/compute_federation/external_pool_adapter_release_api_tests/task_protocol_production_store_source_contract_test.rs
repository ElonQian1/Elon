const STORE_ROOT: &str = include_str!("../../store.rs");
const STORE: &str = include_str!("../../store/compute_external_pool_adapter_task_delivery.rs");
const COLUMNS: &str =
    include_str!("../../store/compute_external_pool_adapter_task_delivery/columns.rs");
const TYPES: &str =
    include_str!("../../store/compute_external_pool_adapter_task_delivery/types.rs");
const READ: &str = include_str!("../../store/compute_external_pool_adapter_task_delivery/read.rs");
const POLLS: &str =
    include_str!("../../store/compute_external_pool_adapter_task_delivery/polls.rs");
const RECOVERY: &str =
    include_str!("../../store/compute_external_pool_adapter_task_delivery/recovery.rs");
const RECOVERY_AUDIT: &str =
    include_str!("../../store/compute_external_pool_adapter_task_delivery/recovery/audit.rs");
const MAPPING_EXCHANGE: &str =
    include_str!("../../store/compute_external_pool_adapter_task_delivery/mapping/exchange.rs");
const MAPPING_POLLS: &str =
    include_str!("../../store/compute_external_pool_adapter_task_delivery/mapping/polls.rs");
const MAPPING_EVENTS: &str =
    include_str!("../../store/compute_external_pool_adapter_task_delivery/mapping/events.rs");
const ATTEMPTS: &str = include_str!(
    "../../store_migrations/compute_external_pool_adapter_task_delivery/tables/exchange_attempts.sql"
);
const RECEIPTS: &str = include_str!(
    "../../store_migrations/compute_external_pool_adapter_task_delivery/tables/exchange_receipts.sql"
);
const POLL_TABLES: &str = include_str!(
    "../../store_migrations/compute_external_pool_adapter_task_delivery/tables/polls.sql"
);
const EVENT_TABLES: &str = include_str!(
    "../../store_migrations/compute_external_pool_adapter_task_delivery/tables/events.sql"
);

#[test]
fn task_protocol_production_store_audits_every_durable_column_in_order() {
    for (table_source, table, rust_array, expected) in [
        (
            ATTEMPTS,
            "compute_external_pool_adapter_task_exchange_attempts",
            "EXCHANGE_ATTEMPT_COLUMNS",
            52,
        ),
        (
            RECEIPTS,
            "compute_external_pool_adapter_task_exchange_receipts",
            "EXCHANGE_RECEIPT_COLUMNS",
            65,
        ),
        (
            POLL_TABLES,
            "compute_external_pool_adapter_task_reconcile_polls",
            "RECONCILE_POLL_COLUMNS",
            39,
        ),
        (
            POLL_TABLES,
            "compute_external_pool_adapter_task_event_polls",
            "EVENT_POLL_COLUMNS",
            42,
        ),
        (
            EVENT_TABLES,
            "compute_external_pool_adapter_task_event_batches",
            "EVENT_BATCH_COLUMNS",
            35,
        ),
        (
            EVENT_TABLES,
            "compute_external_pool_adapter_task_events",
            "EVENT_COLUMNS",
            21,
        ),
    ] {
        let ddl = table_columns(table_source, table);
        let store = rust_columns(COLUMNS, rust_array);
        assert_eq!(ddl.len(), expected, "DDL count drifted for {table}");
        assert_eq!(store.len(), expected, "Store count drifted for {table}");
        assert_eq!(ddl, store, "Store column order drifted for {table}");
    }

    for (mapping, count) in [
        (MAPPING_EXCHANGE, 52),
        (MAPPING_EXCHANGE, 65),
        (MAPPING_POLLS, 39),
        (MAPPING_POLLS, 42),
        (MAPPING_EVENTS, 35),
        (MAPPING_EVENTS, 21),
    ] {
        assert!(
            mapping.contains(&format!("values.len() == {count}")),
            "Store mapping lost {count}-column readback"
        );
    }
    assert_eq!(READ.matches("ensure_exact(").count(), 7);
    for validator in [
        "validate_task_production_exchange_attempt",
        "validate_task_production_exchange_receipt",
        "validate_task_production_reconcile_poll",
        "validate_task_production_event_poll",
        "validate_task_production_event_batch",
        "validate_task_production_event",
    ] {
        assert!(READ.contains(validator), "Store read lost {validator}");
    }
}

#[test]
fn task_protocol_production_store_recovers_only_exact_poll_claim_projection() {
    assert!(STORE_ROOT.contains("mod compute_external_pool_adapter_task_delivery;"));
    assert!(!STORE_ROOT.contains("use compute_external_pool_adapter_task_delivery::"));
    assert_ordered(
        STORE,
        &[
            "TransactionBehavior::Immediate",
            "recovery::recover_on(&transaction)?",
            "let eligible_rows = report.eligible_rows;",
            "transaction.commit()?",
            "Ok(eligible_rows)",
        ],
    );
    assert!(STORE.contains("-> Result<usize>"));
    assert!(TYPES.contains("pub(super) struct ExternalPoolAdapterTaskDeliveryRecoveryReport"));
    let writers = format!("{POLLS}{RECOVERY}");
    assert_eq!(
        writers
            .matches("UPDATE compute_external_pool_adapter_task_")
            .count(),
        4
    );
    assert!(!writers.contains("INSERT INTO compute_external_pool_adapter_task_"));
    assert!(!writers.contains("DELETE FROM compute_external_pool_adapter_task_"));
    for update in update_sets(&writers) {
        assert_eq!(update.split(',').count(), 6, "poll UPDATE gained a column");
        for column in [
            "claim_status",
            "claim_revision",
            "claim_generation",
            "claim_owner_id",
            "claim_token_digest",
            "claim_expires_at",
        ] {
            assert!(update.contains(column), "poll UPDATE lost {column}");
        }
    }
    for marker in [
        "read_reconcile_poll_on(conn, poll_id)?",
        "read_event_poll_on(conn, poll_id)?",
        "finish_claim(",
        "V273 poll claim readback is not exact",
    ] {
        assert!(POLLS.contains(marker), "claim CAS lost {marker}");
    }
    assert_ordered(
        RECOVERY,
        &[
            "read_reconcile_poll_on(conn, &id)?",
            "reconcile_target_on(conn, &id)?",
            "audit_reconcile_target_on(conn, &before, target)?",
            "transition_reconcile_on(",
            "read_reconcile_poll_on(conn, &id)?",
            "ensure_recovered(&before.claim, &after.claim, target)?",
        ],
    );
    assert_eq!(
        RECOVERY.matches("poll.claim_expires_at<=strftime").count(),
        8
    );
    assert_eq!(
        RECOVERY
            .matches(
                "AND EXISTS (SELECT 1 FROM compute_external_pool_adapter_task_exchange_attempts attempt"
            )
            .count(),
        4
    );
    assert_ordered(
        RECOVERY,
        &[
            "read_event_poll_on(conn, &id)?",
            "event_target_on(conn, &id)?",
            "audit_event_target_on(conn, &before, target)?",
            "transition_event_on(",
            "read_event_poll_on(conn, &id)?",
            "ensure_recovered(&before.claim, &after.claim, target)?",
        ],
    );
}

#[test]
fn task_protocol_production_store_reproves_sources_before_transition() {
    for marker in [
        "audit_unknown_attempt_on(",
        "ensure_no_attempt_receipt_on(",
        "audit_receipt_attempt_on(conn, &receipt)?",
        "read_event_batch_on(conn, &batch_id)?",
        "read_exchange_receipt_on(conn, &batch.batch.exchange_receipt_id)?",
        "read_event_on(conn, event_id)?",
        "TASK_PRODUCTION_MAX_EVENTS_PER_BATCH",
        "validate_task_production_event_remote_state_transition(",
        "receipt.receipt.semantic_observation_sha256",
        "event.event.event_ordinal == expected_ordinal",
        "event.event.event_root == expected_root.as_str()",
    ] {
        assert!(
            RECOVERY_AUDIT.contains(marker),
            "recovery source audit lost {marker}"
        );
    }
    assert!(RECOVERY.contains("report.eligible_rows = 0;"));
    let production = format!("{STORE}{TYPES}{READ}{POLLS}{RECOVERY}{RECOVERY_AUDIT}");
    for forbidden in [
        "exchange_external_pool_adapter_broker_task",
        "begin_application_exchange",
        "PreparedStartSendRequest",
        "CommittedStartSendAuthority",
        "VerifiedComputeStartOutboxRemoteObservation",
        "persist_route_authority_on",
        "INSERT INTO compute_external_pool_adapter_task_exchange_attempts",
        "INSERT INTO compute_external_pool_adapter_task_exchange_receipts",
        "INSERT INTO compute_external_pool_adapter_task_event_batches",
        "INSERT INTO compute_external_pool_adapter_task_events",
    ] {
        assert!(!production.contains(forbidden), "Store gained {forbidden}");
    }

    let claim = source_block(
        TYPES,
        "/// Process-local scheduling custody only. It is deliberately non-Clone/non-Debug/non-Serde.",
        "#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]",
    );
    assert!(claim.contains("pub(in crate::store) struct ExternalPoolAdapterTaskPollClaim"));
    assert!(claim.contains("raw_claim_token: String"));
    for forbidden in ["#[derive", "Clone", "Debug", "Serialize", "Deserialize"] {
        assert!(
            !claim.contains(forbidden),
            "private claim gained {forbidden}"
        );
    }
    assert!(!COLUMNS.contains("raw_claim_token"));
    assert!(!format!("{ATTEMPTS}{RECEIPTS}{POLL_TABLES}{EVENT_TABLES}").contains("raw_claim_token"));
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

fn rust_columns<'a>(source: &'a str, name: &str) -> Vec<&'a str> {
    source
        .split_once(&format!("const {name}:"))
        .unwrap()
        .1
        .split_once("[")
        .unwrap()
        .1
        .split_once("];")
        .unwrap()
        .0
        .lines()
        .filter_map(|line| {
            let value = line.trim().trim_end_matches(',');
            value
                .strip_prefix('"')
                .and_then(|value| value.strip_suffix('"'))
        })
        .collect()
}

fn update_sets(source: &str) -> Vec<&str> {
    source
        .split("UPDATE compute_external_pool_adapter_task_")
        .skip(1)
        .map(|update| {
            update
                .split_once("SET ")
                .unwrap()
                .1
                .split_once("WHERE")
                .unwrap()
                .0
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

fn assert_ordered(source: &str, needles: &[&str]) {
    let mut cursor = 0;
    for needle in needles {
        let offset = source[cursor..]
            .find(needle)
            .unwrap_or_else(|| panic!("missing ordered source marker {needle}"));
        cursor += offset + needle.len();
    }
}
