//! Exact process-owner ledger seal for one real JointClose boundary.

use anyhow::anyhow;

use super::{BoundaryEvidence, ShmBoundary, ShmClass, ShmPhase, S};
use crate::node_agent_managed_fs::{
    ManagedSqliteMainFileCloseFailurePhase as MainPhase,
    ManagedSqliteWalMainCloseFailureTestBoundary as PhysicalBoundary,
};

pub(super) fn validate(evidence: &BoundaryEvidence<'_>) -> anyhow::Result<()> {
    let custody = evidence.custody;
    if evidence.selector == S::RawStateTakeRejected {
        return validate_active(custody, false);
    }
    if evidence.selector == S::PhysicalSuccess {
        return validate_active(custody, true);
    }

    let terminal = custody
        .terminal_route()
        .ok_or_else(|| anyhow!("JointClose failure has no terminal route receipt"))?;
    let expected = expected(evidence.selector)?;
    if custody.active_route_present()
        || custody.route_removal_count() != 1
        || custody.retention_count() != expected.total
        || custody.terminal_route_observation_count() != 1
        || custody.explicit_failure_custody_retained_count() != expected.explicit
        || custody.callback_lease_retention_count() != expected.callback
        || custody.completion_evidence_retention_count() != expected.completion
        || custody.wal_main_physical_custody_retention_count() != expected.wal_main
        || custody.other_terminal_custody_retention_count() != expected.other
        || custody.physical_success_handoff_retention_count() != 0
        || terminal.connection_owner() == false
        || terminal.main_file_lock_owner_lease() != expected.main_lease
        || terminal.shm_lease() != expected.shm_lease
        || terminal.callbacks_in_flight() != expected.callbacks
        || terminal.sidecar_lease_count() != 1
        || terminal.access_callback_allowed()
        || !match expected.reason {
            Reason::Failure => terminal.terminal_reason_is_failure_custody_retained(),
            Reason::Shm => terminal.terminal_reason_is_shm_teardown_unproven(),
            Reason::Handle => terminal.terminal_reason_is_handle_close_unproven(),
        }
    {
        return Err(anyhow!(
            "JointClose terminal route ledger is not exact: custody={custody:?} terminal={terminal:?} expected={expected:?}"
        ));
    }
    validate_physical_failure(evidence, expected.physical)
}

fn validate_active(
    custody: crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::registry::ManagedSqliteRegistryTerminalCustodyTestSnapshot,
    physical_success: bool,
) -> anyhow::Result<()> {
    let handoff = usize::from(physical_success);
    if !custody.active_route_present()
        || custody.active_access_callback_allowed() != !physical_success
        || custody.route_removal_count() != 0
        || custody.retention_count() != 0
        || custody.terminal_route().is_some()
        || custody.terminal_route_observation_count() != 0
        || custody.explicit_failure_custody_retained_count() != 0
        || custody.callback_lease_retention_count() != 0
        || custody.completion_evidence_retention_count() != 0
        || custody.wal_main_physical_custody_retention_count() != 0
        || custody.other_terminal_custody_retention_count() != 0
        || custody.joint_close_physical_failure_retention_count() != 0
        || custody.joint_close_physical_failure().is_some()
        || custody.physical_success_handoff_retention_count() != handoff
        || (physical_success
            && (custody.physical_success_handoff_shape() != (true, true, true, 1)
                || custody.physical_success_access_callback_allowed()))
    {
        return Err(anyhow!("JointClose active route custody is not exact"));
    }
    Ok(())
}

#[derive(Debug, Clone, Copy)]
struct Expected {
    total: usize,
    callback: usize,
    completion: usize,
    wal_main: usize,
    other: usize,
    explicit: usize,
    main_lease: bool,
    shm_lease: bool,
    callbacks: u32,
    reason: Reason,
    physical: bool,
}

#[derive(Debug, Clone, Copy)]
enum Reason {
    Failure,
    Shm,
    Handle,
}

fn expected(selector: S) -> anyhow::Result<Expected> {
    let early = matches!(
        selector,
        S::BeginConnectionCloseRejected | S::CallbackAdmissionRejected | S::CallbackWrapperBefore
    );
    if early {
        return Ok(shape(
            1,
            0,
            0,
            1,
            0,
            1,
            true,
            true,
            0,
            Reason::Failure,
            false,
        ));
    }
    if super::is_shm(selector) {
        return Ok(shape(2, 1, 0, 0, 1, 0, true, true, 1, Reason::Shm, true));
    }
    if super::is_main(selector) {
        return Ok(shape(2, 1, 0, 0, 1, 0, true, true, 1, Reason::Handle, true));
    }
    if selector == S::RegistryWalMainCloseBefore {
        return Ok(shape(
            2,
            0,
            1,
            1,
            0,
            1,
            true,
            true,
            0,
            Reason::Failure,
            false,
        ));
    }
    if selector == S::RegistryWalMainCloseNativeUncertain {
        return Ok(shape(
            2,
            1,
            0,
            1,
            0,
            1,
            true,
            true,
            1,
            Reason::Failure,
            false,
        ));
    }
    if selector == S::RegistryWalMainCloseAfterKnown {
        return Ok(shape(
            1,
            0,
            1,
            0,
            0,
            1,
            false,
            false,
            0,
            Reason::Failure,
            false,
        ));
    }
    Err(anyhow!("JointClose terminal selector is not frozen"))
}

#[allow(clippy::too_many_arguments)]
const fn shape(
    total: usize,
    callback: usize,
    completion: usize,
    wal_main: usize,
    other: usize,
    explicit: usize,
    main_lease: bool,
    shm_lease: bool,
    callbacks: u32,
    reason: Reason,
    physical: bool,
) -> Expected {
    Expected {
        total,
        callback,
        completion,
        wal_main,
        other,
        explicit,
        main_lease,
        shm_lease,
        callbacks,
        reason,
        physical,
    }
}

fn validate_physical_failure(
    evidence: &BoundaryEvidence<'_>,
    expected: bool,
) -> anyhow::Result<()> {
    let custody = evidence.custody;
    if !expected {
        if custody.joint_close_physical_failure_retention_count() != 0
            || custody.joint_close_physical_failure().is_some()
        {
            return Err(anyhow!(
                "JointClose non-physical failure has physical custody"
            ));
        }
        return Ok(());
    }
    if custody.joint_close_physical_failure_retention_count() != 1 {
        return Err(anyhow!(
            "JointClose physical failure receipt is not exact-once"
        ));
    }
    let actual = custody
        .joint_close_physical_failure()
        .ok_or_else(|| anyhow!("JointClose physical failure shape is absent"))?;
    if super::is_shm(evidence.selector) {
        let observed = evidence
            .shm
            .ok_or_else(|| anyhow!("JointClose SHM physical receipt is absent"))?;
        let class = match observed.boundary {
            ShmBoundary::Before if observed.phase == ShmPhase::ViewUnmap => {
                ShmClass::IoBeforeMutation
            }
            ShmBoundary::Before => ShmClass::MutatedButKnown,
            ShmBoundary::Native(_) => ShmClass::OutcomeUncertainPoisoned,
            ShmBoundary::After(class) => class,
        };
        if actual.boundary()
            != (PhysicalBoundary::Shm {
                phase: observed.phase,
                class,
            })
            || !actual.main_file_custody()
            || !actual.main_lock_owner_custody()
        {
            return Err(anyhow!("JointClose SHM typed failure custody is not exact"));
        }
        return Ok(());
    }
    let phase = if evidence.selector == S::MainLockReleaseBefore
        || evidence.selector == S::MainLockReleaseNativeUncertainShared
        || evidence.selector == S::MainLockReleaseNativeUncertainReserved
        || evidence.selector == S::MainLockReleaseAfterKnown
    {
        MainPhase::LockRelease
    } else {
        MainPhase::FileClose
    };
    let outcome_uncertain = matches!(
        evidence.selector,
        S::MainLockReleaseNativeUncertainShared
            | S::MainLockReleaseNativeUncertainReserved
            | S::MainFileCloseNativeUncertain
    );
    let retained =
        !(phase == MainPhase::FileClose && evidence.selector == S::MainFileCloseAfterKnown);
    if actual.boundary()
        != (PhysicalBoundary::Main {
            phase,
            outcome_uncertain,
        })
        || actual.main_file_custody() != retained
        || actual.main_lock_owner_custody() != retained
    {
        return Err(anyhow!(
            "JointClose main typed failure custody is not exact"
        ));
    }
    Ok(())
}
