use std::sync::atomic::Ordering;

use super::{
    ManagedTestLifecycleFaultController, ManagedTestLifecycleFaultPhase,
    ManagedTestLifecycleFaultTiming, ManagedTestRouteOrdinal,
};

impl ManagedTestLifecycleFaultController {
    pub(super) fn claim_native_failure_gate(
        &self,
        route: ManagedTestRouteOrdinal,
        phase: ManagedTestLifecycleFaultPhase,
    ) -> Result<bool, ()> {
        let mut state = self.state.lock().map_err(|_| {
            self.terminal.store(true, Ordering::SeqCst);
        })?;
        let key = (Some(route), phase);
        let native_step_exists = state.steps.iter().any(|(step, _)| {
            step.route == Some(route)
                && step.timing == ManagedTestLifecycleFaultTiming::NativeFailure
                && step.phase == phase
        });
        if !native_step_exists {
            return Ok(false);
        }
        let Some(occurrence) = state.occurrences.get(&key).copied() else {
            self.terminal.store(true, Ordering::SeqCst);
            return Err(());
        };
        let Some((_, consumed)) = state.steps.iter_mut().find(|(step, _)| {
            step.route == key.0
                && step.phase == key.1
                && step.occurrence.get() == occurrence
                && step.timing == ManagedTestLifecycleFaultTiming::NativeFailure
        }) else {
            self.terminal.store(true, Ordering::SeqCst);
            return Err(());
        };
        if *consumed {
            self.terminal.store(true, Ordering::SeqCst);
            return Err(());
        }
        *consumed = true;
        Ok(true)
    }
}
