//! Route-bound controls and append-only evidence for RegistryLifecycle dynamic cases.

use std::collections::HashMap;
use std::sync::atomic::Ordering;

use super::*;
use crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::registry::{
    ManagedSqliteRegistryCloseLifecycleFaults, ManagedSqliteRegistryCloseLifecyclePhase,
    ManagedSqliteRegistryLifecycleStage,
    ManagedSqliteRegistryOrdinaryShmLockRoutePreemptionReceipt,
    ManagedSqliteRegistryUnsafeShmRoutePreemptionReceipt,
};
use crate::node_agent_managed_fs::PinnedManagedSqliteFile;

mod binding;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in super::super) enum ManagedTestRegistryLifecycleControl {
    RejectRetirementPublish,
    RejectRetirementClaim,
}

#[derive(Default)]
pub(super) struct ManagedTestRegistryLifecycleState {
    traces: HashMap<ManagedTestRouteOrdinal, Vec<ManagedSqliteRegistryLifecycleStage>>,
    controls: HashMap<ManagedTestRouteOrdinal, (ManagedTestRegistryLifecycleControl, bool)>,
    connection_observation_sidecars: HashMap<ManagedTestRouteOrdinal, PinnedManagedSqliteFile>,
    retained_registry_retirements: HashMap<ManagedTestRouteOrdinal, usize>,
    retained_logical_removals: HashMap<ManagedTestRouteOrdinal, usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in super::super) struct ManagedTestRegistryLifecycleTraceSnapshot {
    stages: Vec<ManagedSqliteRegistryLifecycleStage>,
    pending_controls: usize,
    retained_registry_retirements: usize,
    published_registry_retirements: usize,
    retained_logical_removals: usize,
}

impl ManagedTestRegistryLifecycleTraceSnapshot {
    pub(in super::super) fn stages(&self) -> &[ManagedSqliteRegistryLifecycleStage] {
        &self.stages
    }

    pub(in super::super) fn pending_controls(&self) -> usize {
        self.pending_controls
    }

    pub(in super::super) fn receipt_custody_count(&self) -> usize {
        self.retained_registry_retirements
            + self.published_registry_retirements
            + self.retained_logical_removals
    }

    pub(in super::super) fn retained_registry_retirement_count(&self) -> usize {
        self.retained_registry_retirements
    }

    pub(in super::super) fn published_registry_retirement_count(&self) -> usize {
        self.published_registry_retirements
    }

    pub(in super::super) fn retained_logical_removal_count(&self) -> usize {
        self.retained_logical_removals
    }

    pub(in super::super) fn count(&self, expected: ManagedSqliteRegistryLifecycleStage) -> usize {
        self.stages
            .iter()
            .filter(|stage| **stage == expected)
            .count()
    }
}

pub(super) const fn supports_native_failure(phase: ManagedTestLifecycleFaultPhase) -> bool {
    matches!(
        phase,
        ManagedTestLifecycleFaultPhase::BarrierCallbackCompletion
            | ManagedTestLifecycleFaultPhase::UnmapCallbackCompletion
            | ManagedTestLifecycleFaultPhase::CallbackCompletion
            | ManagedTestLifecycleFaultPhase::RouteRetirement
            | ManagedTestLifecycleFaultPhase::LogicalRouteRemoval
    )
}

impl ManagedTestRegistryLifecycleState {
    fn install(
        &mut self,
        route: ManagedTestRouteOrdinal,
        control: ManagedTestRegistryLifecycleControl,
    ) -> Result<(), &'static str> {
        if self.controls.contains_key(&route) {
            return Err("registry lifecycle control already installed for route");
        }
        self.controls.insert(route, (control, false));
        Ok(())
    }

    fn consume(
        &mut self,
        route: ManagedTestRouteOrdinal,
        expected: ManagedTestRegistryLifecycleControl,
    ) -> Result<bool, ()> {
        let Some((control, consumed)) = self.controls.get_mut(&route) else {
            return Ok(false);
        };
        if *control != expected {
            return Ok(false);
        }
        if *consumed {
            return Err(());
        }
        *consumed = true;
        Ok(true)
    }

    fn install_connection_observation_sidecar(
        &mut self,
        route: ManagedTestRouteOrdinal,
        file: PinnedManagedSqliteFile,
    ) -> Result<(), PinnedManagedSqliteFile> {
        if self.connection_observation_sidecars.contains_key(&route) {
            return Err(file);
        }
        self.connection_observation_sidecars.insert(route, file);
        Ok(())
    }

    fn take_connection_observation_sidecar(
        &mut self,
        route: ManagedTestRouteOrdinal,
    ) -> Option<PinnedManagedSqliteFile> {
        self.connection_observation_sidecars.remove(&route)
    }

    fn record(
        &mut self,
        route: ManagedTestRouteOrdinal,
        stage: ManagedSqliteRegistryLifecycleStage,
    ) -> Result<(), ()> {
        let trace = self.traces.entry(route).or_default();
        if trace.contains(&stage)
            || trace
                .last()
                .is_some_and(|previous| previous.order() >= stage.order())
        {
            return Err(());
        }
        trace.push(stage);
        Ok(())
    }

    fn record_retained_registry_retirement(
        &mut self,
        route: ManagedTestRouteOrdinal,
    ) -> Result<(), ()> {
        record_single_custody(&mut self.retained_registry_retirements, route)
    }

    fn record_retained_logical_removal(
        &mut self,
        route: ManagedTestRouteOrdinal,
    ) -> Result<(), ()> {
        record_single_custody(&mut self.retained_logical_removals, route)
    }

    fn snapshot(
        &self,
        route: ManagedTestRouteOrdinal,
        published_registry_retirements: usize,
    ) -> ManagedTestRegistryLifecycleTraceSnapshot {
        ManagedTestRegistryLifecycleTraceSnapshot {
            stages: self.traces.get(&route).cloned().unwrap_or_default(),
            pending_controls: usize::from(
                self.controls
                    .get(&route)
                    .is_some_and(|(_, consumed)| !*consumed),
            ) + usize::from(
                self.connection_observation_sidecars.contains_key(&route),
            ),
            retained_registry_retirements: self
                .retained_registry_retirements
                .get(&route)
                .copied()
                .unwrap_or(0),
            published_registry_retirements,
            retained_logical_removals: self
                .retained_logical_removals
                .get(&route)
                .copied()
                .unwrap_or(0),
        }
    }
}

fn record_single_custody(
    records: &mut HashMap<ManagedTestRouteOrdinal, usize>,
    route: ManagedTestRouteOrdinal,
) -> Result<(), ()> {
    let count = records.entry(route).or_default();
    *count = count.checked_add(1).ok_or(())?;
    if *count == 1 {
        Ok(())
    } else {
        Err(())
    }
}

impl ManagedTestLifecycleFaultController {
    pub(super) fn install_connection_observation_sidecar(
        &self,
        route: ManagedTestRouteOrdinal,
        file: PinnedManagedSqliteFile,
    ) -> Result<(), &'static str> {
        let mut state = match self.state.lock() {
            Ok(state) => state,
            Err(_) => {
                self.retain_terminal(file);
                return Err("lifecycle fault controller poisoned");
            }
        };
        match state
            .registry_lifecycle
            .install_connection_observation_sidecar(route, file)
        {
            Ok(()) => Ok(()),
            Err(file) => {
                drop(state);
                self.retain_terminal(file);
                Err("connection-observation sidecar already installed for route")
            }
        }
    }

    fn take_connection_observation_sidecar(
        &self,
        route: ManagedTestRouteOrdinal,
    ) -> Result<Option<PinnedManagedSqliteFile>, ()> {
        self.state
            .lock()
            .map_err(|_| {
                self.terminal.store(true, Ordering::SeqCst);
            })
            .map(|mut state| {
                state
                    .registry_lifecycle
                    .take_connection_observation_sidecar(route)
            })
    }

    pub(super) fn install_registry_control(
        &self,
        route: ManagedTestRouteOrdinal,
        control: ManagedTestRegistryLifecycleControl,
    ) -> Result<(), &'static str> {
        self.state
            .lock()
            .map_err(|_| "lifecycle fault controller poisoned")?
            .registry_lifecycle
            .install(route, control)
    }

    pub(super) fn observe_registry_stage(
        &self,
        route: ManagedTestRouteOrdinal,
        stage: ManagedSqliteRegistryLifecycleStage,
    ) -> Result<(), ()> {
        let recorded = self
            .state
            .lock()
            .map_err(|_| ())?
            .registry_lifecycle
            .record(route, stage);
        if recorded.is_err() {
            self.terminal.store(true, Ordering::SeqCst);
        }
        recorded
    }

    pub(in super::super) fn registry_trace(
        &self,
        route: ManagedTestRouteOrdinal,
    ) -> Result<ManagedTestRegistryLifecycleTraceSnapshot, &'static str> {
        self.state
            .lock()
            .map(|state| {
                state
                    .registry_lifecycle
                    .snapshot(route, usize::from(state.retirements.contains_key(&route)))
            })
            .map_err(|_| "lifecycle fault controller poisoned")
    }

    fn retain_registry_retirement(
        &self,
        route: ManagedTestRouteOrdinal,
        receipt: ManagedSqliteRegistryRetirementReceipt,
    ) {
        let recorded = self.state.lock().map_or(Err(()), |mut state| {
            state
                .registry_lifecycle
                .record_retained_registry_retirement(route)
        });
        if recorded.is_err() {
            self.terminal.store(true, Ordering::SeqCst);
        }
        let _retained = Box::leak(Box::new(receipt));
        self.terminal.store(true, Ordering::SeqCst);
    }

    fn retain_logical_removal(
        &self,
        route: ManagedTestRouteOrdinal,
        receipt: super::super::shared_namespace::ManagedTestLogicalRouteRemovalReceipt,
    ) {
        let recorded = self.state.lock().map_or(Err(()), |mut state| {
            state
                .registry_lifecycle
                .record_retained_logical_removal(route)
        });
        if recorded.is_err() {
            self.terminal.store(true, Ordering::SeqCst);
        }
        let _retained = Box::leak(Box::new(receipt));
        self.terminal.store(true, Ordering::SeqCst);
    }

    pub(super) fn publish_registry_retirement(
        &self,
        route: ManagedTestRouteOrdinal,
        receipt: ManagedSqliteRegistryRetirementReceipt,
    ) -> Result<(), ()> {
        let mut state = match self.state.lock() {
            Ok(state) => state,
            Err(_) => {
                self.retain_registry_retirement(route, receipt);
                return Err(());
            }
        };
        if state
            .registry_lifecycle
            .record(
                route,
                ManagedSqliteRegistryLifecycleStage::RetirementPublishAttempt,
            )
            .is_err()
            || state.retirements.contains_key(&route)
        {
            drop(state);
            self.retain_registry_retirement(route, receipt);
            return Err(());
        }
        match state.registry_lifecycle.consume(
            route,
            ManagedTestRegistryLifecycleControl::RejectRetirementPublish,
        ) {
            Ok(true) => {
                drop(state);
                self.retain_registry_retirement(route, receipt);
                return Err(());
            }
            Ok(false) => {}
            Err(()) => {
                drop(state);
                self.retain_registry_retirement(route, receipt);
                return Err(());
            }
        }
        state.retirements.insert(route, receipt);
        if state
            .registry_lifecycle
            .record(
                route,
                ManagedSqliteRegistryLifecycleStage::RetirementPublishSucceeded,
            )
            .is_err()
        {
            self.terminal.store(true, Ordering::SeqCst);
            return Err(());
        }
        Ok(())
    }

    pub(super) fn claim_registry_retirement(
        &self,
        route: ManagedTestRouteOrdinal,
    ) -> Result<ManagedSqliteRegistryRetirementReceipt, ()> {
        let mut state = self.state.lock().map_err(|_| {
            self.terminal.store(true, Ordering::SeqCst);
        })?;
        state
            .registry_lifecycle
            .record(
                route,
                ManagedSqliteRegistryLifecycleStage::RetirementClaimAttempt,
            )
            .map_err(|()| self.terminal.store(true, Ordering::SeqCst))?;
        match state.registry_lifecycle.consume(
            route,
            ManagedTestRegistryLifecycleControl::RejectRetirementClaim,
        ) {
            Ok(true) => {
                self.terminal.store(true, Ordering::SeqCst);
                return Err(());
            }
            Ok(false) => {}
            Err(()) => {
                self.terminal.store(true, Ordering::SeqCst);
                return Err(());
            }
        }
        let receipt = state.retirements.remove(&route).ok_or_else(|| {
            self.terminal.store(true, Ordering::SeqCst);
        })?;
        if state
            .registry_lifecycle
            .record(
                route,
                ManagedSqliteRegistryLifecycleStage::RetirementClaimSucceeded,
            )
            .is_err()
        {
            drop(state);
            self.retain_registry_retirement(route, receipt);
            return Err(());
        }
        Ok(receipt)
    }
}
