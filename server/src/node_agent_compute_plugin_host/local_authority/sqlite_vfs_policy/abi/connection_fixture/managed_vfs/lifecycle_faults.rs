//! Registration-scoped one-shot faults for close, route retirement and VFS shutdown.

use std::{
    collections::HashMap,
    num::NonZeroU32,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
};

use super::ManagedTestRouteOrdinal;
use crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::registry::ManagedSqliteRegistryRetirementReceipt;
use crate::node_agent_managed_fs::{
    ManagedSqliteMainCloseTestFaultPhase, ManagedSqliteMainCloseTestFaults,
};

const MAX_LIFECYCLE_FAULT_STEPS: usize = 32;

mod native_gate;
mod registry_lifecycle;
use registry_lifecycle::ManagedTestRegistryLifecycleState;
pub(super) use registry_lifecycle::{
    ManagedTestRegistryLifecycleControl, ManagedTestRegistryLifecycleTraceSnapshot,
};

#[cfg(all(test, windows))]
mod registration_shutdown;
#[cfg(all(test, windows))]
use registration_shutdown::ManagedTestRegistrationShutdownQuarantineState;
#[cfg(all(test, windows))]
pub(super) use registration_shutdown::{
    ManagedTestRegistrationShutdownQuarantineClaim,
    ManagedTestRegistrationShutdownQuarantineWitness,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) enum ManagedTestLifecycleFaultPhase {
    BarrierCallbackCompletion,
    MainUnlock,
    MainFileClose,
    RegistryWalMainClose,
    CallbackCompletion,
    ConnectionObservation,
    RouteRetirement,
    LogicalRouteRemoval,
    VfsUnregister,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ManagedTestLifecycleFaultTiming {
    BeforeCall,
    AfterSuccess,
    NativeFailure,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct ManagedTestLifecycleFaultStep {
    route: Option<ManagedTestRouteOrdinal>,
    phase: ManagedTestLifecycleFaultPhase,
    occurrence: NonZeroU32,
    timing: ManagedTestLifecycleFaultTiming,
}

impl ManagedTestLifecycleFaultStep {
    pub(super) fn route(
        route: ManagedTestRouteOrdinal,
        phase: ManagedTestLifecycleFaultPhase,
        occurrence: u32,
        timing: ManagedTestLifecycleFaultTiming,
    ) -> Result<Self, &'static str> {
        if phase == ManagedTestLifecycleFaultPhase::VfsUnregister {
            return Err("VFS unregister is registration-scoped");
        }
        Ok(Self {
            route: Some(route),
            phase,
            occurrence: NonZeroU32::new(occurrence)
                .ok_or("lifecycle fault occurrence must be non-zero")?,
            timing,
        })
    }

    pub(super) fn registration(
        phase: ManagedTestLifecycleFaultPhase,
        occurrence: u32,
        timing: ManagedTestLifecycleFaultTiming,
    ) -> Result<Self, &'static str> {
        if phase != ManagedTestLifecycleFaultPhase::VfsUnregister {
            return Err("only VFS unregister is registration-scoped");
        }
        Ok(Self {
            route: None,
            phase,
            occurrence: NonZeroU32::new(occurrence)
                .ok_or("lifecycle fault occurrence must be non-zero")?,
            timing,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct ManagedTestLifecycleFaultObservation {
    pub(super) route: Option<ManagedTestRouteOrdinal>,
    pub(super) phase: ManagedTestLifecycleFaultPhase,
    pub(super) occurrence: u32,
    pub(super) timing: ManagedTestLifecycleFaultTiming,
    pub(super) triggered: bool,
}

struct ManagedTestLifecycleFaultState {
    steps: Vec<(ManagedTestLifecycleFaultStep, bool)>,
    occurrences: HashMap<
        (
            Option<ManagedTestRouteOrdinal>,
            ManagedTestLifecycleFaultPhase,
        ),
        u32,
    >,
    observations: Vec<ManagedTestLifecycleFaultObservation>,
    retirements: HashMap<ManagedTestRouteOrdinal, ManagedSqliteRegistryRetirementReceipt>,
    registry_lifecycle: ManagedTestRegistryLifecycleState,
    installed: bool,
    #[cfg(all(test, windows))]
    registration_shutdown_quarantine: ManagedTestRegistrationShutdownQuarantineState,
}

pub(super) struct ManagedTestLifecycleFaultController {
    state: Mutex<ManagedTestLifecycleFaultState>,
    terminal: AtomicBool,
}

impl ManagedTestLifecycleFaultController {
    pub(super) fn new() -> Arc<Self> {
        Arc::new(Self {
            state: Mutex::new(ManagedTestLifecycleFaultState {
                steps: Vec::new(),
                occurrences: HashMap::new(),
                observations: Vec::new(),
                retirements: HashMap::new(),
                registry_lifecycle: ManagedTestRegistryLifecycleState::default(),
                installed: false,
                #[cfg(all(test, windows))]
                registration_shutdown_quarantine:
                    ManagedTestRegistrationShutdownQuarantineState::Vacant,
            }),
            terminal: AtomicBool::new(false),
        })
    }

    pub(super) fn install(
        &self,
        steps: &[ManagedTestLifecycleFaultStep],
    ) -> Result<(), &'static str> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| "lifecycle fault controller poisoned")?;
        if state.installed || steps.is_empty() || steps.len() > MAX_LIFECYCLE_FAULT_STEPS {
            return Err("lifecycle fault script must be a single non-empty 1..=32 installation");
        }
        for (index, step) in steps.iter().copied().enumerate() {
            if (step.timing == ManagedTestLifecycleFaultTiming::NativeFailure
                && (step.route.is_none()
                    || !registry_lifecycle::supports_native_failure(step.phase)
                    || step.occurrence.get() != 1))
                || steps[..index].iter().any(|prior| prior == &step)
            {
                return Err("invalid or duplicate lifecycle fault step");
            }
        }
        for step in steps {
            let key = (step.route, step.phase);
            state.occurrences.insert(key, 0);
        }
        state.steps = steps.iter().copied().map(|step| (step, false)).collect();
        state.installed = true;
        Ok(())
    }

    #[cfg(all(test, windows))]
    pub(super) fn begin_unfaulted_barrier_observation_window(
        &self,
        route: ManagedTestRouteOrdinal,
    ) -> Result<(), &'static str> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| "lifecycle fault controller poisoned")?;
        if state.installed {
            return Err("unfaulted Barrier observation window cannot overlap a fault script");
        }
        state.occurrences.insert(
            (
                Some(route),
                ManagedTestLifecycleFaultPhase::BarrierCallbackCompletion,
            ),
            0,
        );
        Ok(())
    }

    pub(super) fn binding(
        self: &Arc<Self>,
        route: ManagedTestRouteOrdinal,
    ) -> ManagedTestLifecycleFaultBinding {
        ManagedTestLifecycleFaultBinding {
            controller: Arc::clone(self),
            route,
        }
    }

    fn before(
        &self,
        route: Option<ManagedTestRouteOrdinal>,
        phase: ManagedTestLifecycleFaultPhase,
    ) -> Result<bool, ()> {
        let mut state = self.state.lock().map_err(|_| {
            self.terminal.store(true, Ordering::SeqCst);
        })?;
        let key = (route, phase);
        let occurrence = state.occurrences.entry(key).or_insert(0);
        *occurrence = occurrence.checked_add(1).ok_or_else(|| {
            self.terminal.store(true, Ordering::SeqCst);
        })?;
        let occurrence = *occurrence;
        Ok(record(
            &mut state,
            key,
            occurrence,
            ManagedTestLifecycleFaultTiming::BeforeCall,
        ))
    }

    fn after_success(
        &self,
        route: Option<ManagedTestRouteOrdinal>,
        phase: ManagedTestLifecycleFaultPhase,
    ) -> Result<bool, ()> {
        let mut state = self.state.lock().map_err(|_| {
            self.terminal.store(true, Ordering::SeqCst);
        })?;
        let key = (route, phase);
        let occurrence = state.occurrences.get(&key).copied().ok_or_else(|| {
            self.terminal.store(true, Ordering::SeqCst);
        })?;
        Ok(record(
            &mut state,
            key,
            occurrence,
            ManagedTestLifecycleFaultTiming::AfterSuccess,
        ))
    }

    pub(super) fn native_failure(
        &self,
        route: Option<ManagedTestRouteOrdinal>,
        phase: ManagedTestLifecycleFaultPhase,
    ) {
        let occurrence = self.state.lock().ok().and_then(|mut state| {
            let occurrence = state.occurrences.get(&(route, phase)).copied()?;
            state
                .observations
                .push(ManagedTestLifecycleFaultObservation {
                    route,
                    phase,
                    occurrence,
                    timing: ManagedTestLifecycleFaultTiming::NativeFailure,
                    triggered: false,
                });
            Some(occurrence)
        });
        if occurrence.is_none() {
            self.terminal.store(true, Ordering::SeqCst);
        }
    }

    pub(super) fn before_registration(
        &self,
        phase: ManagedTestLifecycleFaultPhase,
    ) -> Result<bool, ()> {
        self.before(None, phase)
    }

    pub(super) fn after_registration_success(
        &self,
        phase: ManagedTestLifecycleFaultPhase,
    ) -> Result<bool, ()> {
        self.after_success(None, phase)
    }

    pub(super) fn retain_terminal<Retained: 'static>(&self, retained: Retained) {
        let _retained = Box::leak(Box::new(retained));
        self.terminal.store(true, Ordering::SeqCst);
    }

    pub(super) fn is_terminal(&self) -> bool {
        self.terminal.load(Ordering::SeqCst)
    }

    pub(super) fn pending_count(&self) -> Result<usize, &'static str> {
        let state = self
            .state
            .lock()
            .map_err(|_| "lifecycle fault controller poisoned")?;
        Ok(state.steps.iter().filter(|(_, consumed)| !consumed).count())
    }

    pub(super) fn observations(
        &self,
    ) -> Result<Vec<ManagedTestLifecycleFaultObservation>, &'static str> {
        self.state
            .lock()
            .map(|state| state.observations.clone())
            .map_err(|_| "lifecycle fault controller poisoned")
    }
}

#[derive(Clone)]
pub(super) struct ManagedTestLifecycleFaultBinding {
    controller: Arc<ManagedTestLifecycleFaultController>,
    route: ManagedTestRouteOrdinal,
}

impl ManagedTestLifecycleFaultBinding {
    pub(super) fn before(&self, phase: ManagedTestLifecycleFaultPhase) -> Result<bool, ()> {
        self.controller.before(Some(self.route), phase)
    }

    pub(super) fn after_success(&self, phase: ManagedTestLifecycleFaultPhase) -> Result<bool, ()> {
        self.controller.after_success(Some(self.route), phase)
    }

    pub(super) fn native_failure(&self, phase: ManagedTestLifecycleFaultPhase) {
        self.controller.native_failure(Some(self.route), phase);
    }

    pub(super) fn claim_native_failure_gate(
        &self,
        phase: ManagedTestLifecycleFaultPhase,
    ) -> Result<bool, ()> {
        self.controller.claim_native_failure_gate(self.route, phase)
    }

    pub(super) fn claim_retirement(&self) -> Result<ManagedSqliteRegistryRetirementReceipt, ()> {
        self.controller.claim_registry_retirement(self.route)
    }

    pub(super) fn retain_terminal<Retained: 'static>(&self, retained: Retained) {
        self.controller.retain_terminal(retained);
    }
}

impl ManagedSqliteMainCloseTestFaults for ManagedTestLifecycleFaultBinding {
    fn before(&self, phase: ManagedSqliteMainCloseTestFaultPhase) -> Result<bool, ()> {
        self.before(main_close_phase(phase))
    }

    fn after_success(&self, phase: ManagedSqliteMainCloseTestFaultPhase) -> Result<bool, ()> {
        self.after_success(main_close_phase(phase))
    }

    fn native_failure(&self, phase: ManagedSqliteMainCloseTestFaultPhase) {
        self.controller
            .native_failure(Some(self.route), main_close_phase(phase));
    }
}

fn main_close_phase(phase: ManagedSqliteMainCloseTestFaultPhase) -> ManagedTestLifecycleFaultPhase {
    match phase {
        ManagedSqliteMainCloseTestFaultPhase::Unlock => ManagedTestLifecycleFaultPhase::MainUnlock,
        ManagedSqliteMainCloseTestFaultPhase::FileClose => {
            ManagedTestLifecycleFaultPhase::MainFileClose
        }
    }
}

fn record(
    state: &mut ManagedTestLifecycleFaultState,
    key: (
        Option<ManagedTestRouteOrdinal>,
        ManagedTestLifecycleFaultPhase,
    ),
    occurrence: u32,
    timing: ManagedTestLifecycleFaultTiming,
) -> bool {
    let triggered = state
        .steps
        .iter_mut()
        .find_map(|(step, consumed)| {
            if !*consumed
                && step.route == key.0
                && step.phase == key.1
                && step.occurrence.get() == occurrence
                && step.timing == timing
            {
                *consumed = true;
                Some(true)
            } else {
                None
            }
        })
        .unwrap_or(false);
    state
        .observations
        .push(ManagedTestLifecycleFaultObservation {
            route: key.0,
            phase: key.1,
            occurrence,
            timing,
            triggered,
        });
    triggered
}

#[cfg(test)]
mod tests {
    use super::*;

    fn native_step(route: ManagedTestRouteOrdinal) -> ManagedTestLifecycleFaultStep {
        ManagedTestLifecycleFaultStep::route(
            route,
            ManagedTestLifecycleFaultPhase::BarrierCallbackCompletion,
            1,
            ManagedTestLifecycleFaultTiming::NativeFailure,
        )
        .expect("exact barrier native-failure step")
    }

    #[test]
    fn exact_barrier_native_failure_gate_is_linear_and_observed_only_after_rejection() {
        let route = ManagedTestRouteOrdinal::test_value(1);
        let controller = ManagedTestLifecycleFaultController::new();
        controller.install(&[native_step(route)]).expect("install");
        let binding = controller.binding(route);

        assert!(!binding
            .before(ManagedTestLifecycleFaultPhase::BarrierCallbackCompletion)
            .expect("record before"));
        assert!(binding
            .claim_native_failure_gate(ManagedTestLifecycleFaultPhase::BarrierCallbackCompletion)
            .expect("claim exact native gate"));
        assert_eq!(controller.pending_count().expect("pending count"), 0);
        assert_eq!(controller.observations().expect("observations").len(), 1);

        binding.native_failure(ManagedTestLifecycleFaultPhase::BarrierCallbackCompletion);
        assert_eq!(
            controller.observations().expect("observations"),
            vec![
                ManagedTestLifecycleFaultObservation {
                    route: Some(route),
                    phase: ManagedTestLifecycleFaultPhase::BarrierCallbackCompletion,
                    occurrence: 1,
                    timing: ManagedTestLifecycleFaultTiming::BeforeCall,
                    triggered: false,
                },
                ManagedTestLifecycleFaultObservation {
                    route: Some(route),
                    phase: ManagedTestLifecycleFaultPhase::BarrierCallbackCompletion,
                    occurrence: 1,
                    timing: ManagedTestLifecycleFaultTiming::NativeFailure,
                    triggered: false,
                },
            ]
        );
        assert!(!controller.is_terminal());
    }

    #[test]
    fn late_install_uses_key_relative_occurrence_without_erasing_baseline() {
        let route = ManagedTestRouteOrdinal::test_value(1);
        let controller = ManagedTestLifecycleFaultController::new();
        let binding = controller.binding(route);
        assert!(!binding
            .before(ManagedTestLifecycleFaultPhase::BarrierCallbackCompletion)
            .expect("record fixture baseline"));
        let baseline = controller.observations().expect("baseline observations");
        controller
            .install(&[native_step(route)])
            .expect("late install");

        assert!(!binding
            .before(ManagedTestLifecycleFaultPhase::BarrierCallbackCompletion)
            .expect("record first relative occurrence"));
        assert!(binding
            .claim_native_failure_gate(ManagedTestLifecycleFaultPhase::BarrierCallbackCompletion)
            .expect("claim first relative occurrence"));
        binding.native_failure(ManagedTestLifecycleFaultPhase::BarrierCallbackCompletion);

        let observations = controller.observations().expect("observations");
        assert_eq!(
            observations
                .strip_prefix(baseline.as_slice())
                .map(|suffix| suffix.len()),
            Some(2)
        );
        assert_eq!(observations[1].occurrence, 1);
        assert_eq!(
            observations[1].timing,
            ManagedTestLifecycleFaultTiming::BeforeCall
        );
        assert_eq!(observations[2].occurrence, 1);
        assert_eq!(
            observations[2].timing,
            ManagedTestLifecycleFaultTiming::NativeFailure
        );
        assert_eq!(controller.pending_count().expect("pending count"), 0);
        assert!(!controller.is_terminal());
    }

    #[test]
    fn unfaulted_barrier_window_is_key_relative_and_preserves_baseline() {
        let route = ManagedTestRouteOrdinal::test_value(1);
        let controller = ManagedTestLifecycleFaultController::new();
        let binding = controller.binding(route);
        assert!(!binding
            .before(ManagedTestLifecycleFaultPhase::BarrierCallbackCompletion)
            .expect("record fixture baseline"));
        assert!(!binding
            .after_success(ManagedTestLifecycleFaultPhase::BarrierCallbackCompletion)
            .expect("complete fixture baseline"));
        let baseline = controller.observations().expect("baseline observations");

        controller
            .begin_unfaulted_barrier_observation_window(route)
            .expect("begin exact Barrier observation window");
        assert!(!binding
            .before(ManagedTestLifecycleFaultPhase::BarrierCallbackCompletion)
            .expect("record relative before"));
        assert!(!binding
            .after_success(ManagedTestLifecycleFaultPhase::BarrierCallbackCompletion)
            .expect("record relative after"));

        let observations = controller.observations().expect("observations");
        assert_eq!(observations[..baseline.len()], baseline);
        assert_eq!(observations[baseline.len()].occurrence, 1);
        assert_eq!(observations[baseline.len() + 1].occurrence, 1);
        assert_eq!(controller.pending_count().expect("pending count"), 0);
    }

    #[test]
    fn barrier_native_failure_gate_rejects_out_of_order_claim() {
        let route = ManagedTestRouteOrdinal::test_value(1);
        let controller = ManagedTestLifecycleFaultController::new();
        controller.install(&[native_step(route)]).expect("install");

        assert!(controller
            .binding(route)
            .claim_native_failure_gate(ManagedTestLifecycleFaultPhase::BarrierCallbackCompletion)
            .is_err());
        assert!(controller.is_terminal());
        assert_eq!(controller.pending_count().expect("pending count"), 1);
    }

    #[test]
    fn barrier_native_failure_gate_ignores_other_route() {
        let route = ManagedTestRouteOrdinal::test_value(1);
        let sibling = ManagedTestRouteOrdinal::test_value(2);
        let controller = ManagedTestLifecycleFaultController::new();
        controller.install(&[native_step(route)]).expect("install");
        let sibling_binding = controller.binding(sibling);
        assert!(!sibling_binding
            .before(ManagedTestLifecycleFaultPhase::BarrierCallbackCompletion)
            .expect("record sibling before"));

        assert!(!sibling_binding
            .claim_native_failure_gate(ManagedTestLifecycleFaultPhase::BarrierCallbackCompletion)
            .expect("ignore another route's native gate"));
        assert!(!controller.is_terminal());
        assert_eq!(controller.pending_count().expect("pending count"), 1);

        let exact_binding = controller.binding(route);
        assert!(!exact_binding
            .before(ManagedTestLifecycleFaultPhase::BarrierCallbackCompletion)
            .expect("record exact route before"));
        assert!(exact_binding
            .claim_native_failure_gate(ManagedTestLifecycleFaultPhase::BarrierCallbackCompletion)
            .expect("claim exact route native gate"));
        assert!(!controller.is_terminal());
        assert_eq!(controller.pending_count().expect("pending count"), 0);
    }

    #[test]
    fn barrier_native_failure_gate_rejects_double_claim() {
        let route = ManagedTestRouteOrdinal::test_value(1);
        let controller = ManagedTestLifecycleFaultController::new();
        controller.install(&[native_step(route)]).expect("install");
        let binding = controller.binding(route);
        assert!(!binding
            .before(ManagedTestLifecycleFaultPhase::BarrierCallbackCompletion)
            .expect("record before"));
        assert!(binding
            .claim_native_failure_gate(ManagedTestLifecycleFaultPhase::BarrierCallbackCompletion)
            .expect("first claim"));

        assert!(binding
            .claim_native_failure_gate(ManagedTestLifecycleFaultPhase::BarrierCallbackCompletion)
            .is_err());
        assert!(controller.is_terminal());
        assert_eq!(controller.pending_count().expect("pending count"), 0);
    }
}
