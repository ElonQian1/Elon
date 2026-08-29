//! Exact-route callback-completion plan for one real xShmUnmap invocation.

use super::{
    ManagedTestLifecycleFaultController, ManagedTestLifecycleFaultPhase,
    ManagedTestLifecycleFaultStep, ManagedTestLifecycleFaultTiming, ManagedTestRouteOrdinal,
};
use crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::registry::ManagedSqliteRegistryUnmapRuntimeEvent;

pub(in super::super) struct ManagedTestUnmapCompletionFault;

impl ManagedTestUnmapCompletionFault {
    pub(in super::super) fn native_uncertain(
        route: ManagedTestRouteOrdinal,
    ) -> Result<ManagedTestLifecycleFaultStep, &'static str> {
        ManagedTestLifecycleFaultStep::route(
            route,
            ManagedTestLifecycleFaultPhase::UnmapCallbackCompletion,
            1,
            ManagedTestLifecycleFaultTiming::NativeFailure,
        )
    }
}

impl ManagedTestLifecycleFaultController {
    pub(in super::super) fn enable_unmap_runtime_observation(
        &self,
        route: ManagedTestRouteOrdinal,
    ) -> Result<(), &'static str> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| "lifecycle fault controller poisoned")?;
        if state.unmap_runtime_routes.contains(&route)
            || state
                .unmap_runtime_events
                .iter()
                .any(|(candidate, _)| *candidate == route)
        {
            return Err("Unmap runtime observation is already enabled or non-empty");
        }
        state.unmap_runtime_routes.push(route);
        Ok(())
    }

    pub(super) fn unmap_runtime_observation_enabled(
        &self,
        route: ManagedTestRouteOrdinal,
    ) -> Result<bool, ()> {
        self.state
            .lock()
            .map(|state| state.unmap_runtime_routes.contains(&route))
            .map_err(|_| ())
    }

    pub(super) fn observe_unmap_runtime_event(
        &self,
        route: ManagedTestRouteOrdinal,
        event: ManagedSqliteRegistryUnmapRuntimeEvent,
    ) -> Result<(), ()> {
        use ManagedSqliteRegistryUnmapRuntimeEvent as E;

        let mut state = self.state.lock().map_err(|_| ())?;
        if !state.unmap_runtime_routes.contains(&route) {
            return Ok(());
        }
        let observed = state
            .unmap_runtime_events
            .iter()
            .filter_map(|(candidate, event)| (*candidate == route).then_some(*event))
            .collect::<Vec<_>>();
        let valid = match observed.as_slice() {
            [] => event == E::CallbackBeginAttempt,
            [E::CallbackBeginAttempt] => event == E::CallbackBeginSuccess,
            [E::CallbackBeginAttempt, E::CallbackBeginSuccess] => {
                matches!(
                    event,
                    E::SelectedActionAttempt | E::CallbackCompletionAttempt
                )
            }
            [E::CallbackBeginAttempt, E::CallbackBeginSuccess, E::SelectedActionAttempt] => {
                event == E::SelectedActionSuccess
            }
            [E::CallbackBeginAttempt, E::CallbackBeginSuccess, E::SelectedActionAttempt, E::SelectedActionSuccess] => {
                event == E::CallbackCompletionAttempt
            }
            [E::CallbackBeginAttempt, E::CallbackBeginSuccess, E::CallbackCompletionAttempt]
            | [E::CallbackBeginAttempt, E::CallbackBeginSuccess, E::SelectedActionAttempt, E::SelectedActionSuccess, E::CallbackCompletionAttempt] => {
                event == E::CallbackCompletionSuccess
            }
            _ => false,
        };
        if !valid {
            return Err(());
        }
        state.unmap_runtime_events.push((route, event));
        Ok(())
    }

    pub(in super::super) fn unmap_runtime_trace(
        &self,
        route: ManagedTestRouteOrdinal,
    ) -> Result<Vec<ManagedSqliteRegistryUnmapRuntimeEvent>, &'static str> {
        self.state
            .lock()
            .map(|state| {
                state
                    .unmap_runtime_events
                    .iter()
                    .filter_map(|(candidate, event)| (*candidate == route).then_some(*event))
                    .collect()
            })
            .map_err(|_| "Unmap runtime trace poisoned")
    }

    pub(in super::super) fn finish_unmap_runtime_observation(
        &self,
        route: ManagedTestRouteOrdinal,
    ) -> Result<Vec<ManagedSqliteRegistryUnmapRuntimeEvent>, &'static str> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| "Unmap runtime trace poisoned")?;
        let Some(window) = state
            .unmap_runtime_routes
            .iter()
            .position(|candidate| *candidate == route)
        else {
            return Err("Unmap runtime observation window is not enabled");
        };
        state.unmap_runtime_routes.remove(window);
        let mut trace = Vec::new();
        state.unmap_runtime_events.retain(|(candidate, event)| {
            if *candidate == route {
                trace.push(*event);
                false
            } else {
                true
            }
        });
        Ok(trace)
    }
}
