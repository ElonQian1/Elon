//! Exact-route, one-shot state for the q4 unsafe-SHM retention preemption receipt.

use std::collections::{hash_map::Entry as MapEntry, HashMap};

use super::super::ManagedTestRouteOrdinal;
use super::ManagedTestLifecycleFaultController;
use crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::registry::ManagedSqliteRegistryUnsafeShmRoutePreemptionReceipt;

#[derive(Default)]
pub(super) struct ManagedTestUnsafeShmRoutePreemptionState {
    entries: HashMap<ManagedTestRouteOrdinal, Entry>,
}

struct Entry {
    claim_count: u64,
    receipt: Option<ManagedSqliteRegistryUnsafeShmRoutePreemptionReceipt>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in super::super) struct ManagedTestUnsafeShmRoutePreemptionSnapshot {
    arm_count: u64,
    claim_count: u64,
    receipt: ManagedSqliteRegistryUnsafeShmRoutePreemptionReceipt,
}

impl ManagedTestUnsafeShmRoutePreemptionSnapshot {
    pub(in super::super) const fn ordered_values(self) -> [u64; 5] {
        let receipt = self.receipt.ordered_values();
        [
            self.arm_count,
            self.claim_count,
            receipt[0],
            receipt[1],
            receipt[2],
        ]
    }
}

impl ManagedTestUnsafeShmRoutePreemptionState {
    pub(super) fn arm(&mut self, route: ManagedTestRouteOrdinal) -> Result<(), &'static str> {
        match self.entries.entry(route) {
            MapEntry::Vacant(entry) => {
                entry.insert(Entry {
                    claim_count: 0,
                    receipt: None,
                });
                Ok(())
            }
            MapEntry::Occupied(_) => Err("unsafe SHM route preemption already armed for route"),
        }
    }

    pub(super) fn claim(&mut self, route: ManagedTestRouteOrdinal) -> bool {
        let Some(entry) = self.entries.get_mut(&route) else {
            return false;
        };
        if entry.claim_count != 0 {
            return false;
        }
        entry.claim_count = 1;
        true
    }

    pub(super) fn record(
        &mut self,
        route: ManagedTestRouteOrdinal,
        receipt: ManagedSqliteRegistryUnsafeShmRoutePreemptionReceipt,
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
    ) -> Result<ManagedTestUnsafeShmRoutePreemptionSnapshot, &'static str> {
        let entry = self
            .entries
            .get(&route)
            .ok_or("unsafe SHM route preemption was not armed")?;
        Ok(ManagedTestUnsafeShmRoutePreemptionSnapshot {
            arm_count: 1,
            claim_count: entry.claim_count,
            receipt: entry
                .receipt
                .ok_or("unsafe SHM route preemption receipt missing")?,
        })
    }
}

impl ManagedTestLifecycleFaultController {
    pub(in super::super) fn arm_unsafe_shm_route_preemption(
        &self,
        route: ManagedTestRouteOrdinal,
    ) -> Result<(), &'static str> {
        self.state
            .lock()
            .map_err(|_| "lifecycle fault controller poisoned")?
            .unsafe_shm_preemption
            .arm(route)
    }

    pub(in super::super) fn claim_unsafe_shm_route_preemption(
        &self,
        route: ManagedTestRouteOrdinal,
    ) -> Result<bool, ()> {
        Ok(self
            .state
            .lock()
            .map_err(|_| ())?
            .unsafe_shm_preemption
            .claim(route))
    }

    pub(in super::super) fn record_unsafe_shm_route_preemption_receipt(
        &self,
        route: ManagedTestRouteOrdinal,
        receipt: ManagedSqliteRegistryUnsafeShmRoutePreemptionReceipt,
    ) -> Result<(), ()> {
        self.state
            .lock()
            .map_err(|_| ())?
            .unsafe_shm_preemption
            .record(route, receipt)
    }

    pub(in super::super) fn unsafe_shm_route_preemption_snapshot(
        &self,
        route: ManagedTestRouteOrdinal,
    ) -> Result<ManagedTestUnsafeShmRoutePreemptionSnapshot, &'static str> {
        self.state
            .lock()
            .map_err(|_| "lifecycle fault controller poisoned")?
            .unsafe_shm_preemption
            .snapshot(route)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_route_state_is_one_shot_and_rejections_do_not_mutate_first_receipt() {
        let route = ManagedTestRouteOrdinal::test_value(7);
        let unknown = ManagedTestRouteOrdinal::test_value(8);
        let first = ManagedSqliteRegistryUnsafeShmRoutePreemptionReceipt::new(true, true, true);
        let different =
            ManagedSqliteRegistryUnsafeShmRoutePreemptionReceipt::new(false, false, false);
        let mut state = ManagedTestUnsafeShmRoutePreemptionState::default();

        assert!(state.snapshot(unknown).is_err());
        assert!(state.record(unknown, first).is_err());
        state.arm(route).expect("arm exact route once");
        assert!(state.record(route, first).is_err());
        assert!(state.snapshot(route).is_err());
        assert!(state.claim(route));
        assert!(!state.claim(route));
        state.record(route, first).expect("record first receipt");
        assert_eq!(
            state.snapshot(route).unwrap().ordered_values(),
            [1, 1, 1, 1, 1]
        );

        assert!(state.arm(route).is_err());
        assert!(state.record(route, different).is_err());
        assert_eq!(
            state.snapshot(route).unwrap().ordered_values(),
            [1, 1, 1, 1, 1]
        );
    }
}
