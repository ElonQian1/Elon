//! Process-local executor for one exact real `xShmBarrier` child observation.

use std::path::Path;

use anyhow::{anyhow, Context};

use super::{
    a2b2_cases::{BarrierActual, BarrierActualTopology, BarrierSelector},
    ManagedTestCallbackFaultOperation, ManagedTestCallbackFaultStep,
    ManagedTestCallbackFaultTiming, ManagedTestLifecycleFaultPhase, ManagedTestLifecycleFaultStep,
    ManagedTestLifecycleFaultTiming,
};
use crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::ManagedSqliteLogicalFileRole;
use crate::node_agent_managed_fs::{
    ManagedSqliteShmFailureClass, ManagedSqliteShmFailurePhase, ManagedSqliteShmTestTargetSnapshot,
};

mod custody;
mod observe;
mod outcome;
mod prepare;

use custody::{observe_post_witness, require_live_route, require_registration};
use observe::{classify_and_count, BarrierEventSet};
use outcome::ObservedBarrierOutcome;
use prepare::{checked_u8, RetainedBarrierFixture};

const SELECTED: usize = 0;
const SIBLING: usize = 1;

pub(super) fn exercise_barrier(
    root: &Path,
    selected: BarrierSelector,
) -> anyhow::Result<BarrierActual> {
    let fixture = RetainedBarrierFixture::prepare(root)?;
    let route = fixture.route_ordinal(SELECTED)?;
    let target_binding = fixture
        .route(SELECTED)?
        .installed_shm_fault_witness()
        .map_err(anyhow::Error::msg)?;
    let target_observer = target_binding.observer().map_err(anyhow::Error::msg)?;
    let target = target_binding
        .target_witness()
        .map_err(anyhow::Error::msg)?;
    let sibling_binding = fixture
        .route(SIBLING)?
        .installed_shm_fault_witness()
        .map_err(anyhow::Error::msg)?;
    let sibling_observer = sibling_binding.observer().map_err(anyhow::Error::msg)?;
    let sibling_target = sibling_binding
        .target_witness()
        .map_err(anyhow::Error::msg)?;

    let pre_target = target_observer.snapshot()?;
    let pre_sibling_target = sibling_observer.snapshot()?;
    let pre_sibling_raw = fixture
        .route(SIBLING)?
        .observe_main_raw_slots()
        .map_err(anyhow::Error::msg)?;
    let pre_selected_route = fixture
        .route(SELECTED)?
        .route_custody_snapshot()
        .map_err(anyhow::Error::msg)?;
    let pre_sibling_route = fixture
        .route(SIBLING)?
        .route_custody_snapshot()
        .map_err(anyhow::Error::msg)?;
    require_live_route(pre_selected_route, "selected")?;
    require_live_route(pre_sibling_route, "sibling")?;
    let pre_terminal = fixture
        .route(SELECTED)?
        .terminal_custody_test_snapshot()
        .map_err(anyhow::Error::msg)?;
    if !pre_terminal.active_route_present()
        || pre_terminal.retention_count() != 0
        || pre_terminal.route_removal_count() != 0
    {
        return Err(anyhow!(
            "Barrier selected route was terminal before selection"
        ));
    }
    let pre_topology = observe_topology(&fixture, pre_target)?;
    let pre_registration = fixture.live_registration_snapshot()?;
    let callback_baseline = fixture
        .callback_fault_observations()
        .map_err(anyhow::Error::msg)?;
    let lifecycle_baseline = fixture
        .lifecycle_fault_observations()
        .map_err(anyhow::Error::msg)?;

    install_selected_stimulus(&fixture, selected, route)?;
    let terminal_before_call = fixture
        .route(SELECTED)?
        .terminal_custody_test_snapshot()
        .map_err(anyhow::Error::msg)?;
    let raw = fixture
        .route(SELECTED)?
        .call_main_shm_barrier()
        .map_err(anyhow::Error::msg)?;

    let callback_all = fixture
        .callback_fault_observations()
        .map_err(anyhow::Error::msg)?;
    let callback_observations = callback_all
        .strip_prefix(callback_baseline.as_slice())
        .context("Barrier callback observation baseline changed")?;
    let lifecycle_all = fixture
        .lifecycle_fault_observations()
        .map_err(anyhow::Error::msg)?;
    let lifecycle_observations = lifecycle_all
        .strip_prefix(lifecycle_baseline.as_slice())
        .context("Barrier lifecycle observation baseline changed")?;
    let shm_trigger = target_binding
        .triggered_observation(ManagedSqliteShmFailurePhase::Barrier, 1)
        .map_err(anyhow::Error::msg)?;
    let terminal_after_call = fixture
        .route(SELECTED)?
        .terminal_custody_test_snapshot()
        .map_err(anyhow::Error::msg)?;
    let (outcome, counts) = classify_and_count(BarrierEventSet {
        route,
        raw,
        terminal_before_call,
        terminal_after_call,
        callback_observations,
        lifecycle_observations,
        shm_trigger,
        callback_pending: fixture
            .pending_callback_fault_count()
            .map_err(anyhow::Error::msg)?,
        lifecycle_pending: fixture
            .pending_lifecycle_fault_count()
            .map_err(anyhow::Error::msg)?,
        shm_pending: target_binding.pending_count().map_err(anyhow::Error::msg)?,
    })?;
    if outcome.selector() != selected {
        return Err(anyhow!(
            "parent-selected Barrier case differs from sealed observed outcome"
        ));
    }

    let post_target = target_observer.snapshot()?;
    let post_sibling_target = sibling_observer.snapshot()?;
    let post_sibling_raw = fixture
        .route(SIBLING)?
        .observe_main_raw_slots()
        .map_err(anyhow::Error::msg)?;
    validate_physical_outcome(outcome, pre_target, post_target)?;
    let post_topology = observe_topology(&fixture, post_target)?;
    let post_registration = fixture.live_registration_snapshot()?;
    require_registration(pre_registration)?;
    let post_witness = observe_post_witness(
        root,
        outcome,
        &fixture,
        target,
        sibling_target,
        pre_sibling_target,
        post_sibling_target,
        pre_sibling_raw,
        post_sibling_raw,
        post_target,
        terminal_after_call,
        post_registration,
        post_topology,
    )?;
    Ok(BarrierActual {
        selector: outcome.selector(),
        identity: outcome.into_identity(target, pre_target.shared_mask, pre_target.exclusive_mask),
        mutation_may_have_occurred: post_target.topology.mutation_may_have_occurred,
        lock_outcome_uncertain: post_target.topology.lock_outcome_uncertain,
        domain_terminal: post_target.topology.domain_terminal,
        registry_route_phase: post_witness.registry_route_phase,
        logical_route_phase: post_witness.logical_route_phase,
        registration_phase: post_witness.registration_phase,
        later_callback_allowed: post_witness.later_callback_allowed,
        pre: pre_topology,
        post: post_topology,
        retained: post_witness.retained,
        counts,
    })
}

fn install_selected_stimulus(
    fixture: &RetainedBarrierFixture,
    selector: BarrierSelector,
    route: super::ManagedTestRouteOrdinal,
) -> anyhow::Result<()> {
    match selector {
        BarrierSelector::AdmissionRejected => fixture
            .route(SELECTED)?
            .quarantine_for_barrier_admission_test()
            .map_err(anyhow::Error::msg),
        BarrierSelector::WrapperBefore => {
            let step = ManagedTestCallbackFaultStep::new(
                route,
                ManagedSqliteLogicalFileRole::Main,
                ManagedTestCallbackFaultOperation::ShmBarrier,
                1,
                ManagedTestCallbackFaultTiming::BeforeCall,
            )
            .map_err(anyhow::Error::msg)?;
            fixture
                .install_callback_fault_script(&[step])
                .map_err(anyhow::Error::msg)
        }
        BarrierSelector::FenceBefore => fixture
            .route(SELECTED)?
            .install_shm_fault_script(&[(ManagedSqliteShmFailurePhase::Barrier, 1)], &[])
            .map_err(anyhow::Error::msg),
        BarrierSelector::FenceAfter => fixture
            .route(SELECTED)?
            .install_shm_fault_script(
                &[],
                &[((
                    ManagedSqliteShmFailurePhase::Barrier,
                    1,
                    ManagedSqliteShmFailureClass::OutcomeUncertainPoisoned,
                ))],
            )
            .map_err(anyhow::Error::msg),
        BarrierSelector::CompletionBefore
        | BarrierSelector::CompletionNativeUncertain
        | BarrierSelector::CompletionAfterSuccessKnown => {
            let timing = match selector {
                BarrierSelector::CompletionBefore => ManagedTestLifecycleFaultTiming::BeforeCall,
                BarrierSelector::CompletionNativeUncertain => {
                    ManagedTestLifecycleFaultTiming::NativeFailure
                }
                BarrierSelector::CompletionAfterSuccessKnown => {
                    ManagedTestLifecycleFaultTiming::AfterSuccess
                }
                _ => unreachable!(),
            };
            let step = ManagedTestLifecycleFaultStep::route(
                route,
                ManagedTestLifecycleFaultPhase::BarrierCallbackCompletion,
                1,
                timing,
            )
            .map_err(anyhow::Error::msg)?;
            fixture
                .install_lifecycle_fault_script(&[step])
                .map_err(anyhow::Error::msg)
        }
        BarrierSelector::Success => fixture
            .begin_unfaulted_barrier_observation_window(route)
            .map_err(anyhow::Error::msg),
    }
}

fn observe_topology(
    fixture: &RetainedBarrierFixture,
    target: ManagedSqliteShmTestTargetSnapshot,
) -> anyhow::Result<BarrierActualTopology> {
    let (routes, logical_names) = fixture.logical_route_counts()?;
    Ok(BarrierActualTopology {
        sqlite_connections: checked_u8(
            fixture.live_connection_count(),
            "Barrier SQLite connection count",
        )?,
        shm_connections: target.topology.shm_connections,
        registry_routes: checked_u8(routes, "Barrier registry route count")?,
        logical_names: checked_u8(logical_names, "Barrier logical name count")?,
    })
}

fn validate_physical_outcome(
    outcome: ObservedBarrierOutcome,
    pre: ManagedSqliteShmTestTargetSnapshot,
    post: ManagedSqliteShmTestTargetSnapshot,
) -> anyhow::Result<()> {
    if !pre.target_attached
        || !post.target_attached
        || pre.topology.shm_connections != 2
        || post.topology.shm_connections != 2
        || pre.shared_mask != 0
        || pre.exclusive_mask != 0
        || post.shared_mask != 0
        || post.exclusive_mask != 0
    {
        return Err(anyhow!("Barrier exact physical target/topology changed"));
    }
    let fence_terminal = matches!(
        outcome,
        ObservedBarrierOutcome::FenceBefore | ObservedBarrierOutcome::FenceAfter
    );
    if post.topology.domain_terminal != fence_terminal {
        return Err(anyhow!(
            "Barrier SHM-domain terminal state disagrees with physical event"
        ));
    }
    Ok(())
}
