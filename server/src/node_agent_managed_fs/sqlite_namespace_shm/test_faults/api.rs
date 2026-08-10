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

const CONTROLLER_POISONED: &str = "NODE_MANAGED_SQLITE_SHM_TEST_FAULT_CONTROLLER_POISONED";

/// Move-only observation authority for one exact installed SHM fault script.
///
/// The exact runtime generation and connection target remain private to this module.
pub(crate) struct ManagedSqliteShmTestFaultProbe {
    coordinator: Arc<ManagedSqliteShmCoordinator>,
    target: ManagedSqliteShmTestFaultTarget,
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
}

impl ManagedSqliteShmCoordinator {
    pub(in crate::node_agent_managed_fs::sqlite_namespace::shm) fn observe_test_fault(
        &self,
        connection_id: u64,
        phase: ManagedSqliteShmFailurePhase,
        known_mutation: bool,
    ) -> Result<Option<ManagedSqliteShmMatchedTestFault>, ManagedSqliteShmFailure> {
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
