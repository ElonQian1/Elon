//! Exact-state priming for the Lock callback-counter overflow source program.

use super::*;
use crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::registry::types::ManagedSqliteRegistryCallbackCounterPrimeReceipt;

impl ManagedSqliteRegistrySessionState {
    pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::registry) fn prime_lock_callback_counter_overflow_for_test(
        &mut self,
    ) -> Result<
        ManagedSqliteRegistryCallbackCounterPrimeReceipt,
        ManagedSqliteRegistryTransitionRejection,
    > {
        self.ensure_shape()?;
        if self.phase != ManagedSqliteRegistrySessionPhase::Active {
            return Err(self.phase_rejection());
        }
        if self.callbacks_in_flight != 0 {
            return Err(ManagedSqliteRegistryTransitionRejection::OutstandingCallbacks);
        }
        self.callbacks_in_flight = u32::MAX;
        Ok(ManagedSqliteRegistryCallbackCounterPrimeReceipt::exact_active_zero_to_max())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::num::NonZeroU64;

    fn pending() -> ManagedSqliteRegistrySessionState {
        ManagedSqliteRegistrySessionState {
            session_id: ManagedSqliteRegistrySessionId::test_value(7),
            route_epoch: NonZeroU64::new(9).unwrap(),
            phase: ManagedSqliteRegistrySessionPhase::PendingMain,
            next_lease_ordinal: 0,
            connection_owner: false,
            main_was_claimed: false,
            main_lease: None,
            sidecar_leases: [None; 4],
            shm_lease: None,
            callbacks_in_flight: 0,
            terminal_reason: None,
        }
    }

    #[test]
    fn priming_only_changes_an_exact_active_quiescent_counter() {
        let mut wrong_phase = pending();
        assert!(wrong_phase
            .prime_lock_callback_counter_overflow_for_test()
            .is_err());

        let mut state = pending();
        state.phase = ManagedSqliteRegistrySessionPhase::Active;
        state.connection_owner = true;
        state.main_was_claimed = true;
        state.next_lease_ordinal = 1;
        state.main_lease = Some(ManagedSqliteRegistryLeaseRecord {
            ordinal: NonZeroU64::new(1).unwrap(),
            role: ManagedSqliteLogicalFileRole::Main,
        });
        assert_eq!(
            state
                .prime_lock_callback_counter_overflow_for_test()
                .unwrap()
                .ordered_values(),
            [1, 0, u32::MAX as u64, 1]
        );
        assert_eq!(state.callbacks_in_flight, u32::MAX);
        assert!(matches!(
            state.begin_callback(ManagedSqliteRegistryCallbackKind::Shm),
            Err(ManagedSqliteRegistryTransitionRejection::CounterOverflow)
        ));
        assert_eq!(
            state.terminal_reason,
            Some(ManagedSqliteRegistryTerminalReason::CallbackCounterOverflow)
        );
    }
}
