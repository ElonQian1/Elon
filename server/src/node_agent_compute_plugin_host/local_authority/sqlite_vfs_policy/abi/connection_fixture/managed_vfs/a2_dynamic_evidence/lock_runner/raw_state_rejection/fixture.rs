//! Child-only q11 fixture for one controlled raw-state rejection.
//!
//! The selected raw representation may leave the live SQLite allocation detached or otherwise
//! terminal. The fixture therefore enters `ManuallyDrop` before invoking the saved production
//! callback and remains process-owned until child exit. No q11 path performs normal close.

use std::{mem::ManuallyDrop, path::Path};

use anyhow::{anyhow, Context};
use rusqlite::ffi;

use crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_abi::{
    HandleBoundSqliteAbiRawLockEvidenceV1, HandleBoundSqliteAbiRawLockRejectionCaseV1,
};

use super::super::super::super::{
    ManagedSqliteRoutedConnectionFixture, ManagedSqliteTestVfsRouteCustodySnapshot,
    ManagedSqliteTestVfsRoutePhase,
};
use super::super::super::{SanitizedChildReport, A2_DYNAMIC_CHILD_NONCE_ENV};
use super::{
    payload, validate_binding, LockRunnerRawStateRejectionBindingV1,
    LockRunnerRawStateRejectionV1,
};

pub(super) fn exercise_child(
    root: &Path,
    binding: LockRunnerRawStateRejectionBindingV1,
) -> anyhow::Result<()> {
    validate_binding(binding)?;
    let nonce = std::env::var(A2_DYNAMIC_CHILD_NONCE_ENV)
        .context("read parent-created q11 Lock raw-state child nonce")?;
    SanitizedChildReport::validate_root_before_exercise(&nonce, root)
        .map_err(anyhow::Error::msg)?;

    let fixture = ManagedSqliteRoutedConnectionFixture::open(root, [0xab; 16])?;
    let registration_id = fixture.registration_id_for_test();
    let route_ordinal = fixture.route_ordinal().counter_value();
    if registration_id == 0 || route_ordinal != 1 {
        return Err(anyhow!(
            "q11 Lock raw-state fixture registration/route identity mismatch"
        ));
    }
    let journal_mode: String = fixture
        .connection()
        .query_row("PRAGMA journal_mode=WAL", [], |row| row.get(0))?;
    if !journal_mode.eq_ignore_ascii_case("wal") {
        return Err(anyhow!("q11 Lock raw-state fixture did not enter WAL mode"));
    }
    fixture.into_schema_migration()?;
    fixture.into_runtime()?;

    let target_before = fixture
        .exact_main_shm_target_presence()
        .map_err(anyhow::Error::msg)?;
    let route_before = fixture
        .route_custody_snapshot()
        .map_err(anyhow::Error::msg)?;
    validate_live_route(route_before)?;
    if target_before {
        return Err(anyhow!(
            "q11 Lock raw-state target existed before controlled callback"
        ));
    }

    // Every selected representation may invalidate ordinary SQLite teardown. Establish linear
    // process-exit custody before the production callback can mutate either raw slot.
    let fixture = ManuallyDrop::new(fixture);
    let observation = fixture
        .observe_main_shm_lock_raw_state_rejection_v1(abi_case(binding.rejection))
        .map_err(anyhow::Error::msg)?;
    let abi = observation.abi();
    let route_no_entry = observation.route_no_entry();
    if abi.case_v1() != abi_case(binding.rejection)
        || abi.evidence_v1() != HandleBoundSqliteAbiRawLockEvidenceV1::ControlledFaultActual
        || abi.observation_id() == 0
        || abi.result_code() != ffi::SQLITE_IOERR_SHMLOCK
        || abi.ordered_values()[3] != abi.observation_id()
        || route_no_entry
            != [1, 8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
    {
        return Err(anyhow!(
            "q11 Lock controlled raw-state ABI/route receipt mismatch"
        ));
    }

    let target_after = fixture
        .exact_main_shm_target_presence()
        .map_err(anyhow::Error::msg)?;
    let route_after = fixture
        .route_custody_snapshot()
        .map_err(anyhow::Error::msg)?;
    validate_live_route(route_after)?;
    if target_after || route_before != route_after {
        return Err(anyhow!(
            "q11 Lock controlled rejection changed managed route/target state"
        ));
    }
    let registration = fixture.live_registration_snapshot_for_test()?;
    let registration_values = [
        u64::from(registration.registered()),
        u64::from(registration.table_present()),
        u64::from(registration.name_present()),
        u64::from(registration.context_present()),
    ];
    let retained_values = [
        1,
        0,
        u64::from(root.is_dir()),
        u64::from(root.join("db").is_dir()),
    ];
    if registration_values != [1; 4] || retained_values != [1, 0, 1, 1] {
        return Err(anyhow!(
            "q11 Lock retained registration/process-exit custody mismatch"
        ));
    }

    let payload = payload::encode(
        binding,
        registration_id,
        route_ordinal,
        abi.ordered_values(),
        route_no_entry,
        [u64::from(target_before), u64::from(target_after)],
        route_values(route_before),
        route_values(route_after),
        registration_values,
        retained_values,
    );
    let report =
        SanitizedChildReport::encode_for_current_child(&nonce, root, registration_id, &payload)
            .map_err(anyhow::Error::msg)?;
    println!("{report}");
    Ok(())
}

fn validate_live_route(
    route: ManagedSqliteTestVfsRouteCustodySnapshot,
) -> anyhow::Result<()> {
    if route_values(route) != [3, 1, 0, 0, 0, 1] {
        return Err(anyhow!(
            "q11 Lock raw-state route was not active and callback-free"
        ));
    }
    Ok(())
}

fn route_values(route: ManagedSqliteTestVfsRouteCustodySnapshot) -> [u64; 6] {
    [
        phase_tag(route.phase()),
        u64::from(route.connection_owner()),
        u64::from(route.main_file_lock_owner_lease()),
        u64::from(route.shm_lease()),
        route.callbacks_in_flight() as u64,
        u64::from(route.access_callback_allowed()),
    ]
}

const fn phase_tag(phase: ManagedSqliteTestVfsRoutePhase) -> u64 {
    match phase {
        ManagedSqliteTestVfsRoutePhase::PendingMain => 1,
        ManagedSqliteTestVfsRoutePhase::Opening => 2,
        ManagedSqliteTestVfsRoutePhase::Active => 3,
        ManagedSqliteTestVfsRoutePhase::Closing => 4,
        ManagedSqliteTestVfsRoutePhase::AwaitingRouteRetirement => 5,
        ManagedSqliteTestVfsRoutePhase::Retired => 6,
        ManagedSqliteTestVfsRoutePhase::TerminalQuarantine => 7,
    }
}

const fn abi_case(
    rejection: LockRunnerRawStateRejectionV1,
) -> HandleBoundSqliteAbiRawLockRejectionCaseV1 {
    use HandleBoundSqliteAbiRawLockRejectionCaseV1 as A;
    use LockRunnerRawStateRejectionV1 as R;
    match rejection {
        R::NullFileDirect => A::NullFileDirect,
        R::UninstalledDirect => A::UninstalledDirect,
        R::MethodsNullStatePresentDirect => A::MethodsNullStatePresentDirect,
        R::ForeignMethodsStateNullDirect => A::ForeignMethodsStateNullDirect,
        R::ForeignMethodsStatePresentDirect => A::ForeignMethodsStatePresentDirect,
        R::ExactMethodsStateNullDirect => A::ExactMethodsStateNullDirect,
        R::OtherTypePayloadMissingDropCompleted => A::OtherTypePayloadMissingDropCompleted,
        R::OtherTypePayloadPresentDropCompleted => A::OtherTypePayloadPresentDropCompleted,
        R::OtherTypePayloadPresentDropUnwindCaught => A::OtherTypePayloadPresentDropUnwindCaught,
        R::ExpectedTypePayloadMissingDropCompleted => A::ExpectedTypePayloadMissingDropCompleted,
        R::HandleBoundFileMissingDirect => A::HandleBoundFileMissingDirect,
    }
}
