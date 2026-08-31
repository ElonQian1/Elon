#[cfg(all(test, windows))]
use std::fmt;
use std::sync::Arc;

use super::{
    super::{
        coordinator::{ManagedSqliteShmCoordinator, PinnedManagedSqliteWalMainFile},
        types::{
            ManagedSqliteShmFailure, ManagedSqliteShmFailureClass, ManagedSqliteShmFailurePhase,
        },
    },
    controller::{ManagedSqliteShmMatchedTestFault, ManagedSqliteShmTestFaultTarget},
};

#[cfg(all(test, windows))]
use super::super::test_lock_runtime::{
    ManagedSqliteShmTestLockExpectation, ManagedSqliteShmTestLockReceipt,
};
#[cfg(all(test, windows))]
use super::super::test_map_runtime::{
    ManagedSqliteShmTestMapExpectation, ManagedSqliteShmTestMapReceipt,
};
#[cfg(all(test, windows))]
use super::super::test_snapshot::{
    test_target_snapshot as snapshot_test_target, ManagedSqliteShmTestTargetSnapshot,
};
#[cfg(all(test, windows))]
use super::super::test_unmap_runtime::{
    ManagedSqliteShmTestUnmapDeletePrestate, ManagedSqliteShmTestUnmapNativeOperation,
    ManagedSqliteShmTestUnmapReceipt,
};

const CONTROLLER_POISONED: &str = "NODE_MANAGED_SQLITE_SHM_TEST_FAULT_CONTROLLER_POISONED";

/// Move-only observation authority for one exact installed SHM fault script.
///
/// The exact runtime generation and connection target remain private to this module.
pub(crate) struct ManagedSqliteShmTestFaultProbe {
    coordinator: Arc<ManagedSqliteShmCoordinator>,
    target: ManagedSqliteShmTestFaultTarget,
}

/// Cloneable, redacted read authority for the exact target of an installed test fault script.
///
/// Keeping this value alive retains only the sealed coordinator and private target identity. Its
/// public observation contains no runtime generation, connection id, handle, pointer or path.
#[cfg(all(test, windows))]
#[derive(Clone)]
#[must_use = "the exact-target observer must be retained for post-close observation"]
pub(crate) struct ManagedSqliteShmTestTargetObserver {
    pub(super) coordinator: Arc<ManagedSqliteShmCoordinator>,
    pub(super) target: ManagedSqliteShmTestFaultTarget,
}

/// Numeric identity observed from the exact live physical target. This copy-only value is
/// available solely to the process-isolated Windows evidence runner and carries no custody.
#[cfg(all(test, windows))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ManagedSqliteShmTestTargetIdentity {
    pub(crate) runtime_generation: u64,
    pub(crate) shm_connection_id: u64,
}

#[cfg(all(test, windows))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ManagedSqliteShmTriggeredTestFaultObservation {
    pub(crate) before_call: bool,
    pub(crate) class: ManagedSqliteShmFailureClass,
}

impl ManagedSqliteShmTestFaultProbe {
    pub(crate) fn pending_count(&self) -> Result<usize, &'static str> {
        match self.coordinator.test_faults.lock() {
            Ok(faults) => Ok(faults.pending_count(self.target)),
            Err(poison) => {
                drop(poison.into_inner());
                self.coordinator
                    .poison_test_fault_controller_from_external_access();
                Err(CONTROLLER_POISONED)
            }
        }
    }

    pub(crate) fn was_triggered(
        &self,
        phase: ManagedSqliteShmFailurePhase,
        ordinal: u32,
    ) -> Result<bool, &'static str> {
        match self.coordinator.test_faults.lock() {
            Ok(faults) => Ok(faults.was_triggered(self.target, phase, ordinal)),
            Err(poison) => {
                drop(poison.into_inner());
                self.coordinator
                    .poison_test_fault_controller_from_external_access();
                Err(CONTROLLER_POISONED)
            }
        }
    }

    /// Reports whether the exact phase occurrence was reached, even when an after-success token
    /// intentionally remains pending because the native outcome was NotFound rather than Deleted.
    pub(crate) fn was_observed(
        &self,
        phase: ManagedSqliteShmFailurePhase,
        ordinal: u32,
    ) -> Result<bool, &'static str> {
        match self.coordinator.test_faults.lock() {
            Ok(faults) => Ok(faults.was_observed(self.target, phase, ordinal)),
            Err(poison) => {
                drop(poison.into_inner());
                self.coordinator
                    .poison_test_fault_controller_from_external_access();
                Err(CONTROLLER_POISONED)
            }
        }
    }

    #[cfg(all(test, windows))]
    pub(crate) fn triggered_observation(
        &self,
        phase: ManagedSqliteShmFailurePhase,
        ordinal: u32,
    ) -> Result<Option<ManagedSqliteShmTriggeredTestFaultObservation>, &'static str> {
        match self.coordinator.test_faults.lock() {
            Ok(faults) => Ok(faults
                .triggered_timing_and_class(self.target, phase, ordinal)
                .map(
                    |(before_call, class)| ManagedSqliteShmTriggeredTestFaultObservation {
                        before_call,
                        class,
                    },
                )),
            Err(poison) => {
                drop(poison.into_inner());
                self.coordinator
                    .poison_test_fault_controller_from_external_access();
                Err(CONTROLLER_POISONED)
            }
        }
    }

    #[cfg(all(test, windows))]
    pub(crate) fn test_target_snapshot(
        &self,
    ) -> Result<ManagedSqliteShmTestTargetSnapshot, ManagedSqliteShmFailure> {
        self.observer().snapshot()
    }

    /// The installed exact fault probe is the only constructor for target observation authority.
    #[cfg(all(test, windows))]
    pub(crate) fn observer(&self) -> ManagedSqliteShmTestTargetObserver {
        ManagedSqliteShmTestTargetObserver {
            coordinator: Arc::clone(&self.coordinator),
            target: self.target,
        }
    }
}

#[cfg(all(test, windows))]
impl ManagedSqliteShmTestTargetObserver {
    /// Observes coordinator state first and the domain registry second; this is intentionally a
    /// sequential diagnostic observation, not an atomic snapshot across those authorities.
    pub(crate) fn snapshot(
        &self,
    ) -> Result<ManagedSqliteShmTestTargetSnapshot, ManagedSqliteShmFailure> {
        snapshot_test_target(&self.coordinator, |connection_id| {
            self.coordinator.test_fault_target(connection_id) == self.target
        })
    }

    pub(crate) fn identity(&self) -> ManagedSqliteShmTestTargetIdentity {
        let (runtime_generation, shm_connection_id) = self.target.identity();
        ManagedSqliteShmTestTargetIdentity {
            runtime_generation,
            shm_connection_id,
        }
    }

    /// Arms one exact managed Lock action after any setup transitions are complete.
    pub(crate) fn begin_lock_action_observation(
        &self,
        expectation: ManagedSqliteShmTestLockExpectation,
    ) -> Result<(), &'static str> {
        let snapshot = self
            .snapshot()
            .map_err(|_| "NODE_MANAGED_SQLITE_SHM_TEST_LOCK_TARGET_SNAPSHOT_FAILED")?;
        if !snapshot.target_attached || snapshot.topology.shm_connections == 0 {
            return Err("NODE_MANAGED_SQLITE_SHM_TEST_LOCK_TARGET_NOT_ATTACHED");
        }
        let mut runtime = self
            .coordinator
            .test_lock_runtime
            .lock()
            .map_err(|_| "NODE_MANAGED_SQLITE_SHM_TEST_LOCK_RUNTIME_POISONED")?;
        runtime.arm(self.target.identity(), expectation)
    }

    /// Seals and disarms the exact-target Lock ledger so later fixture cleanup is not observed.
    pub(crate) fn finish_lock_action_observation(
        &self,
    ) -> Result<ManagedSqliteShmTestLockReceipt, &'static str> {
        self.coordinator
            .test_lock_runtime
            .lock()
            .map_err(|_| "NODE_MANAGED_SQLITE_SHM_TEST_LOCK_RUNTIME_POISONED")?
            .finish(self.target.identity())
    }

    /// Seals an armed Lock ledger only when stored-poison admission returned before every managed,
    /// native and local Lock event.
    pub(crate) fn finish_stored_poison_lock_observation(
        &self,
    ) -> Result<ManagedSqliteShmTestLockReceipt, &'static str> {
        self.coordinator
            .test_lock_runtime
            .lock()
            .map_err(|_| "NODE_MANAGED_SQLITE_SHM_TEST_LOCK_RUNTIME_POISONED")?
            .finish_stored_poison_without_attempt(self.target.identity())
    }

    /// Disarms an unfinished exact-target Lock ledger before fixture cleanup or unwind proceeds.
    pub(crate) fn cancel_lock_action_observation(&self) -> Result<(), &'static str> {
        self.coordinator
            .test_lock_runtime
            .lock()
            .map_err(|_| "NODE_MANAGED_SQLITE_SHM_TEST_LOCK_RUNTIME_POISONED")?
            .cancel(self.target.identity())
    }

    /// Arms one exact managed Map action after all setup Map calls are complete.
    pub(crate) fn begin_map_action_observation(
        &self,
        expectation: ManagedSqliteShmTestMapExpectation,
    ) -> Result<(), &'static str> {
        let snapshot = self
            .snapshot()
            .map_err(|_| "NODE_MANAGED_SQLITE_SHM_TEST_MAP_TARGET_SNAPSHOT_FAILED")?;
        if !snapshot.target_attached || snapshot.topology.shm_connections == 0 {
            return Err("NODE_MANAGED_SQLITE_SHM_TEST_MAP_TARGET_NOT_ATTACHED");
        }
        self.coordinator
            .test_map_runtime
            .lock()
            .map_err(|_| "NODE_MANAGED_SQLITE_SHM_TEST_MAP_RUNTIME_POISONED")?
            .arm(self.target.identity(), expectation)
    }

    /// Seals and disarms the one-shot Map ledger before later fixture cleanup can be observed.
    pub(crate) fn finish_map_action_observation(
        &self,
    ) -> Result<ManagedSqliteShmTestMapReceipt, &'static str> {
        self.coordinator
            .test_map_runtime
            .lock()
            .map_err(|_| "NODE_MANAGED_SQLITE_SHM_TEST_MAP_RUNTIME_POISONED")?
            .finish(self.target.identity())
    }

    /// Disarms only this exact target's unfinished Map ledger during unwind.
    pub(crate) fn cancel_map_action_observation(&self) -> Result<(), &'static str> {
        self.coordinator
            .test_map_runtime
            .lock()
            .map_err(|_| "NODE_MANAGED_SQLITE_SHM_TEST_MAP_RUNTIME_POISONED")?
            .cancel(self.target.identity())
    }

    /// Starts one append-only Unmap action observation for this exact generation/connection.
    pub(crate) fn begin_unmap_action_observation(&self) -> Result<(), &'static str> {
        let snapshot = self
            .snapshot()
            .map_err(|_| "NODE_MANAGED_SQLITE_SHM_TEST_UNMAP_TARGET_SNAPSHOT_FAILED")?;
        if !snapshot.target_attached || snapshot.topology.shm_connections == 0 {
            return Err("NODE_MANAGED_SQLITE_SHM_TEST_UNMAP_TARGET_NOT_ATTACHED");
        }
        let mut runtime = self
            .coordinator
            .test_unmap_runtime
            .lock()
            .map_err(|_| CONTROLLER_POISONED)?;
        runtime.begin(self.target.identity())
    }

    /// Installs one one-shot native adapter. The adapter can trigger only at its declared phase
    /// and only while this exact target executes final-connection Unmap.
    pub(crate) fn install_unmap_native_operation(
        &self,
        operation: ManagedSqliteShmTestUnmapNativeOperation,
    ) -> Result<(), &'static str> {
        let mut runtime = self
            .coordinator
            .test_unmap_runtime
            .lock()
            .map_err(|_| CONTROLLER_POISONED)?;
        runtime.install_native(self.target.identity(), operation)
    }

    /// Installs one exact delete-authority/filesystem prestate control.
    pub(crate) fn set_unmap_delete_prestate(
        &self,
        prestate: ManagedSqliteShmTestUnmapDeletePrestate,
    ) -> Result<(), &'static str> {
        let mut runtime = self
            .coordinator
            .test_unmap_runtime
            .lock()
            .map_err(|_| CONTROLLER_POISONED)?;
        runtime.install_prestate(self.target.identity(), prestate)
    }

    pub(crate) fn observe_unmap_test_receipt(
        &self,
    ) -> Result<ManagedSqliteShmTestUnmapReceipt, &'static str> {
        self.coordinator
            .test_unmap_runtime
            .lock()
            .map_err(|_| CONTROLLER_POISONED)?
            .receipt(self.target.identity())
    }

    /// Seals the action ledger after the one raw Unmap call. The returned receipt still reports
    /// any unconsumed native or prestate adapter so the harness cannot mistake it for evidence.
    pub(crate) fn finish_unmap_test_receipt(
        &self,
    ) -> Result<ManagedSqliteShmTestUnmapReceipt, &'static str> {
        self.coordinator
            .test_unmap_runtime
            .lock()
            .map_err(|_| CONTROLLER_POISONED)?
            .finish(self.target.identity())
    }

    /// Installs one script only after a fixture has observed this exact live physical target.
    /// This authority is sealed to Windows tests and cannot redirect to a sibling connection.
    pub(crate) fn install_test_fault_script(
        &self,
        before_call: &[(ManagedSqliteShmFailurePhase, u32)],
        after_success: &[(
            ManagedSqliteShmFailurePhase,
            u32,
            ManagedSqliteShmFailureClass,
        )],
    ) -> Result<ManagedSqliteShmTestFaultProbe, &'static str> {
        install_exact_test_fault_script(
            Arc::clone(&self.coordinator),
            self.target,
            before_call,
            after_success,
        )
    }
}

#[cfg(all(test, windows))]
impl fmt::Debug for ManagedSqliteShmTestTargetObserver {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ManagedSqliteShmTestTargetObserver")
            .field("authority", &"<redacted>")
            .finish()
    }
}

impl ManagedSqliteShmCoordinator {
    pub(in crate::node_agent_managed_fs::sqlite_namespace::shm) fn observe_test_fault(
        &self,
        connection_id: u64,
        phase: ManagedSqliteShmFailurePhase,
        known_mutation: bool,
    ) -> Result<Option<ManagedSqliteShmMatchedTestFault>, ManagedSqliteShmFailure> {
        #[cfg(all(test, windows))]
        self.record_test_map_dms_phase(connection_id, phase, known_mutation)?;
        let target = self.test_fault_target(connection_id);
        let mut faults = self
            .test_faults
            .lock()
            .map_err(|_| self.test_fault_internal_failure(phase, known_mutation))?;
        faults
            .observe(target, phase)
            .map_err(|_| self.test_fault_internal_failure(phase, known_mutation))
    }

    pub(in crate::node_agent_managed_fs::sqlite_namespace::shm) fn trigger_before_test_fault(
        &self,
        matched: ManagedSqliteShmMatchedTestFault,
        prior_mutation: bool,
    ) -> Result<ManagedSqliteShmFailure, ManagedSqliteShmFailure> {
        let phase = matched.phase();
        let mut faults = self
            .test_faults
            .lock()
            .map_err(|_| self.test_fault_internal_failure(phase, prior_mutation))?;
        let triggered = faults
            .activate_before(matched)
            .map_err(|_| self.test_fault_internal_failure(phase, prior_mutation))?;
        Ok(triggered.into_before_failure(prior_mutation))
    }

    pub(in crate::node_agent_managed_fs::sqlite_namespace::shm) fn trigger_after_test_fault(
        &self,
        matched: ManagedSqliteShmMatchedTestFault,
        known_mutation: bool,
    ) -> Result<ManagedSqliteShmFailure, ManagedSqliteShmFailure> {
        let phase = matched.phase();
        let mut faults = self
            .test_faults
            .lock()
            .map_err(|_| self.test_fault_internal_failure(phase, known_mutation))?;
        let triggered = faults
            .activate_after(matched, known_mutation)
            .map_err(|_| self.test_fault_internal_failure(phase, known_mutation))?;
        Ok(triggered.into_after_failure(known_mutation))
    }

    pub(super) fn test_fault_target(&self, connection_id: u64) -> ManagedSqliteShmTestFaultTarget {
        ManagedSqliteShmTestFaultTarget::new(self.generation, connection_id)
    }

    pub(super) fn test_fault_internal_failure(
        &self,
        phase: ManagedSqliteShmFailurePhase,
        known_mutation: bool,
    ) -> ManagedSqliteShmFailure {
        self.mark_domain_terminal();
        ManagedSqliteShmFailure::poisoned_code(phase, CONTROLLER_POISONED, known_mutation, false)
    }

    fn poison_test_fault_controller_from_external_access(&self) {
        match self.state.lock() {
            Ok(mut state) => {
                self.mark_poisoned(&mut state, ManagedSqliteShmFailurePhase::Gate, false, false)
            }
            Err(_) => self.mark_domain_terminal(),
        }
    }
}

impl PinnedManagedSqliteWalMainFile {
    /// Creates a read-only exact-target witness without installing a fault. The observer is
    /// sealed to Windows tests and retains neither file nor route custody.
    #[cfg(all(test, windows))]
    pub(crate) fn test_shm_target_observer(
        &self,
    ) -> Result<ManagedSqliteShmTestTargetObserver, &'static str> {
        let (coordinator, target) = self.exact_test_fault_target()?;
        Ok(ManagedSqliteShmTestTargetObserver {
            coordinator,
            target,
        })
    }

    /// Installs one script for this exact live SHM attachment. Before-call entries always request
    /// `IoBeforeMutation`; after-success entries may request only known or uncertain mutation.
    pub(crate) fn install_shm_test_fault_script(
        &self,
        before_call: &[(ManagedSqliteShmFailurePhase, u32)],
        after_success: &[(
            ManagedSqliteShmFailurePhase,
            u32,
            ManagedSqliteShmFailureClass,
        )],
    ) -> Result<ManagedSqliteShmTestFaultProbe, &'static str> {
        let (coordinator, target) = self.exact_test_fault_target()?;
        install_exact_test_fault_script(coordinator, target, before_call, after_success)
    }

    fn exact_test_fault_target(
        &self,
    ) -> Result<
        (
            Arc<ManagedSqliteShmCoordinator>,
            ManagedSqliteShmTestFaultTarget,
        ),
        &'static str,
    > {
        let connection = self
            .shm
            .as_ref()
            .ok_or("NODE_MANAGED_SQLITE_SHM_TEST_FAULT_CONNECTION_DETACHED")?;
        if !connection.active || self.runtime_generation != connection.coordinator.generation {
            return Err("NODE_MANAGED_SQLITE_SHM_TEST_FAULT_TARGET_MISMATCH");
        }
        let coordinator = Arc::clone(&connection.coordinator);
        let target = coordinator.test_fault_target(connection.connection_id);
        Ok((coordinator, target))
    }
}

fn install_exact_test_fault_script(
    coordinator: Arc<ManagedSqliteShmCoordinator>,
    target: ManagedSqliteShmTestFaultTarget,
    before_call: &[(ManagedSqliteShmFailurePhase, u32)],
    after_success: &[(
        ManagedSqliteShmFailurePhase,
        u32,
        ManagedSqliteShmFailureClass,
    )],
) -> Result<ManagedSqliteShmTestFaultProbe, &'static str> {
    let mut faults = match coordinator.test_faults.lock() {
        Ok(faults) => faults,
        Err(poison) => {
            drop(poison.into_inner());
            coordinator.poison_test_fault_controller_from_external_access();
            return Err(CONTROLLER_POISONED);
        }
    };
    let installed = faults.install(target, before_call, after_success);
    drop(faults);
    installed?;
    Ok(ManagedSqliteShmTestFaultProbe {
        coordinator,
        target,
    })
}
