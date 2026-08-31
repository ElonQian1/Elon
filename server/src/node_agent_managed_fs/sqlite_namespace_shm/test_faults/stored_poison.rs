//! Windows-test-only exact-target installer for one already-poisoned SHM prestate.

use super::{
    super::types::{ManagedSqliteShmFailureClass, ManagedSqliteShmFailurePhase},
    api::ManagedSqliteShmTestTargetObserver,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ManagedSqliteShmTestStoredPoisonV1 {
    GateNoMutation,
    FileCloseNoMutation,
    ExactSiblingDeleteNoMutation,
    ExactSiblingOpenUncertain,
    DmsTruncateUncertain,
    FileCloseUncertain,
    ExactSiblingDeleteUncertain,
    FileGrowUncertain,
    MappingCloseUncertain,
    ViewUnmapUncertain,
    LockReleaseUncertain,
    ConnectionDetachUncertain,
    DeleteAuthorizationUncertain,
    DmsExclusiveReleaseUncertain,
    DmsSharedReleaseUncertain,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ManagedSqliteShmTestStoredPoisonReceiptV1 {
    pub(crate) runtime_generation: u64,
    pub(crate) shm_connection_id: u64,
    pub(crate) profile: ManagedSqliteShmTestStoredPoisonV1,
    pub(crate) phase: ManagedSqliteShmFailurePhase,
    pub(crate) class: ManagedSqliteShmFailureClass,
    pub(crate) mutation_may_have_occurred: bool,
    pub(crate) lock_outcome_uncertain: bool,
    pub(crate) domain_terminal: bool,
}

impl ManagedSqliteShmTestStoredPoisonV1 {
    const fn facts(self) -> (ManagedSqliteShmFailurePhase, bool, bool) {
        use ManagedSqliteShmFailurePhase as Phase;
        match self {
            Self::GateNoMutation => (Phase::Gate, false, false),
            Self::FileCloseNoMutation => (Phase::FileClose, false, false),
            Self::ExactSiblingDeleteNoMutation => (Phase::ExactSiblingDelete, false, false),
            Self::ExactSiblingOpenUncertain => (Phase::ExactSiblingOpen, true, false),
            Self::DmsTruncateUncertain => (Phase::DmsTruncate, true, false),
            Self::FileCloseUncertain => (Phase::FileClose, true, false),
            Self::ExactSiblingDeleteUncertain => (Phase::ExactSiblingDelete, true, false),
            Self::FileGrowUncertain => (Phase::FileGrow, true, false),
            Self::MappingCloseUncertain => (Phase::MappingClose, true, false),
            Self::ViewUnmapUncertain => (Phase::ViewUnmap, true, false),
            Self::LockReleaseUncertain => (Phase::LockRelease, false, true),
            Self::ConnectionDetachUncertain => (Phase::ConnectionDetach, false, true),
            Self::DeleteAuthorizationUncertain => (Phase::DeleteAuthorization, false, true),
            Self::DmsExclusiveReleaseUncertain => (Phase::DmsExclusiveRelease, true, true),
            Self::DmsSharedReleaseUncertain => (Phase::DmsSharedRelease, true, true),
        }
    }
}

impl ManagedSqliteShmTestTargetObserver {
    /// Installs only the frozen stored-poison facts on this exact active target. It neither calls
    /// the Lock path nor fabricates its callback/route-retention evidence.
    pub(crate) fn install_stored_poison_prestate_v1(
        &self,
        profile: ManagedSqliteShmTestStoredPoisonV1,
    ) -> Result<ManagedSqliteShmTestStoredPoisonReceiptV1, &'static str> {
        let before = self
            .snapshot()
            .map_err(|_| "NODE_MANAGED_SQLITE_SHM_STORED_POISON_SNAPSHOT_FAILED")?;
        if !before.target_attached
            || before.shared_mask != 0
            || before.exclusive_mask != 0
            || before.topology.poisoned
            || before.topology.domain_terminal
        {
            return Err("NODE_MANAGED_SQLITE_SHM_STORED_POISON_PRESTATE_INVALID");
        }

        let (runtime_generation, shm_connection_id) = self.target.identity();
        if runtime_generation == 0
            || shm_connection_id == 0
            || self.coordinator.generation.get() != runtime_generation
            || self.coordinator.test_fault_target(shm_connection_id) != self.target
        {
            return Err("NODE_MANAGED_SQLITE_SHM_STORED_POISON_TARGET_MISMATCH");
        }
        let (phase, mutation_may_have_occurred, lock_outcome_uncertain) = profile.facts();
        let mut state = self
            .coordinator
            .state
            .lock()
            .map_err(|_| "NODE_MANAGED_SQLITE_SHM_STORED_POISON_STATE_POISONED")?;
        let connection = state
            .connections
            .get(&shm_connection_id)
            .ok_or("NODE_MANAGED_SQLITE_SHM_STORED_POISON_TARGET_DETACHED")?;
        if connection.shared_mask != 0 || connection.exclusive_mask != 0 || state.poisoned.is_some()
        {
            return Err("NODE_MANAGED_SQLITE_SHM_STORED_POISON_PRESTATE_DRIFT");
        }
        self.coordinator.mark_poisoned(
            &mut state,
            phase,
            mutation_may_have_occurred,
            lock_outcome_uncertain,
        );
        drop(state);
        let domain_terminal = self
            .coordinator
            .test_domain_terminal()
            .map_err(|_| "NODE_MANAGED_SQLITE_SHM_STORED_POISON_DOMAIN_SNAPSHOT_FAILED")?;
        if !domain_terminal {
            return Err("NODE_MANAGED_SQLITE_SHM_STORED_POISON_DOMAIN_NOT_TERMINAL");
        }
        Ok(ManagedSqliteShmTestStoredPoisonReceiptV1 {
            runtime_generation,
            shm_connection_id,
            profile,
            phase,
            class: ManagedSqliteShmFailureClass::OutcomeUncertainPoisoned,
            mutation_may_have_occurred,
            lock_outcome_uncertain,
            domain_terminal,
        })
    }
}
