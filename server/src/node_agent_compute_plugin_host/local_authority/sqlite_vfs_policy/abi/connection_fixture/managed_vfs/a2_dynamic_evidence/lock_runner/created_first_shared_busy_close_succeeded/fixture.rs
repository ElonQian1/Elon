//! Retained child fixture for real Q18 DMS contention and target-close success.

use std::{ops::Deref, path::Path};

use anyhow::{anyhow, Context};
use rusqlite::ffi;

use crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::registry::ManagedSqliteRegistryTerminalCustodyTestSnapshot;
use crate::node_agent_managed_fs::{
    ManagedSqliteShmTestCreatedFirstSharedBusyCloseSucceededReceiptV1,
    ManagedSqliteShmTestDmsCustody, ManagedSqliteShmTestInitializationExpectationV1,
    ManagedSqliteShmTestInitializationFailureV1, ManagedSqliteShmTestLockPath,
    ManagedSqliteShmTestLockReceipt, ManagedSqliteShmTestTargetSnapshot,
};

use super::super::super::super::{
    connection::{
        ManagedTestLockCreatedFirstSharedBusyCloseSucceededObservationV1,
        ManagedTestShmMapCallbackObservation,
    },
    ManagedSqliteMultiConnectionFixture,
};
use super::super::super::{SanitizedChildReport, A2_DYNAMIC_CHILD_NONCE_ENV};
use super::super::{lifecycle, LockRunnerActionV1};
use super::{
    payload, validate_binding, LockRunnerCreatedFirstSharedBusyCloseSucceededCompletionV1,
    LockRunnerNativeAcquireCreatedFirstSharedBusyCloseSucceededBindingV1,
};

const SELECTED: usize = 0;
const COLD_REGION: i32 = 256;
const COLD_REGION_SIZE: i32 = 32 * 1024;
const COLD_RAW_EXTEND: i32 = 0;

struct RetainedQ18Fixture {
    fixture: Option<ManagedSqliteMultiConnectionFixture>,
    cold_map: ManagedTestShmMapCallbackObservation,
    cold_snapshot: ManagedSqliteShmTestTargetSnapshot,
    target_absent_before: bool,
}

impl Deref for RetainedQ18Fixture {
    type Target = ManagedSqliteMultiConnectionFixture;

    fn deref(&self) -> &Self::Target {
        self.fixture.as_ref().expect("retained q18 Lock fixture")
    }
}

impl Drop for RetainedQ18Fixture {
    fn drop(&mut self) {
        if let Some(fixture) = self.fixture.take() {
            // Q18 already closed the target SHM file and explicitly released the holder. The
            // quarantined route and retained callback evidence remain child-owned until exit.
            std::mem::forget(fixture);
        }
    }
}

pub(super) fn exercise_child(
    root: &Path,
    binding: LockRunnerNativeAcquireCreatedFirstSharedBusyCloseSucceededBindingV1,
) -> anyhow::Result<()> {
    validate_binding(binding)?;
    let nonce = std::env::var(A2_DYNAMIC_CHILD_NONCE_ENV)
        .context("read parent-created q18 Lock initialization child nonce")?;
    SanitizedChildReport::validate_root_before_exercise(&nonce, root)
        .map_err(anyhow::Error::msg)?;

    let fixture = prepare(root)?;
    let witness = fixture
        .route(SELECTED)?
        .installed_shm_fault_witness()
        .map_err(anyhow::Error::msg)?;
    let target = witness.target_witness().map_err(anyhow::Error::msg)?;

    if binding.completion
        == LockRunnerCreatedFirstSharedBusyCloseSucceededCompletionV1::RetentionRouteUnknown
    {
        fixture
            .arm_unsafe_shm_route_preemption(SELECTED)
            .map_err(anyhow::Error::msg)?;
    }

    let pending = fixture
        .route(SELECTED)?
        .observe_main_shm_lock_created_first_shared_busy_close_succeeded_v1(
            initialization_expectation(binding),
            lifecycle::raw_flags(binding.action),
        )
        .map_err(anyhow::Error::msg)?;

    // The Q18 holder is still locked here. Every inspection failure consumes `pending` through
    // the explicit abort path; only a complete terminal snapshot reaches the finish call below.
    let inspection =
        (|| {
            let terminal = fixture
                .route(SELECTED)?
                .terminal_custody_test_snapshot()
                .map_err(anyhow::Error::msg)?;
            let terminal_values = terminal_values(terminal, binding.completion)?;
            let preemption_values = match binding.completion {
                LockRunnerCreatedFirstSharedBusyCloseSucceededCompletionV1::RetentionSucceeded => {
                    [0; 6]
                }
                LockRunnerCreatedFirstSharedBusyCloseSucceededCompletionV1::RetentionRouteUnknown => {
                    let receipt = fixture
                        .unsafe_shm_route_preemption_snapshot(SELECTED)
                        .map_err(anyhow::Error::msg)?
                        .ordered_values();
                    if receipt != [1; 5] {
                        return Err(anyhow!("q18 route-unknown preemption receipt mismatch"));
                    }
                    [1, receipt[0], receipt[1], receipt[2], receipt[3], receipt[4]]
                }
            };
            Ok((terminal_values, preemption_values))
        })();
    let (terminal_values, preemption_values) = match inspection {
        Ok(values) => values,
        Err(error) => {
            pending
                .abort_after_inspection_failure()
                .map_err(anyhow::Error::msg)?;
            return Err(error);
        }
    };

    // This is the only successful finish: it explicitly unlocks the distinct holder.
    let observation = pending
        .finish_after_terminal_custody_observed()
        .map_err(anyhow::Error::msg)?;
    validate_observation(binding, fixture.cold_snapshot, observation)?;

    let registration = fixture.live_registration_snapshot()?;
    let registration_values = [
        u64::from(registration.registered()),
        u64::from(registration.table_present()),
        u64::from(registration.name_present()),
        u64::from(registration.context_present()),
    ];
    let (routes, logical_names) = fixture.logical_route_counts()?;
    let route_values = [
        fixture.live_connection_count() as u64,
        routes as u64,
        logical_names as u64,
    ];
    let root_shape_present = root.is_dir() && root.join("db").is_dir();
    if registration_values != [1; 4]
        || target.route_ordinal() != 1
        || route_values != [1, 1, 3]
        || !root_shape_present
    {
        return Err(anyhow!("q18 retained registration/root shape mismatch"));
    }

    let payload = payload::encode(
        binding,
        target.registration_id(),
        target.route_ordinal(),
        target.runtime_generation(),
        target.shm_connection_id(),
        cold_setup_values(&fixture),
        callback_values(observation),
        snapshot_values(observation.after),
        observation.initialization.initialization_values(),
        observation.initialization.holder_values(),
        lock_values(observation.lock_no_requested_native),
        observation.pending_count as u64,
        terminal_values,
        preemption_values,
        registration_values,
        route_values,
        u64::from(root_shape_present),
    );
    let report = SanitizedChildReport::encode_for_current_child(
        &nonce,
        root,
        target.registration_id(),
        &payload,
    )
    .map_err(anyhow::Error::msg)?;
    println!("{report}");
    Ok(())
}

fn prepare(root: &Path) -> anyhow::Result<RetainedQ18Fixture> {
    let fixture = ManagedSqliteMultiConnectionFixture::open_single(root, [0xb2; 16])?;
    let route = fixture.route(SELECTED)?;
    let target_absent_before = !route
        .exact_main_shm_target_presence()
        .map_err(anyhow::Error::msg)?;
    let mode: String = fixture
        .connection(SELECTED)?
        .query_row("PRAGMA journal_mode=WAL", [], |row| row.get(0))
        .context("enable WAL for q18 cold initialization")?;
    if !mode.eq_ignore_ascii_case("wal") || !target_absent_before {
        return Err(anyhow!("q18 fixture was not cold before WAL-main attach"));
    }
    route.into_schema_migration()?;
    route.into_runtime()?;
    let cold_map = route
        .call_main_shm_map_raw(COLD_REGION, COLD_REGION_SIZE, COLD_RAW_EXTEND)
        .map_err(anyhow::Error::msg)?;
    let observer = route
        .installed_shm_fault_witness()
        .map_err(anyhow::Error::msg)?
        .observer()
        .map_err(anyhow::Error::msg)?;
    let cold_snapshot = observer.snapshot()?;
    if cold_map.region() != COLD_REGION
        || cold_map.region_size() != COLD_REGION_SIZE
        || cold_map.raw_extend() != COLD_RAW_EXTEND
        || cold_map.result_code() != ffi::SQLITE_IOERR_SHMMAP
        || !cold_map.output_was_cleared()
        || !cold_map.output_pointer().is_null()
        || !cold_map.before().methods_installed
        || !cold_map.before().state_installed
        || !cold_map.after().methods_installed
        || !cold_map.after().state_installed
        || snapshot_values(cold_snapshot) != [1, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
    {
        return Err(anyhow!("q18 cold WAL-main attach receipt mismatch"));
    }
    Ok(RetainedQ18Fixture {
        fixture: Some(fixture),
        cold_map,
        cold_snapshot,
        target_absent_before,
    })
}

fn validate_observation(
    binding: LockRunnerNativeAcquireCreatedFirstSharedBusyCloseSucceededBindingV1,
    cold: ManagedSqliteShmTestTargetSnapshot,
    value: ManagedTestLockCreatedFirstSharedBusyCloseSucceededObservationV1,
) -> anyhow::Result<()> {
    let expected = initialization_expectation(binding);
    if value.callback.offset() != i32::from(binding.first)
        || value.callback.count() != i32::from(binding.count)
        || value.callback.raw_flags() != lifecycle::raw_flags(binding.action)
        || value.callback.result_code() != ffi::SQLITE_IOERR_SHMLOCK
        || !value.callback.before().methods_installed
        || !value.callback.before().state_installed
        || !value.callback.after().methods_installed
        || !value.callback.after().state_installed
        || value.before != cold
        || snapshot_values(value.after) != [1, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
        || value.initialization.expectation() != expected
        || !exact_initialization_values(binding, value.initialization)
        || value.initialization.requested_lock_receipt() != value.lock_no_requested_native
        || !exact_no_requested_native_lock(expected, value.lock_no_requested_native)
        || value.pending_count != 0
    {
        return Err(anyhow!("q18 real initialization receipt mismatch"));
    }
    Ok(())
}

fn exact_initialization_values(
    binding: LockRunnerNativeAcquireCreatedFirstSharedBusyCloseSucceededBindingV1,
    receipt: ManagedSqliteShmTestCreatedFirstSharedBusyCloseSucceededReceiptV1,
) -> bool {
    let values = receipt.initialization_values();
    let holder = receipt.holder_values();
    values[0..3] == [1, 7, 1]
        && values[3] != 0
        && values[4] != 0
        && values[5] == lifecycle::action_tag(binding.action)
        && values[6..9]
            == [
                u64::from(binding.first),
                u64::from(binding.count),
                u64::from(binding.mask),
            ]
        && values[9..13] == [1, 1, 1, 0]
        && values[13..23] == [1; 10]
        && values[23..29] == [0, 1, 0, 1, 1, 0]
        && values[29..39] == [2, 1, 1, 0, 1, 1, 1, 1, 1, 0]
        && values[39..43] == [1, 0, 1, 1]
        && holder
            == [
                values[3], values[4], 128, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
            ]
}

fn initialization_expectation(
    binding: LockRunnerNativeAcquireCreatedFirstSharedBusyCloseSucceededBindingV1,
) -> ManagedSqliteShmTestInitializationExpectationV1 {
    ManagedSqliteShmTestInitializationExpectationV1 {
        case_v1: ManagedSqliteShmTestInitializationFailureV1::CreatedFirstSharedBusyCloseSucceeded,
        action: lifecycle::managed_action(binding.action),
        first: binding.first,
        count: binding.count,
        mask: binding.mask,
    }
}

fn exact_no_requested_native_lock(
    expectation: ManagedSqliteShmTestInitializationExpectationV1,
    value: ManagedSqliteShmTestLockReceipt,
) -> bool {
    value.expectation.action == expectation.action
        && value.expectation.first == expectation.first
        && value.expectation.count == expectation.count
        && value.expectation.mask == expectation.mask
        && value.expectation.path == ManagedSqliteShmTestLockPath::InitializationFailure
        && value.runtime_generation != 0
        && value.shm_connection_id != 0
        && value.managed_attempts == 1
        && value.managed_successes == 0
        && value.native_lock_attempts == 0
        && value.native_lock_acquired == 0
        && value.native_lock_contended == 0
        && value.native_lock_errors == 0
        && value.native_unlock_attempts == 0
        && value.native_unlock_successes == 0
        && value.native_unlock_errors == 0
        && value.local_transitions == 0
        && value.finished
}

fn terminal_values(
    terminal: ManagedSqliteRegistryTerminalCustodyTestSnapshot,
    completion: LockRunnerCreatedFirstSharedBusyCloseSucceededCompletionV1,
) -> anyhow::Result<[u64; 18]> {
    let route = terminal
        .terminal_route()
        .ok_or_else(|| anyhow!("q18 terminal route missing"))?;
    let values = [
        terminal.retention_count() as u64,
        terminal.callback_lease_retention_count() as u64,
        terminal.completion_evidence_retention_count() as u64,
        terminal.wal_main_physical_custody_retention_count() as u64,
        terminal.other_terminal_custody_retention_count() as u64,
        terminal.explicit_failure_custody_retained_count() as u64,
        terminal.terminal_route_observation_count() as u64,
        terminal.route_removal_count() as u64,
        u64::from(terminal.active_route_present()),
        terminal.physical_success_handoff_retention_count() as u64,
        u64::from(terminal.active_access_callback_allowed()),
        u64::from(route.terminal_reason_is_failure_custody_retained()),
        u64::from(route.connection_owner()),
        u64::from(route.main_file_lock_owner_lease()),
        route.sidecar_lease_count() as u64,
        u64::from(route.shm_lease()),
        u64::from(route.callbacks_in_flight()),
        u64::from(route.access_callback_allowed()),
    ];
    let expected = match completion {
        LockRunnerCreatedFirstSharedBusyCloseSucceededCompletionV1::RetentionSucceeded => {
            [2, 1, 0, 0, 1, 1, 1, 1, 0, 0, 0, 1, 1, 1, 1, 1, 1, 0]
        }
        LockRunnerCreatedFirstSharedBusyCloseSucceededCompletionV1::RetentionRouteUnknown => {
            [3, 1, 0, 0, 2, 2, 1, 1, 0, 0, 0, 1, 1, 1, 1, 1, 1, 0]
        }
    };
    if values != expected
        || terminal.joint_close_physical_failure_retention_count() != 0
        || terminal.joint_close_physical_failure().is_some()
    {
        return Err(anyhow!("q18 terminal custody ledger mismatch"));
    }
    Ok(values)
}

pub(super) fn snapshot_values(value: ManagedSqliteShmTestTargetSnapshot) -> [u64; 14] {
    let topology = value.topology;
    [
        u64::from(value.target_attached),
        u64::from(value.shared_mask),
        u64::from(value.exclusive_mask),
        u64::from(topology.shm_connections),
        u64::from(topology.node_present),
        u64::from(topology.views),
        u64::from(topology.mappings),
        match topology.dms {
            ManagedSqliteShmTestDmsCustody::Absent => 0,
            ManagedSqliteShmTestDmsCustody::Shared => 1,
            ManagedSqliteShmTestDmsCustody::SharedOutcomeUncertain => 2,
            ManagedSqliteShmTestDmsCustody::ExclusiveKnown => 3,
            ManagedSqliteShmTestDmsCustody::ExclusiveOutcomeUncertain => 4,
            ManagedSqliteShmTestDmsCustody::Released => 5,
        },
        u64::from(topology.shm_file_present),
        u64::from(topology.poisoned),
        u64::from(topology.mutation_may_have_occurred),
        u64::from(topology.lock_outcome_uncertain),
        u64::from(topology.domain_terminal),
        u64::from(topology.quarantined_file_closes),
    ]
}

fn cold_setup_values(value: &RetainedQ18Fixture) -> [u64; 25] {
    let map = value.cold_map;
    let mut fields = [0; 25];
    fields[0] = u64::from(value.target_absent_before);
    fields[1] = 1;
    fields[2..11].copy_from_slice(&[
        map.region() as u64,
        map.region_size() as u64,
        map.raw_extend() as u64,
        map.result_code() as u64,
        u64::from(map.output_was_cleared()),
        u64::from(map.before().methods_installed),
        u64::from(map.before().state_installed),
        u64::from(map.after().methods_installed),
        u64::from(map.after().state_installed),
    ]);
    fields[11..25].copy_from_slice(&snapshot_values(value.cold_snapshot));
    fields
}

fn callback_values(
    value: ManagedTestLockCreatedFirstSharedBusyCloseSucceededObservationV1,
) -> [u64; 8] {
    let callback = value.callback;
    [
        callback.offset() as u64,
        callback.count() as u64,
        callback.raw_flags() as u64,
        callback.result_code() as u64,
        u64::from(callback.before().methods_installed),
        u64::from(callback.before().state_installed),
        u64::from(callback.after().methods_installed),
        u64::from(callback.after().state_installed),
    ]
}

pub(super) fn lock_values(value: ManagedSqliteShmTestLockReceipt) -> [u64; 18] {
    [
        value.runtime_generation,
        value.shm_connection_id,
        managed_action_tag(value.expectation.action),
        u64::from(value.expectation.first),
        u64::from(value.expectation.count),
        u64::from(value.expectation.mask),
        6,
        u64::from(value.managed_attempts),
        u64::from(value.managed_successes),
        u64::from(value.native_lock_attempts),
        u64::from(value.native_lock_acquired),
        u64::from(value.native_lock_contended),
        u64::from(value.native_lock_errors),
        u64::from(value.native_unlock_attempts),
        u64::from(value.native_unlock_successes),
        u64::from(value.native_unlock_errors),
        u64::from(value.local_transitions),
        u64::from(value.finished),
    ]
}

const fn managed_action_tag(
    action: crate::node_agent_managed_fs::ManagedSqliteShmLockAction,
) -> u64 {
    use crate::node_agent_managed_fs::ManagedSqliteShmLockAction as Action;
    match action {
        Action::LockShared => 1,
        Action::LockExclusive => 2,
        Action::UnlockShared => 3,
        Action::UnlockExclusive => 4,
    }
}
