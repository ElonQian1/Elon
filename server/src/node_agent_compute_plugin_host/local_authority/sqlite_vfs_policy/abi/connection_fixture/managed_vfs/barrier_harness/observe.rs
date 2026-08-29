//! Exact event classification and operation counts for one direct void Barrier call.

use anyhow::{anyhow, Context};

use super::super::connection::ManagedTestVoidCallbackObservation;
use super::super::{
    a2b2_cases::BarrierActualCounts, ManagedTestCallbackFaultObservation,
    ManagedTestCallbackFaultOperation, ManagedTestCallbackFaultTiming,
    ManagedTestLifecycleFaultObservation, ManagedTestLifecycleFaultPhase,
    ManagedTestLifecycleFaultTiming, ManagedTestRouteOrdinal,
};
use super::ObservedBarrierOutcome;
use crate::{
    node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::{
        registry::ManagedSqliteRegistryTerminalCustodyTestSnapshot, ManagedSqliteLogicalFileRole,
    },
    node_agent_managed_fs::{
        ManagedSqliteShmFailureClass, ManagedSqliteShmTriggeredTestFaultObservation,
    },
};

pub(super) struct BarrierEventSet<'a> {
    pub(super) route: ManagedTestRouteOrdinal,
    pub(super) raw: ManagedTestVoidCallbackObservation,
    pub(super) terminal_before_call: ManagedSqliteRegistryTerminalCustodyTestSnapshot,
    pub(super) terminal_after_call: ManagedSqliteRegistryTerminalCustodyTestSnapshot,
    pub(super) callback_observations: &'a [ManagedTestCallbackFaultObservation],
    pub(super) lifecycle_observations: &'a [ManagedTestLifecycleFaultObservation],
    pub(super) shm_trigger: Option<ManagedSqliteShmTriggeredTestFaultObservation>,
    pub(super) callback_pending: usize,
    pub(super) lifecycle_pending: usize,
    pub(super) shm_pending: usize,
}

pub(super) fn classify_and_count(
    events: BarrierEventSet<'_>,
) -> anyhow::Result<(ObservedBarrierOutcome, BarrierActualCounts)> {
    validate_raw_slots(events.raw)?;
    if events.callback_pending != 0 || events.lifecycle_pending != 0 || events.shm_pending != 0 {
        return Err(anyhow!(
            "Barrier direct call left an installed fault pending"
        ));
    }

    let outcome = if admission_was_preterminal(&events) {
        require_no_fault_events(&events)?;
        ObservedBarrierOutcome::AdmissionRejected
    } else if wrapper_before_was_observed(&events)? {
        if events.shm_trigger.is_some() || !events.lifecycle_observations.is_empty() {
            return Err(anyhow!(
                "Barrier wrapper fault overlapped another event family"
            ));
        }
        ObservedBarrierOutcome::WrapperBefore
    } else if let Some(trigger) = events.shm_trigger {
        if !events.callback_observations.is_empty() || !events.lifecycle_observations.is_empty() {
            return Err(anyhow!(
                "Barrier fence fault overlapped another event family"
            ));
        }
        match (trigger.before_call, trigger.class) {
            (true, ManagedSqliteShmFailureClass::IoBeforeMutation) => {
                ObservedBarrierOutcome::FenceBefore
            }
            (false, ManagedSqliteShmFailureClass::OutcomeUncertainPoisoned) => {
                ObservedBarrierOutcome::FenceAfter
            }
            _ => {
                return Err(anyhow!(
                    "Barrier physical trigger class/timing is unsupported"
                ))
            }
        }
    } else {
        if !events.callback_observations.is_empty() {
            return Err(anyhow!(
                "Barrier callback observation is not the wrapper-before shape"
            ));
        }
        classify_completion(events.route, events.lifecycle_observations)?
    };

    validate_terminal_custody(outcome, &events)?;
    let counts = counts_from_events(outcome, &events)?;
    Ok((outcome, counts))
}

fn validate_raw_slots(raw: ManagedTestVoidCallbackObservation) -> anyhow::Result<()> {
    if !raw.before.methods_installed || !raw.before.state_installed {
        return Err(anyhow!(
            "Barrier raw state was not installed before the direct call"
        ));
    }
    let after_is_installed = raw.after.methods_installed && raw.after.state_installed;
    let after_is_cleared = !raw.after.methods_installed && !raw.after.state_installed;
    if !after_is_installed && !after_is_cleared {
        return Err(anyhow!(
            "Barrier raw method/state slots diverged after the direct call"
        ));
    }
    Ok(())
}

fn admission_was_preterminal(events: &BarrierEventSet<'_>) -> bool {
    !events.terminal_before_call.active_route_present()
        && events.terminal_before_call.route_removal_count() == 1
}

fn require_no_fault_events(events: &BarrierEventSet<'_>) -> anyhow::Result<()> {
    if !events.callback_observations.is_empty()
        || !events.lifecycle_observations.is_empty()
        || events.shm_trigger.is_some()
    {
        return Err(anyhow!(
            "Barrier admission rejection reached a later fault family"
        ));
    }
    Ok(())
}

fn wrapper_before_was_observed(events: &BarrierEventSet<'_>) -> anyhow::Result<bool> {
    match events.callback_observations {
        [] => Ok(false),
        [observation] => {
            let step = observation.step();
            Ok(step.route_ordinal() == events.route
                && step.role() == ManagedSqliteLogicalFileRole::Main
                && step.operation() == ManagedTestCallbackFaultOperation::ShmBarrier
                && step.occurrence() == 1
                && step.timing() == ManagedTestCallbackFaultTiming::BeforeCall)
        }
        _ => Err(anyhow!(
            "Barrier wrapper produced more than one callback observation"
        )),
    }
}

fn classify_completion(
    route: ManagedTestRouteOrdinal,
    observations: &[ManagedTestLifecycleFaultObservation],
) -> anyhow::Result<ObservedBarrierOutcome> {
    for observation in observations {
        if observation.route != Some(route)
            || observation.phase != ManagedTestLifecycleFaultPhase::BarrierCallbackCompletion
            || observation.occurrence != 1
        {
            return Err(anyhow!(
                "Barrier completion observation escaped the exact route/key"
            ));
        }
    }
    match observations {
        [before] if lifecycle(before, ManagedTestLifecycleFaultTiming::BeforeCall, true) => {
            Ok(ObservedBarrierOutcome::CompletionBefore)
        }
        [before, native]
            if lifecycle(before, ManagedTestLifecycleFaultTiming::BeforeCall, false)
                && lifecycle(
                    native,
                    ManagedTestLifecycleFaultTiming::NativeFailure,
                    false,
                ) =>
        {
            Ok(ObservedBarrierOutcome::CompletionNativeUncertain)
        }
        [before, after]
            if lifecycle(before, ManagedTestLifecycleFaultTiming::BeforeCall, false)
                && lifecycle(after, ManagedTestLifecycleFaultTiming::AfterSuccess, true) =>
        {
            Ok(ObservedBarrierOutcome::CompletionAfterSuccessKnown)
        }
        [before, after]
            if lifecycle(before, ManagedTestLifecycleFaultTiming::BeforeCall, false)
                && lifecycle(after, ManagedTestLifecycleFaultTiming::AfterSuccess, false) =>
        {
            Ok(ObservedBarrierOutcome::Success)
        }
        _ => Err(anyhow!(
            "Barrier completion observations have no sealed outcome"
        )),
    }
}

fn lifecycle(
    observation: &ManagedTestLifecycleFaultObservation,
    timing: ManagedTestLifecycleFaultTiming,
    triggered: bool,
) -> bool {
    observation.timing == timing && observation.triggered == triggered
}

fn validate_terminal_custody(
    outcome: ObservedBarrierOutcome,
    events: &BarrierEventSet<'_>,
) -> anyhow::Result<()> {
    let after = events.terminal_after_call;
    if outcome.is_success() {
        if !after.active_route_present()
            || after.retention_count() != 0
            || after.route_removal_count() != 0
        {
            return Err(anyhow!(
                "successful Barrier unexpectedly retained terminal custody"
            ));
        }
        return Ok(());
    }
    if after.active_route_present()
        || after.route_removal_count() != 1
        || after.terminal_route_observation_count() != 1
        || after.wal_main_physical_custody_retention_count() != 1
    {
        return Err(anyhow!(
            "failed Barrier lacks exact terminal custody evidence"
        ));
    }
    let expected_callbacks = usize::from(matches!(
        outcome,
        ObservedBarrierOutcome::FenceBefore
            | ObservedBarrierOutcome::FenceAfter
            | ObservedBarrierOutcome::CompletionBefore
            | ObservedBarrierOutcome::CompletionNativeUncertain
    ));
    if after.callback_lease_retention_count() != expected_callbacks {
        return Err(anyhow!(
            "Barrier callback-lease retention count is not event-exact"
        ));
    }
    let expected_completion_evidence = usize::from(matches!(
        outcome,
        ObservedBarrierOutcome::CompletionAfterSuccessKnown
    ));
    if after.completion_evidence_retention_count() != expected_completion_evidence {
        return Err(anyhow!(
            "Barrier completion-evidence retention disagrees with outcome"
        ));
    }
    let expected_other = usize::from(outcome == ObservedBarrierOutcome::AdmissionRejected);
    if after.other_terminal_custody_retention_count() != expected_other {
        return Err(anyhow!(
            "Barrier other terminal-custody retention is not selector-exact"
        ));
    }
    let expected_total = expected_callbacks + expected_completion_evidence + expected_other + 1;
    if after.retention_count() != expected_total
        || after.explicit_failure_custody_retained_count() != expected_total
    {
        return Err(anyhow!(
            "Barrier terminal retention family is not event-exact"
        ));
    }
    Ok(())
}

fn counts_from_events(
    outcome: ObservedBarrierOutcome,
    events: &BarrierEventSet<'_>,
) -> anyhow::Result<BarrierActualCounts> {
    let raw_failed = !events.raw.after.methods_installed && !events.raw.after.state_installed;
    if raw_failed == outcome.is_success() {
        return Err(anyhow!("Barrier raw state and sealed outcome disagree"));
    }
    let (fault_observe, fault_trigger) = match outcome {
        ObservedBarrierOutcome::AdmissionRejected | ObservedBarrierOutcome::Success => (0, 0),
        ObservedBarrierOutcome::CompletionNativeUncertain => (1, 0),
        _ => (1, 1),
    };
    Ok(BarrierActualCounts {
        raw_state_abandon: u8::from(raw_failed),
        methods_clear: u8::from(raw_failed),
        callback_begin: u8::from(outcome.callback_began()),
        callback_complete_attempt: u8::from(outcome.completion_attempted()),
        callback_complete_success: u8::from(outcome.completion_succeeded()),
        selected_action_attempt: u8::from(outcome.action_succeeded()),
        selected_action_success: u8::from(outcome.action_succeeded()),
        fault_observe,
        fault_trigger,
        fault_pending: checked_u8(
            events.callback_pending + events.lifecycle_pending + events.shm_pending,
            "Barrier pending fault count",
        )?,
        custody_retain: u8::from(
            events
                .terminal_after_call
                .wal_main_physical_custody_retention_count()
                == 1,
        ),
        ..BarrierActualCounts::default()
    })
}

fn checked_u8(value: usize, label: &'static str) -> anyhow::Result<u8> {
    u8::try_from(value).with_context(|| format!("{label} exceeds u8"))
}
