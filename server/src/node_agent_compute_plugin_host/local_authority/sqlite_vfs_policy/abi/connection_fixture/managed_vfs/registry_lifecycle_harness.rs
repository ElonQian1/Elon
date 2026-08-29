//! Process-local executor for one exact real RegistryLifecycle `xClose` child.

use std::path::Path;

use anyhow::{anyhow, Context};

use super::{
    a2b2_cases::{RegistryLifecycleActual, RegistryLifecycleSelector},
    connection::ManagedTestRegistryLifecycleCloseOutcome,
    lifecycle_faults::ManagedTestRegistryLifecycleControl,
    ManagedTestLifecycleFaultPhase, ManagedTestLifecycleFaultStep, ManagedTestLifecycleFaultTiming,
    ManagedTestRouteOrdinal,
};

mod observe;
mod outcome;
mod prepare;
mod state;

use observe::{
    classify_and_count, validate_raw_close_pre, RegistryLifecycleCloseDisposition,
    RegistryLifecycleEvents,
};
use prepare::RetainedRegistryLifecycleFixture;
use state::observe_state;

pub(super) fn exercise_registry_lifecycle(
    root: &Path,
    selected: RegistryLifecycleSelector,
) -> anyhow::Result<RegistryLifecycleActual> {
    let shared = selected == RegistryLifecycleSelector::SuccessSharedNonFinal;
    let mut fixture = RetainedRegistryLifecycleFixture::prepare(root, shared)?;
    let route = fixture.selected_ordinal();
    let target_binding = fixture.target_binding()?;
    let target_observer = target_binding.observer().map_err(anyhow::Error::msg)?;
    let target = target_binding
        .target_witness()
        .map_err(anyhow::Error::msg)?;
    let pre_target = target_observer.snapshot()?;
    let pre_routes = fixture.route_snapshot()?;
    let pre_sqlite_connections = fixture.sqlite_connection_count()?;
    let pre_terminal = fixture.terminal_custody()?;
    if !pre_terminal.active_route_present()
        || pre_terminal.retention_count() != 0
        || pre_terminal.route_removal_count() != 0
    {
        return Err(anyhow!(
            "RegistryLifecycle selected route was terminal before close"
        ));
    }
    let observation_baseline = fixture.lifecycle_observations()?;
    let trace_baseline = fixture.registry_trace()?;
    validate_raw_close_pre(fixture.raw_close_snapshot())?;

    install_selected_stimulus(&fixture, selected, route)?;
    let close = fixture.close_selected_once()?;
    if fixture.selected_close_attempts() != 1 {
        return Err(anyhow!(
            "RegistryLifecycle selected connection did not close exactly once"
        ));
    }
    let close_disposition = match close {
        ManagedTestRegistryLifecycleCloseOutcome::XCloseRejected(_) => {
            RegistryLifecycleCloseDisposition::XCloseRejected
        }
        ManagedTestRegistryLifecycleCloseOutcome::LogicalRetirementRejected(_) => {
            RegistryLifecycleCloseDisposition::LogicalRetirementRejected
        }
        ManagedTestRegistryLifecycleCloseOutcome::Success(_) => {
            RegistryLifecycleCloseDisposition::Success
        }
    };

    let all_observations = fixture.lifecycle_observations()?;
    let observations = all_observations
        .strip_prefix(observation_baseline.as_slice())
        .context("RegistryLifecycle lifecycle-observation baseline changed")?;
    let trace = fixture.registry_trace()?;
    let raw_close = fixture.raw_close_snapshot();
    let stages = trace
        .stages()
        .strip_prefix(trace_baseline.stages())
        .context("RegistryLifecycle append-only stage baseline changed")?;
    let (outcome, mut counts) = classify_and_count(RegistryLifecycleEvents {
        route,
        stages,
        observations,
        pending_steps: fixture.lifecycle_pending()?,
        pending_controls: trace.pending_controls(),
        close_disposition,
        shared_pre_topology: shared,
        raw_close,
    })?;
    if outcome.selector() != selected {
        return Err(anyhow!(
            "parent-selected RegistryLifecycle case differs from sealed observed outcome"
        ));
    }

    let post_target = target_observer.snapshot()?;
    let post_routes = fixture.route_snapshot()?;
    let post_sqlite_connections = fixture.sqlite_connection_count()?;
    let post_runtime = fixture.runtime_snapshot()?;
    let terminal = fixture.terminal_custody()?;
    let registration = fixture.live_registration_snapshot()?;
    let sibling = if shared {
        fixture.verify_sibling_sql()?;
        if !fixture.sibling_is_live() {
            return Err(anyhow!(
                "RegistryLifecycle shared sibling connection was consumed"
            ));
        }
        Some(fixture.sibling_custody()?)
    } else {
        None
    };
    let state = observe_state(
        root,
        outcome,
        pre_target,
        post_target,
        &pre_routes,
        &post_routes,
        pre_sqlite_connections,
        post_sqlite_connections,
        post_runtime,
        terminal,
        registration,
        sibling,
        &trace,
    )?;
    counts.custody_retain = state.custody_retain;

    Ok(RegistryLifecycleActual {
        selector: outcome.selector(),
        identity: outcome.into_identity(target, pre_target.shared_mask, pre_target.exclusive_mask),
        mutation_may_have_occurred: state.mutation_may_have_occurred,
        lock_outcome_uncertain: state.lock_outcome_uncertain,
        domain_terminal: state.domain_terminal,
        registry_route_phase: state.registry_route_phase,
        logical_route_phase: state.logical_route_phase,
        registration_phase: state.registration_phase,
        later_callback_allowed: state.later_callback_allowed,
        pre: state.pre,
        post: state.post,
        retained: state.retained,
        counts,
    })
}

fn install_selected_stimulus(
    fixture: &RetainedRegistryLifecycleFixture,
    selector: RegistryLifecycleSelector,
    route: ManagedTestRouteOrdinal,
) -> anyhow::Result<()> {
    use RegistryLifecycleSelector as S;

    let fault = match selector {
        S::CallbackCompletionBefore => Some((
            ManagedTestLifecycleFaultPhase::CallbackCompletion,
            ManagedTestLifecycleFaultTiming::BeforeCall,
        )),
        S::CallbackCompletionNativeUncertain => Some((
            ManagedTestLifecycleFaultPhase::CallbackCompletion,
            ManagedTestLifecycleFaultTiming::NativeFailure,
        )),
        S::CallbackCompletionAfterSuccessKnown => Some((
            ManagedTestLifecycleFaultPhase::CallbackCompletion,
            ManagedTestLifecycleFaultTiming::AfterSuccess,
        )),
        S::ConnectionObservationBefore => Some((
            ManagedTestLifecycleFaultPhase::ConnectionObservation,
            ManagedTestLifecycleFaultTiming::BeforeCall,
        )),
        S::ConnectionObservationAfterSuccessKnown => Some((
            ManagedTestLifecycleFaultPhase::ConnectionObservation,
            ManagedTestLifecycleFaultTiming::AfterSuccess,
        )),
        S::RegistryRouteRemovalBefore => Some((
            ManagedTestLifecycleFaultPhase::RouteRetirement,
            ManagedTestLifecycleFaultTiming::BeforeCall,
        )),
        S::RegistryRouteRemovalOwnerNative => Some((
            ManagedTestLifecycleFaultPhase::RouteRetirement,
            ManagedTestLifecycleFaultTiming::NativeFailure,
        )),
        S::RegistryRouteRemovalAfterSuccessKnown => Some((
            ManagedTestLifecycleFaultPhase::RouteRetirement,
            ManagedTestLifecycleFaultTiming::AfterSuccess,
        )),
        S::LogicalRouteRemovalBefore => Some((
            ManagedTestLifecycleFaultPhase::LogicalRouteRemoval,
            ManagedTestLifecycleFaultTiming::BeforeCall,
        )),
        S::LogicalRouteRemovalIndexNative => Some((
            ManagedTestLifecycleFaultPhase::LogicalRouteRemoval,
            ManagedTestLifecycleFaultTiming::NativeFailure,
        )),
        S::LogicalRouteRemovalAfterSuccessKnown => Some((
            ManagedTestLifecycleFaultPhase::LogicalRouteRemoval,
            ManagedTestLifecycleFaultTiming::AfterSuccess,
        )),
        S::ConnectionObservationOutstandingSidecar => {
            fixture.retain_outstanding_sidecar()?;
            None
        }
        S::RegistryRouteRemovalPublishNative => {
            fixture
                .install_control(ManagedTestRegistryLifecycleControl::RejectRetirementPublish)?;
            None
        }
        S::LogicalRouteRemovalClaimNative => {
            fixture.install_control(ManagedTestRegistryLifecycleControl::RejectRetirementClaim)?;
            None
        }
        S::SuccessSharedNonFinal | S::SuccessFinal => None,
    };
    if let Some((phase, timing)) = fault {
        let step = ManagedTestLifecycleFaultStep::route(route, phase, 1, timing)
            .map_err(anyhow::Error::msg)?;
        fixture.install_lifecycle_fault(step)?;
    }
    Ok(())
}
