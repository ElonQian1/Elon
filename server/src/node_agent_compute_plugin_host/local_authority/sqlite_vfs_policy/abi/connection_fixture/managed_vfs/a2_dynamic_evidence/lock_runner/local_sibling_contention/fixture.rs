//! Two-connection WAL fixture and exact coordinator-contention receipt checks.

use std::path::Path;

use anyhow::anyhow;
use rusqlite::ffi;

use crate::node_agent_managed_fs::{
    ManagedSqliteShmLockAction, ManagedSqliteShmTestDmsCustody,
    ManagedSqliteShmTestLockExpectation, ManagedSqliteShmTestLockPath,
    ManagedSqliteShmTestLockReceipt, ManagedSqliteShmTestTargetSnapshot,
};

use super::super::super::super::{
    connection::ManagedTestShmLockCallbackObservation, ManagedSqliteMultiConnectionFixture,
};
use super::super::{lifecycle, LockRunnerActionV1, LockRunnerLifecyclePathV1};
use super::LockRunnerLocalSiblingContentionBindingV1;

pub(super) const SELECTED: usize = 0;
pub(super) const SIBLING: usize = 1;

pub(in super::super) fn prepare(
    root: &Path,
) -> anyhow::Result<ManagedSqliteMultiConnectionFixture> {
    lifecycle::fixture::prepare(root, LockRunnerLifecyclePathV1::SharedLocalAcquire)
}

pub(super) fn lock_expectation(
    binding: LockRunnerLocalSiblingContentionBindingV1,
) -> ManagedSqliteShmTestLockExpectation {
    ManagedSqliteShmTestLockExpectation {
        action: match binding.action {
            LockRunnerActionV1::LockShared => ManagedSqliteShmLockAction::LockShared,
            LockRunnerActionV1::LockExclusive => ManagedSqliteShmLockAction::LockExclusive,
            _ => unreachable!("validated local sibling-contention acquire action"),
        },
        first: binding.first,
        count: binding.count,
        mask: binding.mask,
        path: ManagedSqliteShmTestLockPath::SiblingContention,
    }
}

pub(super) fn raw_flags(action: LockRunnerActionV1) -> i32 {
    lifecycle::raw_flags(action)
}

pub(super) fn action_tag(action: LockRunnerActionV1) -> u64 {
    lifecycle::action_tag(action)
}

pub(in super::super) fn install_prestate(
    fixture: &ManagedSqliteMultiConnectionFixture,
    binding: LockRunnerLocalSiblingContentionBindingV1,
) -> anyhow::Result<()> {
    match binding.action {
        LockRunnerActionV1::LockShared => call_ok(
            fixture,
            SIBLING,
            binding.first,
            1,
            LockRunnerActionV1::LockExclusive,
            "local sibling-contention exclusive prestate",
        ),
        LockRunnerActionV1::LockExclusive => {
            for first in binding.first..binding.first + binding.count {
                call_ok(
                    fixture,
                    SIBLING,
                    first,
                    1,
                    LockRunnerActionV1::LockShared,
                    "local sibling-contention shared prestate",
                )?;
            }
            Ok(())
        }
        _ => Err(anyhow!(
            "Lock local sibling-contention prestate action mismatch"
        )),
    }
}

pub(in super::super) fn cleanup_locks(
    fixture: &ManagedSqliteMultiConnectionFixture,
    binding: LockRunnerLocalSiblingContentionBindingV1,
) -> anyhow::Result<()> {
    match binding.action {
        LockRunnerActionV1::LockShared => call_ok(
            fixture,
            SIBLING,
            binding.first,
            1,
            LockRunnerActionV1::UnlockExclusive,
            "local sibling-contention exclusive cleanup",
        ),
        LockRunnerActionV1::LockExclusive => {
            for first in binding.first..binding.first + binding.count {
                call_ok(
                    fixture,
                    SIBLING,
                    first,
                    1,
                    LockRunnerActionV1::UnlockShared,
                    "local sibling-contention shared cleanup",
                )?;
            }
            Ok(())
        }
        _ => Err(anyhow!(
            "Lock local sibling-contention cleanup action mismatch"
        )),
    }
}

fn call_ok(
    fixture: &ManagedSqliteMultiConnectionFixture,
    index: usize,
    first: u8,
    count: u8,
    action: LockRunnerActionV1,
    label: &'static str,
) -> anyhow::Result<()> {
    let code = fixture
        .route(index)?
        .call_main_shm_lock_raw(i32::from(first), i32::from(count), raw_flags(action))
        .map_err(anyhow::Error::msg)?;
    if code == ffi::SQLITE_OK {
        Ok(())
    } else {
        Err(anyhow!("{label} failed with SQLite code {code}"))
    }
}

pub(super) fn validate_prestate(
    binding: LockRunnerLocalSiblingContentionBindingV1,
    selected: ManagedSqliteShmTestTargetSnapshot,
    sibling: ManagedSqliteShmTestTargetSnapshot,
) -> anyhow::Result<()> {
    let (sibling_shared, sibling_exclusive) = sibling_masks(binding)?;
    if !exact_live_snapshot(selected, 0, 0)
        || !exact_live_snapshot(sibling, sibling_shared, sibling_exclusive)
    {
        return Err(anyhow!(
            "Lock local sibling-contention exact prestate mismatch"
        ));
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub(super) fn validate_action(
    binding: LockRunnerLocalSiblingContentionBindingV1,
    callback: ManagedTestShmLockCallbackObservation,
    selected_before: ManagedSqliteShmTestTargetSnapshot,
    selected_after: ManagedSqliteShmTestTargetSnapshot,
    sibling_before: ManagedSqliteShmTestTargetSnapshot,
    sibling_after: ManagedSqliteShmTestTargetSnapshot,
    lower: ManagedSqliteShmTestLockReceipt,
    pending_count: usize,
) -> anyhow::Result<()> {
    validate_prestate(binding, selected_before, sibling_before)?;
    let (sibling_shared, sibling_exclusive) = sibling_masks(binding)?;
    if callback.offset() != i32::from(binding.first)
        || callback.count() != i32::from(binding.count)
        || callback.raw_flags() != raw_flags(binding.action)
        || callback.result_code() != ffi::SQLITE_BUSY
        || !callback.before().methods_installed
        || !callback.before().state_installed
        || !callback.after().methods_installed
        || !callback.after().state_installed
        || !exact_live_snapshot(selected_after, 0, 0)
        || !exact_live_snapshot(sibling_after, sibling_shared, sibling_exclusive)
        || !exact_lower_receipt(binding, lower)
        || pending_count != 0
    {
        return Err(anyhow!(
            "Lock local sibling-contention installed callback receipt mismatch"
        ));
    }
    Ok(())
}

pub(super) fn validate_cleanup(
    selected: ManagedSqliteShmTestTargetSnapshot,
    sibling: ManagedSqliteShmTestTargetSnapshot,
) -> anyhow::Result<()> {
    if exact_live_snapshot(selected, 0, 0) && exact_live_snapshot(sibling, 0, 0) {
        Ok(())
    } else {
        Err(anyhow!(
            "Lock local sibling-contention cleanup snapshot mismatch"
        ))
    }
}

fn sibling_masks(binding: LockRunnerLocalSiblingContentionBindingV1) -> anyhow::Result<(u8, u8)> {
    match binding.action {
        LockRunnerActionV1::LockShared => Ok((0, binding.mask)),
        LockRunnerActionV1::LockExclusive => Ok((binding.mask, 0)),
        _ => Err(anyhow!(
            "Lock local sibling-contention sibling action mismatch"
        )),
    }
}

fn exact_live_snapshot(
    value: ManagedSqliteShmTestTargetSnapshot,
    shared: u8,
    exclusive: u8,
) -> bool {
    let topology = value.topology;
    value.target_attached
        && value.shared_mask == shared
        && value.exclusive_mask == exclusive
        && topology.shm_connections == 2
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
    binding: LockRunnerLocalSiblingContentionBindingV1,
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

pub(super) fn sibling_values(value: ManagedSqliteShmTestTargetSnapshot) -> [u64; 3] {
    [
        u64::from(value.target_attached),
        u64::from(value.shared_mask),
        u64::from(value.exclusive_mask),
    ]
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
