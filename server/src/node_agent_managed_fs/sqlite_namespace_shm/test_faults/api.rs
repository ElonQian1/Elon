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

    pub(in crate::node_agent_managed_fs::sqlite_namespace::shm) fn activate_test_fault(
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
            .activate(matched, known_mutation)
            .map_err(|_| self.test_fault_internal_failure(phase, known_mutation))?;
        Ok(triggered.into_failure(known_mutation))
    }

    fn test_fault_target(&self, connection_id: u64) -> ManagedSqliteShmTestFaultTarget {
        ManagedSqliteShmTestFaultTarget::new(self.generation, connection_id)
    }

    fn test_fault_internal_failure(
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
    ) -> Result<(), &'static str> {
        let (coordinator, target) = self.exact_test_fault_target()?;
        let mut faults = match coordinator.test_faults.lock() {
            Ok(faults) => faults,
            Err(poison) => {
                drop(poison.into_inner());
                coordinator.poison_test_fault_controller_from_external_access();
                return Err(CONTROLLER_POISONED);
            }
        };
        faults.install(target, before_call, after_success)
    }

    pub(crate) fn pending_shm_test_fault_count(&self) -> Result<usize, &'static str> {
        let (coordinator, target) = self.exact_test_fault_target()?;
        match coordinator.test_faults.lock() {
            Ok(faults) => Ok(faults.pending_count(target)),
            Err(poison) => {
                drop(poison.into_inner());
                coordinator.poison_test_fault_controller_from_external_access();
                Err(CONTROLLER_POISONED)
            }
        }
    }

    pub(crate) fn shm_test_fault_was_triggered(
        &self,
        phase: ManagedSqliteShmFailurePhase,
        ordinal: u32,
    ) -> Result<bool, &'static str> {
        let (coordinator, target) = self.exact_test_fault_target()?;
        match coordinator.test_faults.lock() {
            Ok(faults) => Ok(faults.was_triggered(target, phase, ordinal)),
            Err(poison) => {
                drop(poison.into_inner());
                coordinator.poison_test_fault_controller_from_external_access();
                Err(CONTROLLER_POISONED)
            }
        }
    }

    fn exact_test_fault_target(
        &self,
    ) -> Result<
        (
            &ManagedSqliteShmCoordinator,
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
        let coordinator = connection.coordinator.as_ref();
        Ok((
            coordinator,
            coordinator.test_fault_target(connection.connection_id),
        ))
    }
}
