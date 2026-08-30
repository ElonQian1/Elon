//! Real installed-ABI executor for the frozen 49-case Windows Unmap family.

use std::path::Path;

use anyhow::{anyhow, Context};

use super::a2b2_cases::{UnmapActual, UnmapActualTopology, UnmapSelector};
use crate::node_agent_managed_fs::ManagedSqliteShmFailurePhase;

mod custody;
mod final_connection;
mod observe;
mod outcome;
mod prepare;
mod stimulus;

use custody::{observe_post_witness, require_registration};
use observe::{classify_and_count, UnmapEventSet};
use prepare::{RetainedUnmapFixture, SELECTED, SIBLING};

pub(super) fn exercise_unmap(root: &Path, selected: UnmapSelector) -> anyhow::Result<UnmapActual> {
    if final_connection::supports(selected) {
        return final_connection::exercise(root, selected);
    }
    exercise_shared_unmap(root, selected)
}

fn exercise_shared_unmap(root: &Path, selected: UnmapSelector) -> anyhow::Result<UnmapActual> {
    if !stimulus::supports_shared(selected) {
        return Err(anyhow!(
            "Unmap selector is outside this SharedNonFinal runtime batch"
        ));
    }
    let fixture = RetainedUnmapFixture::prepare(root)?;
    let route = fixture.route_ordinal(SELECTED)?;
    let target_binding = fixture
        .route(SELECTED)?
        .installed_shm_fault_witness()
        .map_err(anyhow::Error::msg)?;
    target_binding
        .begin_unmap_action_observation()
        .map_err(anyhow::Error::msg)?;
    let callback_baseline = fixture
        .callback_fault_observations()
        .map_err(anyhow::Error::msg)?;
    let lifecycle_baseline = fixture
        .lifecycle_fault_observations()
        .map_err(anyhow::Error::msg)?;
    let runtime_baseline = fixture.unmap_runtime_trace(SELECTED)?;
    if !runtime_baseline.is_empty() {
        return Err(anyhow!(
            "Unmap fixture reached the selected runtime boundary before its stimulus"
        ));
    }
    fixture.enable_unmap_runtime_observation(SELECTED)?;
    let prepared_selected = fixture.observe_unmap_route(SELECTED)?;
    let prepared_selected_names = fixture
        .route(SELECTED)?
        .barrier_logical_route_snapshot()?
        .exact_route_names();
    if prepared_selected_names == 0 {
        return Err(anyhow!(
            "Unmap selected route has no pre-stimulus logical-name witness"
        ));
    }
    let pre_topology = observe_topology(&fixture, prepared_selected, prepared_selected_names)?;

    stimulus::install(&fixture, selected, route)?;
    let pre_selected = fixture.observe_unmap_route(SELECTED)?;
    let pre_sibling = fixture.observe_unmap_route(SIBLING)?;
    require_registration(fixture.live_registration_snapshot()?)?;

    let raw = fixture.call_unmap_raw(SELECTED, stimulus::raw_delete(selected)?)?;
    let callback_all = fixture
        .callback_fault_observations()
        .map_err(anyhow::Error::msg)?;
    let callback_observations = callback_all
        .strip_prefix(callback_baseline.as_slice())
        .context("Unmap callback observation baseline changed")?;
    let lifecycle_all = fixture
        .lifecycle_fault_observations()
        .map_err(anyhow::Error::msg)?;
    let lifecycle_observations = lifecycle_all
        .strip_prefix(lifecycle_baseline.as_slice())
        .context("Unmap lifecycle observation baseline changed")?;
    let runtime_trace = fixture.finish_unmap_runtime_observation(SELECTED)?;
    let low_level = target_binding
        .finish_unmap_test_receipt()
        .map_err(anyhow::Error::msg)?;
    let shm_trigger = target_binding
        .triggered_observation(ManagedSqliteShmFailurePhase::ConnectionDetach, 1)
        .map_err(anyhow::Error::msg)?;
    let post_selected = fixture.observe_unmap_route(SELECTED)?;
    let post_sibling = fixture.observe_unmap_route(SIBLING)?;
    // A terminal SHM domain rejects every later SHM callback by contract. Re-entering SQLite on
    // that sibling would deliberately call a terminal native domain, so its observed usability is
    // false without issuing an unsafe second callback. Non-terminal routes prove usability by SQL.
    let sibling_sql_usable = !post_selected.physical.topology.domain_terminal
        && fixture
            .verify_unmap_sibling_sql(SIBLING, prepare::PROBE_VALUE)
            .is_ok();
    let (outcome, counts) = classify_and_count(UnmapEventSet {
        route,
        raw,
        pre: pre_selected,
        post: post_selected,
        callback_observations,
        lifecycle_observations,
        runtime_trace: runtime_trace.as_slice(),
        low_level: &low_level,
        shm_trigger,
        callback_pending: fixture
            .pending_callback_fault_count()
            .map_err(anyhow::Error::msg)?,
        lifecycle_pending: fixture
            .pending_lifecycle_fault_count()
            .map_err(anyhow::Error::msg)?,
        shm_pending: target_binding.pending_count().map_err(anyhow::Error::msg)?,
        sibling_sql_usable,
    })?;
    if outcome.selector() != selected {
        return Err(anyhow!(
            "parent-selected Unmap case differs from sealed runtime outcome"
        ));
    }

    let post_topology = observe_topology(&fixture, post_selected, prepared_selected_names)?;
    let post_registration = fixture.live_registration_snapshot()?;
    let post_witness = observe_post_witness(
        root,
        outcome,
        &fixture,
        prepared_selected_names,
        pre_selected,
        post_selected,
        pre_sibling,
        post_sibling,
        post_registration,
        post_topology,
    )?;
    let observed_detach_failure = outcome.action_succeeded() && !outcome.is_success();
    Ok(UnmapActual {
        selector: outcome.selector(),
        identity: outcome.into_identity(
            pre_selected.target,
            pre_selected.physical.shared_mask,
            pre_selected.physical.exclusive_mask,
        ),
        mutation_may_have_occurred: post_selected.physical.topology.mutation_may_have_occurred
            || observed_detach_failure,
        lock_outcome_uncertain: post_selected.physical.topology.lock_outcome_uncertain,
        domain_terminal: post_selected.physical.topology.domain_terminal,
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

fn observe_topology(
    fixture: &RetainedUnmapFixture,
    selected: super::multi_connection::ManagedTestUnmapRouteObservation,
    prepared_selected_names: usize,
) -> anyhow::Result<UnmapActualTopology> {
    let (live_routes, indexed_names) = fixture.logical_route_counts()?;
    let retained_route = selected.terminal_custody.terminal_route();
    let terminal_route_count = usize::from(retained_route.is_some());
    if selected.terminal_custody.terminal_route_observation_count() != terminal_route_count {
        return Err(anyhow!(
            "Unmap retained route count disagrees with its terminal route witness"
        ));
    }
    let indexed_selected_names = fixture
        .route(SELECTED)?
        .barrier_logical_route_snapshot()?
        .exact_route_names();
    if indexed_selected_names != 0 && indexed_selected_names != prepared_selected_names {
        return Err(anyhow!(
            "Unmap selected logical-name index changed without full removal"
        ));
    }
    if retained_route.is_none() && indexed_selected_names == 0 {
        return Err(anyhow!(
            "Unmap active selected route disappeared from the logical index"
        ));
    }
    let retained_was_removed_from_index = retained_route.is_some() && indexed_selected_names == 0;
    let retained_routes = usize::from(retained_was_removed_from_index);
    let retained_names = if retained_was_removed_from_index {
        prepared_selected_names
    } else {
        0
    };
    let routes = live_routes
        .checked_add(retained_routes)
        .context("Unmap observed route count overflow")?;
    let logical_names = indexed_names
        .checked_add(retained_names)
        .context("Unmap observed logical-name count overflow")?;
    Ok(UnmapActualTopology {
        sqlite_connections: checked_u8(
            fixture.live_connection_count(),
            "Unmap SQLite connection count",
        )?,
        shm_connections: selected.physical.topology.shm_connections,
        registry_routes: checked_u8(routes, "Unmap registry route count")?,
        logical_names: checked_u8(logical_names, "Unmap logical name count")?,
    })
}

fn checked_u8(value: usize, label: &'static str) -> anyhow::Result<u8> {
    u8::try_from(value).with_context(|| format!("{label} exceeds u8"))
}
