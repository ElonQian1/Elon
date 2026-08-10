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
use crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::registry::{
    ManagedSqliteRegistryCloseLifecycleFaults, ManagedSqliteRegistryCloseLifecyclePhase,
    ManagedSqliteRegistryRetirementReceipt,
};
use crate::node_agent_managed_fs::{
    ManagedSqliteMainCloseTestFaultPhase, ManagedSqliteMainCloseTestFaults,
};

const MAX_LIFECYCLE_FAULT_STEPS: usize = 32;

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
    installed: bool,
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
                installed: false,
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
            if step.timing == ManagedTestLifecycleFaultTiming::NativeFailure
                || steps[..index].iter().any(|prior| prior == &step)
            {
                return Err("invalid or duplicate lifecycle fault step");
            }
        }
        state.steps = steps.iter().copied().map(|step| (step, false)).collect();
        state.installed = true;
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

    fn publish_retirement(
        &self,
        route: ManagedTestRouteOrdinal,
        receipt: ManagedSqliteRegistryRetirementReceipt,
    ) -> Result<(), ()> {
        let mut state = match self.state.lock() {
            Ok(state) => state,
            Err(_) => {
                self.retain_terminal(receipt);
                return Err(());
            }
        };
        if state.retirements.contains_key(&route) {
            drop(state);
            self.retain_terminal(receipt);
            return Err(());
        }
        state.retirements.insert(route, receipt);
        Ok(())
    }

    pub(super) fn claim_retirement(
        &self,
        route: ManagedTestRouteOrdinal,
    ) -> Result<ManagedSqliteRegistryRetirementReceipt, ()> {
        let mut state = self.state.lock().map_err(|_| {
            self.terminal.store(true, Ordering::SeqCst);
        })?;
        state.retirements.remove(&route).ok_or_else(|| {
            self.terminal.store(true, Ordering::SeqCst);
        })
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

    pub(super) fn claim_retirement(&self) -> Result<ManagedSqliteRegistryRetirementReceipt, ()> {
        self.controller.claim_retirement(self.route)
    }

    pub(super) fn retain_terminal<Retained: 'static>(&self, retained: Retained) {
        self.controller.retain_terminal(retained);
    }
}

impl ManagedSqliteRegistryCloseLifecycleFaults for ManagedTestLifecycleFaultBinding {
    fn before(&self, phase: ManagedSqliteRegistryCloseLifecyclePhase) -> Result<bool, ()> {
        self.before(registry_phase(phase))
    }

    fn after_success(&self, phase: ManagedSqliteRegistryCloseLifecyclePhase) -> Result<bool, ()> {
        self.after_success(registry_phase(phase))
    }

    fn native_failure(&self, phase: ManagedSqliteRegistryCloseLifecyclePhase) {
        self.controller
            .native_failure(Some(self.route), registry_phase(phase));
    }

    fn publish_retirement(
        &self,
        receipt: ManagedSqliteRegistryRetirementReceipt,
    ) -> Result<(), ()> {
        self.controller.publish_retirement(self.route, receipt)
    }

    fn retain_retirement_failure(&self, receipt: ManagedSqliteRegistryRetirementReceipt) {
        self.retain_terminal(receipt);
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

fn registry_phase(
    phase: ManagedSqliteRegistryCloseLifecyclePhase,
) -> ManagedTestLifecycleFaultPhase {
    match phase {
        ManagedSqliteRegistryCloseLifecyclePhase::BarrierCallbackCompletion => {
            ManagedTestLifecycleFaultPhase::BarrierCallbackCompletion
        }
        ManagedSqliteRegistryCloseLifecyclePhase::RegistryWalMainClose => {
            ManagedTestLifecycleFaultPhase::RegistryWalMainClose
        }
        ManagedSqliteRegistryCloseLifecyclePhase::CallbackCompletion => {
            ManagedTestLifecycleFaultPhase::CallbackCompletion
        }
        ManagedSqliteRegistryCloseLifecyclePhase::ConnectionObservation => {
            ManagedTestLifecycleFaultPhase::ConnectionObservation
        }
        ManagedSqliteRegistryCloseLifecyclePhase::RouteRetirement => {
            ManagedTestLifecycleFaultPhase::RouteRetirement
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
