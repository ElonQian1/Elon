//! Low-level receipt for a complete SHM teardown before a main/registry boundary.

use anyhow::anyhow;

use crate::node_agent_managed_fs::{
    ManagedSqliteShmFailurePhase as Phase, ManagedSqliteShmTestUnmapActionEvent as Event,
    ManagedSqliteShmTestUnmapActionOutcome as Outcome, ManagedSqliteShmTestUnmapReceipt,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct JointClosePhysicalObserved {
    pub(super) shm_detach_attempt: u8,
    pub(super) shm_detach_success: u8,
}

impl JointClosePhysicalObserved {
    pub(super) const NONE: Self = Self {
        shm_detach_attempt: 0,
        shm_detach_success: 0,
    };
}

pub(super) fn validate_complete(
    receipt: &ManagedSqliteShmTestUnmapReceipt,
) -> anyhow::Result<JointClosePhysicalObserved> {
    if !receipt.finished
        || receipt.pending != 0
        || receipt.native.is_some()
        || receipt.prestate.is_some()
        || receipt.delete_outcome.is_some()
        || receipt.delete_authority.is_some()
    {
        return Err(anyhow!(
            "JointClose complete physical receipt contains pending or foreign evidence"
        ));
    }
    let expected = [
        event(Phase::ViewUnmap, Outcome::Attempt),
        event(Phase::ViewUnmap, Outcome::Success),
        event(Phase::MappingClose, Outcome::Attempt),
        event(Phase::MappingClose, Outcome::Success),
        event(Phase::DmsSharedRelease, Outcome::Attempt),
        event(Phase::DmsSharedRelease, Outcome::Success),
        event(Phase::FileClose, Outcome::Attempt),
        event(Phase::FileClose, Outcome::Success),
    ];
    if receipt.actions.as_slice() != expected {
        return Err(anyhow!(
            "JointClose complete physical action ledger is not exact"
        ));
    }
    validate_connection_detach(receipt, true)
}

pub(super) fn validate_connection_detach(
    receipt: &ManagedSqliteShmTestUnmapReceipt,
    expected_success: bool,
) -> anyhow::Result<JointClosePhysicalObserved> {
    let pair = [
        event(Phase::ConnectionDetach, Outcome::Attempt),
        event(Phase::ConnectionDetach, Outcome::Success),
    ];
    let expected = if expected_success {
        pair.as_slice()
    } else {
        &[]
    };
    if receipt.connection_detach.events.as_slice() != expected {
        return Err(anyhow!(
            "JointClose source-bound connection-detach receipt is not exact"
        ));
    }
    Ok(if expected_success {
        JointClosePhysicalObserved {
            shm_detach_attempt: 1,
            shm_detach_success: 1,
        }
    } else {
        JointClosePhysicalObserved::NONE
    })
}

fn event(phase: Phase, outcome: Outcome) -> Event {
    Event {
        phase,
        outcome,
        ordinal: 1,
    }
}
