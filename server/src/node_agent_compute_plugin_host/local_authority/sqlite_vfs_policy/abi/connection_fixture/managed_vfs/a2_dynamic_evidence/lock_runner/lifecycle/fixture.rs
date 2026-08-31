//! Exact WAL fixtures, action prestates and lower-ledger validation for Lock lifecycles.

use std::path::Path;

use anyhow::anyhow;
use rusqlite::ffi;

use crate::node_agent_managed_fs::{
    ManagedSqliteShmTestDmsCustody, ManagedSqliteShmTestLockReceipt,
    ManagedSqliteShmTestTargetSnapshot,
};

use super::super::super::super::{
    connection::ManagedTestShmLockCallbackObservation, ManagedSqliteMultiConnectionFixture,
};
use super::{
    lock_expectation, raw_flags, LockRunnerActionV1, LockRunnerLifecycleBindingV1,
    LockRunnerLifecyclePathV1, SELECTED, SIBLING,
};

pub(super) fn prepare(
    root: &Path,
    path: LockRunnerLifecyclePathV1,
) -> anyhow::Result<ManagedSqliteMultiConnectionFixture> {
    let fixture = if path.is_local() {
        ManagedSqliteMultiConnectionFixture::open(root, [0xa5; 16])?
    } else {
        ManagedSqliteMultiConnectionFixture::open_single(root, [0xa4; 16])?
    };
    let mode: String =
        fixture
            .connection(SELECTED)?
            .query_row("PRAGMA journal_mode=WAL", [], |row| row.get(0))?;
    if !mode.eq_ignore_ascii_case("wal") {
        return Err(anyhow!("Lock lifecycle fixture did not enter WAL mode"));
    }
    fixture.route(SELECTED)?.into_schema_migration()?;
    fixture.connection(SELECTED)?.execute_batch(
        "CREATE TABLE lock_lifecycle_probe (
             probe_id INTEGER PRIMARY KEY,
             value INTEGER NOT NULL
         );",
    )?;
    fixture.route(SELECTED)?.into_runtime()?;
    fixture.connection(SELECTED)?.execute(
        "INSERT INTO lock_lifecycle_probe(probe_id, value) VALUES (1, 104)",
        [],
    )?;
    if path.is_local() {
        fixture.route(SIBLING)?.into_schema_migration()?;
        fixture.route(SIBLING)?.into_runtime()?;
        let value: i64 = fixture.connection(SIBLING)?.query_row(
            "SELECT value FROM lock_lifecycle_probe WHERE probe_id=1",
            [],
            |row| row.get(0),
        )?;
        if value != 104 {
            return Err(anyhow!(
                "Lock lifecycle sibling did not share the WAL database"
            ));
        }
    }
    Ok(fixture)
}

pub(super) fn install_prestate(
    fixture: &ManagedSqliteMultiConnectionFixture,
    binding: LockRunnerLifecycleBindingV1,
) -> anyhow::Result<()> {
    match binding.path {
        LockRunnerLifecyclePathV1::NativeAcquire => {}
        LockRunnerLifecyclePathV1::NativeRelease => call_ok(
            fixture,
            SELECTED,
            binding.first,
            binding.count,
            binding.action.acquire_pair(),
            "native-release selected prestate",
        )?,
        LockRunnerLifecyclePathV1::SharedLocalAcquire => call_ok(
            fixture,
            SIBLING,
            binding.first,
            1,
            LockRunnerActionV1::LockShared,
            "shared-local-acquire sibling prestate",
        )?,
        LockRunnerLifecyclePathV1::SharedLocalRelease => {
            call_ok(
                fixture,
                SIBLING,
                binding.first,
                1,
                LockRunnerActionV1::LockShared,
                "shared-local-release sibling prestate",
            )?;
            call_ok(
                fixture,
                SELECTED,
                binding.first,
                1,
                LockRunnerActionV1::LockShared,
                "shared-local-release selected prestate",
            )?;
        }
    }
    Ok(())
}

pub(super) fn cleanup_locks(
    fixture: &ManagedSqliteMultiConnectionFixture,
    binding: LockRunnerLifecycleBindingV1,
) -> anyhow::Result<()> {
    match binding.path {
        LockRunnerLifecyclePathV1::NativeAcquire => call_ok(
            fixture,
            SELECTED,
            binding.first,
            binding.count,
            binding.action.release_pair(),
            "native-acquire cleanup",
        ),
        LockRunnerLifecyclePathV1::NativeRelease => Ok(()),
        LockRunnerLifecyclePathV1::SharedLocalAcquire => {
            call_ok(
                fixture,
                SELECTED,
                binding.first,
                1,
                LockRunnerActionV1::UnlockShared,
                "shared-local-acquire selected cleanup",
            )?;
            call_ok(
                fixture,
                SIBLING,
                binding.first,
                1,
                LockRunnerActionV1::UnlockShared,
                "shared-local-acquire sibling cleanup",
            )
        }
        LockRunnerLifecyclePathV1::SharedLocalRelease => call_ok(
            fixture,
            SIBLING,
            binding.first,
            1,
            LockRunnerActionV1::UnlockShared,
            "shared-local-release sibling cleanup",
        ),
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
    binding: LockRunnerLifecycleBindingV1,
    selected: ManagedSqliteShmTestTargetSnapshot,
    sibling: [u64; 3],
) -> anyhow::Result<()> {
    let (shared, exclusive, sibling_shared) = match binding.path {
        LockRunnerLifecyclePathV1::NativeAcquire => (0, 0, 0),
        LockRunnerLifecyclePathV1::NativeRelease => match binding.action {
            LockRunnerActionV1::UnlockShared => (binding.mask, 0, 0),
            LockRunnerActionV1::UnlockExclusive => (0, binding.mask, 0),
            _ => return Err(anyhow!("Lock lifecycle release action mismatch")),
        },
        LockRunnerLifecyclePathV1::SharedLocalAcquire => (0, 0, binding.mask),
        LockRunnerLifecyclePathV1::SharedLocalRelease => (binding.mask, 0, binding.mask),
    };
    if !exact_live_snapshot(selected, binding.path.connection_count(), shared, exclusive)
        || sibling
            != if binding.path.is_local() {
                [1, u64::from(sibling_shared), 0]
            } else {
                [0; 3]
            }
    {
        return Err(anyhow!("Lock lifecycle exact prestate mismatch"));
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub(super) fn validate_action(
    binding: LockRunnerLifecycleBindingV1,
    callback: ManagedTestShmLockCallbackObservation,
    before: ManagedSqliteShmTestTargetSnapshot,
    after: ManagedSqliteShmTestTargetSnapshot,
    sibling_before: [u64; 3],
    sibling_after: [u64; 3],
    receipt: ManagedSqliteShmTestLockReceipt,
    pending_count: usize,
) -> anyhow::Result<()> {
    validate_prestate(binding, before, sibling_before)?;
    let (shared, exclusive) = match binding.path {
        LockRunnerLifecyclePathV1::NativeAcquire => match binding.action {
            LockRunnerActionV1::LockShared => (binding.mask, 0),
            LockRunnerActionV1::LockExclusive => (0, binding.mask),
            _ => return Err(anyhow!("Lock lifecycle acquire action mismatch")),
        },
        LockRunnerLifecyclePathV1::NativeRelease
        | LockRunnerLifecyclePathV1::SharedLocalRelease => (0, 0),
        LockRunnerLifecyclePathV1::SharedLocalAcquire => (binding.mask, 0),
    };
    let expected_sibling = if binding.path.is_local() {
        [1, u64::from(binding.mask), 0]
    } else {
        [0; 3]
    };
    if callback.offset() != i32::from(binding.first)
        || callback.count() != i32::from(binding.count)
        || callback.raw_flags() != raw_flags(binding.action)
        || callback.result_code() != ffi::SQLITE_OK
        || !callback.before().methods_installed
        || !callback.before().state_installed
        || !callback.after().methods_installed
        || !callback.after().state_installed
        || !exact_live_snapshot(after, binding.path.connection_count(), shared, exclusive)
        || sibling_after != expected_sibling
        || !exact_lock_receipt(binding, receipt)
        || pending_count != 0
    {
        return Err(anyhow!("Lock lifecycle native callback receipt mismatch"));
    }
    Ok(())
}

fn exact_live_snapshot(
    value: ManagedSqliteShmTestTargetSnapshot,
    connection_count: u8,
    shared: u8,
    exclusive: u8,
) -> bool {
    let topology = value.topology;
    value.target_attached
        && value.shared_mask == shared
        && value.exclusive_mask == exclusive
        && topology.shm_connections == connection_count
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

fn exact_lock_receipt(
    binding: LockRunnerLifecycleBindingV1,
    value: ManagedSqliteShmTestLockReceipt,
) -> bool {
    value.runtime_generation != 0
        && value.shm_connection_id != 0
        && value.expectation == lock_expectation(binding)
        && value.managed_attempts == 1
        && value.managed_successes == 1
        && value.native_lock_attempts
            == u8::from(binding.path == LockRunnerLifecyclePathV1::NativeAcquire)
        && value.native_lock_acquired
            == u8::from(binding.path == LockRunnerLifecyclePathV1::NativeAcquire)
        && value.native_lock_contended == 0
        && value.native_lock_errors == 0
        && value.native_unlock_attempts
            == u8::from(binding.path == LockRunnerLifecyclePathV1::NativeRelease)
        && value.native_unlock_successes
            == u8::from(binding.path == LockRunnerLifecyclePathV1::NativeRelease)
        && value.native_unlock_errors == 0
        && value.local_transitions == u8::from(binding.path.is_local())
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

pub(super) fn dms_tag(value: ManagedSqliteShmTestDmsCustody) -> u64 {
    match value {
        ManagedSqliteShmTestDmsCustody::Absent => 0,
        ManagedSqliteShmTestDmsCustody::Shared => 1,
        ManagedSqliteShmTestDmsCustody::SharedOutcomeUncertain => 2,
        ManagedSqliteShmTestDmsCustody::ExclusiveKnown => 3,
        ManagedSqliteShmTestDmsCustody::ExclusiveOutcomeUncertain => 4,
        ManagedSqliteShmTestDmsCustody::Released => 5,
    }
}
