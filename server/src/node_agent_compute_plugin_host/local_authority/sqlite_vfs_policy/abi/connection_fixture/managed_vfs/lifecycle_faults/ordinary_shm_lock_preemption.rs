//! Exact-route one-shot preemption after an ordinary managed SHM Lock result.

use std::collections::{hash_map::Entry as MapEntry, HashMap};

use super::{
    ManagedTestLifecycleFaultBinding, ManagedTestLifecycleFaultController, ManagedTestRouteOrdinal,
};
use crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::registry::ManagedSqliteRegistryOrdinaryShmLockRoutePreemptionReceipt;
use crate::node_agent_managed_fs::{ManagedSqliteShmLockAttempt, ManagedSqliteShmLockRequest};

#[derive(Default)]
pub(super) struct ManagedTestOrdinaryShmLockRoutePreemptionState {
    entries: HashMap<ManagedTestRouteOrdinal, Entry>,
}

struct Entry {
    expected_request: ManagedSqliteShmLockRequest,
    expected_outcome: ManagedSqliteShmLockAttempt,
    claim_count: u64,
    receipt: Option<ManagedSqliteRegistryOrdinaryShmLockRoutePreemptionReceipt>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in super::super) struct ManagedTestOrdinaryShmLockRoutePreemptionSnapshot {
    arm_count: u64,
    claim_count: u64,
    receipt: ManagedSqliteRegistryOrdinaryShmLockRoutePreemptionReceipt,
}

impl ManagedTestOrdinaryShmLockRoutePreemptionSnapshot {
    pub(in super::super) const fn ordered_values(self) -> [u64; 6] {
        let receipt = self.receipt.ordered_values();
        [
            self.arm_count,
            self.claim_count,
            receipt[0],
            receipt[1],
            receipt[2],
            receipt[3],
        ]
    }
}

impl ManagedTestOrdinaryShmLockRoutePreemptionState {
    pub(super) fn arm(
        &mut self,
        route: ManagedTestRouteOrdinal,
        expected_request: ManagedSqliteShmLockRequest,
        expected_outcome: ManagedSqliteShmLockAttempt,
    ) -> Result<(), &'static str> {
        match self.entries.entry(route) {
            MapEntry::Vacant(entry) => {
                entry.insert(Entry {
                    expected_request,
                    expected_outcome,
                    claim_count: 0,
                    receipt: None,
                });
                Ok(())
            }
            MapEntry::Occupied(_) => {
                Err("ordinary SHM Lock route preemption already armed for route")
            }
        }
    }

    pub(super) fn claim(
        &mut self,
        route: ManagedTestRouteOrdinal,
        request: ManagedSqliteShmLockRequest,
        outcome: ManagedSqliteShmLockAttempt,
    ) -> bool {
        let Some(entry) = self.entries.get_mut(&route) else {
            return false;
        };
        if entry.claim_count != 0
            || entry.expected_request != request
            || entry.expected_outcome != outcome
        {
            return false;
        }
        entry.claim_count = 1;
        true
    }

    pub(super) fn record(
        &mut self,
        route: ManagedTestRouteOrdinal,
        receipt: ManagedSqliteRegistryOrdinaryShmLockRoutePreemptionReceipt,
    ) -> Result<(), ()> {
        let entry = self.entries.get_mut(&route).ok_or(())?;
        if entry.claim_count != 1 || entry.receipt.is_some() {
            return Err(());
        }
        entry.receipt = Some(receipt);
        Ok(())
    }

    pub(super) fn snapshot(
        &self,
        route: ManagedTestRouteOrdinal,
    ) -> Result<ManagedTestOrdinaryShmLockRoutePreemptionSnapshot, &'static str> {
        let entry = self
            .entries
            .get(&route)
            .ok_or("ordinary SHM Lock route preemption was not armed")?;
        Ok(ManagedTestOrdinaryShmLockRoutePreemptionSnapshot {
            arm_count: 1,
            claim_count: entry.claim_count,
            receipt: entry
                .receipt
                .ok_or("ordinary SHM Lock route preemption receipt missing")?,
        })
    }
}

impl ManagedTestLifecycleFaultController {
    pub(in super::super) fn arm_ordinary_shm_lock_route_preemption(
        &self,
        route: ManagedTestRouteOrdinal,
        expected_request: ManagedSqliteShmLockRequest,
        expected_outcome: ManagedSqliteShmLockAttempt,
    ) -> Result<(), &'static str> {
        self.state
            .lock()
            .map_err(|_| "lifecycle fault controller poisoned")?
            .ordinary_shm_lock_preemption
            .arm(route, expected_request, expected_outcome)
    }

    pub(in super::super) fn claim_ordinary_shm_lock_route_preemption(
        &self,
        route: ManagedTestRouteOrdinal,
        request: ManagedSqliteShmLockRequest,
        outcome: ManagedSqliteShmLockAttempt,
    ) -> Result<bool, ()> {
        Ok(self
            .state
            .lock()
            .map_err(|_| ())?
            .ordinary_shm_lock_preemption
            .claim(route, request, outcome))
    }

    pub(in super::super) fn record_ordinary_shm_lock_route_preemption_receipt(
        &self,
        route: ManagedTestRouteOrdinal,
        receipt: ManagedSqliteRegistryOrdinaryShmLockRoutePreemptionReceipt,
    ) -> Result<(), ()> {
        self.state
            .lock()
            .map_err(|_| ())?
            .ordinary_shm_lock_preemption
            .record(route, receipt)
    }

    pub(in super::super) fn ordinary_shm_lock_route_preemption_snapshot(
        &self,
        route: ManagedTestRouteOrdinal,
    ) -> Result<ManagedTestOrdinaryShmLockRoutePreemptionSnapshot, &'static str> {
        self.state
            .lock()
            .map_err(|_| "lifecycle fault controller poisoned")?
            .ordinary_shm_lock_preemption
            .snapshot(route)
    }
}

impl ManagedTestLifecycleFaultBinding {
    pub(super) fn claim_ordinary_shm_lock_route_preemption(
        &self,
        request: ManagedSqliteShmLockRequest,
        outcome: ManagedSqliteShmLockAttempt,
    ) -> Result<bool, ()> {
        self.controller
            .claim_ordinary_shm_lock_route_preemption(self.route, request, outcome)
    }

    pub(super) fn record_ordinary_shm_lock_route_preemption_receipt(
        &self,
        receipt: ManagedSqliteRegistryOrdinaryShmLockRoutePreemptionReceipt,
    ) -> Result<(), ()> {
        self.controller
            .record_ordinary_shm_lock_route_preemption_receipt(self.route, receipt)
    }
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroU8;

    use super::*;
    use crate::node_agent_managed_fs::ManagedSqliteShmLockAction;

    fn request(first: u8) -> ManagedSqliteShmLockRequest {
        ManagedSqliteShmLockRequest::new(
            first,
            NonZeroU8::new(1).expect("non-zero"),
            ManagedSqliteShmLockAction::LockShared,
        )
        .expect("valid one-slot request")
    }

    #[test]
    fn exact_route_request_and_outcome_are_one_shot_without_negative_consumption() {
        let route = ManagedTestRouteOrdinal::test_value(7);
        let other_route = ManagedTestRouteOrdinal::test_value(8);
        let receipt =
            ManagedSqliteRegistryOrdinaryShmLockRoutePreemptionReceipt::new(true, true, true, true);
        let mut state = ManagedTestOrdinaryShmLockRoutePreemptionState::default();

        state
            .arm(route, request(1), ManagedSqliteShmLockAttempt::Acquired)
            .expect("arm exact ordinary result once");
        assert!(!state.claim(
            other_route,
            request(1),
            ManagedSqliteShmLockAttempt::Acquired
        ));
        assert!(!state.claim(route, request(2), ManagedSqliteShmLockAttempt::Acquired));
        assert!(!state.claim(route, request(1), ManagedSqliteShmLockAttempt::Contended));
        assert!(state.snapshot(route).is_err());
        assert!(state.claim(route, request(1), ManagedSqliteShmLockAttempt::Acquired));
        assert!(!state.claim(route, request(1), ManagedSqliteShmLockAttempt::Acquired));
        state.record(route, receipt).expect("record claimed route");
        assert_eq!(state.snapshot(route).unwrap().ordered_values(), [1; 6]);
        assert!(state
            .arm(route, request(1), ManagedSqliteShmLockAttempt::Acquired)
            .is_err());
        assert!(state.record(route, receipt).is_err());
    }
}
