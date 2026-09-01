//! Single-connection NodeLive fixture and exact native-busy receipt checks.

use std::path::Path;

use anyhow::anyhow;
use rusqlite::ffi;

use crate::node_agent_managed_fs::{
    ManagedSqliteShmLockAction, ManagedSqliteShmTestDmsCustody,
    ManagedSqliteShmTestLockExpectation, ManagedSqliteShmTestLockPath,
    ManagedSqliteShmTestLockReceipt, ManagedSqliteShmTestNativeContentionReceipt,
    ManagedSqliteShmTestTargetSnapshot,
};

use super::super::super::super::{
    connection::ManagedTestShmLockCallbackObservation, ManagedSqliteMultiConnectionFixture,
};
use super::super::{lifecycle, LockRunnerActionV1};
use super::LockRunnerNativeAcquireBusyBindingV1;

pub(super) const SELECTED: usize = 0;

pub(in super::super) fn prepare(
    root: &Path,
) -> anyhow::Result<ManagedSqliteMultiConnectionFixture> {
    lifecycle::fixture::prepare(root, super::super::LockRunnerLifecyclePathV1::NativeAcquire)
}

pub(super) fn lock_expectation(
    binding: LockRunnerNativeAcquireBusyBindingV1,
) -> ManagedSqliteShmTestLockExpectation {
    ManagedSqliteShmTestLockExpectation {
        action: match binding.action {
            LockRunnerActionV1::LockShared => ManagedSqliteShmLockAction::LockShared,
            LockRunnerActionV1::LockExclusive => ManagedSqliteShmLockAction::LockExclusive,
            _ => unreachable!("validated native-busy acquire action"),
        },
        first: binding.first,
        count: binding.count,
        mask: binding.mask,
        path: ManagedSqliteShmTestLockPath::NativeAcquire,
    }
}

pub(super) fn raw_flags(action: LockRunnerActionV1) -> i32 {
    lifecycle::raw_flags(action)
}

pub(super) fn action_tag(action: LockRunnerActionV1) -> u64 {
    lifecycle::action_tag(action)
}

pub(super) fn validate_baseline(value: ManagedSqliteShmTestTargetSnapshot) -> anyhow::Result<()> {
    if exact_live_snapshot(value) {
        Ok(())
    } else {
        Err(anyhow!("Lock native-busy NodeLive baseline mismatch"))
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn validate_action(
    binding: LockRunnerNativeAcquireBusyBindingV1,
    callback: ManagedTestShmLockCallbackObservation,
    before: ManagedSqliteShmTestTargetSnapshot,
    after: ManagedSqliteShmTestTargetSnapshot,
    lower: ManagedSqliteShmTestLockReceipt,
    holder: ManagedSqliteShmTestNativeContentionReceipt,
    pending_count: usize,
) -> anyhow::Result<()> {
    validate_baseline(before)?;
    if callback.offset() != i32::from(binding.first)
        || callback.count() != i32::from(binding.count)
        || callback.raw_flags() != raw_flags(binding.action)
        || callback.result_code() != ffi::SQLITE_BUSY
        || !callback.before().methods_installed
        || !callback.before().state_installed
        || !callback.after().methods_installed
        || !callback.after().state_installed
        || !exact_live_snapshot(after)
        || !exact_lower_receipt(binding, lower)
        || !exact_holder_receipt(binding, holder, lower)
        || pending_count != 0
    {
        return Err(anyhow!(
            "Lock native-busy installed callback receipt mismatch"
        ));
    }
    Ok(())
}

fn exact_live_snapshot(value: ManagedSqliteShmTestTargetSnapshot) -> bool {
    let topology = value.topology;
    value.target_attached
        && value.shared_mask == 0
        && value.exclusive_mask == 0
        && topology.shm_connections == 1
        && topology.node_present
        && topology.views == 1
        && topology.mappings == 1
        && topology.dms == ManagedSqliteShmTestDmsCustody::Shared
        && topology.shm_file_present
        && !topology.poisoned
        && !topology.mutation_may_have_occurred
        && !topology.lock_outcome_uncertain
        && !topology.domain_terminal
        && topology.quarantined_file_closes == 0
}

fn exact_lower_receipt(
    binding: LockRunnerNativeAcquireBusyBindingV1,
    value: ManagedSqliteShmTestLockReceipt,
) -> bool {
    value.runtime_generation != 0
        && value.shm_connection_id != 0
        && value.expectation == lock_expectation(binding)
        && value.managed_attempts == 1
        && value.managed_successes == 0
        && value.native_lock_attempts == 1
        && value.native_lock_acquired == 0
        && value.native_lock_contended == 1
        && value.native_lock_errors == 0
        && value.native_unlock_attempts == 0
        && value.native_unlock_successes == 0
        && value.native_unlock_errors == 0
        && value.local_transitions == 0
        && value.finished
}

fn exact_holder_receipt(
    binding: LockRunnerNativeAcquireBusyBindingV1,
    holder: ManagedSqliteShmTestNativeContentionReceipt,
    lower: ManagedSqliteShmTestLockReceipt,
) -> bool {
    holder.runtime_generation == lower.runtime_generation
        && holder.shm_connection_id == lower.shm_connection_id
        && holder.absolute_offset == 120 + u64::from(binding.first)
        && holder.length == u64::from(binding.count)
        && holder.target_identity_verified
        && holder.holder_identity_verified
        && holder.distinct_handle
        && holder.exclusive_holder
        && holder.acquire_attempts == 1
        && holder.acquired
        && holder.held_during_callback
        && holder.released
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
