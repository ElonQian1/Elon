//! Strict lower, snapshot and terminal-ledger validation for the q7 fixture.

use anyhow::anyhow;
use rusqlite::ffi;

use crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::registry::ManagedSqliteRegistryTerminalCustodyTestSnapshot;
use crate::node_agent_managed_fs::{
    ManagedSqliteShmTestDmsCustody, ManagedSqliteShmTestLockReceipt,
    ManagedSqliteShmTestNativeContentionReceipt, ManagedSqliteShmTestTargetSnapshot,
};

use super::super::super::super::super::connection::ManagedTestShmLockCallbackObservation;
use super::super::super::{lifecycle, LockRunnerActionV1};
use super::super::{LockRunnerCallbackRouteUnknownBindingV1, LockRunnerCallbackRouteUnknownPathV1};
use super::lock_expectation;

#[allow(clippy::too_many_arguments)]
pub(super) fn validate_lower(
    binding: LockRunnerCallbackRouteUnknownBindingV1,
    callback: ManagedTestShmLockCallbackObservation,
    selected_before: ManagedSqliteShmTestTargetSnapshot,
    selected_after: ManagedSqliteShmTestTargetSnapshot,
    sibling_before: Option<ManagedSqliteShmTestTargetSnapshot>,
    sibling_after: Option<ManagedSqliteShmTestTargetSnapshot>,
    lower: ManagedSqliteShmTestLockReceipt,
    holder: Option<ManagedSqliteShmTestNativeContentionReceipt>,
    pending_count: usize,
) -> anyhow::Result<()> {
    validate_snapshots(binding, selected_before, sibling_before, false)?;
    validate_snapshots(binding, selected_after, sibling_after, true)?;
    let path = binding.path;
    let native_acquired = path == LockRunnerCallbackRouteUnknownPathV1::NativeAcquireAcquired;
    let native_busy = path == LockRunnerCallbackRouteUnknownPathV1::NativeAcquireBusy;
    let native_release = path == LockRunnerCallbackRouteUnknownPathV1::NativeRelease;
    let local = matches!(
        path,
        LockRunnerCallbackRouteUnknownPathV1::SharedLocalAcquire
            | LockRunnerCallbackRouteUnknownPathV1::SharedLocalRelease
    );
    if callback.offset() != i32::from(binding.first)
        || callback.count() != i32::from(binding.count)
        || callback.raw_flags() != lifecycle::raw_flags(binding.action)
        || callback.result_code() != ffi::SQLITE_IOERR_SHMLOCK
        || !callback.before().methods_installed
        || !callback.before().state_installed
        || !callback.after().methods_installed
        || !callback.after().state_installed
        || lower.expectation != lock_expectation(binding)
        || lower.managed_attempts != 1
        || lower.managed_successes != u8::from(!path.is_contended())
        || lower.native_lock_attempts != u8::from(native_acquired || native_busy)
        || lower.native_lock_acquired != u8::from(native_acquired)
        || lower.native_lock_contended != u8::from(native_busy)
        || lower.native_lock_errors != 0
        || lower.native_unlock_attempts != u8::from(native_release)
        || lower.native_unlock_successes != u8::from(native_release)
        || lower.native_unlock_errors != 0
        || lower.local_transitions != u8::from(local)
        || !lower.finished
        || pending_count != 0
        || !exact_holder(binding, holder, lower)
    {
        return Err(anyhow!(
            "Lock callback RouteUnknown real lower receipt mismatch"
        ));
    }
    Ok(())
}

fn exact_holder(
    binding: LockRunnerCallbackRouteUnknownBindingV1,
    holder: Option<ManagedSqliteShmTestNativeContentionReceipt>,
    lower: ManagedSqliteShmTestLockReceipt,
) -> bool {
    match (binding.path, holder) {
        (LockRunnerCallbackRouteUnknownPathV1::NativeAcquireBusy, Some(holder)) => {
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
        (LockRunnerCallbackRouteUnknownPathV1::NativeAcquireBusy, None) => false,
        (_, None) => true,
        (_, Some(_)) => false,
    }
}

pub(super) fn validate_snapshots(
    binding: LockRunnerCallbackRouteUnknownBindingV1,
    selected: ManagedSqliteShmTestTargetSnapshot,
    sibling: Option<ManagedSqliteShmTestTargetSnapshot>,
    after: bool,
) -> anyhow::Result<()> {
    let (selected_shared, selected_exclusive, sibling_shared, sibling_exclusive) =
        expected_masks(binding, after)?;
    if snapshot_values(selected)
        != live_snapshot(
            binding.path.connection_count(),
            selected_shared,
            selected_exclusive,
        )
        || sibling.map(snapshot_values)
            != if binding.path.connection_count() == 2 {
                Some(live_snapshot(2, sibling_shared, sibling_exclusive))
            } else {
                None
            }
    {
        return Err(anyhow!(
            "Lock callback RouteUnknown exact lower snapshot mismatch"
        ));
    }
    Ok(())
}

pub(super) fn validate_cleaned(
    binding: LockRunnerCallbackRouteUnknownBindingV1,
    selected: ManagedSqliteShmTestTargetSnapshot,
    sibling: Option<ManagedSqliteShmTestTargetSnapshot>,
) -> anyhow::Result<()> {
    let (selected_shared, selected_exclusive, _, _) = expected_masks(binding, true)?;
    if snapshot_values(selected)
        != live_snapshot(
            binding.path.connection_count(),
            selected_shared,
            selected_exclusive,
        )
        || sibling.map(snapshot_values)
            != if binding.path.connection_count() == 2 {
                Some(live_snapshot(2, 0, 0))
            } else {
                None
            }
    {
        return Err(anyhow!(
            "Lock callback RouteUnknown isolated sibling cleanup mismatch"
        ));
    }
    Ok(())
}

fn expected_masks(
    binding: LockRunnerCallbackRouteUnknownBindingV1,
    after: bool,
) -> anyhow::Result<(u8, u8, u8, u8)> {
    use LockRunnerCallbackRouteUnknownPathV1 as Path;
    let masks = match binding.path {
        Path::NativeAcquireAcquired if after => match binding.action {
            LockRunnerActionV1::LockShared => (binding.mask, 0, 0, 0),
            LockRunnerActionV1::LockExclusive => (0, binding.mask, 0, 0),
            _ => return Err(anyhow!("native-acquire action mismatch")),
        },
        Path::NativeAcquireAcquired | Path::NativeAcquireBusy => (0, 0, 0, 0),
        Path::NativeRelease if !after => match binding.action {
            LockRunnerActionV1::UnlockShared => (binding.mask, 0, 0, 0),
            LockRunnerActionV1::UnlockExclusive => (0, binding.mask, 0, 0),
            _ => return Err(anyhow!("native-release action mismatch")),
        },
        Path::NativeRelease => (0, 0, 0, 0),
        Path::SharedLocalAcquire if after => (binding.mask, 0, binding.mask, 0),
        Path::SharedLocalAcquire => (0, 0, binding.mask, 0),
        Path::SharedLocalRelease if after => (0, 0, binding.mask, 0),
        Path::SharedLocalRelease => (binding.mask, 0, binding.mask, 0),
        Path::LocalSiblingContention => match binding.action {
            LockRunnerActionV1::LockShared => (0, 0, 0, binding.mask),
            LockRunnerActionV1::LockExclusive => (0, 0, binding.mask, 0),
            _ => return Err(anyhow!("local sibling-contention action mismatch")),
        },
    };
    Ok(masks)
}

pub(in super::super) fn snapshot_values(value: ManagedSqliteShmTestTargetSnapshot) -> [u64; 14] {
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

fn live_snapshot(connection_count: u8, shared: u8, exclusive: u8) -> [u64; 14] {
    [
        1,
        u64::from(shared),
        u64::from(exclusive),
        u64::from(connection_count),
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

pub(in super::super) fn target_values(value: ManagedSqliteShmTestTargetSnapshot) -> [u64; 3] {
    [
        u64::from(value.target_attached),
        u64::from(value.shared_mask),
        u64::from(value.exclusive_mask),
    ]
}

pub(super) fn terminal_values(
    terminal: ManagedSqliteRegistryTerminalCustodyTestSnapshot,
) -> anyhow::Result<[u64; 18]> {
    let route = terminal
        .terminal_route()
        .ok_or_else(|| anyhow!("Lock callback RouteUnknown terminal route missing"))?;
    if terminal.joint_close_physical_failure_retention_count() != 0
        || terminal.joint_close_physical_failure().is_some()
    {
        return Err(anyhow!(
            "Lock callback RouteUnknown retained joint-close failure custody"
        ));
    }
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
    if values != [2, 1, 0, 0, 1, 1, 1, 1, 0, 0, 0, 1, 1, 1, 1, 1, 1, 0] {
        return Err(anyhow!(
            "Lock callback RouteUnknown completion custody ledger mismatch"
        ));
    }
    Ok(values)
}

impl LockRunnerCallbackRouteUnknownPathV1 {
    pub(in super::super) const fn connection_count(self) -> u8 {
        match self {
            Self::SharedLocalAcquire | Self::SharedLocalRelease | Self::LocalSiblingContention => 2,
            Self::NativeAcquireAcquired | Self::NativeAcquireBusy | Self::NativeRelease => 1,
        }
    }

    pub(in super::super) const fn is_contended(self) -> bool {
        matches!(self, Self::NativeAcquireBusy | Self::LocalSiblingContention)
    }
}
