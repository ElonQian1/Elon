//! Exact-target SHM fault installation and low-level receipt validation for JointClose.

use anyhow::anyhow;

use super::{
    super::{a2b2_cases::JointCloseSelector, ManagedTestShmFaultPlanBinding},
    action::{self, JointClosePhysicalObserved},
};
use crate::node_agent_managed_fs::{
    ManagedSqliteShmFailureClass as Class, ManagedSqliteShmFailurePhase as Phase,
    ManagedSqliteShmTestUnmapActionEvent as Event,
    ManagedSqliteShmTestUnmapActionOutcome as Action,
    ManagedSqliteShmTestUnmapNativeObservation as NativeObservation,
    ManagedSqliteShmTestUnmapNativeOperation as Native,
    ManagedSqliteShmTestUnmapNativeTiming as NativeTiming, ManagedSqliteShmTestUnmapReceipt,
};

const PHYSICAL_PHASES: [Phase; 4] = [
    Phase::ViewUnmap,
    Phase::MappingClose,
    Phase::DmsSharedRelease,
    Phase::FileClose,
];
const OBSERVED_PHASES: [Phase; 5] = [
    Phase::ViewUnmap,
    Phase::MappingClose,
    Phase::DmsSharedRelease,
    Phase::FileClose,
    Phase::ConnectionDetach,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct ShmObserved {
    pub(super) phase: Phase,
    pub(super) boundary: ShmBoundary,
    pub(super) selected_action_attempt: u8,
    pub(super) selected_action_success: u8,
    pub(super) fault_observe: u8,
    pub(super) fault_trigger: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ShmBoundary {
    Before,
    Native(Native),
    After(Class),
}

#[derive(Debug, Clone, Copy)]
struct Selection {
    phase: Phase,
    boundary: ShmBoundary,
}

pub(super) fn install(
    binding: &ManagedTestShmFaultPlanBinding,
    selector: JointCloseSelector,
) -> anyhow::Result<()> {
    let selection = selection(selector)?;
    binding
        .begin_unmap_action_observation()
        .map_err(anyhow::Error::msg)?;
    match selection.boundary {
        ShmBoundary::Before => binding
            .install(&[(selection.phase, 1)], &[])
            .map_err(anyhow::Error::msg),
        ShmBoundary::Native(operation) => binding
            .install_unmap_native_operation(operation)
            .map_err(anyhow::Error::msg),
        ShmBoundary::After(class) => binding
            .install(&[], &[(selection.phase, 1, class)])
            .map_err(anyhow::Error::msg),
    }
}

pub(super) fn validate(
    binding: &ManagedTestShmFaultPlanBinding,
    selector: JointCloseSelector,
    receipt: &ManagedSqliteShmTestUnmapReceipt,
) -> anyhow::Result<(ShmObserved, JointClosePhysicalObserved)> {
    let selection = selection(selector)?;
    validate_receipt_shape(receipt)?;
    validate_actions(receipt, selection)?;
    validate_native(receipt, selection)?;
    let (fault_observe, fault_trigger) = validate_generic_fault(binding, selection)?;
    let physical = action::validate_connection_detach(
        receipt,
        selection.phase == Phase::ConnectionDetach
            && matches!(selection.boundary, ShmBoundary::After(_)),
    )?;
    let (selected_action_attempt, selected_action_success) =
        selected_action_counts(receipt, selection, physical)?;

    Ok((
        ShmObserved {
            phase: selection.phase,
            boundary: selection.boundary,
            selected_action_attempt,
            selected_action_success,
            fault_observe,
            fault_trigger,
        },
        physical,
    ))
}

fn selection(selector: JointCloseSelector) -> anyhow::Result<Selection> {
    use JointCloseSelector as S;

    let selected = match selector {
        S::ShmViewUnmapBefore => before(Phase::ViewUnmap),
        S::ShmViewUnmapNativeUncertain => native(Native::ViewUnmapOutcomeUncertain),
        S::ShmViewUnmapAfterKnown => after_known(Phase::ViewUnmap),
        S::ShmViewUnmapAfterUncertain => after_uncertain(Phase::ViewUnmap),
        S::ShmMappingCloseBefore => before(Phase::MappingClose),
        S::ShmMappingCloseNativeUncertain => native(Native::MappingCloseOutcomeUncertain),
        S::ShmMappingCloseAfterKnown => after_known(Phase::MappingClose),
        S::ShmMappingCloseAfterUncertain => after_uncertain(Phase::MappingClose),
        S::ShmDmsReleaseBefore => before(Phase::DmsSharedRelease),
        S::ShmDmsReleaseNativeUncertain => native(Native::DmsSharedReleaseOutcomeUncertain),
        S::ShmDmsReleaseAfterKnown => after_known(Phase::DmsSharedRelease),
        S::ShmDmsReleaseAfterUncertain => after_uncertain(Phase::DmsSharedRelease),
        S::ShmFileCloseBefore => before(Phase::FileClose),
        S::ShmFileCloseNativeRetryable => native(Native::FileCloseRetryable),
        S::ShmFileCloseNativeUncertain => native(Native::FileCloseOutcomeUncertain),
        S::ShmFileCloseAfterKnown => after_known(Phase::FileClose),
        S::ShmFileCloseAfterUncertain => after_uncertain(Phase::FileClose),
        S::ShmDetachBefore => before(Phase::ConnectionDetach),
        S::ShmDetachAfterKnown => after_known(Phase::ConnectionDetach),
        S::ShmDetachAfterUncertain => after_uncertain(Phase::ConnectionDetach),
        _ => {
            return Err(anyhow!(
                "JointClose selector is outside SHM adapter authority"
            ))
        }
    };
    Ok(selected)
}

fn before(phase: Phase) -> Selection {
    Selection {
        phase,
        boundary: ShmBoundary::Before,
    }
}

fn native(operation: Native) -> Selection {
    Selection {
        phase: operation.phase(),
        boundary: ShmBoundary::Native(operation),
    }
}

fn after_known(phase: Phase) -> Selection {
    Selection {
        phase,
        boundary: ShmBoundary::After(Class::MutatedButKnown),
    }
}

fn after_uncertain(phase: Phase) -> Selection {
    Selection {
        phase,
        boundary: ShmBoundary::After(Class::OutcomeUncertainPoisoned),
    }
}

fn validate_receipt_shape(receipt: &ManagedSqliteShmTestUnmapReceipt) -> anyhow::Result<()> {
    if !receipt.finished
        || receipt.pending != 0
        || receipt.prestate.is_some()
        || receipt.delete_outcome.is_some()
        || receipt.delete_authority.is_some()
    {
        return Err(anyhow!(
            "JointClose SHM receipt is unsealed, pending, or contains Delete-only evidence"
        ));
    }
    Ok(())
}

fn validate_actions(
    receipt: &ManagedSqliteShmTestUnmapReceipt,
    selection: Selection,
) -> anyhow::Result<()> {
    let expected = expected_actions(selection);
    if receipt.actions != expected {
        return Err(anyhow!(
            "JointClose SHM action ledger disagrees with the selected exact boundary"
        ));
    }
    Ok(())
}

fn expected_actions(selection: Selection) -> Vec<Event> {
    let mut events = Vec::with_capacity(PHYSICAL_PHASES.len() * 2);
    for phase in PHYSICAL_PHASES {
        if selection.phase == phase {
            match selection.boundary {
                ShmBoundary::Before => {}
                ShmBoundary::Native(_) => events.push(event(phase, Action::Attempt)),
                ShmBoundary::After(_) => push_success(&mut events, phase),
            }
            return events;
        }
        push_success(&mut events, phase);
    }
    // ConnectionDetach has its own source-bound occurrence receipt, separate from this physical
    // action ledger. Reaching it proves that all four physical phases completed.
    events
}

fn push_success(events: &mut Vec<Event>, phase: Phase) {
    events.push(event(phase, Action::Attempt));
    events.push(event(phase, Action::Success));
}

fn event(phase: Phase, outcome: Action) -> Event {
    Event {
        phase,
        outcome,
        ordinal: 1,
    }
}

fn validate_native(
    receipt: &ManagedSqliteShmTestUnmapReceipt,
    selection: Selection,
) -> anyhow::Result<()> {
    let expected = match selection.boundary {
        ShmBoundary::Native(operation) => Some(operation),
        ShmBoundary::Before | ShmBoundary::After(_) => None,
    };
    match (expected, receipt.native) {
        (None, None) => Ok(()),
        (Some(operation), Some(actual))
            if actual.operation == operation
                && actual.phase == selection.phase
                && actual.timing == operation.timing()
                && actual.triggered
                && actual.witnessed
                && actual.observation == Some(native_observation(operation.timing())) =>
        {
            Ok(())
        }
        _ => Err(anyhow!(
            "JointClose SHM native adapter receipt is absent, extra, or inexact"
        )),
    }
}

fn native_observation(timing: NativeTiming) -> NativeObservation {
    match timing {
        NativeTiming::Retryable => NativeObservation::NativeFailureObserved,
        NativeTiming::OutcomeUncertain => NativeObservation::ReturnReceiptUnavailable,
    }
}

fn validate_generic_fault(
    binding: &ManagedTestShmFaultPlanBinding,
    selection: Selection,
) -> anyhow::Result<(u8, u8)> {
    let expected = match selection.boundary {
        ShmBoundary::Before => Some((true, Class::IoBeforeMutation)),
        ShmBoundary::After(class) => Some((false, class)),
        ShmBoundary::Native(_) => None,
    };
    let mut observed_count = 0_u8;
    let mut trigger_count = 0_u8;
    for phase in OBSERVED_PHASES {
        let observed = binding
            .unmap_fault_was_observed(phase, 1)
            .map_err(anyhow::Error::msg)?;
        let triggered = binding
            .was_triggered(phase, 1)
            .map_err(anyhow::Error::msg)?;
        let trigger = binding
            .triggered_observation(phase, 1)
            .map_err(anyhow::Error::msg)?;
        if let (true, Some((before_call, class))) = (phase == selection.phase, expected) {
            if !observed
                || !triggered
                || !matches!(trigger, Some(value)
                    if value.before_call == before_call && value.class == class)
            {
                return Err(anyhow!(
                    "JointClose SHM generic fault did not prove its exact phase, timing, and class"
                ));
            }
            observed_count = observed_count
                .checked_add(1)
                .ok_or_else(|| anyhow!("JointClose SHM observed count overflow"))?;
            trigger_count = trigger_count
                .checked_add(1)
                .ok_or_else(|| anyhow!("JointClose SHM trigger count overflow"))?;
        } else if observed || triggered || trigger.is_some() {
            return Err(anyhow!(
                "JointClose SHM generic fault escaped the selected exact phase"
            ));
        }
    }
    if binding.pending_count().map_err(anyhow::Error::msg)? != 0 {
        return Err(anyhow!("JointClose SHM exact-target fault remains pending"));
    }
    Ok((observed_count, trigger_count))
}

fn selected_action_counts(
    receipt: &ManagedSqliteShmTestUnmapReceipt,
    selection: Selection,
    physical: JointClosePhysicalObserved,
) -> anyhow::Result<(u8, u8)> {
    if selection.phase == Phase::ConnectionDetach {
        return Ok((physical.shm_detach_attempt, physical.shm_detach_success));
    }
    let count = |outcome| {
        receipt
            .actions
            .iter()
            .filter(|event| event.phase == selection.phase && event.outcome == outcome)
            .count()
    };
    Ok((
        u8::try_from(count(Action::Attempt))?,
        u8::try_from(count(Action::Success))?,
    ))
}
