//! Single-connection WAL fixture and exact q8 local-protocol observations.

use std::path::Path;

use anyhow::anyhow;
use rusqlite::ffi;

use crate::node_agent_managed_fs::{
    ManagedSqliteShmTestDmsCustody, ManagedSqliteShmTestLockExpectation,
    ManagedSqliteShmTestLockReceipt, ManagedSqliteShmTestTargetObserver,
    ManagedSqliteShmTestTargetSnapshot,
};

use super::super::super::super::{
    connection::ManagedTestShmLockCallbackObservation, ManagedSqliteMultiConnectionFixture,
    ManagedSqliteTestVfsRouteCustodySnapshot, ManagedSqliteTestVfsRoutePhase,
};
use super::super::{lifecycle, LockRunnerActionV1, LockRunnerLifecyclePathV1};
use super::{
    lock_expectation, LocalProtocolRejectionPathV1, LockRunnerLocalProtocolRejectionBindingV1,
};

pub(super) const SELECTED: usize = 0;

pub(super) struct ArmedLockObservation<'a> {
    observer: &'a ManagedSqliteShmTestTargetObserver,
    active: bool,
}

impl<'a> ArmedLockObservation<'a> {
    pub(super) fn begin(
        observer: &'a ManagedSqliteShmTestTargetObserver,
        expectation: ManagedSqliteShmTestLockExpectation,
    ) -> anyhow::Result<Self> {
        observer
            .begin_lock_action_observation(expectation)
            .map_err(anyhow::Error::msg)?;
        Ok(Self {
            observer,
            active: true,
        })
    }

    pub(super) fn finish(mut self) -> anyhow::Result<ManagedSqliteShmTestLockReceipt> {
        let receipt = self
            .observer
            .finish_lock_action_observation()
            .map_err(anyhow::Error::msg)?;
        self.active = false;
        Ok(receipt)
    }
}

impl Drop for ArmedLockObservation<'_> {
    fn drop(&mut self) {
        if self.active {
            let _ = self.observer.cancel_lock_action_observation();
        }
    }
}

pub(super) fn prepare(root: &Path) -> anyhow::Result<ManagedSqliteMultiConnectionFixture> {
    lifecycle::fixture::prepare(root, LockRunnerLifecyclePathV1::NativeAcquire)
}

pub(super) fn install_prestate(
    fixture: &ManagedSqliteMultiConnectionFixture,
    binding: LockRunnerLocalProtocolRejectionBindingV1,
) -> anyhow::Result<Option<ManagedTestShmLockCallbackObservation>> {
    if binding.path == LocalProtocolRejectionPathV1::NotHeld {
        return Ok(None);
    }
    let callback = observe(fixture, binding.action, binding.first, binding.count)?;
    validate_installed_callback(
        callback,
        binding.action,
        binding.first,
        binding.count,
        ffi::SQLITE_OK,
        "Lock local protocol-rejection setup",
    )?;
    Ok(Some(callback))
}

pub(super) fn validate_prestate(
    binding: LockRunnerLocalProtocolRejectionBindingV1,
    before: ManagedSqliteShmTestTargetSnapshot,
    setup: Option<ManagedTestShmLockCallbackObservation>,
) -> anyhow::Result<()> {
    let (shared, exclusive) = expected_held_masks(binding);
    let setup_present = setup.is_some();
    if setup_present != (binding.path == LocalProtocolRejectionPathV1::OwnOverlap)
        || !exact_live_snapshot(before, shared, exclusive)
    {
        return Err(anyhow!(
            "Lock local protocol-rejection exact prestate mismatch"
        ));
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub(super) fn validate_action(
    binding: LockRunnerLocalProtocolRejectionBindingV1,
    callback: ManagedTestShmLockCallbackObservation,
    before: ManagedSqliteShmTestTargetSnapshot,
    after: ManagedSqliteShmTestTargetSnapshot,
    lower: ManagedSqliteShmTestLockReceipt,
    pending_count: usize,
    active_route: [u64; 6],
) -> anyhow::Result<()> {
    let (shared, exclusive) = expected_held_masks(binding);
    validate_installed_callback(
        callback,
        binding.action,
        binding.first,
        binding.count,
        ffi::SQLITE_IOERR_SHMLOCK,
        "Lock local protocol-rejection target",
    )?;
    if !exact_live_snapshot(before, shared, exclusive)
        || after != before
        || !exact_lower_receipt(binding, lower)
        || pending_count != 0
        || active_route != [3, 1, 1, 1, 0, 1]
    {
        return Err(anyhow!(
            "Lock local protocol-rejection callback/receipt mismatch"
        ));
    }
    Ok(())
}

pub(super) fn cleanup(
    fixture: &ManagedSqliteMultiConnectionFixture,
    binding: LockRunnerLocalProtocolRejectionBindingV1,
) -> anyhow::Result<Option<ManagedTestShmLockCallbackObservation>> {
    if binding.path == LocalProtocolRejectionPathV1::NotHeld {
        return Ok(None);
    }
    let action = binding.action.release_pair();
    let callback = observe(fixture, action, binding.first, binding.count)?;
    validate_installed_callback(
        callback,
        action,
        binding.first,
        binding.count,
        ffi::SQLITE_OK,
        "Lock local protocol-rejection cleanup",
    )?;
    Ok(Some(callback))
}

pub(super) fn validate_cleanup(
    binding: LockRunnerLocalProtocolRejectionBindingV1,
    cleaned: ManagedSqliteShmTestTargetSnapshot,
    cleanup: Option<ManagedTestShmLockCallbackObservation>,
) -> anyhow::Result<()> {
    if cleanup.is_some() != (binding.path == LocalProtocolRejectionPathV1::OwnOverlap)
        || !exact_live_snapshot(cleaned, 0, 0)
    {
        return Err(anyhow!("Lock local protocol-rejection cleanup mismatch"));
    }
    Ok(())
}

pub(super) fn active_route_values(
    fixture: &ManagedSqliteMultiConnectionFixture,
) -> anyhow::Result<[u64; 6]> {
    let route = fixture
        .route(SELECTED)?
        .route_custody_snapshot()
        .map_err(anyhow::Error::msg)?;
    let values = route_values(route);
    if values != [3, 1, 1, 1, 0, 1] {
        return Err(anyhow!(
            "Lock local protocol-rejection active callback completion mismatch"
        ));
    }
    Ok(values)
}

fn observe(
    fixture: &ManagedSqliteMultiConnectionFixture,
    action: LockRunnerActionV1,
    first: u8,
    count: u8,
) -> anyhow::Result<ManagedTestShmLockCallbackObservation> {
    fixture
        .route(SELECTED)?
        .observe_main_shm_lock_raw(
            i32::from(first),
            i32::from(count),
            lifecycle::raw_flags(action),
        )
        .map_err(anyhow::Error::msg)
}

fn validate_installed_callback(
    callback: ManagedTestShmLockCallbackObservation,
    action: LockRunnerActionV1,
    first: u8,
    count: u8,
    result_code: i32,
    label: &'static str,
) -> anyhow::Result<()> {
    if callback.offset() != i32::from(first)
        || callback.count() != i32::from(count)
        || callback.raw_flags() != lifecycle::raw_flags(action)
        || callback.result_code() != result_code
        || !callback.before().methods_installed
        || !callback.before().state_installed
        || !callback.after().methods_installed
        || !callback.after().state_installed
    {
        return Err(anyhow!("{label} installed-ABI receipt mismatch"));
    }
    Ok(())
}

fn exact_lower_receipt(
    binding: LockRunnerLocalProtocolRejectionBindingV1,
    value: ManagedSqliteShmTestLockReceipt,
) -> bool {
    value.runtime_generation != 0
        && value.shm_connection_id != 0
        && value.expectation == lock_expectation(binding)
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

fn expected_held_masks(binding: LockRunnerLocalProtocolRejectionBindingV1) -> (u8, u8) {
    if binding.path == LocalProtocolRejectionPathV1::NotHeld {
        return (0, 0);
    }
    match binding.action {
        LockRunnerActionV1::LockShared => (binding.mask, 0),
        LockRunnerActionV1::LockExclusive => (0, binding.mask),
        LockRunnerActionV1::UnlockShared | LockRunnerActionV1::UnlockExclusive => (0, 0),
    }
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
        dms_tag(topology.dms),
        u64::from(topology.shm_file_present),
        u64::from(topology.poisoned),
        u64::from(topology.mutation_may_have_occurred),
        u64::from(topology.lock_outcome_uncertain),
        u64::from(topology.domain_terminal),
        u64::from(topology.quarantined_file_closes),
    ]
}

fn exact_live_snapshot(
    value: ManagedSqliteShmTestTargetSnapshot,
    shared: u8,
    exclusive: u8,
) -> bool {
    snapshot_values(value)
        == [
            1,
            u64::from(shared),
            u64::from(exclusive),
            1,
            1,
            1,
            1,
            1,
            1,
            0,
            0,
            0,
            0,
            0,
        ]
}

fn dms_tag(value: ManagedSqliteShmTestDmsCustody) -> u64 {
    match value {
        ManagedSqliteShmTestDmsCustody::Absent => 0,
        ManagedSqliteShmTestDmsCustody::Shared => 1,
        ManagedSqliteShmTestDmsCustody::SharedOutcomeUncertain => 2,
        ManagedSqliteShmTestDmsCustody::ExclusiveKnown => 3,
        ManagedSqliteShmTestDmsCustody::ExclusiveOutcomeUncertain => 4,
        ManagedSqliteShmTestDmsCustody::Released => 5,
    }
}

fn route_values(value: ManagedSqliteTestVfsRouteCustodySnapshot) -> [u64; 6] {
    [
        route_phase_tag(value.phase()),
        u64::from(value.connection_owner()),
        u64::from(value.main_file_lock_owner_lease()),
        u64::from(value.shm_lease()),
        value.callbacks_in_flight() as u64,
        u64::from(value.access_callback_allowed()),
    ]
}

fn route_phase_tag(value: ManagedSqliteTestVfsRoutePhase) -> u64 {
    match value {
        ManagedSqliteTestVfsRoutePhase::PendingMain => 1,
        ManagedSqliteTestVfsRoutePhase::Opening => 2,
        ManagedSqliteTestVfsRoutePhase::Active => 3,
        ManagedSqliteTestVfsRoutePhase::Closing => 4,
        ManagedSqliteTestVfsRoutePhase::AwaitingRouteRetirement => 5,
        ManagedSqliteTestVfsRoutePhase::Retired => 6,
        ManagedSqliteTestVfsRoutePhase::TerminalQuarantine => 7,
    }
}
