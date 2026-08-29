//! Independent runtime event, physical transition and fault-receipt validation.

use anyhow::anyhow;
use rusqlite::ffi;

use super::super::super::{
    a2b2_cases::{UnmapActualCounts, UnmapSelector},
    connection::ManagedTestUnmapCallbackObservation,
    multi_connection::ManagedTestUnmapRouteObservation,
    ManagedTestLifecycleFaultObservation, ManagedTestLifecycleFaultPhase,
    ManagedTestLifecycleFaultTiming, ManagedTestRouteOrdinal,
};
use crate::{
    node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::registry::ManagedSqliteRegistryUnmapRuntimeEvent,
    node_agent_managed_fs::{
        ManagedSqliteShmFailureClass as Class, ManagedSqliteShmFailurePhase as Phase,
        ManagedSqliteShmTestUnmapReceipt, ManagedSqliteShmTriggeredTestFaultObservation,
    },
};

use super::liveness::{self, FinalSqliteLivenessReceipt};
use super::{action, outcome};

pub(super) struct ObservedGenericFault {
    pub(super) phase: Phase,
    pub(super) observed: bool,
    pub(super) trigger: Option<ManagedSqliteShmTriggeredTestFaultObservation>,
}

pub(super) struct FinalEventSet<'a> {
    pub(super) selector: UnmapSelector,
    pub(super) route: ManagedTestRouteOrdinal,
    pub(super) raw: ManagedTestUnmapCallbackObservation,
    pub(super) pre: ManagedTestUnmapRouteObservation,
    pub(super) post: ManagedTestUnmapRouteObservation,
    pub(super) callback_observation_count: usize,
    pub(super) lifecycle_observations: &'a [ManagedTestLifecycleFaultObservation],
    pub(super) runtime_trace: &'a [ManagedSqliteRegistryUnmapRuntimeEvent],
    pub(super) generic_faults: &'a [ObservedGenericFault],
    pub(super) callback_pending: usize,
    pub(super) lifecycle_pending: usize,
    pub(super) generic_pending: usize,
    pub(super) low_level: &'a ManagedSqliteShmTestUnmapReceipt,
    pub(super) sqlite_liveness: Option<&'a FinalSqliteLivenessReceipt>,
}

pub(super) fn validate_and_count(events: FinalEventSet<'_>) -> anyhow::Result<UnmapActualCounts> {
    liveness::validate(events.sqlite_liveness, events.selector, events.pre.target)?;
    validate_raw_and_physical(&events)?;
    validate_lifecycle(&events)?;
    let (outer_attempt, outer_success) = validate_runtime_trace(&events)?;
    let (fault_observe, fault_trigger) = validate_generic_fault(&events)?;
    let (selected_action_attempt, selected_action_success) = action::validate_and_count(
        events.selector,
        events.low_level,
        outer_attempt,
        outer_success,
    )?;
    let fault_pending = events
        .callback_pending
        .checked_add(events.lifecycle_pending)
        .and_then(|value| value.checked_add(events.generic_pending))
        .and_then(|value| value.checked_add(events.low_level.pending))
        .ok_or_else(|| anyhow!("final Unmap pending count overflow"))?;
    if events.callback_observation_count != 0
        || events.callback_pending != 0
        || events.lifecycle_pending != 0
        || fault_pending
            != usize::from(events.selector == UnmapSelector::FinalDeleteSuccessNotFound)
    {
        return Err(anyhow!(
            "final Unmap fault family overlap or pending mismatch"
        ));
    }

    Ok(UnmapActualCounts {
        callback_begin: runtime_count(
            events.runtime_trace,
            ManagedSqliteRegistryUnmapRuntimeEvent::CallbackBeginSuccess,
        )?,
        callback_complete_attempt: runtime_count(
            events.runtime_trace,
            ManagedSqliteRegistryUnmapRuntimeEvent::CallbackCompletionAttempt,
        )?,
        callback_complete_success: runtime_count(
            events.runtime_trace,
            ManagedSqliteRegistryUnmapRuntimeEvent::CallbackCompletionSuccess,
        )?,
        selected_action_attempt,
        selected_action_success,
        shm_detach: outer_success,
        fault_observe,
        fault_trigger,
        fault_pending: u8::try_from(fault_pending)?,
        custody_retain: u8::from(outcome::route_terminal(events.selector)),
        ..UnmapActualCounts::default()
    })
}

fn validate_raw_and_physical(events: &FinalEventSet<'_>) -> anyhow::Result<()> {
    let before = events.raw.before();
    let after = events.raw.after();
    let expected_delete = i32::from(outcome::is_delete(events.selector));
    let expected_code = if outcome::is_success(events.selector) {
        ffi::SQLITE_OK
    } else {
        ffi::SQLITE_IOERR
    };
    if events.raw.raw_delete() != expected_delete
        || events.raw.result_code() != expected_code
        || !before.methods_installed
        || !before.state_installed
        || !after.methods_installed
        || !after.state_installed
    {
        return Err(anyhow!("final Unmap raw ABI observation mismatch"));
    }

    let pre = events.pre.physical;
    let post = events.post.physical;
    let absent = outcome::node_absent(events.selector);
    if !pre.target_attached
        || pre.shared_mask != 0
        || pre.exclusive_mask != 0
        || pre.topology.shm_connections != 1
        || pre.topology.node_present == absent
        || (!absent
            && (pre.topology.views != 1
                || pre.topology.mappings != 1
                || !pre.topology.shm_file_present))
        || (absent
            && (pre.topology.views != 0
                || pre.topology.mappings != 0
                || pre.topology.shm_file_present))
    {
        return Err(anyhow!("final Unmap physical precondition mismatch"));
    }
    let detached = expected_detached(events.selector);
    if post.target_attached == detached
        || post.topology.shm_connections != u8::from(!detached)
        || post.topology.domain_terminal != outcome::domain_terminal(events.selector)
        || events.pre.target != events.post.target
    {
        return Err(anyhow!("final Unmap physical post-transition mismatch"));
    }
    Ok(())
}

fn validate_lifecycle(events: &FinalEventSet<'_>) -> anyhow::Result<()> {
    if events.lifecycle_observations.iter().any(|observation| {
        observation.route != Some(events.route)
            || observation.phase != ManagedTestLifecycleFaultPhase::UnmapCallbackCompletion
            || observation.occurrence != 1
    }) {
        return Err(anyhow!("final Unmap lifecycle key escaped its exact route"));
    }
    let observed = events.lifecycle_observations;
    let before = |value: &ManagedTestLifecycleFaultObservation| {
        value.timing == ManagedTestLifecycleFaultTiming::BeforeCall && !value.triggered
    };
    let after = |value: &ManagedTestLifecycleFaultObservation| {
        value.timing == ManagedTestLifecycleFaultTiming::AfterSuccess && !value.triggered
    };
    let native = |value: &ManagedTestLifecycleFaultObservation| {
        value.timing == ManagedTestLifecycleFaultTiming::NativeFailure && !value.triggered
    };
    let valid = if matches!(
        events.selector,
        UnmapSelector::FinalKeepCompletionNativeUncertain
            | UnmapSelector::FinalDeleteCompletionNativeUncertain
    ) {
        matches!(observed, [first, second] if before(first) && native(second))
    } else if outcome::completion_succeeds(events.selector) {
        matches!(observed, [first, second] if before(first) && after(second))
    } else {
        matches!(observed, [first] if before(first))
    };
    if !valid {
        return Err(anyhow!("final Unmap completion receipt shape mismatch"));
    }
    Ok(())
}

fn validate_runtime_trace(events: &FinalEventSet<'_>) -> anyhow::Result<(u8, u8)> {
    use ManagedSqliteRegistryUnmapRuntimeEvent as E;

    let detached = expected_detached(events.selector);
    let mut expected = vec![E::CallbackBeginAttempt, E::CallbackBeginSuccess];
    if detached {
        expected.extend([E::SelectedActionAttempt, E::SelectedActionSuccess]);
    }
    expected.push(E::CallbackCompletionAttempt);
    if outcome::completion_succeeds(events.selector) {
        expected.push(E::CallbackCompletionSuccess);
    }
    if events.runtime_trace != expected {
        return Err(anyhow!("final Unmap ordered registry trace mismatch"));
    }
    Ok((
        runtime_count(events.runtime_trace, E::SelectedActionAttempt)?,
        runtime_count(events.runtime_trace, E::SelectedActionSuccess)?,
    ))
}

fn validate_generic_fault(events: &FinalEventSet<'_>) -> anyhow::Result<(u8, u8)> {
    let expected = expected_generic(events.selector);
    let observed = events
        .generic_faults
        .iter()
        .filter(|fault| fault.observed || fault.trigger.is_some())
        .collect::<Vec<_>>();
    match expected {
        None if observed.is_empty() && events.generic_pending == 0 => Ok((0, 0)),
        Some((phase, None))
            if matches!(observed.as_slice(), [fault] if fault.phase == phase && fault.observed && fault.trigger.is_none())
                && events.generic_pending == 1 =>
        {
            Ok((1, 0))
        }
        Some((phase, Some((before_call, class))))
            if matches!(observed.as_slice(), [fault]
                if fault.phase == phase
                    && fault.observed
                    && matches!(fault.trigger, Some(trigger)
                        if trigger.before_call == before_call && trigger.class == class))
                && events.generic_pending == 0 =>
        {
            Ok((1, 1))
        }
        _ => Err(anyhow!("final Unmap generic fault receipt mismatch")),
    }
}

fn expected_generic(selector: UnmapSelector) -> Option<(Phase, Option<(bool, Class)>)> {
    use UnmapSelector as S;
    let before = Some((true, Class::IoBeforeMutation));
    let known = Some((false, Class::MutatedButKnown));
    let uncertain = Some((false, Class::OutcomeUncertainPoisoned));
    match selector {
        S::FinalKeepViewUnmapBefore => Some((Phase::ViewUnmap, before)),
        S::FinalKeepViewUnmapAfterKnown => Some((Phase::ViewUnmap, known)),
        S::FinalKeepViewUnmapAfterUncertain => Some((Phase::ViewUnmap, uncertain)),
        S::FinalKeepMappingCloseBefore => Some((Phase::MappingClose, before)),
        S::FinalKeepMappingCloseAfterKnown => Some((Phase::MappingClose, known)),
        S::FinalKeepMappingCloseAfterUncertain => Some((Phase::MappingClose, uncertain)),
        S::FinalKeepDmsReleaseBefore => Some((Phase::DmsSharedRelease, before)),
        S::FinalKeepDmsReleaseAfterKnown => Some((Phase::DmsSharedRelease, known)),
        S::FinalKeepDmsReleaseAfterUncertain => Some((Phase::DmsSharedRelease, uncertain)),
        S::FinalKeepFileCloseBefore => Some((Phase::FileClose, before)),
        S::FinalKeepFileCloseAfterKnown => Some((Phase::FileClose, known)),
        S::FinalKeepFileCloseAfterUncertain => Some((Phase::FileClose, uncertain)),
        S::FinalKeepDetachBefore | S::FinalDeleteDetachBefore => {
            Some((Phase::ConnectionDetach, before))
        }
        S::FinalKeepDetachAfterKnown | S::FinalDeleteDetachAfterKnown => {
            Some((Phase::ConnectionDetach, known))
        }
        S::FinalKeepDetachAfterUncertain | S::FinalDeleteDetachAfterUncertain => {
            Some((Phase::ConnectionDetach, uncertain))
        }
        S::FinalDeleteSiblingBefore => Some((Phase::ExactSiblingDelete, before)),
        S::FinalDeleteSiblingAfterKnown => Some((Phase::ExactSiblingDelete, known)),
        S::FinalDeleteSiblingAfterUncertain => Some((Phase::ExactSiblingDelete, uncertain)),
        S::FinalDeleteSuccessNotFound => Some((Phase::ExactSiblingDelete, None)),
        _ => None,
    }
}

pub(super) const GENERIC_PHASES: [Phase; 6] = [
    Phase::ViewUnmap,
    Phase::MappingClose,
    Phase::DmsSharedRelease,
    Phase::FileClose,
    Phase::ExactSiblingDelete,
    Phase::ConnectionDetach,
];

fn expected_detached(selector: UnmapSelector) -> bool {
    outcome::is_success(selector)
        || matches!(
            selector,
            UnmapSelector::FinalKeepDetachAfterKnown
                | UnmapSelector::FinalKeepDetachAfterUncertain
                | UnmapSelector::FinalKeepCompletionNativeUncertain
                | UnmapSelector::FinalDeleteDetachAfterKnown
                | UnmapSelector::FinalDeleteDetachAfterUncertain
                | UnmapSelector::FinalDeleteCompletionNativeUncertain
        )
}

fn runtime_count(
    trace: &[ManagedSqliteRegistryUnmapRuntimeEvent],
    expected: ManagedSqliteRegistryUnmapRuntimeEvent,
) -> anyhow::Result<u8> {
    Ok(u8::try_from(
        trace.iter().filter(|event| **event == expected).count(),
    )?)
}
