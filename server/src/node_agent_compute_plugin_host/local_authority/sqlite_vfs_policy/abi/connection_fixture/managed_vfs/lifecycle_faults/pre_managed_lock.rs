//! Exact-route observation of Lock callbacks rejected before managed/native dispatch.

use std::collections::{hash_map::Entry as MapEntry, HashMap};

use super::{
    ManagedTestLifecycleFaultBinding, ManagedTestLifecycleFaultController, ManagedTestRouteOrdinal,
};
use crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::registry::{
    ManagedSqliteRegistryPreManagedLockAdmissionOutcome as Admission,
    ManagedSqliteRegistryPreManagedLockCompletionOutcome as Completion,
    ManagedSqliteRegistryPreManagedLockCustody as Custody,
    ManagedSqliteRegistryPreManagedLockEvent as Event,
    ManagedSqliteRegistryPreManagedLockRejection as Rejection,
    ManagedSqliteRegistryPreManagedLockRoutePreemptionReceipt as PreemptionReceipt,
};
use crate::node_agent_managed_fs::ManagedSqliteShmLockRequest;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in super::super) enum ManagedTestPreManagedLockPath {
    AdmissionRouteUnknown,
    AdmissionCounterOverflow,
    UnsupportedCompleted,
    UnsupportedRouteUnknown,
    ShmDetachedCompleted,
    ShmDetachedRouteUnknown,
}

impl ManagedTestPreManagedLockPath {
    const fn tag(self) -> u64 {
        match self {
            Self::AdmissionRouteUnknown => 1,
            Self::AdmissionCounterOverflow => 2,
            Self::UnsupportedCompleted => 3,
            Self::UnsupportedRouteUnknown => 4,
            Self::ShmDetachedCompleted => 5,
            Self::ShmDetachedRouteUnknown => 6,
        }
    }

    const fn preemption_rejection(self) -> Option<Rejection> {
        match self {
            Self::UnsupportedRouteUnknown => Some(Rejection::UnsupportedFileRole),
            Self::ShmDetachedRouteUnknown => Some(Rejection::ShmDetached),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in super::super) struct ManagedTestPreManagedLockSnapshot {
    values: [u64; 18],
}

impl ManagedTestPreManagedLockSnapshot {
    pub(in super::super) const fn ordered_values(self) -> [u64; 18] {
        self.values
    }

    /// The exact validated dispatch says whether the managed/native lower boundary was reached.
    /// Native Lock is downstream of managed Lock, so a pre-managed rejection proves both ledgers
    /// are zero without accepting caller-supplied counts.
    pub(in super::super) const fn lower_ledger_values(self) -> [u64; 3] {
        let effect = if self.values[1] <= 2 { 1 } else { 2 };
        [effect, self.values[9], self.values[9]]
    }
}

#[derive(Default)]
pub(super) struct ManagedTestPreManagedLockState {
    entries: HashMap<ManagedTestRouteOrdinal, Entry>,
}

struct Entry {
    path: ManagedTestPreManagedLockPath,
    request: ManagedSqliteShmLockRequest,
    entry_count: u64,
    admission: Option<Admission>,
    dispatch: Option<(Custody, bool, Option<Rejection>, bool)>,
    completion: Option<Completion>,
    claim_count: u64,
    receipt: Option<PreemptionReceipt>,
    violations: u64,
}

impl ManagedTestPreManagedLockState {
    fn arm(
        &mut self,
        route: ManagedTestRouteOrdinal,
        path: ManagedTestPreManagedLockPath,
        request: ManagedSqliteShmLockRequest,
    ) -> Result<(), &'static str> {
        match self.entries.entry(route) {
            MapEntry::Vacant(slot) => {
                slot.insert(Entry {
                    path,
                    request,
                    entry_count: 0,
                    admission: None,
                    dispatch: None,
                    completion: None,
                    claim_count: 0,
                    receipt: None,
                    violations: 0,
                });
                Ok(())
            }
            MapEntry::Occupied(_) => Err("pre-managed Lock observation already armed for route"),
        }
    }

    fn observe(&mut self, route: ManagedTestRouteOrdinal, event: Event) {
        let Some(entry) = self.entries.get_mut(&route) else {
            return;
        };
        let request = event_request(event);
        if request != entry.request {
            entry.violations = entry.violations.saturating_add(1);
            return;
        }
        let valid = match event {
            Event::Entry { .. } if entry.entry_count == 0 && entry.admission.is_none() => {
                entry.entry_count = 1;
                true
            }
            Event::Admission { outcome, .. }
                if entry.entry_count == 1 && entry.admission.is_none() =>
            {
                entry.admission = Some(outcome);
                true
            }
            Event::Dispatch {
                custody,
                shm_present,
                rejection,
                managed_reached,
                ..
            } if entry.admission == Some(Admission::Succeeded) && entry.dispatch.is_none() => {
                entry.dispatch = Some((custody, shm_present, rejection, managed_reached));
                true
            }
            Event::Completion { outcome, .. }
                if entry.dispatch.is_some() && entry.completion.is_none() =>
            {
                entry.completion = Some(outcome);
                true
            }
            _ => false,
        };
        if !valid {
            entry.violations = entry.violations.saturating_add(1);
        }
    }

    fn claim(
        &mut self,
        route: ManagedTestRouteOrdinal,
        request: ManagedSqliteShmLockRequest,
        rejection: Rejection,
    ) -> bool {
        let Some(entry) = self.entries.get_mut(&route) else {
            return false;
        };
        let expected_dispatch = match entry.path {
            ManagedTestPreManagedLockPath::UnsupportedRouteUnknown => {
                Some((Custody::Main, false, Some(Rejection::UnsupportedFileRole), false))
            }
            ManagedTestPreManagedLockPath::ShmDetachedRouteUnknown => {
                Some((Custody::WalMain, false, Some(Rejection::ShmDetached), false))
            }
            _ => None,
        };
        if entry.request != request
            || entry.path.preemption_rejection() != Some(rejection)
            || entry.claim_count != 0
            || entry.dispatch != expected_dispatch
            || entry.completion.is_some()
            || entry.violations != 0
        {
            return false;
        }
        entry.claim_count = 1;
        true
    }

    fn record(
        &mut self,
        route: ManagedTestRouteOrdinal,
        receipt: PreemptionReceipt,
    ) -> Result<(), ()> {
        let entry = self.entries.get_mut(&route).ok_or(())?;
        if entry.claim_count != 1 || entry.receipt.is_some() {
            return Err(());
        }
        entry.receipt = Some(receipt);
        Ok(())
    }

    fn snapshot(
        &self,
        route: ManagedTestRouteOrdinal,
    ) -> Result<ManagedTestPreManagedLockSnapshot, &'static str> {
        let entry = self
            .entries
            .get(&route)
            .ok_or("pre-managed Lock observation was not armed")?;
        let (custody, shm_present, rejection, managed_reached) =
            entry.dispatch.unwrap_or((Custody::Sidecar, false, None, false));
        let receipt = entry
            .receipt
            .map(PreemptionReceipt::ordered_values)
            .unwrap_or([0; 4]);
        let values = [
            1,
            entry.path.tag(),
            entry.entry_count,
            entry.admission.is_some() as u64,
            admission_tag(entry.admission),
            entry.dispatch.is_some() as u64,
            custody_tag(entry.dispatch.map(|_| custody)),
            shm_present as u64,
            rejection_tag(rejection),
            managed_reached as u64,
            entry.completion.is_some() as u64,
            completion_tag(entry.completion),
            entry.claim_count,
            receipt[0],
            receipt[1],
            receipt[2],
            receipt[3],
            entry.violations,
        ];
        validate_snapshot(entry.path, values)?;
        Ok(ManagedTestPreManagedLockSnapshot { values })
    }
}

fn validate_snapshot(path: ManagedTestPreManagedLockPath, values: [u64; 18]) -> Result<(), &'static str> {
    let expected = match path {
        ManagedTestPreManagedLockPath::AdmissionRouteUnknown => {
            [1, 1, 1, 1, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
        }
        ManagedTestPreManagedLockPath::AdmissionCounterOverflow => {
            [1, 2, 1, 1, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
        }
        ManagedTestPreManagedLockPath::UnsupportedCompleted => {
            [1, 3, 1, 1, 1, 1, 1, 0, 1, 0, 1, 1, 0, 0, 0, 0, 0, 0]
        }
        ManagedTestPreManagedLockPath::UnsupportedRouteUnknown => {
            [1, 4, 1, 1, 1, 1, 1, 0, 1, 0, 1, 2, 1, 1, 1, 1, 1, 0]
        }
        ManagedTestPreManagedLockPath::ShmDetachedCompleted => {
            [1, 5, 1, 1, 1, 1, 2, 0, 2, 0, 1, 1, 0, 0, 0, 0, 0, 0]
        }
        ManagedTestPreManagedLockPath::ShmDetachedRouteUnknown => {
            [1, 6, 1, 1, 1, 1, 2, 0, 2, 0, 1, 2, 1, 1, 1, 1, 1, 0]
        }
    };
    (values == expected)
        .then_some(())
        .ok_or("pre-managed Lock observation does not match exact expected vector")
}

fn event_request(event: Event) -> ManagedSqliteShmLockRequest {
    match event {
        Event::Entry { request }
        | Event::Admission { request, .. }
        | Event::Dispatch { request, .. }
        | Event::Completion { request, .. } => request,
    }
}

const fn admission_tag(outcome: Option<Admission>) -> u64 {
    match outcome {
        None => 0,
        Some(Admission::Succeeded) => 1,
        Some(Admission::RouteUnknown) => 2,
        Some(Admission::CounterOverflow) => 3,
        Some(Admission::OtherRejection) => 4,
    }
}

const fn custody_tag(custody: Option<Custody>) -> u64 {
    match custody {
        None => 0,
        Some(Custody::Main) => 1,
        Some(Custody::WalMain) => 2,
        Some(Custody::Sidecar) => 3,
    }
}

const fn rejection_tag(rejection: Option<Rejection>) -> u64 {
    match rejection {
        None => 0,
        Some(Rejection::UnsupportedFileRole) => 1,
        Some(Rejection::ShmDetached) => 2,
    }
}

const fn completion_tag(outcome: Option<Completion>) -> u64 {
    match outcome {
        None => 0,
        Some(Completion::Succeeded) => 1,
        Some(Completion::RouteUnknown) => 2,
        Some(Completion::OtherRejection) => 3,
    }
}

impl ManagedTestLifecycleFaultController {
    pub(in super::super) fn arm_pre_managed_lock_observation(
        &self,
        route: ManagedTestRouteOrdinal,
        path: ManagedTestPreManagedLockPath,
        request: ManagedSqliteShmLockRequest,
    ) -> Result<(), &'static str> {
        self.state.lock().map_err(|_| "lifecycle fault controller poisoned")?
            .pre_managed_lock.arm(route, path, request)
    }

    fn observe_pre_managed_lock(&self, route: ManagedTestRouteOrdinal, event: Event) -> Result<(), ()> {
        self.state.lock().map_err(|_| ())?.pre_managed_lock.observe(route, event);
        Ok(())
    }

    fn claim_pre_managed_lock(
        &self,
        route: ManagedTestRouteOrdinal,
        request: ManagedSqliteShmLockRequest,
        rejection: Rejection,
    ) -> Result<bool, ()> {
        Ok(self.state.lock().map_err(|_| ())?.pre_managed_lock.claim(route, request, rejection))
    }

    fn record_pre_managed_lock(&self, route: ManagedTestRouteOrdinal, receipt: PreemptionReceipt) -> Result<(), ()> {
        self.state.lock().map_err(|_| ())?.pre_managed_lock.record(route, receipt)
    }

    pub(in super::super) fn pre_managed_lock_snapshot(
        &self,
        route: ManagedTestRouteOrdinal,
    ) -> Result<ManagedTestPreManagedLockSnapshot, &'static str> {
        self.state.lock().map_err(|_| "lifecycle fault controller poisoned")?
            .pre_managed_lock.snapshot(route)
    }
}

impl ManagedTestLifecycleFaultBinding {
    pub(super) fn observe_pre_managed_lock(&self, event: Event) -> Result<(), ()> {
        self.controller.observe_pre_managed_lock(self.route, event)
    }

    pub(super) fn claim_pre_managed_lock(
        &self,
        request: ManagedSqliteShmLockRequest,
        rejection: Rejection,
    ) -> Result<bool, ()> {
        self.controller.claim_pre_managed_lock(self.route, request, rejection)
    }

    pub(super) fn record_pre_managed_lock(&self, receipt: PreemptionReceipt) -> Result<(), ()> {
        self.controller.record_pre_managed_lock(self.route, receipt)
    }
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroU8;
    use super::*;
    use crate::node_agent_managed_fs::ManagedSqliteShmLockAction;

    fn request(first: u8) -> ManagedSqliteShmLockRequest {
        ManagedSqliteShmLockRequest::new(first, NonZeroU8::new(1).unwrap(), ManagedSqliteShmLockAction::LockShared).unwrap()
    }

    #[test]
    fn route_unknown_preemption_requires_real_exact_lower_event_and_is_one_shot() {
        let route = ManagedTestRouteOrdinal::test_value(7);
        let mut state = ManagedTestPreManagedLockState::default();
        state.arm(route, ManagedTestPreManagedLockPath::UnsupportedRouteUnknown, request(1)).unwrap();
        assert!(!state.claim(route, request(2), Rejection::UnsupportedFileRole));
        state.observe(route, Event::Entry { request: request(1) });
        state.observe(route, Event::Admission { request: request(1), outcome: Admission::Succeeded });
        state.observe(route, Event::Dispatch { request: request(1), custody: Custody::Main, shm_present: false, rejection: Some(Rejection::UnsupportedFileRole), managed_reached: false });
        assert!(state.claim(route, request(1), Rejection::UnsupportedFileRole));
        assert!(!state.claim(route, request(1), Rejection::UnsupportedFileRole));
        state.observe(route, Event::Completion { request: request(1), outcome: Completion::RouteUnknown });
        state.record(route, PreemptionReceipt::new(true, true, true, true)).unwrap();
        assert_eq!(state.snapshot(route).unwrap().ordered_values(), [1, 4, 1, 1, 1, 1, 1, 0, 1, 0, 1, 2, 1, 1, 1, 1, 1, 0]);
    }

    #[test]
    fn wrong_dispatch_actual_or_prior_violation_cannot_consume_preemption() {
        for (index, dispatch) in [
            (Custody::Sidecar, false, Some(Rejection::UnsupportedFileRole), false),
            (Custody::Main, true, Some(Rejection::UnsupportedFileRole), false),
            (Custody::Main, false, Some(Rejection::UnsupportedFileRole), true),
        ].into_iter().enumerate() {
            let route = ManagedTestRouteOrdinal::test_value(20 + index as u64);
            let mut state = ManagedTestPreManagedLockState::default();
            state.arm(route, ManagedTestPreManagedLockPath::UnsupportedRouteUnknown, request(1)).unwrap();
            state.observe(route, Event::Entry { request: request(1) });
            state.observe(route, Event::Admission { request: request(1), outcome: Admission::Succeeded });
            state.observe(route, Event::Dispatch { request: request(1), custody: dispatch.0, shm_present: dispatch.1, rejection: dispatch.2, managed_reached: dispatch.3 });
            assert!(!state.claim(route, request(1), Rejection::UnsupportedFileRole));
        }

        let route = ManagedTestRouteOrdinal::test_value(30);
        let mut state = ManagedTestPreManagedLockState::default();
        state.arm(route, ManagedTestPreManagedLockPath::UnsupportedRouteUnknown, request(1)).unwrap();
        state.observe(route, Event::Entry { request: request(2) });
        state.observe(route, Event::Entry { request: request(1) });
        state.observe(route, Event::Admission { request: request(1), outcome: Admission::Succeeded });
        state.observe(route, Event::Dispatch { request: request(1), custody: Custody::Main, shm_present: false, rejection: Some(Rejection::UnsupportedFileRole), managed_reached: false });
        assert!(!state.claim(route, request(1), Rejection::UnsupportedFileRole));

        let route = ManagedTestRouteOrdinal::test_value(31);
        let mut state = ManagedTestPreManagedLockState::default();
        state.arm(route, ManagedTestPreManagedLockPath::UnsupportedRouteUnknown, request(1)).unwrap();
        state.observe(route, Event::Entry { request: request(1) });
        state.observe(route, Event::Admission { request: request(1), outcome: Admission::Succeeded });
        state.observe(route, Event::Dispatch { request: request(1), custody: Custody::Main, shm_present: false, rejection: Some(Rejection::UnsupportedFileRole), managed_reached: false });
        state.observe(route, Event::Completion { request: request(1), outcome: Completion::Succeeded });
        assert!(!state.claim(route, request(1), Rejection::UnsupportedFileRole));
    }
}
