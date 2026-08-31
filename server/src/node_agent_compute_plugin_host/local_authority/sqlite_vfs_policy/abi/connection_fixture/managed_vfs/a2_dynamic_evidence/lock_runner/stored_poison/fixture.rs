//! Exact single-connection WAL prestate and physical snapshot checks for Lock q3.

use std::{ops::Deref, path::Path};

use anyhow::anyhow;

use crate::node_agent_managed_fs::{
    ManagedSqliteShmFailureClass, ManagedSqliteShmFailurePhase, ManagedSqliteShmLockAction,
    ManagedSqliteShmTestDmsCustody, ManagedSqliteShmTestLockExpectation,
    ManagedSqliteShmTestLockPath, ManagedSqliteShmTestLockReceipt,
    ManagedSqliteShmTestStoredPoisonReceiptV1, ManagedSqliteShmTestStoredPoisonV1,
    ManagedSqliteShmTestTargetSnapshot,
};

use super::super::super::super::ManagedSqliteMultiConnectionFixture;
use super::{LockRunnerStoredPoisonBindingV1, LockRunnerStoredPoisonProfileV1, SELECTED};

pub(super) struct RetainedStoredPoisonFixture {
    fixture: Option<ManagedSqliteMultiConnectionFixture>,
}

pub(super) fn prepare(root: &Path) -> anyhow::Result<RetainedStoredPoisonFixture> {
    let fixture = ManagedSqliteMultiConnectionFixture::open_single(root, [0xa6; 16])?;
    let mode: String =
        fixture
            .connection(SELECTED)?
            .query_row("PRAGMA journal_mode=WAL", [], |row| row.get(0))?;
    if !mode.eq_ignore_ascii_case("wal") {
        return Err(anyhow!("Lock stored-poison fixture did not enter WAL mode"));
    }
    fixture.route(SELECTED)?.into_schema_migration()?;
    fixture.connection(SELECTED)?.execute_batch(
        "CREATE TABLE lock_stored_poison_probe (
             probe_id INTEGER PRIMARY KEY,
             value INTEGER NOT NULL
         );",
    )?;
    fixture.route(SELECTED)?.into_runtime()?;
    fixture.connection(SELECTED)?.execute(
        "INSERT INTO lock_stored_poison_probe(probe_id, value) VALUES (1, 1320)",
        [],
    )?;
    Ok(RetainedStoredPoisonFixture {
        fixture: Some(fixture),
    })
}

impl Deref for RetainedStoredPoisonFixture {
    type Target = ManagedSqliteMultiConnectionFixture;

    fn deref(&self) -> &Self::Target {
        self.fixture
            .as_ref()
            .expect("retained Lock stored-poison fixture")
    }
}

impl Drop for RetainedStoredPoisonFixture {
    fn drop(&mut self) {
        if let Some(fixture) = self.fixture.take() {
            // A poisoned child must never retry SQLite teardown. All unsafe custody remains in
            // the child until process exit, after which the parent proves root deletion.
            std::mem::forget(fixture);
        }
    }
}

pub(super) fn managed_profile(
    profile: LockRunnerStoredPoisonProfileV1,
) -> ManagedSqliteShmTestStoredPoisonV1 {
    match profile {
        LockRunnerStoredPoisonProfileV1::GateNoMutation => {
            ManagedSqliteShmTestStoredPoisonV1::GateNoMutation
        }
        LockRunnerStoredPoisonProfileV1::FileCloseNoMutation => {
            ManagedSqliteShmTestStoredPoisonV1::FileCloseNoMutation
        }
        LockRunnerStoredPoisonProfileV1::ExactSiblingDeleteNoMutation => {
            ManagedSqliteShmTestStoredPoisonV1::ExactSiblingDeleteNoMutation
        }
        LockRunnerStoredPoisonProfileV1::ExactSiblingOpenUncertain => {
            ManagedSqliteShmTestStoredPoisonV1::ExactSiblingOpenUncertain
        }
        LockRunnerStoredPoisonProfileV1::DmsTruncateUncertain => {
            ManagedSqliteShmTestStoredPoisonV1::DmsTruncateUncertain
        }
        LockRunnerStoredPoisonProfileV1::FileCloseUncertain => {
            ManagedSqliteShmTestStoredPoisonV1::FileCloseUncertain
        }
        LockRunnerStoredPoisonProfileV1::ExactSiblingDeleteUncertain => {
            ManagedSqliteShmTestStoredPoisonV1::ExactSiblingDeleteUncertain
        }
        LockRunnerStoredPoisonProfileV1::FileGrowUncertain => {
            ManagedSqliteShmTestStoredPoisonV1::FileGrowUncertain
        }
        LockRunnerStoredPoisonProfileV1::MappingCloseUncertain => {
            ManagedSqliteShmTestStoredPoisonV1::MappingCloseUncertain
        }
        LockRunnerStoredPoisonProfileV1::ViewUnmapUncertain => {
            ManagedSqliteShmTestStoredPoisonV1::ViewUnmapUncertain
        }
        LockRunnerStoredPoisonProfileV1::LockReleaseUncertain => {
            ManagedSqliteShmTestStoredPoisonV1::LockReleaseUncertain
        }
        LockRunnerStoredPoisonProfileV1::ConnectionDetachUncertain => {
            ManagedSqliteShmTestStoredPoisonV1::ConnectionDetachUncertain
        }
        LockRunnerStoredPoisonProfileV1::DeleteAuthorizationUncertain => {
            ManagedSqliteShmTestStoredPoisonV1::DeleteAuthorizationUncertain
        }
        LockRunnerStoredPoisonProfileV1::DmsExclusiveReleaseUncertain => {
            ManagedSqliteShmTestStoredPoisonV1::DmsExclusiveReleaseUncertain
        }
        LockRunnerStoredPoisonProfileV1::DmsSharedReleaseUncertain => {
            ManagedSqliteShmTestStoredPoisonV1::DmsSharedReleaseUncertain
        }
    }
}

pub(super) fn validate_baseline(value: ManagedSqliteShmTestTargetSnapshot) -> anyhow::Result<()> {
    if snapshot_values(value) != [1, 0, 0, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0] {
        return Err(anyhow!("Lock stored-poison live baseline mismatch"));
    }
    Ok(())
}

pub(super) fn validate_poisoned_snapshot(
    profile: LockRunnerStoredPoisonProfileV1,
    value: ManagedSqliteShmTestTargetSnapshot,
) -> anyhow::Result<()> {
    let expected = [
        1,
        0,
        0,
        1,
        1,
        1,
        1,
        1,
        1,
        1,
        u64::from(profile.mutation_may_have_occurred()),
        u64::from(profile.lock_outcome_uncertain()),
        1,
        0,
    ];
    if snapshot_values(value) != expected {
        return Err(anyhow!(
            "Lock stored-poison exact poisoned snapshot mismatch"
        ));
    }
    Ok(())
}

pub(super) fn validate_poison_receipt(
    binding: LockRunnerStoredPoisonBindingV1,
    runtime_generation: u64,
    shm_connection_id: u64,
    receipt: ManagedSqliteShmTestStoredPoisonReceiptV1,
) -> anyhow::Result<()> {
    if receipt.runtime_generation != runtime_generation
        || receipt.shm_connection_id != shm_connection_id
        || receipt.profile != managed_profile(binding.profile)
        || receipt.phase != managed_phase(binding.profile)
        || receipt.class != ManagedSqliteShmFailureClass::OutcomeUncertainPoisoned
        || receipt.mutation_may_have_occurred != binding.profile.mutation_may_have_occurred()
        || receipt.lock_outcome_uncertain != binding.profile.lock_outcome_uncertain()
        || !receipt.domain_terminal
    {
        return Err(anyhow!("Lock stored-poison installer receipt mismatch"));
    }
    Ok(())
}

pub(super) fn lock_expectation(
    binding: LockRunnerStoredPoisonBindingV1,
) -> ManagedSqliteShmTestLockExpectation {
    ManagedSqliteShmTestLockExpectation {
        action: match binding.action {
            super::LockRunnerActionV1::LockShared => ManagedSqliteShmLockAction::LockShared,
            super::LockRunnerActionV1::LockExclusive => ManagedSqliteShmLockAction::LockExclusive,
            super::LockRunnerActionV1::UnlockShared => ManagedSqliteShmLockAction::UnlockShared,
            super::LockRunnerActionV1::UnlockExclusive => {
                ManagedSqliteShmLockAction::UnlockExclusive
            }
        },
        first: binding.first,
        count: binding.count,
        mask: binding.mask,
        path: match binding.action {
            super::LockRunnerActionV1::LockShared | super::LockRunnerActionV1::LockExclusive => {
                ManagedSqliteShmTestLockPath::NativeAcquire
            }
            super::LockRunnerActionV1::UnlockShared
            | super::LockRunnerActionV1::UnlockExclusive => {
                ManagedSqliteShmTestLockPath::NativeRelease
            }
        },
    }
}

pub(super) fn validate_no_attempt_receipt(
    binding: LockRunnerStoredPoisonBindingV1,
    runtime_generation: u64,
    shm_connection_id: u64,
    receipt: ManagedSqliteShmTestLockReceipt,
    pending_before: usize,
    pending_after: usize,
) -> anyhow::Result<()> {
    if receipt.runtime_generation != runtime_generation
        || receipt.shm_connection_id != shm_connection_id
        || receipt.expectation != lock_expectation(binding)
        || receipt.managed_attempts != 0
        || receipt.managed_successes != 0
        || receipt.native_lock_attempts != 0
        || receipt.native_lock_acquired != 0
        || receipt.native_lock_contended != 0
        || receipt.native_lock_errors != 0
        || receipt.native_unlock_attempts != 0
        || receipt.native_unlock_successes != 0
        || receipt.native_unlock_errors != 0
        || receipt.local_transitions != 0
        || pending_before != 0
        || pending_after != 0
        || !receipt.finished
    {
        return Err(anyhow!(
            "Lock stored-poison lower zero-event receipt mismatch"
        ));
    }
    Ok(())
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

pub(super) fn phase_tag(phase: ManagedSqliteShmFailurePhase) -> u64 {
    match phase {
        ManagedSqliteShmFailurePhase::Gate => 1,
        ManagedSqliteShmFailurePhase::FileClose => 2,
        ManagedSqliteShmFailurePhase::ExactSiblingDelete => 3,
        ManagedSqliteShmFailurePhase::ExactSiblingOpen => 4,
        ManagedSqliteShmFailurePhase::DmsTruncate => 5,
        ManagedSqliteShmFailurePhase::FileGrow => 6,
        ManagedSqliteShmFailurePhase::MappingClose => 7,
        ManagedSqliteShmFailurePhase::ViewUnmap => 8,
        ManagedSqliteShmFailurePhase::LockRelease => 9,
        ManagedSqliteShmFailurePhase::ConnectionDetach => 10,
        ManagedSqliteShmFailurePhase::DeleteAuthorization => 11,
        ManagedSqliteShmFailurePhase::DmsExclusiveRelease => 12,
        ManagedSqliteShmFailurePhase::DmsSharedRelease => 13,
        _ => 0,
    }
}

pub(super) fn managed_phase(
    profile: LockRunnerStoredPoisonProfileV1,
) -> ManagedSqliteShmFailurePhase {
    use ManagedSqliteShmFailurePhase as Phase;
    match profile {
        LockRunnerStoredPoisonProfileV1::GateNoMutation => Phase::Gate,
        LockRunnerStoredPoisonProfileV1::FileCloseNoMutation
        | LockRunnerStoredPoisonProfileV1::FileCloseUncertain => Phase::FileClose,
        LockRunnerStoredPoisonProfileV1::ExactSiblingDeleteNoMutation
        | LockRunnerStoredPoisonProfileV1::ExactSiblingDeleteUncertain => Phase::ExactSiblingDelete,
        LockRunnerStoredPoisonProfileV1::ExactSiblingOpenUncertain => Phase::ExactSiblingOpen,
        LockRunnerStoredPoisonProfileV1::DmsTruncateUncertain => Phase::DmsTruncate,
        LockRunnerStoredPoisonProfileV1::FileGrowUncertain => Phase::FileGrow,
        LockRunnerStoredPoisonProfileV1::MappingCloseUncertain => Phase::MappingClose,
        LockRunnerStoredPoisonProfileV1::ViewUnmapUncertain => Phase::ViewUnmap,
        LockRunnerStoredPoisonProfileV1::LockReleaseUncertain => Phase::LockRelease,
        LockRunnerStoredPoisonProfileV1::ConnectionDetachUncertain => Phase::ConnectionDetach,
        LockRunnerStoredPoisonProfileV1::DeleteAuthorizationUncertain => Phase::DeleteAuthorization,
        LockRunnerStoredPoisonProfileV1::DmsExclusiveReleaseUncertain => Phase::DmsExclusiveRelease,
        LockRunnerStoredPoisonProfileV1::DmsSharedReleaseUncertain => Phase::DmsSharedRelease,
    }
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
