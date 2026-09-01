//! Test-only exact-route preemption for a successful ordinary SHM Lock lower call.

use super::*;
use crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::registry::ManagedSqliteRegistryOrdinaryShmLockRoutePreemptionReceipt;

#[derive(Debug, Clone, Copy)]
struct Marker {
    request: ManagedSqliteShmLockRequest,
    outcome: ManagedSqliteShmLockAttempt,
}

impl<Custody, NonceSource> ManagedSqliteRegistryPinnedFile<Custody, NonceSource>
where
    Custody: ManagedSqliteRegistryCustody + 'static,
    NonceSource: ManagedSqliteRegistryNonceSource + 'static,
{
    pub(super) fn preempt_ordinary_shm_lock_route(
        &self,
        request: ManagedSqliteShmLockRequest,
        outcome: ManagedSqliteShmLockAttempt,
    ) -> Option<(bool, bool, bool)> {
        self.close_faults
            .as_ref()
            .is_some_and(|faults| {
                faults
                    .claim_ordinary_shm_lock_route_preemption(request, outcome)
                    .unwrap_or(false)
            })
            .then(|| {
                let retained = self
                    .owner
                    .retain_terminal_custody(
                        self.route,
                        ManagedSqliteRegistryTerminalReason::FailureCustodyRetained,
                        Marker { request, outcome },
                    )
                    .is_ok();
                (true, true, retained)
            })
    }

    pub(super) fn record_ordinary_shm_lock_route_preemption(
        &self,
        receipt: Option<(bool, bool, bool)>,
        callback_completion: &Result<(), ManagedSqliteRegistryProcessRouteRejection>,
    ) {
        let Some((request_matched, lower_outcome_matched, preemption_retained)) = receipt else {
            return;
        };
        if let Some(faults) = self.close_faults.as_ref() {
            let _ = faults.record_ordinary_shm_lock_route_preemption_receipt(
                ManagedSqliteRegistryOrdinaryShmLockRoutePreemptionReceipt::new(
                    request_matched,
                    lower_outcome_matched,
                    preemption_retained,
                    route_was_unknown(callback_completion),
                ),
            );
        }
    }
}
