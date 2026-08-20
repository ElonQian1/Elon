macro_rules! route_store_source {
    ($path:literal) => {
        include_str!(concat!(
            "../../store/compute_external_pool_adapter_route_renewal/",
            $path
        ))
    };
}

macro_rules! preparation_source {
    ($path:literal) => {
        include_str!(concat!(
            "../../store/compute_external_pool_adapter_runtime_bundle/active_preparation/",
            $path
        ))
    };
}

const ROUTE_ROOT: &str = include_str!("../../store/compute_external_pool_adapter_route_renewal.rs");
const ROUTE_PENDING: &str = route_store_source!("pending.rs");
const ROUTE_PERSISTENCE: &str = route_store_source!("persistence.rs");
const ROUTE_WRITES: &str = route_store_source!("writes.rs");
const ROUTE_RECEIPT: &str = route_store_source!("receipt.rs");
const ROUTE_READ: &str = route_store_source!("read.rs");
const PREPARATION_ROOT: &str =
    include_str!("../../store/compute_external_pool_adapter_runtime_bundle/active_preparation.rs");
const REGISTERING: &str = preparation_source!("registering.rs");
const REGISTERING_SELECTION: &str = preparation_source!("registering/selection.rs");
const ACTIVE_CYCLE: &str = preparation_source!("cycle.rs");
const ACTIVE_SELECTION: &str = preparation_source!("selection.rs");
const COMPOSITE_REPROOF: &str = preparation_source!("reproof.rs");
const ACTIVE_NO_WORK: &str = include_str!(
    "../../store/compute_external_pool_adapter_runtime_bundle/no_work_probe/active/cycle.rs"
);
const V274_REFRESH: &str = include_str!(
    "../../store/compute_external_pool_adapter_provider_active_successor/append/refresh.rs"
);
const V274_POSTCOMMIT: &str = include_str!(
    "../../store/compute_external_pool_adapter_provider_active_successor/append/refresh_postcommit.rs"
);
const V274_PLAN: &str = include_str!(
    "../../store/compute_external_pool_adapter_runtime_bundle/runtime/custody/provider_active_successor_refresh_plan.rs"
);

#[test]
fn route_renewal_store_is_ordered_connection_local_and_exactly_replayed() {
    for marker in [
        "build_external_pool_adapter_route_renewal_receipt",
        "renew_external_pool_adapter_route_on",
        "finalize_external_pool_adapter_route_renewal_after_commit_on",
        "historical_external_pool_adapter_route_recovery_authority_on",
        "require_current_external_pool_adapter_renewed_route_on",
    ] {
        assert!(ROUTE_ROOT.contains(marker), "route root lost {marker}");
    }
    assert_eq!(
        ROUTE_RECEIPT
            .split("pub(crate) const RECEIPT_COLUMNS: &str = \"")
            .nth(1)
            .and_then(|tail| tail.split('"').next())
            .expect("route columns")
            .split(',')
            .count(),
        77
    );
    for marker in [
        "active.writes.get(active.next)",
        "active.next += 1",
        "ensure_fully_consumed",
        "ensure_same_connection",
        "Arc::ptr_eq",
        "FunctionFlags::SQLITE_UTF8 | FunctionFlags::SQLITE_INNOCUOUS",
        "create_scalar_function(\n        UDF,\n        -1",
    ] {
        assert!(ROUTE_PENDING.contains(marker), "pending plan lost {marker}");
    }
    assert!(!ROUTE_PENDING.contains("SQLITE_DETERMINISTIC"));
    assert_ordered(
        ROUTE_PERSISTENCE,
        &[
            "let plan = build_plan(&built.receipt, route, &old)?",
            "pending::install(transaction, plan)?",
            "insert_actor_on(transaction",
            "insert_credential_on(transaction",
            "cas_credential_root_on(transaction",
            "insert_authorization_on(transaction",
            "insert_capabilities_and_seal_on(transaction",
            "insert_receipt_on(transaction",
            "plan_guard.ensure_fully_consumed()?",
            "receipt_by_id_on(transaction",
        ],
    );
    for marker in [
        "INSERT INTO compute_service_actor_authorizations",
        "INSERT INTO compute_route_credential_versions",
        "UPDATE compute_route_credentials",
        "INSERT INTO compute_route_authorization_receipts",
        "INSERT INTO compute_route_authorization_capabilities",
        "INSERT INTO compute_route_authorization_seals",
    ] {
        assert!(ROUTE_WRITES.contains(marker), "route writes lost {marker}");
    }
    assert_ordered(
        ROUTE_PERSISTENCE,
        &[
            "connection.is_autocommit()",
            "guard.ensure_same_connection(connection)?",
            "guard.ensure_fully_consumed()?",
            "receipt_by_id_on(connection",
            "guard.discard()?",
        ],
    );
    assert!(ROUTE_PERSISTENCE.contains("ExternalPoolAdapterRouteRenewalDisposition::ExactReplay"));
    assert!(ROUTE_READ.contains("revocation"));
}

#[test]
fn registering_and_active_selection_are_fair_and_server_anchored() {
    assert!(PREPARATION_ROOT.contains("mod registering;"));
    assert!(PREPARATION_ROOT.contains("mod selection;"));
    for source in [REGISTERING_SELECTION, ACTIVE_SELECTION] {
        assert!(source.contains("SELECT COUNT(*)"));
        assert!(source.contains("OFFSET ?1"));
        assert!(source.contains("selection_slot %"));
    }
    for source in [REGISTERING, ACTIVE_CYCLE] {
        assert!(source.contains("Utc::now().timestamp().div_euclid(60)"));
    }
    assert!(!ACTIVE_CYCLE.contains("Deferred => return"));
    assert!(REGISTERING.contains("persist_external_pool_adapter_atomic_activation_closure_on"));
    assert!(
        REGISTERING.contains("finalize_external_pool_adapter_atomic_activation_after_commit_on")
    );
    assert!(
        COMPOSITE_REPROOF.contains("ReprovedExternalPoolAdapterRouteAndActiveSuccessorAuthority")
    );
    assert!(COMPOSITE_REPROOF
        .contains("require_current_external_pool_adapter_provider_active_successor_on"));
}

#[test]
fn active_preparation_runs_real_six_reopen_io_then_guarded_v274_refresh() {
    for marker in [
        "prepare_projected_active_external_pool_adapter_broker_tls_channel",
        "prepare_projected_active_external_pool_adapter_ephemeral_secret_delivery",
        "execute_external_pool_adapter_no_work_probe",
        "current_external_pool_adapter_projected_active_runtime_bundle_authority_on",
        "transaction.commit()?",
        "postcommit(&connection, pending)",
    ] {
        assert!(
            ACTIVE_NO_WORK.contains(marker),
            "active no-work lost {marker}"
        );
    }
    assert_eq!(ACTIVE_NO_WORK.matches("reopen_prepared()").count(), 4);
    assert!(ACTIVE_CYCLE.contains(
        "create_external_pool_adapter_task_protocol_conformance_run_for_projected_active"
    ));
    assert!(
        ACTIVE_CYCLE.contains("with_projected_active_external_pool_adapter_no_work_observation")
    );
    assert!(
        ACTIVE_CYCLE.contains("append_external_pool_adapter_provider_active_successor_refresh_on")
    );
    assert!(ACTIVE_CYCLE
        .contains("postcommit_external_pool_adapter_provider_active_successor_refresh_on"));
    for marker in [
        "successor.lineage.successor_sequence <= 1",
        "head_by_binding_and_root_on",
        "install_external_pool_adapter_provider_active_successor_refresh_pending_plan_on",
        "insert_and_readback_pending_append_on",
        "plan_guard.ensure_fully_consumed()?",
    ] {
        assert!(V274_REFRESH.contains(marker), "V274 refresh lost {marker}");
    }
    assert!(V274_PLAN.contains("const REFRESH_PENDING_PLAN_ARITY: usize = 17"));
    assert!(V274_PLAN.contains("matches!(successor_sequence, Value::Integer(value) if value > 1)"));
    assert_ordered(
        V274_POSTCOMMIT,
        &[
            "connection.is_autocommit()",
            "require_refresh_guard(&plan_guard, connection)?",
            "require_exact_readback_on(connection, &append)?",
            "promote_provider_active_successor_process_seal_for_refresh",
            "plan_guard.discard()?",
        ],
    );
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
