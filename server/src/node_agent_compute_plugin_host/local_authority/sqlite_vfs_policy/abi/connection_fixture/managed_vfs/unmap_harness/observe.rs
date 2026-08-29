//! Runtime-only classification and counts for one installed `xShmUnmap` call.

use anyhow::{anyhow, Context};
use rusqlite::ffi;

use super::super::{
    a2b2_cases::UnmapActualCounts, connection::ManagedTestUnmapCallbackObservation,
    multi_connection::ManagedTestUnmapRouteObservation, ManagedTestCallbackFaultObservation,
    ManagedTestCallbackFaultOperation, ManagedTestCallbackFaultTiming,
    ManagedTestLifecycleFaultObservation, ManagedTestLifecycleFaultPhase,
    ManagedTestLifecycleFaultTiming, ManagedTestRouteOrdinal,
};
use super::{checked_u8, outcome::ObservedSharedUnmapOutcome};
use crate::{
    node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::{
        registry::ManagedSqliteRegistryUnmapRuntimeEvent, ManagedSqliteLogicalFileRole,
    },
    node_agent_managed_fs::{
        ManagedSqliteShmFailureClass, ManagedSqliteShmTriggeredTestFaultObservation,
    },
};

pub(super) struct UnmapEventSet<'a> {
    pub(super) route: ManagedTestRouteOrdinal,
    pub(super) raw: ManagedTestUnmapCallbackObservation,
    pub(super) pre: ManagedTestUnmapRouteObservation,
    pub(super) post: ManagedTestUnmapRouteObservation,
    pub(super) callback_observations: &'a [ManagedTestCallbackFaultObservation],
    pub(super) lifecycle_observations: &'a [ManagedTestLifecycleFaultObservation],
    pub(super) runtime_trace: &'a [ManagedSqliteRegistryUnmapRuntimeEvent],
    pub(super) shm_trigger: Option<ManagedSqliteShmTriggeredTestFaultObservation>,
    pub(super) callback_pending: usize,
    pub(super) lifecycle_pending: usize,
    pub(super) shm_pending: usize,
    pub(super) sibling_sql_usable: bool,
}

pub(super) fn classify_and_count(
    events: UnmapEventSet<'_>,
) -> anyhow::Result<(ObservedSharedUnmapOutcome, UnmapActualCounts)> {
    validate_raw_slots(events.raw)?;
    validate_initial_lock_masks(events.pre)?;
    if events.callback_pending != 0 || events.lifecycle_pending != 0 || events.shm_pending != 0 {
        return Err(anyhow!("installed Unmap stimulus remained pending"));
    }

    let outcome = if events.raw.raw_delete() == 2 {
        require_no_events(&events, "request validation")?;
        ObservedSharedUnmapOutcome::RequestValidation
    } else if admission_was_preterminal(&events) {
        require_no_events(&events, "callback admission")?;
        ObservedSharedUnmapOutcome::AdmissionRejected
    } else if wrapper_before_was_observed(&events)? {
        if events.shm_trigger.is_some() || !events.lifecycle_observations.is_empty() {
            return Err(anyhow!(
                "Unmap wrapper fault overlapped a later event family"
            ));
        }
        ObservedSharedUnmapOutcome::WrapperBefore
    } else if events.pre.physical.shared_mask == 1 && events.pre.physical.exclusive_mask == 0 {
        require_normal_completion(&events)?;
        ObservedSharedUnmapOutcome::HeldSharedLock
    } else if events.pre.physical.shared_mask == 0 && events.pre.physical.exclusive_mask == 1 {
        require_normal_completion(&events)?;
        ObservedSharedUnmapOutcome::HeldExclusiveLock
    } else if let Some(trigger) = events.shm_trigger {
        if !events.callback_observations.is_empty() {
            return Err(anyhow!(
                "Unmap detach fault overlapped a callback wrapper fault"
            ));
        }
        match (trigger.before_call, trigger.class) {
            (true, ManagedSqliteShmFailureClass::IoBeforeMutation) => {
                require_normal_completion(&events)?;
                ObservedSharedUnmapOutcome::DetachBefore
            }
            (false, ManagedSqliteShmFailureClass::MutatedButKnown) => {
                require_rejected_completion(&events)?;
                ObservedSharedUnmapOutcome::DetachAfterKnown
            }
            (false, ManagedSqliteShmFailureClass::OutcomeUncertainPoisoned) => {
                require_rejected_completion(&events)?;
                ObservedSharedUnmapOutcome::DetachAfterUncertain
            }
            _ => return Err(anyhow!("Unmap detach trigger timing/class is unsupported")),
        }
    } else {
        if !events.callback_observations.is_empty() {
            return Err(anyhow!("Unmap callback observation is not wrapper-before"));
        }
        classify_completion_or_success(&events)?
    };

    validate_runtime_trace(&events)?;
    validate_result_and_physical(outcome, &events)?;
    validate_terminal_custody(outcome, &events)?;
    validate_sibling_usability(outcome, events.sibling_sql_usable)?;
    Ok((outcome, counts_from_events(outcome, &events)?))
}

fn validate_raw_slots(raw: ManagedTestUnmapCallbackObservation) -> anyhow::Result<()> {
    let before = raw.before();
    let after = raw.after();
    if !before.methods_installed
        || !before.state_installed
        || !after.methods_installed
        || !after.state_installed
    {
        return Err(anyhow!(
            "SharedNonFinal Unmap did not preserve its installed raw slots"
        ));
    }
    if !matches!(raw.raw_delete(), 0..=2) {
        return Err(anyhow!(
            "Unmap harness emitted an unsupported raw delete value"
        ));
    }
    Ok(())
}

fn admission_was_preterminal(events: &UnmapEventSet<'_>) -> bool {
    !events.pre.terminal_custody.active_route_present()
        && events.pre.terminal_custody.route_removal_count() == 1
}

fn require_no_events(events: &UnmapEventSet<'_>, label: &'static str) -> anyhow::Result<()> {
    if !events.callback_observations.is_empty()
        || !events.lifecycle_observations.is_empty()
        || events.shm_trigger.is_some()
    {
        return Err(anyhow!("Unmap {label} reached a later event family"));
    }
    Ok(())
}

fn wrapper_before_was_observed(events: &UnmapEventSet<'_>) -> anyhow::Result<bool> {
    match events.callback_observations {
        [] => Ok(false),
        [observation] => {
            let step = observation.step();
            if step.route_ordinal() == events.route
                && step.role() == ManagedSqliteLogicalFileRole::Main
                && step.operation() == ManagedTestCallbackFaultOperation::ShmUnmap
                && step.occurrence() == 1
                && step.timing() == ManagedTestCallbackFaultTiming::BeforeCall
            {
                Ok(true)
            } else {
                Err(anyhow!("Unmap wrapper observation escaped its exact key"))
            }
        }
        _ => Err(anyhow!("Unmap wrapper produced multiple observations")),
    }
}

fn validate_initial_lock_masks(pre: ManagedTestUnmapRouteObservation) -> anyhow::Result<()> {
    if !matches!(
        (pre.physical.shared_mask, pre.physical.exclusive_mask),
        (0, 0) | (1, 0) | (0, 1)
    ) {
        return Err(anyhow!(
            "SharedNonFinal Unmap began with an unsupported lock-mask shape"
        ));
    }
    Ok(())
}

fn validate_runtime_trace(events: &UnmapEventSet<'_>) -> anyhow::Result<()> {
    use ManagedSqliteRegistryUnmapRuntimeEvent as E;

    let mut expected = Vec::with_capacity(6);
    if events.raw.raw_delete() != 2 && events.callback_observations.is_empty() {
        expected.push(E::CallbackBeginAttempt);
        if !admission_was_preterminal(events) {
            expected.push(E::CallbackBeginSuccess);
            if events.pre.physical.target_attached && !events.post.physical.target_attached {
                expected.push(E::SelectedActionAttempt);
                expected.push(E::SelectedActionSuccess);
            }
            expected.push(E::CallbackCompletionAttempt);
            if events.lifecycle_observations.iter().any(|observation| {
                observation.timing == ManagedTestLifecycleFaultTiming::AfterSuccess
            }) {
                expected.push(E::CallbackCompletionSuccess);
            }
        }
    }
    if events.runtime_trace != expected.as_slice() {
        return Err(anyhow!(
            "Unmap append-only runtime trace disagrees with its independent boundary evidence"
        ));
    }
    Ok(())
}

fn classify_completion_or_success(
    events: &UnmapEventSet<'_>,
) -> anyhow::Result<ObservedSharedUnmapOutcome> {
    validate_lifecycle_keys(events.route, events.lifecycle_observations)?;
    match events.lifecycle_observations {
        [before, native]
            if lifecycle(before, ManagedTestLifecycleFaultTiming::BeforeCall, false)
                && lifecycle(
                    native,
                    ManagedTestLifecycleFaultTiming::NativeFailure,
                    false,
                ) =>
        {
            Ok(ObservedSharedUnmapOutcome::CompletionNativeUncertain)
        }
        [before, after]
            if lifecycle(before, ManagedTestLifecycleFaultTiming::BeforeCall, false)
                && lifecycle(after, ManagedTestLifecycleFaultTiming::AfterSuccess, false) =>
        {
            match events.raw.raw_delete() {
                0 => Ok(ObservedSharedUnmapOutcome::KeepSuccess),
                1 => Ok(ObservedSharedUnmapOutcome::DeleteSuccess),
                _ => Err(anyhow!("successful Unmap used an invalid delete value")),
            }
        }
        _ => Err(anyhow!(
            "Unmap completion observations have no sealed outcome"
        )),
    }
}

fn require_normal_completion(events: &UnmapEventSet<'_>) -> anyhow::Result<()> {
    validate_lifecycle_keys(events.route, events.lifecycle_observations)?;
    match events.lifecycle_observations {
        [before, after]
            if lifecycle(before, ManagedTestLifecycleFaultTiming::BeforeCall, false)
                && lifecycle(after, ManagedTestLifecycleFaultTiming::AfterSuccess, false) =>
        {
            Ok(())
        }
        _ => Err(anyhow!("Unmap did not complete its callback normally")),
    }
}

fn require_rejected_completion(events: &UnmapEventSet<'_>) -> anyhow::Result<()> {
    validate_lifecycle_keys(events.route, events.lifecycle_observations)?;
    match events.lifecycle_observations {
        [before] if lifecycle(before, ManagedTestLifecycleFaultTiming::BeforeCall, false) => Ok(()),
        _ => Err(anyhow!(
            "Unmap terminal detach did not reject completion exactly once"
        )),
    }
}

fn validate_lifecycle_keys(
    route: ManagedTestRouteOrdinal,
    observations: &[ManagedTestLifecycleFaultObservation],
) -> anyhow::Result<()> {
    if observations.iter().any(|observation| {
        observation.route != Some(route)
            || observation.phase != ManagedTestLifecycleFaultPhase::UnmapCallbackCompletion
            || observation.occurrence != 1
    }) {
        return Err(anyhow!("Unmap lifecycle observation escaped its exact key"));
    }
    Ok(())
}

fn lifecycle(
    observation: &ManagedTestLifecycleFaultObservation,
    timing: ManagedTestLifecycleFaultTiming,
    triggered: bool,
) -> bool {
    observation.timing == timing && observation.triggered == triggered
}

fn validate_result_and_physical(
    outcome: ObservedSharedUnmapOutcome,
    events: &UnmapEventSet<'_>,
) -> anyhow::Result<()> {
    let expected_code = if outcome.is_success() {
        ffi::SQLITE_OK
    } else {
        ffi::SQLITE_IOERR
    };
    if events.raw.result_code() != expected_code
        || !events.pre.physical.target_attached
        || events.pre.physical.topology.shm_connections != 2
    {
        return Err(anyhow!(
            "Unmap result or pre-target observation is inconsistent"
        ));
    }
    let detached = !events.post.physical.target_attached;
    if detached != outcome.action_succeeded()
        || events.post.physical.topology.shm_connections != if detached { 1 } else { 2 }
        || events.post.physical.topology.domain_terminal != outcome.domain_terminal()
    {
        return Err(anyhow!(
            "Unmap post-target physical transition is inconsistent"
        ));
    }
    Ok(())
}

fn validate_terminal_custody(
    outcome: ObservedSharedUnmapOutcome,
    events: &UnmapEventSet<'_>,
) -> anyhow::Result<()> {
    let terminal = events.post.terminal_custody;
    if !outcome.route_terminal() {
        if !terminal.active_route_present()
            || terminal.retention_count() != 0
            || terminal.route_removal_count() != 0
            || events.post.active_custody.is_none()
        {
            return Err(anyhow!("active Unmap outcome changed registry custody"));
        }
        return Ok(());
    }
    let expected_callbacks = usize::from(outcome.callback_began());
    let expected_other = usize::from(matches!(
        outcome,
        ObservedSharedUnmapOutcome::AdmissionRejected
            | ObservedSharedUnmapOutcome::DetachAfterKnown
            | ObservedSharedUnmapOutcome::DetachAfterUncertain
    ));
    let expected_total = expected_callbacks + expected_other;
    if terminal.active_route_present()
        || events.post.active_custody.is_some()
        || terminal.route_removal_count() != 1
        || terminal.terminal_route_observation_count() != 1
        // Unlike a void Barrier callback, xShmUnmap has an error-code channel and therefore
        // preserves installed raw file custody. The exact terminal route owns that main lease;
        // only the callback lease and/or unsafe-SHM marker move into the retention ledger.
        || terminal.wal_main_physical_custody_retention_count() != 0
        || terminal.callback_lease_retention_count() != expected_callbacks
        || terminal.other_terminal_custody_retention_count() != expected_other
        || terminal.completion_evidence_retention_count() != 0
        || terminal.retention_count() != expected_total
        || terminal.explicit_failure_custody_retained_count() != 1
    {
        return Err(anyhow!(
            "terminal Unmap outcome lacks exact retained custody"
        ));
    }
    Ok(())
}

fn validate_sibling_usability(
    outcome: ObservedSharedUnmapOutcome,
    sibling_sql_usable: bool,
) -> anyhow::Result<()> {
    if outcome.domain_terminal() && sibling_sql_usable {
        return Err(anyhow!(
            "terminal Unmap SHM domain was incorrectly reported SQL-usable"
        ));
    }
    if !outcome.domain_terminal() && !sibling_sql_usable {
        return Err(anyhow!(
            "non-terminal Unmap outcome broke sibling SQL usability"
        ));
    }
    Ok(())
}

fn counts_from_events(
    _outcome: ObservedSharedUnmapOutcome,
    events: &UnmapEventSet<'_>,
) -> anyhow::Result<UnmapActualCounts> {
    use ManagedSqliteRegistryUnmapRuntimeEvent as E;

    let callback_begin = runtime_count(events.runtime_trace, E::CallbackBeginSuccess)?;
    let callback_complete_attempt =
        runtime_count(events.runtime_trace, E::CallbackCompletionAttempt)?;
    let callback_complete_success =
        runtime_count(events.runtime_trace, E::CallbackCompletionSuccess)?;
    let selected_action_attempt = runtime_count(events.runtime_trace, E::SelectedActionAttempt)?;
    let selected_action_success = runtime_count(events.runtime_trace, E::SelectedActionSuccess)?;
    let fault_events = events
        .callback_observations
        .len()
        .checked_add(usize::from(events.shm_trigger.is_some()))
        .context("Unmap observed fault count overflow")?;
    let retained = !events.post.terminal_custody.active_route_present()
        && events.post.terminal_custody.retention_count() != 0;
    Ok(UnmapActualCounts {
        callback_begin,
        callback_complete_attempt,
        callback_complete_success,
        selected_action_attempt,
        selected_action_success,
        shm_detach: selected_action_success,
        fault_observe: checked_u8(fault_events, "Unmap observed fault count")?,
        fault_trigger: checked_u8(fault_events, "Unmap triggered fault count")?,
        fault_pending: checked_u8(
            events.callback_pending + events.lifecycle_pending + events.shm_pending,
            "Unmap pending fault count",
        )?,
        custody_retain: u8::from(retained),
        ..UnmapActualCounts::default()
    })
}

fn runtime_count(
    trace: &[ManagedSqliteRegistryUnmapRuntimeEvent],
    expected: ManagedSqliteRegistryUnmapRuntimeEvent,
) -> anyhow::Result<u8> {
    checked_u8(
        trace.iter().filter(|event| **event == expected).count(),
        "Unmap runtime event count",
    )
}
