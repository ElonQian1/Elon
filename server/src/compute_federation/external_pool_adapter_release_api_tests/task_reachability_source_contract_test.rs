macro_rules! delivery_source {
    ($path:literal) => {
        include_str!(concat!(
            "../../store/compute_external_pool_adapter_task_delivery/",
            $path
        ))
    };
}

macro_rules! worker_source {
    ($path:literal) => {
        include_str!(concat!("../external_pool_adapter_task_worker/", $path))
    };
}

const DELIVERY_ROOT: &str =
    include_str!("../../store/compute_external_pool_adapter_task_delivery.rs");
const PLAN: &str = delivery_source!("reachability_pending_plan.rs");
const GUARDS: &str = include_str!(
    "../../store_migrations/compute_external_pool_adapter_task_delivery/guards/reachability.rs"
);
const OUTBOUND: &str = delivery_source!("outbound.rs");
const WRITES: &str = delivery_source!("write.rs");
const FIRST_RECONCILE: &str = delivery_source!("first_reconcile.rs");
const FIRST_CANCEL_RECONCILE: &str = delivery_source!("first_reconcile_cancel.rs");
const RETRY_POLLS: &str = delivery_source!("retry_polls.rs");
const HISTORICAL_CLEANUP: &str = delivery_source!("historical_cleanup.rs");
const TERMINAL: &str = delivery_source!("terminal_ingress.rs");
const RECEIPT_INGRESS: &str = delivery_source!("receipt_ingress.rs");
const EVENT_INGRESS: &str = delivery_source!("event_ingress.rs");
const RECONCILE_INGRESS: &str = delivery_source!("reconcile_ingress.rs");
const RECONCILE_REPLAY: &str = delivery_source!("reconcile_replay.rs");
const NO_START_INGRESS: &str = delivery_source!("no_start_ingress.rs");
const INGRESS_OBLIGATION: &str = delivery_source!("ingress_obligation.rs");
const ACK_WRITE: &str = include_str!("../../store/compute_attempt_dispatches/ack_write.rs");
const ACCEPTED_CURRENTNESS: &str =
    include_str!("../../store/compute_attempt_start_outbox/accepted_commit/currentness.rs");
const ACCEPTED_DERIVE: &str =
    include_str!("../../store/compute_attempt_start_outbox/accepted_commit/derive.rs");
const ACCEPTED_PERSIST: &str =
    include_str!("../../store/compute_attempt_start_outbox/accepted_commit/persist.rs");
const ACCEPTED_PLAN: &str =
    include_str!("../../store/compute_attempt_start_outbox/accepted_commit/historical_plan.rs");
const HISTORICAL_ACCEPTED_MIGRATION: &str = include_str!(
    "../../store_migrations/compute_external_pool_adapter_route_renewal/historical_accepted.rs"
);
const HISTORICAL_ACCEPTED_SOURCE: &str = include_str!(
    "../../store_migrations/compute_external_pool_adapter_route_renewal/historical_accepted/source.rs"
);
const HISTORICAL_POLL_AUTHORITY: &str = include_str!(
    "../../store_migrations/compute_external_pool_adapter_route_renewal/historical_poll_exchange.rs"
);
const ROUTE_RENEWAL_MIGRATION: &str =
    include_str!("../../store_migrations/compute_external_pool_adapter_route_renewal.rs");
const BROKER_RELAY: &str = include_str!("../external_pool_adapter_broker_tls/task_protocol.rs");
const BROKER_TYPES: &str =
    include_str!("../external_pool_adapter_broker_tls/task_protocol_types.rs");
const STORE_RELAY: &str =
    include_str!("../../store/compute_external_pool_adapter_runtime_bundle/task_delivery.rs");
const WORKER_ROOT: &str = include_str!("../external_pool_adapter_task_worker.rs");
const WORKER_CYCLE: &str = worker_source!("cycle.rs");

#[test]
fn reachability_plan_is_ordered_connection_local_and_appended_to_existing_guards() {
    for marker in [
        "const MAX_ORDERED_WRITES: usize = 263",
        "active.writes.get(active.next_index)",
        "active.next_index += 1",
        "ensure_fully_consumed",
        "create_scalar_function(\n        REACHABILITY_PENDING_PLAN_MATCHES,\n        -1",
        "FunctionFlags::SQLITE_UTF8 | FunctionFlags::SQLITE_INNOCUOUS",
    ] {
        assert!(PLAN.contains(marker), "reachability plan lost {marker}");
    }
    assert!(!PLAN.contains("SQLITE_DETERMINISTIC"));
    let existing_guards = [
        "v273_task_exchange_attempt_no_replace",
        "v273_task_exchange_receipt_no_replace",
        "v273_task_reconcile_poll_no_replace",
        "v273_task_event_poll_no_replace",
        "v273_task_event_batch_no_replace",
        "v273_task_event_no_replace",
        "trg_compute_attempt_start_send_no_replace",
        "trg_compute_attempt_start_outbox_transition",
        "v273_task_reconcile_poll_claim_cas",
        "v273_task_event_poll_claim_cas",
    ];
    for name in existing_guards {
        assert!(GUARDS.contains(name), "reachability guard lost {name}");
    }
    assert!(GUARDS.contains("sql.contains(\"WHEN\") && sql.contains(\"BEFORE \")"));
    assert!(GUARDS.contains("let replacement = format!(\"{}OR ({condition})\\n{}\""));
    assert!(GUARDS.contains("installed.matches(UDF).count() == 1"));
    assert_eq!(GUARDS.matches("v278_task_reachability_").count(), 10);
    assert!(GUARDS.contains("remove_legacy_parallel_guards(conn)?"));
}

#[test]
fn outbound_and_ledger_writers_share_one_exact_transaction_plan() {
    assert_ordered(
        OUTBOUND,
        &[
            "prepare_send_started_at_on",
            "ExternalPoolAdapterTaskReachabilityPendingPlan::new",
            "StartSendAttempt",
            "ExchangeAttempt",
            "StartOutboxCas",
            "install_external_pool_adapter_task_reachability_pending_plan_on",
            "insert_prepared_send_started_on",
            "insert_external_pool_adapter_task_exchange_attempt_on",
            "finish_prepared_send_started_on",
            "plan.ensure_fully_consumed()?",
        ],
    );
    for writer in [
        "insert_external_pool_adapter_task_exchange_attempt_on",
        "insert_external_pool_adapter_task_exchange_receipt_on",
        "insert_external_pool_adapter_task_reconcile_poll_on",
        "insert_external_pool_adapter_task_event_poll_on",
        "insert_external_pool_adapter_task_event_batch_on",
        "insert_external_pool_adapter_task_event_on",
    ] {
        assert!(WRITES.contains(writer), "ledger writer lost {writer}");
    }
    assert!(WRITES.contains("ExternalPoolAdapterTaskLedgerWriteDisposition::ExactReplay"));
    assert!(WRITES.contains("ExternalPoolAdapterTaskLedgerWriteDisposition::Inserted"));
    assert!(FIRST_RECONCILE.contains("predecessor_id: None"));
    assert!(FIRST_RECONCILE.contains("predecessor_digest: None"));
    assert!(FIRST_RECONCILE.contains("poll_ordinal: 1"));
    assert!(FIRST_RECONCILE.contains("pending.ensure_fully_consumed()?"));
    assert!(RETRY_POLLS.contains("checked_add(1)"));
    assert!(RETRY_POLLS.contains("CLAIM_STATUS_IN_FLIGHT_UNKNOWN"));
}

#[test]
fn historical_cleanup_and_terminal_ingress_are_exact_and_bounded() {
    for marker in [
        "compute_external_pool_adapter_route_renewal_receipts",
        "compute_external_pool_adapter_atomic_activation_receipts",
        "successor.successor_sequence=1",
        "authorization.cleanup_expires_at",
        "ensure!(\n        witnesses.len() == 1",
        "AND ?11<cleanup_expires_at",
        "AND ?11<authorization.cleanup_expires_at",
    ] {
        assert!(
            HISTORICAL_CLEANUP.contains(marker),
            "historical cleanup lost {marker}"
        );
    }
    assert!(!HISTORICAL_CLEANUP.contains("current_external_pool_adapter_provider_active_successor"));
    for marker in [
        "v273_task_exchange_attempt_exact_authority",
        "original.task_protocol_conformance_run_receipt_digest=NEW.task_protocol_conformance_run_receipt_digest",
        "cancel_outbox.subject_outbox_id=original.outbox_id",
        "cancel_send.started_at=NEW.started_at",
        "compute_external_pool_adapter_route_renewal_receipts",
        "JOIN compute_provider_versions provider_version",
        "provider.current_policy_revision>=activation.target_active_provider_policy_revision",
        "provider_version.provider_json=activation.target_active_provider_json",
        "successor.successor_sequence=1",
        "NEW.started_at<authorization.cleanup_expires_at",
        "!sql.contains(PENDING_UDF)",
    ] {
        assert!(
            HISTORICAL_POLL_AUTHORITY.contains(marker),
            "historical poll authority lost {marker}"
        );
    }
    assert!(!HISTORICAL_POLL_AUTHORITY.contains("JOIN compute_route_adapters adapter"));
    assert!(!HISTORICAL_POLL_AUTHORITY.contains("delegation_revocations"));
    for marker in [
        "apply_external_pool_adapter_task_terminal_ack_on",
        "apply_external_pool_adapter_task_direct_terminal_ack_on",
        "ack.outcome == COMPUTE_ATTEMPT_ADAPTER_ACK_ACCEPTED",
        "observation.verification_digest == receipt.semantic_observation_sha256",
        "ingest_verified_historical_external_pool_adapter_ack_at_on(",
        "authority: &HistoricalExternalPoolAdapterTaskExchangeCleanupAuthority",
    ] {
        assert!(TERMINAL.contains(marker), "terminal ingress lost {marker}");
    }
}

#[test]
fn relay_returns_only_typed_semantics_and_worker_remains_honestly_ineligible() {
    for source in [BROKER_RELAY, STORE_RELAY] {
        assert!(source.contains("Validator: ExternalPoolAdapterBrokerTaskObservationValidator"));
        assert!(source
            .contains("Result<VerifiedExternalPoolAdapterBrokerTaskExchange<Validator::Output>>"));
    }
    assert!(BROKER_RELAY.contains("validator.validate(raw)"));
    assert!(BROKER_TYPES.contains("sealed::VerifiedObservation + Send"));
    assert!(BROKER_TYPES.contains("sealed::Sealed + Send"));
    assert!(BROKER_TYPES.contains("pub(super) fn new("));
    assert!(BROKER_TYPES.contains("pub(crate) fn into_parts(self)"));
    assert!(!BROKER_RELAY.contains("FnOnce(&[u8])"));
    assert!(!BROKER_RELAY.contains("Result<(ExternalPoolAdapterTaskProtocolHostReceipt"));
    assert!(BROKER_RELAY.contains("Zeroizing::new"));
    assert!(STORE_RELAY.contains("timeout.is_zero() || timeout > MAX_TOTAL_EXCHANGE_TIMEOUT"));
    assert!(WORKER_ROOT.contains("mod cycle;"));
    assert_ordered(
        WORKER_CYCLE,
        &[
            "run_external_pool_adapter_active_preparation_cycle",
            "run_external_pool_adapter_task_delivery_source_cycle",
            "reprove_external_pool_adapter_task_delivery_source",
        ],
    );
    assert!(WORKER_CYCLE.contains("eligible_rows: 0"));
    assert!(WORKER_CYCLE.contains("delivery_attempted: false"));
    for forbidden in [
        "record_external_pool_adapter_task_outbound_on",
        "exchange_external_pool_adapter_task_delivery",
        "apply_external_pool_adapter_task_terminal_ack_on",
    ] {
        assert!(!WORKER_CYCLE.contains(forbidden));
    }
    assert!(DELIVERY_ROOT.contains("no admitted Offer/Job/plan/start producer"));
}

#[test]
fn receipt_ingress_obligations_and_fresh_replay_paths_are_sealed() {
    for marker in [
        "verified_exchange.into_parts()",
        "authority.register_ingress_obligation",
        "ExternalPoolAdapterTaskLedgerWriteDisposition::ExactReplay",
        "ExternalPoolAdapterTaskLedgerWriteDisposition::Inserted",
        "pending.ensure_fully_consumed()?",
    ] {
        assert!(
            RECEIPT_INGRESS.contains(marker),
            "receipt ingress lost {marker}"
        );
    }
    assert!(INGRESS_OBLIGATION.contains("ensure_resolved"));
    assert!(INGRESS_OBLIGATION.contains("self.pending_receipts.borrow().is_empty()"));
    assert_ordered(
        FIRST_CANCEL_RECONCILE,
        &[
            "pending.into_parts_on(transaction)?",
            "semantic.validate_reconcile_poll(&envelope)?",
            "ExternalPoolAdapterTaskLedgerWriteDisposition::Inserted",
            "ExternalPoolAdapterTaskLedgerWriteDisposition::ExactReplay",
            "plan.ensure_fully_consumed()?",
            "obligation.resolve(transaction)?",
        ],
    );
}

#[test]
fn event_reconcile_and_no_start_closures_freeze_exact_replay_dispositions() {
    for marker in [
        "ExternalPoolAdapterTaskReachabilityPendingWriteKind::EventBatch",
        "ExternalPoolAdapterTaskReachabilityPendingWriteKind::EventPollCas",
        "fresh V278 event ingress cannot reuse a successor poll",
        "V278 event replay successor differs from durable cursor disposition",
        "obligation.resolve(connection)?",
    ] {
        assert!(
            EVENT_INGRESS.contains(marker),
            "event ingress lost {marker}"
        );
    }
    for marker in [
        "ExternalPoolAdapterTaskReachabilityPendingWriteKind::ReconcilePollCas",
        "SealedReconcileClosure::Successor",
        "SealedReconcileClosure::NoStart",
        "SealedReconcileClosure::Terminal",
        "pending.ensure_fully_consumed()?",
    ] {
        assert!(
            RECONCILE_INGRESS.contains(marker),
            "reconcile ingress lost {marker}"
        );
    }
    assert!(RECONCILE_REPLAY.contains("usize::from(successor_branch)"));
    assert!(RECONCILE_REPLAY.contains("usize::from(event_branch)"));
    assert!(RECONCILE_REPLAY.contains("usize::from(terminal_branch)"));
    assert!(RECONCILE_REPLAY.contains("usize::from(no_start_branch)"));
    assert_ordered(
        NO_START_INGRESS,
        &[
            "semantic.validate_terminal_no_start",
            "record_verified_observation_at_on",
            "exact_no_start_proof_on",
            "obligation.resolve(transaction)?",
        ],
    );
}

#[test]
fn historical_accepted_mode_is_typed_ordered_and_guarded_at_every_write() {
    assert_ordered(
        ROUTE_RENEWAL_MIGRATION,
        &[
            "historical_accepted::install(&transaction)?",
            "historical_poll_exchange::install(&transaction)?",
            "transaction.commit()?",
        ],
    );
    assert!(ACK_WRITE.contains("AcceptedAckIngressMode::HistoricalTerminal(authority)"));
    assert!(ACK_WRITE.contains("ingest_verified_historical_external_pool_adapter_ack_at_on"));
    assert!(ACCEPTED_CURRENTNESS.contains("CurrentnessMode::HistoricalTerminal(authority)"));
    assert!(ACCEPTED_DERIVE.contains("derive_historical_closure"));
    assert!(ACCEPTED_DERIVE.contains("historical_actor_horizon"));
    assert!(ACCEPTED_PERSIST.contains("PersistMode::HistoricalTerminal(authority)"));
    assert_ordered(
        ACCEPTED_PLAN,
        &[
            "HistoricalAcceptedActor",
            "HistoricalAcceptedLeaseAuthority",
            "HistoricalAcceptedCommit",
            "HistoricalAcceptedApplication",
        ],
    );
    for trigger in [
        "trg_compute_attempt_dispatch_actor_exact_source",
        "trg_compute_attempt_lease_authority_projection",
        "trg_compute_attempt_commit_live_authority_v215",
        "trg_compute_attempt_application_live_authority_v215",
        "trg_compute_attempt_application_commit_closure_v213",
    ] {
        assert!(
            HISTORICAL_ACCEPTED_MIGRATION.contains(trigger),
            "historical Accepted lost {trigger}"
        );
    }
    for kind in [
        "historical_accepted_actor",
        "historical_accepted_lease_authority",
        "historical_accepted_commit",
        "historical_accepted_application",
    ] {
        assert!(
            HISTORICAL_ACCEPTED_MIGRATION.contains(kind),
            "historical Accepted plan lost {kind}"
        );
    }
    assert!(HISTORICAL_ACCEPTED_SOURCE.contains("receipt.recorded_at<renewal.cleanup_expires_at"));
    assert!(HISTORICAL_ACCEPTED_SOURCE.contains("{closure_at}<renewal.cleanup_expires_at"));
    assert!(HISTORICAL_ACCEPTED_SOURCE.contains("successor.successor_sequence=1"));
    assert!(HISTORICAL_ACCEPTED_SOURCE.contains(")=1"));
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
