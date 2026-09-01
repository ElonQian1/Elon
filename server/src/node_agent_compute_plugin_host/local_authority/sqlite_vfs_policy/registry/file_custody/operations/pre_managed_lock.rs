//! Passive q9 observation plus exact post-lower route preemption.

use super::super::test_faults::{
    ManagedSqliteRegistryPreManagedLockEvent, ManagedSqliteRegistryPreManagedLockRejection,
    ManagedSqliteRegistryPreManagedLockRoutePreemptionReceipt,
};
use super::*;

#[derive(Debug, Clone, Copy)]
struct Marker {
    request: ManagedSqliteShmLockRequest,
    rejection: ManagedSqliteRegistryPreManagedLockRejection,
}

impl<Custody, NonceSource> ManagedSqliteRegistryPinnedFile<Custody, NonceSource>
where
    Custody: ManagedSqliteRegistryCustody + 'static,
    NonceSource: ManagedSqliteRegistryNonceSource + 'static,
{
    pub(super) fn observe_pre_managed_lock(&self, event: ManagedSqliteRegistryPreManagedLockEvent) {
        if let Some(faults) = self.close_faults.as_ref() {
            let _ = faults.observe_pre_managed_shm_lock_event(event);
        }
    }

    pub(super) fn preempt_pre_managed_lock_route(
        &self,
        request: ManagedSqliteShmLockRequest,
        rejection: ManagedSqliteRegistryPreManagedLockRejection,
    ) -> Option<(bool, bool, bool)> {
        self.close_faults
            .as_ref()
            .is_some_and(|faults| {
                faults
                    .claim_pre_managed_shm_lock_route_preemption(request, rejection)
                    .unwrap_or(false)
            })
            .then(|| {
                let retained = self
                    .owner
                    .retain_terminal_custody(
                        self.route,
                        ManagedSqliteRegistryTerminalReason::FailureCustodyRetained,
                        Marker { request, rejection },
                    )
                    .is_ok();
                (true, true, retained)
            })
    }

    pub(super) fn record_pre_managed_lock_preemption(
        &self,
        receipt: Option<(bool, bool, bool)>,
        callback_completion: &Result<(), ManagedSqliteRegistryProcessRouteRejection>,
    ) {
        let Some((request_matched, rejection_matched, retained)) = receipt else {
            return;
        };
        if let Some(faults) = self.close_faults.as_ref() {
            let _ = faults.record_pre_managed_shm_lock_route_preemption_receipt(
                ManagedSqliteRegistryPreManagedLockRoutePreemptionReceipt::new(
                    request_matched,
                    rejection_matched,
                    retained,
                    route_was_unknown(callback_completion),
                ),
            );
        }
    }
}
