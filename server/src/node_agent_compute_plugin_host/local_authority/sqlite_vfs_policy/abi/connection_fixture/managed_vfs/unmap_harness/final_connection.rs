//! Real installed-ABI executor for all final-connection Unmap selectors.

use std::path::Path;

use anyhow::{anyhow, Context};

use super::super::{
    a2b2_cases::{UnmapActual, UnmapSelector},
    ManagedTestShmFaultPlanBinding,
};

mod action;
mod custody;
mod liveness;
mod observe;
mod outcome;
mod prepare;
mod stimulus;

use observe::{FinalEventSet, ObservedGenericFault};
use prepare::{FinalUnmapFixture, SELECTED};

pub(super) fn supports(selector: UnmapSelector) -> bool {
    outcome::supports(selector)
}

pub(super) fn exercise(root: &Path, selected: UnmapSelector) -> anyhow::Result<UnmapActual> {
    if !outcome::supports(selected) {
        return Err(anyhow!(
            "Unmap selector is outside final-connection runtime authority"
        ));
    }
    let fixture = FinalUnmapFixture::prepare(root, selected)?;
    let route = fixture.route_ordinal(SELECTED)?;
    let binding = fixture
        .route(SELECTED)?
        .installed_shm_fault_witness()
        .map_err(anyhow::Error::msg)?;
    binding
        .begin_unmap_action_observation()
        .map_err(anyhow::Error::msg)?;

    let callback_baseline = fixture
        .callback_fault_observations()
        .map_err(anyhow::Error::msg)?;
    let lifecycle_baseline = fixture
        .lifecycle_fault_observations()
        .map_err(anyhow::Error::msg)?;
    if !fixture.unmap_runtime_trace(SELECTED)?.is_empty() {
        return Err(anyhow!("final Unmap runtime trace began before its window"));
    }
    fixture.enable_unmap_runtime_observation(SELECTED)?;
    let prepared = fixture.observe_unmap_route(SELECTED)?;
    let prepared_names = fixture
        .route(SELECTED)?
        .barrier_logical_route_snapshot()?
        .exact_route_names();
    if prepared_names != 3 {
        return Err(anyhow!(
            "final Unmap route does not own exact 3-name custody"
        ));
    }
    let pre_topology = custody::observe_topology(&fixture, prepared, prepared_names)?;

    stimulus::install(&fixture, &binding, route, selected)?;
    let pre = fixture.observe_unmap_route(SELECTED)?;
    fixture.validate_node_absent_setup(selected, pre)?;
    let sqlite_liveness = liveness::prepare(&fixture, selected, pre.target)?;
    let raw = fixture.call_unmap_raw(SELECTED, stimulus::raw_delete(selected))?;
    let sqlite_liveness = sqlite_liveness.probe_after_unmap()?;
    let callback_all = fixture
        .callback_fault_observations()
        .map_err(anyhow::Error::msg)?;
    let callback_observations = callback_all
        .strip_prefix(callback_baseline.as_slice())
        .context("final Unmap callback observation baseline changed")?;
    let lifecycle_all = fixture
        .lifecycle_fault_observations()
        .map_err(anyhow::Error::msg)?;
    let lifecycle_observations = lifecycle_all
        .strip_prefix(lifecycle_baseline.as_slice())
        .context("final Unmap lifecycle observation baseline changed")?;
    let runtime_trace = fixture.finish_unmap_runtime_observation(SELECTED)?;
    let low_level = binding
        .finish_unmap_test_receipt()
        .map_err(anyhow::Error::msg)?;
    let generic_faults = observe_generic_faults(&binding)?;
    let post = fixture.observe_unmap_route(SELECTED)?;

    let counts = observe::validate_and_count(FinalEventSet {
        selector: selected,
        route,
        raw,
        pre,
        post,
        callback_observation_count: callback_observations.len(),
        lifecycle_observations,
        runtime_trace: runtime_trace.as_slice(),
        generic_faults: generic_faults.as_slice(),
        callback_pending: fixture
            .pending_callback_fault_count()
            .map_err(anyhow::Error::msg)?,
        lifecycle_pending: fixture
            .pending_lifecycle_fault_count()
            .map_err(anyhow::Error::msg)?,
        generic_pending: binding.pending_count().map_err(anyhow::Error::msg)?,
        low_level: &low_level,
        sqlite_liveness: sqlite_liveness.as_ref(),
    })?;
    let post_topology = custody::observe_topology(&fixture, post, prepared_names)?;
    let post_witness =
        custody::observe_post_witness(root, selected, &fixture, prepared_names, pre, post)?;
    let detached = pre.physical.target_attached && !post.physical.target_attached;
    Ok(UnmapActual {
        selector: selected,
        identity: outcome::into_identity(
            selected,
            pre.target,
            pre.physical.shared_mask,
            pre.physical.exclusive_mask,
        ),
        mutation_may_have_occurred: post.physical.topology.mutation_may_have_occurred
            || (detached && !outcome::is_success(selected)),
        lock_outcome_uncertain: post.physical.topology.lock_outcome_uncertain,
        domain_terminal: post.physical.topology.domain_terminal,
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

fn observe_generic_faults(
    binding: &ManagedTestShmFaultPlanBinding,
) -> anyhow::Result<Vec<ObservedGenericFault>> {
    observe::GENERIC_PHASES
        .into_iter()
        .map(|phase| {
            Ok(ObservedGenericFault {
                phase,
                observed: binding
                    .unmap_fault_was_observed(phase, 1)
                    .map_err(anyhow::Error::msg)?,
                trigger: binding
                    .triggered_observation(phase, 1)
                    .map_err(anyhow::Error::msg)?,
            })
        })
        .collect()
}
