use super::{
    super::{
        coordinator::{ManagedSqliteShmCoordinator, ManagedSqliteShmCoordinatorState},
        types::{ManagedSqliteShmFailure, ManagedSqliteShmFailurePhase},
    },
    controller::ManagedSqliteShmMatchedTestFault,
};

#[derive(Clone, Copy)]
pub(in crate::node_agent_managed_fs::sqlite_namespace::shm) struct ManagedSqliteShmAfterTestFault(
    ManagedSqliteShmMatchedTestFault,
);

impl ManagedSqliteShmCoordinator {
    /// Observes one exact connection/phase occurrence. A before-call step activates immediately;
    /// an after-success step is returned as a token that cannot be activated before custody sync.
    pub(in crate::node_agent_managed_fs::sqlite_namespace::shm) fn begin_test_fault(
        &self,
        state: &mut ManagedSqliteShmCoordinatorState,
        connection_id: u64,
        phase: ManagedSqliteShmFailurePhase,
        known_mutation: bool,
    ) -> Result<Option<ManagedSqliteShmAfterTestFault>, ManagedSqliteShmFailure> {
        let matched = match self.observe_test_fault(connection_id, phase, known_mutation) {
            Ok(matched) => matched,
            Err(failure) => {
                self.terminalize_test_fault(state, &failure);
                return Err(failure);
            }
        };
        let Some(matched) = matched else {
            return Ok(None);
        };
        if !matched.is_before_call() {
            return Ok(Some(ManagedSqliteShmAfterTestFault(matched)));
        }
        match self.activate_test_fault(matched, known_mutation) {
            Ok(failure) => {
                if failure.mutation_may_have_occurred() || failure.lock_outcome_uncertain() {
                    self.terminalize_test_fault(state, &failure);
                }
                Err(failure)
            }
            Err(failure) => {
                self.terminalize_test_fault(state, &failure);
                Err(failure)
            }
        }
    }

    /// Activates only a matched after-success step. Callers must first synchronize the successful
    /// OS action into `state`; every resulting failure is terminal and preserves that custody.
    pub(in crate::node_agent_managed_fs::sqlite_namespace::shm) fn finish_test_fault(
        &self,
        state: &mut ManagedSqliteShmCoordinatorState,
        matched: Option<ManagedSqliteShmAfterTestFault>,
        known_mutation: bool,
    ) -> Option<ManagedSqliteShmFailure> {
        Some(self.activate_after_test_fault(state, matched?, known_mutation))
    }

    pub(in crate::node_agent_managed_fs::sqlite_namespace::shm) fn activate_after_test_fault(
        &self,
        state: &mut ManagedSqliteShmCoordinatorState,
        matched: ManagedSqliteShmAfterTestFault,
        known_mutation: bool,
    ) -> ManagedSqliteShmFailure {
        let ManagedSqliteShmAfterTestFault(matched) = matched;
        let failure = match self.activate_test_fault(matched, known_mutation) {
            Ok(failure) | Err(failure) => failure,
        };
        self.terminalize_test_fault(state, &failure);
        failure
    }

    pub(in crate::node_agent_managed_fs::sqlite_namespace::shm) fn terminalize_test_fault(
        &self,
        state: &mut ManagedSqliteShmCoordinatorState,
        failure: &ManagedSqliteShmFailure,
    ) {
        self.mark_poisoned(
            state,
            failure.phase(),
            failure.mutation_may_have_occurred(),
            failure.lock_outcome_uncertain(),
        );
    }
}
