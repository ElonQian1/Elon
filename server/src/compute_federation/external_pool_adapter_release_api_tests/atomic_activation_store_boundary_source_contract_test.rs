macro_rules! atomic_store_source {
    ($path:literal) => {
        include_str!(concat!(
            "../../store/compute_external_pool_adapter_provider_active_successor/atomic_activation/",
            $path
        ))
    };
}

macro_rules! active_no_work_source {
    ($path:literal) => {
        include_str!(concat!(
            "../../store/compute_external_pool_adapter_runtime_bundle/no_work_probe/active/",
            $path
        ))
    };
}

macro_rules! active_carrier_source {
    ($path:literal) => {
        include_str!(concat!(
            "../../store/compute_external_pool_adapter_task_protocol_conformance/active_carrier/",
            $path
        ))
    };
}

const STORE_ROOT: &str =
    include_str!("../../store/compute_external_pool_adapter_provider_active_successor.rs");
const ATOMIC_ROOT: &str = include_str!(
    "../../store/compute_external_pool_adapter_provider_active_successor/atomic_activation.rs"
);
const ATOMIC_TRANSACTION: &str = atomic_store_source!("transaction.rs");
const ATOMIC_PENDING: &str = atomic_store_source!("pending.rs");
const ATOMIC_RECEIPT: &str = atomic_store_source!("receipt.rs");
const ATOMIC_READ: &str = atomic_store_source!("read.rs");
const ATOMIC_ROUTE_AUDIT: &str = atomic_store_source!("route_audit.rs");
const ATOMIC_CARRIER: &str = atomic_store_source!("carrier.rs");
const PLAN: &str = include_str!(
    "../../store/compute_external_pool_adapter_runtime_bundle/runtime/custody/atomic_activation_plan.rs"
);
const PLAN_FINGERPRINT: &str = include_str!(
    "../../store/compute_external_pool_adapter_runtime_bundle/runtime/custody/atomic_activation_plan/fingerprint.rs"
);
const ROUTE_PERSISTENCE: &str =
    include_str!("../../store/compute_attempt_start_outbox/route_persistence.rs");
const NO_WORK_ROOT: &str =
    include_str!("../../store/compute_external_pool_adapter_runtime_bundle/no_work_probe.rs");
const NO_WORK_EXECUTION: &str = include_str!(
    "../../store/compute_external_pool_adapter_runtime_bundle/no_work_probe/execution.rs"
);
const ACTIVE_NO_WORK_ROOT: &str = active_no_work_source!("mod.rs");
const ACTIVE_NO_WORK_PREFLIGHT: &str = active_no_work_source!("preflight.rs");
const ACTIVE_NO_WORK_REPROOF: &str = active_no_work_source!("reproof.rs");
const ACTIVE_NO_WORK_TYPES: &str = active_no_work_source!("types.rs");
const V274_APPEND_GENESIS: &str = include_str!(
    "../../store/compute_external_pool_adapter_provider_active_successor/append/genesis.rs"
);
const V274_APPEND_REFRESH: &str = include_str!(
    "../../store/compute_external_pool_adapter_provider_active_successor/append/refresh.rs"
);
const V274_APPEND_READBACK: &str = include_str!(
    "../../store/compute_external_pool_adapter_provider_active_successor/append/readback.rs"
);
const V253_ACTIVE: &str = include_str!(
    "../../store/compute_external_pool_adapter_credential_reattestation/active_subject.rs"
);
const V253_TRANSITION: &str = include_str!(
    "../../store/compute_external_pool_adapter_credential_reattestation/projected_transition.rs"
);
const V268_HANDOFF: &str = include_str!(
    "../../store/compute_external_pool_adapter_runtime_compatibility_verification/handoff.rs"
);
const V272_ROOT: &str =
    include_str!("../../store/compute_external_pool_adapter_task_protocol_conformance.rs");
const V272_ACTIVE_ROOT: &str = include_str!(
    "../../store/compute_external_pool_adapter_task_protocol_conformance/active_carrier.rs"
);
const V272_ACTIVE_TYPES: &str = active_carrier_source!("types.rs");
const V272_ACTIVE_ROOTS: &str = active_carrier_source!("roots.rs");
const V272_ACTIVE_CURRENT: &str = active_carrier_source!("current.rs");
const V272_ACTIVE_WRITE: &str = active_carrier_source!("write.rs");
const RELEASE_API: &str = include_str!("../external_pool_adapter_release_api.rs");

#[test]
fn atomic_activation_store_closure_is_private_exact_and_one_shot() {
    assert!(STORE_ROOT.contains("mod atomic_activation;"));
    assert_ordered(
        ATOMIC_ROOT,
        &[
            "#[cfg(all(target_os = \"linux\", target_arch = \"x86_64\"))]",
            "mod transaction;",
        ],
    );
    for marker in [
        "persist_external_pool_adapter_atomic_activation_closure_on",
        "finalize_external_pool_adapter_atomic_activation_after_commit_on",
    ] {
        let private_definition = format!("pub(super) fn {marker}");
        assert!(
            ATOMIC_TRANSACTION.contains(private_definition.as_str()),
            "private atomic transaction leaf lost {marker}"
        );
        assert!(
            !ATOMIC_ROOT.contains(marker),
            "atomic root re-exported private transaction leaf {marker}"
        );
        assert!(
            !STORE_ROOT.contains(marker),
            "store root re-exported private transaction leaf {marker}"
        );
    }
    for marker in [
        "historical_external_pool_adapter_atomic_activation_authority_on",
        "current_external_pool_adapter_projected_active_historical_carrier_on",
    ] {
        assert!(ATOMIC_ROOT.contains(marker), "atomic root lost {marker}");
    }
    for marker in [
        "build_pending_plan",
        "install_external_pool_adapter_atomic_activation_pending_plan_on",
        "persist_compute_route_authority_on",
        "persist_external_pool_adapter_atomic_activation_receipt_on",
        "prepare_external_pool_adapter_provider_active_successor_genesis_append_on",
        "insert_prepared_external_pool_adapter_provider_active_successor_genesis_on",
        "audit_persisted_compute_route_authority_on",
        "audit_provider_transition",
        "ensure_fully_consumed",
        "ensure_same_connection",
    ] {
        assert!(
            format!("{ATOMIC_TRANSACTION}{ATOMIC_ROUTE_AUDIT}").contains(marker),
            "atomic transaction lost {marker}"
        );
    }
    assert_ordered(
        ATOMIC_TRANSACTION,
        &[
            "let successor = build_genesis_successor(",
            "let v274_append = prepare_external_pool_adapter_provider_active_successor_genesis_append_on(",
            "let plan = build_pending_plan(",
            "install_external_pool_adapter_atomic_activation_pending_plan_on(transaction, plan)?",
            "persist_compute_route_authority_on(transaction, route)?",
            "persist_provider_transition_on(transaction, source, target, target_digest, receipt)?",
            "persist_external_pool_adapter_atomic_activation_receipt_on(transaction, receipt)?",
            "insert_prepared_external_pool_adapter_provider_active_successor_genesis_on(",
        ],
    );
    assert_ordered(
        ATOMIC_TRANSACTION,
        &[
            "pub(super) fn finalize_external_pool_adapter_atomic_activation_after_commit_on(",
            "connection.is_autocommit()",
            "plan_guard.ensure_same_connection(connection)?",
            "plan_guard.ensure_fully_consumed()?",
            "postcommit_external_pool_adapter_provider_active_successor_readback_on(",
            "&plan_guard",
            "v274_append",
            "plan_guard.discard()?",
        ],
    );
    assert!(ATOMIC_TRANSACTION.contains("16 INSERT + 1 CAS UPDATE"));
    for marker in [
        "id_material.service_actor_id != root.service_actor_id",
        "!= root.task_production_carrier_policy_digest",
        "!= root.logical_projection_compatibility_digest",
        "binding_material.lane_subject_digest != root.lane_subject_digest",
    ] {
        assert!(ATOMIC_TRANSACTION.contains(marker));
    }
    assert!(ATOMIC_TRANSACTION.contains("INSERT INTO compute_provider_versions"));
    assert!(ATOMIC_TRANSACTION.contains("UPDATE compute_providers"));
    assert!(ATOMIC_RECEIPT
        .contains("INSERT INTO compute_external_pool_adapter_atomic_activation_receipts"));
    assert_eq!(
        ATOMIC_RECEIPT
            .split("pub(super) const RECEIPT_COLUMNS: &str = \"")
            .nth(1)
            .and_then(|rest| rest.split('\"').next())
            .expect("receipt columns")
            .split(',')
            .count(),
        79
    );
    for marker in [
        "receipt_by_id_on",
        "historical_external_pool_adapter_atomic_activation_for_binding_on",
        "historical_external_pool_adapter_atomic_activation_for_observed_provider_on",
        "audit_historical_route",
        "audit_live_route",
        "SELECT 1 FROM compute_route_credential_revocations revoked",
        "revoked.credential_id=:credential_id",
        "revoked.credential_revision=:credential_revision",
    ] {
        assert!(
            format!("{ATOMIC_READ}{ATOMIC_ROUTE_AUDIT}").contains(marker),
            "atomic readback lost {marker}"
        );
    }
    assert!(ATOMIC_READ.contains(
        "let evidence_checked_at = stored.receipt.activation.evidence_checked_at.clone();"
    ));
    assert!(ATOMIC_READ.contains("ProjectionAudit::Live(Some(&evidence_checked_at))"));
    assert!(!ATOMIC_READ.contains("ProjectionAudit::Live(None)"));
    let atomic_sources = format!(
        "{ATOMIC_ROOT}{ATOMIC_TRANSACTION}{ATOMIC_PENDING}{ATOMIC_RECEIPT}{ATOMIC_READ}{ATOMIC_ROUTE_AUDIT}{ATOMIC_CARRIER}"
    );
    for marker in [
        "persist_external_pool_adapter_atomic_activation_closure_on",
        "finalize_external_pool_adapter_atomic_activation_after_commit_on",
    ] {
        assert_eq!(
            atomic_sources.matches(marker).count(),
            1,
            "private atomic transaction function gained a caller or re-export {marker}"
        );
    }
    assert!(!atomic_sources.contains("impl Store"));
    assert!(!RELEASE_API.contains("atomic-activation"));
}

#[test]
fn atomic_activation_pending_plan_is_connection_local_bounded_and_non_deterministic() {
    assert!(PLAN.contains("const EXPECTED_WRITE_COUNT: usize = 15;"));
    for kind in [
        "ProviderUpdate",
        "ProviderVersion",
        "ProjectionAdapter",
        "ProjectionAdapterVersion",
        "ServiceActorAuthorization",
        "RouteCredential",
        "RouteAuthorization",
        "RouteCapability",
        "RouteSeal",
        "ActivationReceipt",
    ] {
        assert!(PLAN.contains(kind), "pending plan lost {kind}");
    }
    for marker in [
        "SQLITE_UTF8 | FunctionFlags::SQLITE_INNOCUOUS",
        "create_scalar_function(PENDING_PLAN_MATCHES, -1",
        "ensure_fully_consumed",
        "ensure_same_connection",
        "discard(mut self)",
        "next_index",
        "active.writes.get(active.next_index)",
        "active.next_index += 1",
        "Arc::ptr_eq(&registered, &self.registry)",
    ] {
        assert!(PLAN.contains(marker), "pending registry lost {marker}");
    }
    assert!(!PLAN.contains("SQLITE_DETERMINISTIC"));
    for marker in [
        "PendingSqliteType",
        "from_value",
        "from_ref",
        "byte_len: value.len()",
    ] {
        assert!(
            PLAN_FINGERPRINT.contains(marker),
            "fingerprint lost {marker}"
        );
    }
    for marker in [
        "persist_compute_route_authority_on",
        "audit_persisted_compute_route_authority_on",
        "ensure_compute_route_registry_current_on",
    ] {
        assert!(
            ROUTE_PERSISTENCE.contains(marker),
            "shared route kernel lost {marker}"
        );
    }
}

#[test]
fn planned_genesis_runs_real_no_work_io_while_durable_refresh_stays_fail_closed() {
    assert!(NO_WORK_ROOT.contains("mod execution;"));
    assert!(NO_WORK_ROOT.contains("mod active;"));
    for marker in [
        "execute_external_pool_adapter_no_work_probe",
        ".exchange_no_work(",
        "shutdown_and_reap",
    ] {
        assert!(
            NO_WORK_EXECUTION.contains(marker),
            "no-work execution lost {marker}"
        );
    }
    for marker in [
        "with_planned_external_pool_adapter_active_no_work_probe_observation",
        "with_current_external_pool_adapter_no_work_probe_observation",
        "planned_external_pool_adapter_active_no_work_probe_subject_on",
    ] {
        assert!(
            ACTIVE_NO_WORK_PREFLIGHT.contains(marker),
            "planned no-work path lost {marker}"
        );
    }
    for marker in [
        "pub(in crate::store) struct ReprovedPlannedExternalPoolAdapterActiveNoWorkProbeSubject",
        "observation:",
        "pub(in crate::store) fn observation(",
        "self.observation.checked_at()",
        "with_reproved_planned_external_pool_adapter_active_no_work_subject",
    ] {
        assert!(
            ACTIVE_NO_WORK_REPROOF.contains(marker),
            "planned typed no-work reproof lost {marker}"
        );
    }
    for marker in [
        "PlannedExternalPoolAdapterActiveNoWorkProbeSubject",
        "DurableExternalPoolAdapterActiveNoWorkProbeSubject",
    ] {
        assert!(
            ACTIVE_NO_WORK_TYPES.contains(marker),
            "typed subject lost {marker}"
        );
    }
    assert!(ACTIVE_NO_WORK_PREFLIGHT
        .contains("pub(super) fn durable_external_pool_adapter_active_no_work_probe_subject_on"));
    assert!(ACTIVE_NO_WORK_REPROOF.contains(
        "pub(super) fn with_reproved_durable_external_pool_adapter_active_no_work_subject"
    ));
    assert!(V274_APPEND_READBACK.contains(
        "pub(in crate::store::compute_external_pool_adapter_provider_active_successor) fn postcommit_external_pool_adapter_provider_active_successor_readback_on"
    ));
    assert_ordered(
        V274_APPEND_READBACK,
        &[
            "pub(in crate::store::compute_external_pool_adapter_provider_active_successor) fn postcommit_external_pool_adapter_provider_active_successor_readback_on(",
            "plan_guard: &ExternalPoolAdapterAtomicActivationPendingPlanGuard",
            "connection.is_autocommit()",
            "plan_guard.ensure_same_connection(connection)?",
            "plan_guard.ensure_fully_consumed()?",
            "require_exact_readback_on(connection, &pending)?",
            ".promote_provider_active_successor_process_seal(",
        ],
    );
    let public_active_roots = format!("{STORE_ROOT}{NO_WORK_ROOT}{ACTIVE_NO_WORK_ROOT}");
    for forbidden in [
        "durable_external_pool_adapter_active_no_work_probe_subject_on",
        "with_reproved_durable_external_pool_adapter_active_no_work_subject",
        "with_external_pool_adapter_active_no_work_postcommit_callback",
        "postcommit_external_pool_adapter_provider_active_successor_readback_on",
    ] {
        assert!(
            !public_active_roots.contains(forbidden),
            "public root exposed dormant durable helper {forbidden}"
        );
    }
    assert!(
        V274_APPEND_GENESIS.contains("PreparedExternalPoolAdapterTaskProtocolPlannedActiveCarrier")
    );
    assert!(V274_APPEND_REFRESH
        .contains("CurrentExternalPoolAdapterTaskProtocolProjectedActiveAuthority"));
    assert!(V274_APPEND_READBACK.contains(".promote_provider_active_successor_process_seal("));
}

#[test]
fn projected_active_evidence_is_witness_gated_without_current_v274_recursion() {
    for marker in [
        "historical_external_pool_adapter_atomic_activation_for_binding_on",
        "historical_external_pool_adapter_atomic_activation_for_observed_provider_on",
        "current_external_pool_adapter_projected_active_credential_reattestation_authority_on",
        "exact_projected_active",
    ] {
        assert!(
            V253_ACTIVE.contains(marker),
            "V253 active seam lost {marker}"
        );
    }
    assert!(V253_TRANSITION.contains("canonical_projected_active_transition_proof_json_and_digest"));
    for marker in [
        "run_external_pool_adapter_runtime_compatibility_signing_handoff_for_projected_active",
        "current_external_pool_adapter_projected_active_credential_reattestation_authority_on",
        "historical_external_pool_adapter_atomic_activation_for_binding_on",
    ] {
        assert!(
            V268_HANDOFF.contains(marker),
            "V268 active handoff lost {marker}"
        );
    }
    assert!(V272_ROOT.contains("mod active_carrier;"));
    for marker in [
        "PreparedExternalPoolAdapterTaskProtocolPlannedActiveCarrier",
        "CurrentExternalPoolAdapterTaskProtocolProjectedActiveAuthority",
        "prepare_external_pool_adapter_task_protocol_planned_active_carrier_on",
        "no_work: &ReprovedPlannedExternalPoolAdapterActiveNoWorkProbeSubject",
        "let target = no_work.preflight();",
        "let evidence_checked_at = no_work.evidence_checked_at();",
        "binding.provider_binding_id != root.provider_binding_id",
        "binding.provider_binding_digest != root.provider_binding_digest",
    ] {
        assert!(
            V272_ACTIVE_TYPES.contains(marker),
            "V272 carrier type lost {marker}"
        );
    }
    for marker in [
        "historical_external_pool_adapter_atomic_activation_for_binding_on",
        "current_external_pool_adapter_projected_active_historical_carrier_on",
    ] {
        assert!(
            V272_ACTIVE_ROOTS.contains(marker),
            "V272 active roots lost {marker}"
        );
    }
    assert!(V272_ACTIVE_CURRENT.contains("attests_task_protocol_conformance_seal"));
    assert!(V272_ACTIVE_WRITE.contains(
        "create_external_pool_adapter_task_protocol_conformance_run_for_projected_active"
    ));
    assert!(V272_ACTIVE_ROOT.contains("pub(in crate::store) use write::"));
    let active_sources = format!(
        "{V253_ACTIVE}{V253_TRANSITION}{V268_HANDOFF}{V272_ACTIVE_ROOT}{V272_ACTIVE_TYPES}{V272_ACTIVE_ROOTS}{V272_ACTIVE_CURRENT}{V272_ACTIVE_WRITE}"
    );
    assert!(!active_sources.contains("provider_active_successor_current"));
    assert!(!active_sources.contains("eligible_rows"));
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
