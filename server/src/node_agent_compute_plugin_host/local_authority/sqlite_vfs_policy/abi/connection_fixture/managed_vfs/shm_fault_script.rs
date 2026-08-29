//! Exact route binding for one pending managed-SHM test fault script.
//!
//! The route file may consume a plan only when registration, route ordinal and file role all
//! match. The plan contains no runtime generation, SHM connection id or raw custody; those are
//! derived privately only after the main file has been promoted into its live WAL-main holder.

use std::{
    num::NonZeroU64,
    sync::{Arc, Mutex},
};

use crate::{
    node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::ManagedSqliteLogicalFileRole,
    node_agent_managed_fs::{
        ManagedSqliteShmFailureClass, ManagedSqliteShmFailurePhase, ManagedSqliteShmTestFaultProbe,
    },
};

#[cfg(all(test, windows))]
use crate::node_agent_managed_fs::{
    ManagedSqliteShmTestTargetIdentity, ManagedSqliteShmTestTargetObserver,
};

use super::ManagedTestRouteOrdinal;

const MAX_SHM_FAULT_STEPS: usize = 32;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) struct ManagedTestRegistrationId(NonZeroU64);

impl ManagedTestRegistrationId {
    pub(super) fn from_counter(value: u64) -> Result<Self, &'static str> {
        NonZeroU64::new(value)
            .map(Self)
            .ok_or("managed test VFS registration id must be non-zero")
    }

    pub(super) fn counter_value(self) -> u64 {
        self.0.get()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct ManagedTestShmFaultTarget {
    registration: ManagedTestRegistrationId,
    route: ManagedTestRouteOrdinal,
    role: ManagedSqliteLogicalFileRole,
}

impl ManagedTestShmFaultTarget {
    fn new(
        registration: ManagedTestRegistrationId,
        route: ManagedTestRouteOrdinal,
        role: ManagedSqliteLogicalFileRole,
    ) -> Self {
        Self {
            registration,
            route,
            role,
        }
    }

    fn same_route(self, other: Self) -> bool {
        self.registration == other.registration && self.route == other.route
    }
}

impl std::fmt::Debug for ManagedTestRegistrationId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("ManagedTestRegistrationId(<opaque>)")
    }
}

pub(super) struct ManagedTestShmFaultPlan {
    target: ManagedTestShmFaultTarget,
    before_call: Vec<(ManagedSqliteShmFailurePhase, u32)>,
    after_success: Vec<(
        ManagedSqliteShmFailurePhase,
        u32,
        ManagedSqliteShmFailureClass,
    )>,
}

impl ManagedTestShmFaultPlan {
    fn new(
        target: ManagedTestShmFaultTarget,
        before_call: &[(ManagedSqliteShmFailurePhase, u32)],
        after_success: &[(
            ManagedSqliteShmFailurePhase,
            u32,
            ManagedSqliteShmFailureClass,
        )],
    ) -> Result<Self, &'static str> {
        if target.role != ManagedSqliteLogicalFileRole::Main {
            return Err("managed SHM fault plan requires the exact main-file role");
        }
        let count = before_call
            .len()
            .checked_add(after_success.len())
            .ok_or("managed SHM fault plan size overflow")?;
        if count == 0 || count > MAX_SHM_FAULT_STEPS {
            return Err("managed SHM fault plan must contain 1..=32 steps");
        }
        if before_call
            .iter()
            .any(|(phase, occurrence)| *occurrence == 0 || !supported_shm_phase(*phase))
            || after_success.iter().any(|(phase, occurrence, class)| {
                *occurrence == 0
                    || !supported_shm_phase(*phase)
                    || !supports_after_success(*phase)
                    || !supports_after_success_class(*phase, *class)
            })
        {
            return Err("managed SHM fault plan contains an unsupported step");
        }
        let mut exact_steps = Vec::with_capacity(count);
        for &(phase, occurrence) in before_call {
            if exact_steps.contains(&(phase, occurrence)) {
                return Err("managed SHM fault plan contains a duplicate phase occurrence");
            }
            exact_steps.push((phase, occurrence));
        }
        for &(phase, occurrence, _) in after_success {
            if exact_steps.contains(&(phase, occurrence)) {
                return Err("managed SHM fault plan contains a duplicate phase occurrence");
            }
            exact_steps.push((phase, occurrence));
        }
        Ok(Self {
            target,
            before_call: before_call.to_vec(),
            after_success: after_success.to_vec(),
        })
    }

    pub(super) fn before_call(&self) -> &[(ManagedSqliteShmFailurePhase, u32)] {
        &self.before_call
    }

    pub(super) fn after_success(
        &self,
    ) -> &[(
        ManagedSqliteShmFailurePhase,
        u32,
        ManagedSqliteShmFailureClass,
    )] {
        &self.after_success
    }
}

enum ManagedTestShmFaultPlanState {
    Empty,
    Pending(ManagedTestShmFaultPlan),
    Claimed,
    #[cfg(all(test, windows))]
    Promoted(ManagedSqliteShmTestTargetObserver),
    Installed(ManagedSqliteShmTestFaultProbe),
}

/// Copy-only exact physical target evidence. The opaque route objects, coordinator and file
/// custody remain behind the binding; only child-local checked counters cross into the runner.
#[cfg(all(test, windows))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct ManagedTestShmTargetWitness {
    registration_id: u64,
    route_ordinal: u64,
    runtime_generation: u64,
    shm_connection_id: u64,
    role: ManagedSqliteLogicalFileRole,
}

#[cfg(all(test, windows))]
impl ManagedTestShmTargetWitness {
    pub(super) fn registration_id(self) -> u64 {
        self.registration_id
    }

    pub(super) fn route_ordinal(self) -> u64 {
        self.route_ordinal
    }

    pub(super) fn runtime_generation(self) -> u64 {
        self.runtime_generation
    }

    pub(super) fn shm_connection_id(self) -> u64 {
        self.shm_connection_id
    }

    pub(super) fn role(self) -> ManagedSqliteLogicalFileRole {
        self.role
    }
}

pub(super) struct ManagedTestShmFaultPlanSlot {
    route_target: ManagedTestShmFaultTarget,
    state: Mutex<ManagedTestShmFaultPlanState>,
}

impl ManagedTestShmFaultPlanSlot {
    pub(super) fn new(
        registration: ManagedTestRegistrationId,
        route: ManagedTestRouteOrdinal,
    ) -> Arc<Self> {
        Arc::new(Self {
            route_target: ManagedTestShmFaultTarget::new(
                registration,
                route,
                ManagedSqliteLogicalFileRole::Main,
            ),
            state: Mutex::new(ManagedTestShmFaultPlanState::Empty),
        })
    }

    pub(super) fn binding(
        self: &Arc<Self>,
        registration: ManagedTestRegistrationId,
        route: ManagedTestRouteOrdinal,
        role: ManagedSqliteLogicalFileRole,
    ) -> Result<ManagedTestShmFaultPlanBinding, &'static str> {
        let target = ManagedTestShmFaultTarget::new(registration, route, role);
        if !self.route_target.same_route(target) {
            return Err("managed SHM fault binding route identity mismatch");
        }
        Ok(ManagedTestShmFaultPlanBinding {
            target,
            slot: Arc::clone(self),
        })
    }
}

#[derive(Clone)]
pub(super) struct ManagedTestShmFaultPlanBinding {
    target: ManagedTestShmFaultTarget,
    slot: Arc<ManagedTestShmFaultPlanSlot>,
}

impl ManagedTestShmFaultPlanBinding {
    pub(super) fn install(
        &self,
        before_call: &[(ManagedSqliteShmFailurePhase, u32)],
        after_success: &[(
            ManagedSqliteShmFailurePhase,
            u32,
            ManagedSqliteShmFailureClass,
        )],
    ) -> Result<(), &'static str> {
        let plan = ManagedTestShmFaultPlan::new(self.target, before_call, after_success)?;
        let mut state = self
            .slot
            .state
            .lock()
            .map_err(|_| "managed SHM fault plan slot poisoned")?;
        if matches!(&*state, ManagedTestShmFaultPlanState::Empty) {
            *state = ManagedTestShmFaultPlanState::Pending(plan);
            return Ok(());
        }
        #[cfg(all(test, windows))]
        if let ManagedTestShmFaultPlanState::Promoted(observer) = &*state {
            let observer = observer.clone();
            drop(state);
            let probe =
                observer.install_test_fault_script(plan.before_call(), plan.after_success())?;
            let mut state = self
                .slot
                .state
                .lock()
                .map_err(|_| "managed SHM fault plan slot poisoned")?;
            if !matches!(&*state, ManagedTestShmFaultPlanState::Promoted(_)) {
                permanently_retain_probe(probe);
                return Err("managed SHM target changed during late fault installation");
            }
            *state = ManagedTestShmFaultPlanState::Installed(probe);
            return Ok(());
        }
        Err("managed SHM fault plan already installed")
    }

    pub(super) fn claim(&self) -> Result<Option<ManagedTestShmFaultPlan>, &'static str> {
        if self.target.role != ManagedSqliteLogicalFileRole::Main {
            return Ok(None);
        }
        let mut state = self
            .slot
            .state
            .lock()
            .map_err(|_| "managed SHM fault plan slot poisoned")?;
        if !matches!(&*state, ManagedTestShmFaultPlanState::Pending(_)) {
            return Ok(None);
        }
        let previous = std::mem::replace(&mut *state, ManagedTestShmFaultPlanState::Claimed);
        let plan = match previous {
            ManagedTestShmFaultPlanState::Pending(plan) => plan,
            other => {
                *state = other;
                return Err("managed SHM fault plan state changed while claimed");
            }
        };
        if plan.target != self.target {
            return Err("managed SHM fault plan target mismatch");
        }
        Ok(Some(plan))
    }

    pub(super) fn record_installed(
        &self,
        probe: ManagedSqliteShmTestFaultProbe,
    ) -> Result<(), &'static str> {
        if self.target.role != ManagedSqliteLogicalFileRole::Main {
            permanently_retain_probe(probe);
            return Err("managed SHM fault probe requires the exact main-file role");
        }
        let mut state = match self.slot.state.lock() {
            Ok(state) => state,
            Err(poison) => {
                drop(poison.into_inner());
                permanently_retain_probe(probe);
                return Err("managed SHM fault plan slot poisoned");
            }
        };
        if !matches!(&*state, ManagedTestShmFaultPlanState::Claimed) {
            permanently_retain_probe(probe);
            return Err("managed SHM fault probe has no claimed exact plan");
        }
        *state = ManagedTestShmFaultPlanState::Installed(probe);
        Ok(())
    }

    #[cfg(all(test, windows))]
    pub(super) fn record_promoted(
        &self,
        observer: ManagedSqliteShmTestTargetObserver,
    ) -> Result<(), &'static str> {
        if self.target.role != ManagedSqliteLogicalFileRole::Main {
            return Err("managed SHM target observer requires the exact main-file role");
        }
        let mut state = self
            .slot
            .state
            .lock()
            .map_err(|_| "managed SHM fault plan slot poisoned")?;
        match &*state {
            ManagedTestShmFaultPlanState::Empty => {
                *state = ManagedTestShmFaultPlanState::Promoted(observer);
                Ok(())
            }
            ManagedTestShmFaultPlanState::Promoted(_)
            | ManagedTestShmFaultPlanState::Installed(_) => Ok(()),
            ManagedTestShmFaultPlanState::Pending(_) | ManagedTestShmFaultPlanState::Claimed => {
                Err("managed SHM target observer cannot replace a fault plan")
            }
        }
    }

    pub(super) fn pending_count(&self) -> Result<usize, &'static str> {
        if self.target.role != ManagedSqliteLogicalFileRole::Main {
            return Err("managed SHM fault probe query requires the exact main-file role");
        }
        let state = self
            .slot
            .state
            .lock()
            .map_err(|_| "managed SHM fault plan slot poisoned")?;
        match &*state {
            ManagedTestShmFaultPlanState::Promoted(_) => Ok(0),
            ManagedTestShmFaultPlanState::Installed(probe) => probe.pending_count(),
            _ => Err("managed SHM target observer is not installed"),
        }
    }

    pub(super) fn was_triggered(
        &self,
        phase: ManagedSqliteShmFailurePhase,
        occurrence: u32,
    ) -> Result<bool, &'static str> {
        if self.target.role != ManagedSqliteLogicalFileRole::Main {
            return Err("managed SHM fault probe query requires the exact main-file role");
        }
        let state = self
            .slot
            .state
            .lock()
            .map_err(|_| "managed SHM fault plan slot poisoned")?;
        match &*state {
            ManagedTestShmFaultPlanState::Promoted(_) => Ok(false),
            ManagedTestShmFaultPlanState::Installed(probe) => {
                probe.was_triggered(phase, occurrence)
            }
            _ => Err("managed SHM target observer is not installed"),
        }
    }

    #[cfg(all(test, windows))]
    pub(super) fn triggered_observation(
        &self,
        phase: ManagedSqliteShmFailurePhase,
        occurrence: u32,
    ) -> Result<
        Option<crate::node_agent_managed_fs::ManagedSqliteShmTriggeredTestFaultObservation>,
        &'static str,
    > {
        if self.target.role != ManagedSqliteLogicalFileRole::Main {
            return Err("managed SHM fault observation requires the exact main-file role");
        }
        let state = self
            .slot
            .state
            .lock()
            .map_err(|_| "managed SHM fault plan slot poisoned")?;
        match &*state {
            ManagedTestShmFaultPlanState::Promoted(_) => Ok(None),
            ManagedTestShmFaultPlanState::Installed(probe) => {
                probe.triggered_observation(phase, occurrence)
            }
            _ => Err("managed SHM target observer is not installed"),
        }
    }

    #[cfg(all(test, windows))]
    pub(super) fn observer(&self) -> Result<ManagedSqliteShmTestTargetObserver, &'static str> {
        if self.target.role != ManagedSqliteLogicalFileRole::Main {
            return Err("managed SHM fault observer requires the exact main-file role");
        }
        let state = self
            .slot
            .state
            .lock()
            .map_err(|_| "managed SHM fault plan slot poisoned")?;
        match &*state {
            ManagedTestShmFaultPlanState::Promoted(observer) => Ok(observer.clone()),
            ManagedTestShmFaultPlanState::Installed(probe) => Ok(probe.observer()),
            _ => Err("managed SHM target observer is not installed"),
        }
    }

    #[cfg(all(test, windows))]
    pub(super) fn target_witness(&self) -> Result<ManagedTestShmTargetWitness, &'static str> {
        let ManagedSqliteShmTestTargetIdentity {
            runtime_generation,
            shm_connection_id,
        } = self.observer()?.identity();
        Ok(ManagedTestShmTargetWitness {
            registration_id: self.target.registration.counter_value(),
            route_ordinal: self.target.route.counter_value(),
            runtime_generation,
            shm_connection_id,
            role: self.target.role,
        })
    }

    pub(super) fn role(&self) -> ManagedSqliteLogicalFileRole {
        self.target.role
    }
}

fn supported_shm_phase(phase: ManagedSqliteShmFailurePhase) -> bool {
    matches!(
        phase,
        ManagedSqliteShmFailurePhase::ExactSiblingOpen
            | ManagedSqliteShmFailurePhase::DmsExclusiveAcquire
            | ManagedSqliteShmFailurePhase::DmsTruncate
            | ManagedSqliteShmFailurePhase::DmsExclusiveRelease
            | ManagedSqliteShmFailurePhase::DmsSharedAcquire
            | ManagedSqliteShmFailurePhase::FileSize
            | ManagedSqliteShmFailurePhase::FileGrow
            | ManagedSqliteShmFailurePhase::MappingCreate
            | ManagedSqliteShmFailurePhase::ViewMap
            | ManagedSqliteShmFailurePhase::LockAcquire
            | ManagedSqliteShmFailurePhase::LockRelease
            | ManagedSqliteShmFailurePhase::Barrier
            | ManagedSqliteShmFailurePhase::ConnectionDetach
            | ManagedSqliteShmFailurePhase::ViewUnmap
            | ManagedSqliteShmFailurePhase::MappingClose
            | ManagedSqliteShmFailurePhase::DmsSharedRelease
            | ManagedSqliteShmFailurePhase::FileClose
            | ManagedSqliteShmFailurePhase::ExactSiblingDelete
    )
}

fn supports_after_success(phase: ManagedSqliteShmFailurePhase) -> bool {
    supported_shm_phase(phase) && phase != ManagedSqliteShmFailurePhase::FileSize
}

fn supports_after_success_class(
    phase: ManagedSqliteShmFailurePhase,
    class: ManagedSqliteShmFailureClass,
) -> bool {
    if phase == ManagedSqliteShmFailurePhase::Barrier {
        return class == ManagedSqliteShmFailureClass::OutcomeUncertainPoisoned;
    }
    matches!(
        class,
        ManagedSqliteShmFailureClass::MutatedButKnown
            | ManagedSqliteShmFailureClass::OutcomeUncertainPoisoned
    )
}

fn permanently_retain_probe(probe: ManagedSqliteShmTestFaultProbe) {
    let _permanent_probe = Box::leak(Box::new(probe));
}
