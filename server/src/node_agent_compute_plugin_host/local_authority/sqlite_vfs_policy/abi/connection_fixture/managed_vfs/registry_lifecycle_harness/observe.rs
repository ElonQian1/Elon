//! Append-only stage classification and count projection for one selected close.

use anyhow::{anyhow, Context};

use super::{
    super::{
        a2b2_cases::RegistryLifecycleActualCounts, ManagedTestLifecycleFaultObservation,
        ManagedTestLifecycleFaultPhase, ManagedTestLifecycleFaultTiming, ManagedTestRouteOrdinal,
    },
    outcome::ObservedRegistryLifecycleOutcome,
};
use crate::node_agent_compute_plugin_host::local_authority::{
    sqlite_vfs_abi::HandleBoundSqliteAbiRawCloseWitnessSnapshot,
    sqlite_vfs_policy::registry::ManagedSqliteRegistryLifecycleStage,
};

pub(super) struct RegistryLifecycleEvents<'a> {
    pub(super) route: ManagedTestRouteOrdinal,
    pub(super) stages: &'a [ManagedSqliteRegistryLifecycleStage],
    pub(super) observations: &'a [ManagedTestLifecycleFaultObservation],
    pub(super) pending_steps: usize,
    pub(super) pending_controls: usize,
    pub(super) close_disposition: RegistryLifecycleCloseDisposition,
    pub(super) shared_pre_topology: bool,
    pub(super) raw_close: HandleBoundSqliteAbiRawCloseWitnessSnapshot,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum RegistryLifecycleCloseDisposition {
    XCloseRejected,
    LogicalRetirementRejected,
    Success,
}

pub(super) fn classify_and_count(
    events: RegistryLifecycleEvents<'_>,
) -> anyhow::Result<(
    ObservedRegistryLifecycleOutcome,
    RegistryLifecycleActualCounts,
)> {
    if events.pending_steps != 0 || events.pending_controls != 0 {
        return Err(anyhow!(
            "RegistryLifecycle close left a one-shot fault or control pending"
        ));
    }
    validate_observation_scope(events.route, events.observations)?;
    let signal = selected_signal(events.observations)?;
    let outcome = classify(events.stages, signal, events.shared_pre_topology)?;
    let expected = expected_stages(outcome);
    if events.stages != expected.as_slice() {
        return Err(anyhow!(
            "RegistryLifecycle observed stage ledger differs from sealed branch: actual={:?}, expected={expected:?}",
            events.stages,
        ));
    }
    let expected_disposition = if outcome.is_success() {
        RegistryLifecycleCloseDisposition::Success
    } else if outcome.is_logical_failure() {
        RegistryLifecycleCloseDisposition::LogicalRetirementRejected
    } else {
        RegistryLifecycleCloseDisposition::XCloseRejected
    };
    if events.close_disposition != expected_disposition {
        return Err(anyhow!(
            "RegistryLifecycle close disposition differs from the observed branch"
        ));
    }
    validate_raw_close(events.raw_close, events.stages)?;
    Ok((
        outcome,
        project_counts(outcome, events.stages, events.raw_close)?,
    ))
}

pub(super) fn validate_raw_close_pre(
    raw: HandleBoundSqliteAbiRawCloseWitnessSnapshot,
) -> anyhow::Result<()> {
    if raw.raw_close_entries != 0
        || raw.raw_close_entry_order != 0
        || raw.state_take_attempts != 0
        || raw.state_take_attempt_order != 0
        || raw.methods_clears != 0
        || raw.methods_clear_order != 0
        || raw.state_take_successes != 0
        || raw.state_take_success_order != 0
        || raw.state_abandons != 0
        || raw.state_abandon_order != 0
    {
        return Err(anyhow!(
            "RegistryLifecycle raw close witness was not pristine before xClose"
        ));
    }
    Ok(())
}

fn validate_raw_close(
    raw: HandleBoundSqliteAbiRawCloseWitnessSnapshot,
    stages: &[ManagedSqliteRegistryLifecycleStage],
) -> anyhow::Result<()> {
    if raw.raw_close_entries != 1
        || raw.raw_close_entry_order != 1
        || raw.state_take_attempts != 1
        || raw.state_take_attempt_order != 2
        || raw.methods_clears != 1
        || raw.methods_clear_order != 3
        || raw.state_take_successes != 1
        || raw.state_take_success_order != 4
        || raw.state_abandons != 0
        || raw.state_abandon_order != 0
        || stages
            .iter()
            .filter(|stage| **stage == ManagedSqliteRegistryLifecycleStage::RawCloseEntered)
            .count()
            != 1
    {
        return Err(anyhow!(
            "RegistryLifecycle raw xClose/take/clear transition was not exact-once and ordered"
        ));
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SelectedSignal {
    Triggered(
        ManagedTestLifecycleFaultPhase,
        ManagedTestLifecycleFaultTiming,
    ),
    Native(ManagedTestLifecycleFaultPhase),
}

fn selected_signal(
    observations: &[ManagedTestLifecycleFaultObservation],
) -> anyhow::Result<Option<SelectedSignal>> {
    let mut selected = None;
    for observation in observations {
        let candidate = if observation.triggered {
            Some(SelectedSignal::Triggered(
                observation.phase,
                observation.timing,
            ))
        } else if observation.timing == ManagedTestLifecycleFaultTiming::NativeFailure {
            Some(SelectedSignal::Native(observation.phase))
        } else {
            None
        };
        if let Some(candidate) = candidate {
            if selected.replace(candidate).is_some() {
                return Err(anyhow!(
                    "RegistryLifecycle close produced more than one selected fault signal"
                ));
            }
        }
    }
    Ok(selected)
}

fn classify(
    stages: &[ManagedSqliteRegistryLifecycleStage],
    signal: Option<SelectedSignal>,
    shared: bool,
) -> anyhow::Result<ObservedRegistryLifecycleOutcome> {
    use ManagedTestLifecycleFaultPhase as P;
    use ManagedTestLifecycleFaultTiming as T;
    use ObservedRegistryLifecycleOutcome as O;

    let outcome = match signal {
        Some(SelectedSignal::Triggered(P::CallbackCompletion, T::BeforeCall)) => {
            O::CallbackCompletionBefore
        }
        Some(SelectedSignal::Native(P::CallbackCompletion)) => O::CallbackCompletionNativeUncertain,
        Some(SelectedSignal::Triggered(P::CallbackCompletion, T::AfterSuccess)) => {
            O::CallbackCompletionAfterSuccessKnown
        }
        Some(SelectedSignal::Triggered(P::ConnectionObservation, T::BeforeCall)) => {
            O::ConnectionObservationBefore
        }
        Some(SelectedSignal::Native(P::ConnectionObservation))
            if stages
                .contains(&ManagedSqliteRegistryLifecycleStage::OutstandingSidecarRetained) =>
        {
            O::ConnectionObservationOutstandingSidecar
        }
        Some(SelectedSignal::Triggered(P::ConnectionObservation, T::AfterSuccess)) => {
            O::ConnectionObservationAfterSuccessKnown
        }
        Some(SelectedSignal::Triggered(P::RouteRetirement, T::BeforeCall)) => {
            O::RegistryRouteRemovalBefore
        }
        Some(SelectedSignal::Native(P::RouteRetirement)) => O::RegistryRouteRemovalOwnerNative,
        Some(SelectedSignal::Triggered(P::RouteRetirement, T::AfterSuccess)) => {
            O::RegistryRouteRemovalAfterSuccessKnown
        }
        Some(SelectedSignal::Triggered(P::LogicalRouteRemoval, T::BeforeCall)) => {
            O::LogicalRouteRemovalBefore
        }
        Some(SelectedSignal::Native(P::LogicalRouteRemoval)) => O::LogicalRouteRemovalIndexNative,
        Some(SelectedSignal::Triggered(P::LogicalRouteRemoval, T::AfterSuccess)) => {
            O::LogicalRouteRemovalAfterSuccessKnown
        }
        None if stages.last()
            == Some(&ManagedSqliteRegistryLifecycleStage::RetirementPublishAttempt) =>
        {
            O::RegistryRouteRemovalPublishNative
        }
        None if stages.last()
            == Some(&ManagedSqliteRegistryLifecycleStage::RetirementClaimAttempt) =>
        {
            O::LogicalRouteRemovalClaimNative
        }
        None if has_logical_success(stages) && shared => O::SuccessSharedNonFinal,
        None if has_logical_success(stages) => O::SuccessFinal,
        _ => {
            return Err(anyhow!(
                "RegistryLifecycle stages and lifecycle signal do not identify one frozen case"
            ))
        }
    };
    if shared != outcome.is_shared() {
        return Err(anyhow!(
            "RegistryLifecycle shared topology is allowed only for shared success"
        ));
    }
    Ok(outcome)
}

fn validate_observation_scope(
    route: ManagedTestRouteOrdinal,
    observations: &[ManagedTestLifecycleFaultObservation],
) -> anyhow::Result<()> {
    if observations.iter().any(|observation| {
        let selected_close_phase = observation.occurrence == 1
            && matches!(
                observation.phase,
                ManagedTestLifecycleFaultPhase::MainUnlock
                    | ManagedTestLifecycleFaultPhase::MainFileClose
                    | ManagedTestLifecycleFaultPhase::RegistryWalMainClose
                    | ManagedTestLifecycleFaultPhase::CallbackCompletion
                    | ManagedTestLifecycleFaultPhase::ConnectionObservation
                    | ManagedTestLifecycleFaultPhase::RouteRetirement
                    | ManagedTestLifecycleFaultPhase::LogicalRouteRemoval
            );
        let internal_final_barrier = observation.occurrence != 0
            && observation.phase == ManagedTestLifecycleFaultPhase::BarrierCallbackCompletion
            && !observation.triggered
            && matches!(
                observation.timing,
                ManagedTestLifecycleFaultTiming::BeforeCall
                    | ManagedTestLifecycleFaultTiming::AfterSuccess
            );
        observation.route != Some(route) || (!selected_close_phase && !internal_final_barrier)
    }) {
        return Err(anyhow!(
            "RegistryLifecycle close observed a foreign route, phase or occurrence"
        ));
    }
    Ok(())
}

fn expected_stages(
    outcome: ObservedRegistryLifecycleOutcome,
) -> Vec<ManagedSqliteRegistryLifecycleStage> {
    use ManagedSqliteRegistryLifecycleStage as S;
    use ObservedRegistryLifecycleOutcome as O;

    let mut stages = Vec::with_capacity(19);
    stages.extend([
        S::RawCloseEntered,
        S::CallbackBegin,
        S::PhysicalCloseSucceeded,
        S::RegistryWalMainCloseAttempt,
        S::RegistryWalMainCloseSucceeded,
    ]);
    if outcome == O::CallbackCompletionBefore {
        return stages;
    }
    stages.push(S::CallbackCompletionAttempt);
    if outcome == O::CallbackCompletionNativeUncertain {
        return stages;
    }
    stages.push(S::CallbackCompletionSucceeded);
    if matches!(
        outcome,
        O::CallbackCompletionAfterSuccessKnown | O::ConnectionObservationBefore
    ) {
        return stages;
    }
    if outcome == O::ConnectionObservationOutstandingSidecar {
        stages.push(S::OutstandingSidecarRetained);
    }
    stages.push(S::ConnectionObservationAttempt);
    if outcome == O::ConnectionObservationOutstandingSidecar {
        return stages;
    }
    stages.push(S::ConnectionObservationSucceeded);
    if matches!(
        outcome,
        O::ConnectionObservationAfterSuccessKnown | O::RegistryRouteRemovalBefore
    ) {
        return stages;
    }
    stages.push(S::RouteRetirementAttempt);
    if outcome == O::RegistryRouteRemovalOwnerNative {
        return stages;
    }
    stages.push(S::RouteRetirementSucceeded);
    if outcome == O::RegistryRouteRemovalAfterSuccessKnown {
        return stages;
    }
    stages.push(S::RetirementPublishAttempt);
    if outcome == O::RegistryRouteRemovalPublishNative {
        return stages;
    }
    stages.push(S::RetirementPublishSucceeded);
    stages.push(S::RetirementClaimAttempt);
    if outcome == O::LogicalRouteRemovalClaimNative {
        return stages;
    }
    stages.push(S::RetirementClaimSucceeded);
    if outcome == O::LogicalRouteRemovalBefore {
        return stages;
    }
    stages.push(S::LogicalRemovalAttempt);
    if outcome == O::LogicalRouteRemovalIndexNative {
        return stages;
    }
    stages.push(S::LogicalRemovalSucceeded { removed_names: 3 });
    stages
}

fn project_counts(
    outcome: ObservedRegistryLifecycleOutcome,
    stages: &[ManagedSqliteRegistryLifecycleStage],
    raw: HandleBoundSqliteAbiRawCloseWitnessSnapshot,
) -> anyhow::Result<RegistryLifecycleActualCounts> {
    use ManagedSqliteRegistryLifecycleStage as S;
    let has = |stage| u8::from(stages.contains(&stage));
    let physical = has(S::PhysicalCloseSucceeded);
    let (fault_observe, fault_trigger) = outcome.fault_counts();
    Ok(RegistryLifecycleActualCounts {
        raw_state_take_attempt: checked_u8(raw.state_take_attempts, "raw state take attempts")?,
        raw_state_take_success: checked_u8(raw.state_take_successes, "raw state take successes")?,
        raw_state_abandon: checked_u8(raw.state_abandons, "raw state abandons")?,
        methods_clear: checked_u8(raw.methods_clears, "raw methods clears")?,
        callback_begin: has(S::CallbackBegin),
        callback_complete_attempt: has(S::CallbackCompletionAttempt),
        callback_complete_success: has(S::CallbackCompletionSucceeded),
        selected_action_attempt: 0,
        selected_action_success: 0,
        shm_detach: physical,
        main_unlock_attempt: physical,
        main_unlock_success: physical,
        main_file_close_attempt: physical,
        main_file_close_success: physical,
        registry_close_attempt: has(S::RegistryWalMainCloseAttempt),
        registry_close_success: has(S::RegistryWalMainCloseSucceeded),
        connection_observe_attempt: has(S::ConnectionObservationAttempt),
        connection_observe_success: has(S::ConnectionObservationSucceeded),
        registry_route_remove_attempt: has(S::RouteRetirementAttempt),
        registry_route_remove_success: has(S::RouteRetirementSucceeded),
        logical_names_remove_attempt: has(S::LogicalRemovalAttempt),
        logical_names_remove_success: u8::from(has_logical_success(stages)),
        logical_names_remove: stages
            .iter()
            .find_map(|stage| match stage {
                S::LogicalRemovalSucceeded { removed_names } => Some(*removed_names),
                _ => None,
            })
            .unwrap_or(0),
        vfs_unregister_attempt: 0,
        vfs_unregister_success: 0,
        fault_observe,
        fault_trigger,
        fault_pending: 0,
        custody_retain: 0,
        physical_retry: 0,
    })
}

fn has_logical_success(stages: &[ManagedSqliteRegistryLifecycleStage]) -> bool {
    stages.iter().any(|stage| {
        matches!(
            stage,
            ManagedSqliteRegistryLifecycleStage::LogicalRemovalSucceeded { removed_names: 3 }
        )
    })
}

pub(super) fn checked_u8(value: usize, label: &'static str) -> anyhow::Result<u8> {
    u8::try_from(value).with_context(|| format!("{label} exceeds u8"))
}
