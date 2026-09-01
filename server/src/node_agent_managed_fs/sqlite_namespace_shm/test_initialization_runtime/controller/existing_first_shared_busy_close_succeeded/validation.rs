//! Q19 terminal, receipt, and ledger validation.

use crate::node_agent_managed_fs::ManagedSqliteFileKind;

use super::super::super::super::{
    test_lock_runtime::{ManagedSqliteShmTestLockPath, ManagedSqliteShmTestLockReceipt},
    test_snapshot::{ManagedSqliteShmTestDmsCustody, ManagedSqliteShmTestTargetSnapshot},
    types::SHM_DMS_OFFSET,
};
use super::super::super::model::lock_action_tag;
use super::super::ExactTarget;
use super::state::{ArmedQ19ObservationV1, EventCounts, Stage};

pub(super) fn validate_completion(active: &ArmedQ19ObservationV1) -> Result<(), &'static str> {
    if active.violation.is_some()
        || active.stage != Stage::TargetCloseSucceeded
        || active.pending != 0
        || !active.consumed
        || active.close_kind != Some(ManagedSqliteFileKind::Shm)
    {
        return Err("NODE_MANAGED_SQLITE_SHM_TEST_Q19_INCOMPLETE_OR_INVALID");
    }
    validate_counts(&active.counts)
}

pub(super) fn validate_terminal(
    value: ManagedSqliteShmTestTargetSnapshot,
) -> Result<(), &'static str> {
    let topology = value.topology;
    if !value.target_attached
        || value.shared_mask != 0
        || value.exclusive_mask != 0
        || topology.shm_connections != 1
        || topology.node_present
        || topology.views != 0
        || topology.mappings != 0
        || topology.dms != ManagedSqliteShmTestDmsCustody::Absent
        || topology.shm_file_present
        || topology.poisoned
        || topology.mutation_may_have_occurred
        || topology.lock_outcome_uncertain
        || topology.domain_terminal
        || topology.quarantined_file_closes != 0
    {
        return Err("NODE_MANAGED_SQLITE_SHM_TEST_Q19_TERMINAL_STATE_INVALID");
    }
    Ok(())
}

pub(super) fn validate_requested_lock(
    active: &ArmedQ19ObservationV1,
    receipt: ManagedSqliteShmTestLockReceipt,
) -> Result<(), &'static str> {
    let target = active.target;
    let expectation = active.expectation;
    if receipt.runtime_generation != target.0
        || receipt.shm_connection_id != target.1
        || receipt.expectation.path != ManagedSqliteShmTestLockPath::InitializationFailure
        || receipt.expectation.action != expectation.action
        || receipt.expectation.first != expectation.first
        || receipt.expectation.count != expectation.count
        || receipt.expectation.mask != expectation.mask
        || receipt.managed_attempts != 1
        || receipt.managed_successes != 0
        || receipt.native_lock_attempts != 0
        || receipt.native_lock_acquired != 0
        || receipt.native_lock_contended != 0
        || receipt.native_lock_errors != 0
        || receipt.native_unlock_attempts != 0
        || receipt.native_unlock_successes != 0
        || receipt.native_unlock_errors != 0
        || receipt.local_transitions != 0
        || !receipt.finished
    {
        return Err("NODE_MANAGED_SQLITE_SHM_TEST_Q19_REQUESTED_LOCK_LEDGER_INVALID");
    }
    Ok(())
}

pub(super) fn validate_holder_values(
    target: ExactTarget,
    values: [u64; 15],
) -> Result<(), &'static str> {
    if values
        != [
            target.0,
            target.1,
            SHM_DMS_OFFSET,
            1,
            1,
            1,
            1,
            1,
            1,
            1,
            1,
            1,
            1,
            1,
            1,
        ]
    {
        return Err("NODE_MANAGED_SQLITE_SHM_TEST_Q19_HOLDER_RECEIPT_INVALID");
    }
    Ok(())
}

pub(super) fn initialization_values(active: &ArmedQ19ObservationV1) -> [u64; 43] {
    let cold_flags = u64::from(active.cold.node_present)
        | (u64::from(active.cold.shm_file_present) << 1)
        | (u64::from(active.cold.poisoned) << 2)
        | (u64::from(active.cold.domain_terminal) << 3)
        | (u64::from(active.cold.shared_mask != 0) << 4)
        | (u64::from(active.cold.exclusive_mask != 0) << 5);
    [
        1,
        active.expectation.case_v1.tag(),
        1,
        active.target.0,
        active.target.1,
        lock_action_tag(active.expectation.action),
        u64::from(active.expectation.first),
        u64::from(active.expectation.count),
        u64::from(active.expectation.mask),
        u64::from(active.owner_thread == std::thread::current().id()),
        u64::from(active.cold.target_attached),
        u64::from(active.cold.shm_connections),
        cold_flags,
        u64::from(active.counts.request),
        u64::from(active.counts.open_attempt),
        u64::from(active.counts.open_existing),
        u64::from(active.counts.exclusive_lock_attempt),
        u64::from(active.counts.exclusive_lock_acquired),
        u64::from(active.counts.truncate_attempt),
        u64::from(active.counts.truncate_success),
        u64::from(active.counts.exclusive_unlock_attempt),
        u64::from(active.counts.exclusive_unlock_success),
        u64::from(active.counts.target_shared_attempt),
        u64::from(active.counts.target_shared_acquired),
        u64::from(active.counts.target_shared_contended),
        u64::from(active.counts.target_shared_errors),
        u64::from(active.counts.target_close_attempt),
        u64::from(active.counts.target_close_success),
        u64::from(active.counts.target_close_failure),
        2,
        1,
        1,
        0,
        1,
        1,
        1,
        1,
        1,
        0,
        u64::from(active.close_kind == Some(ManagedSqliteFileKind::Shm)),
        u64::from(active.pending),
        u64::from(active.consumed),
        1,
    ]
}

fn validate_counts(value: &EventCounts) -> Result<(), &'static str> {
    if value.request != 1
        || value.open_attempt != 1
        || value.open_existing != 1
        || value.exclusive_lock_attempt != 1
        || value.exclusive_lock_acquired != 1
        || value.truncate_attempt != 1
        || value.truncate_success != 1
        || value.exclusive_unlock_attempt != 1
        || value.exclusive_unlock_success != 1
        || value.target_shared_attempt != 1
        || value.target_shared_acquired != 0
        || value.target_shared_contended != 1
        || value.target_shared_errors != 0
        || value.target_close_attempt != 1
        || value.target_close_success != 1
        || value.target_close_failure != 0
    {
        return Err("NODE_MANAGED_SQLITE_SHM_TEST_Q19_EVENT_COUNTS_INVALID");
    }
    Ok(())
}
