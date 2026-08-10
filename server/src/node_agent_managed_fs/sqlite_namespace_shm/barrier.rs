use std::sync::atomic;

use super::{
    coordinator::{
        ManagedSqliteShmCoordinator, ManagedSqliteShmCoordinatorState,
        PinnedManagedSqliteShmConnection,
    },
    types::{ManagedSqliteShmFailure, ManagedSqliteShmFailurePhase},
};

impl PinnedManagedSqliteShmConnection {
    /// Performs SQLite's no-return-channel SHM memory barrier.
    ///
    /// The result remains internal to the managed VFS adapter. A caller must terminalize its raw
    /// callback state on failure; it cannot translate this result into a SQLite return code.
    pub(crate) fn barrier(&self) -> Result<(), ManagedSqliteShmFailure> {
        #[cfg(test)]
        let fault = {
            let mut state = self.coordinator.lock_barrier_state(
                self.connection_id,
                self.active,
                "NODE_MANAGED_SQLITE_SHM_BARRIER_PREPARE_FAILED",
            )?;
            match self.coordinator.begin_test_fault(
                &mut state,
                self.connection_id,
                ManagedSqliteShmFailurePhase::Barrier,
                false,
            ) {
                Ok(fault) => fault,
                Err(failure) => {
                    // xShmBarrier cannot report an error. Even a mutation-free before fault must
                    // retain terminal custody instead of allowing the callback to look complete.
                    self.coordinator
                        .terminalize_test_fault(&mut state, &failure);
                    return Err(failure);
                }
            }
        };

        atomic::fence(atomic::Ordering::SeqCst);
        {
            let state = self.coordinator.lock_barrier_state(
                self.connection_id,
                self.active,
                "NODE_MANAGED_SQLITE_SHM_BARRIER_STATE_FAILED",
            )?;
            drop(state);
        }
        atomic::fence(atomic::Ordering::SeqCst);

        #[cfg(test)]
        {
            let mut state = self.coordinator.lock_barrier_state(
                self.connection_id,
                self.active,
                "NODE_MANAGED_SQLITE_SHM_BARRIER_COMPLETION_FAILED",
            )?;
            if let Some(failure) = self.coordinator.finish_test_fault(&mut state, fault, false) {
                return Err(failure);
            }
        }
        Ok(())
    }
}

impl ManagedSqliteShmCoordinator {
    fn lock_barrier_state<'state>(
        &'state self,
        connection_id: u64,
        active: bool,
        failure_code: &'static str,
    ) -> Result<
        std::sync::MutexGuard<'state, ManagedSqliteShmCoordinatorState>,
        ManagedSqliteShmFailure,
    > {
        let mut state = self.state.lock().map_err(|_| {
            self.mark_domain_terminal();
            ManagedSqliteShmFailure::poisoned_code(
                ManagedSqliteShmFailurePhase::Barrier,
                failure_code,
                false,
                false,
            )
        })?;
        if let Some(poison) = state.poisoned {
            return Err(poison.failure());
        }
        if !active || !state.connections.contains_key(&connection_id) {
            self.mark_poisoned(
                &mut state,
                ManagedSqliteShmFailurePhase::Barrier,
                false,
                false,
            );
            return Err(ManagedSqliteShmFailure::poisoned_code(
                ManagedSqliteShmFailurePhase::Barrier,
                "NODE_MANAGED_SQLITE_SHM_BARRIER_CONNECTION_UNAVAILABLE",
                false,
                false,
            ));
        }
        Ok(state)
    }
}
