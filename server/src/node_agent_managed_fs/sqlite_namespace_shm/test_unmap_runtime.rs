//! Exact-target, Windows-test-only observation for real final-connection Unmap actions.

#[path = "test_unmap_runtime/authority.rs"]
mod authority;
#[path = "test_unmap_runtime/native.rs"]
mod native;
#[path = "test_unmap_runtime/prestate.rs"]
mod prestate;

pub(crate) use authority::ManagedSqliteShmTestUnmapDeleteAuthorityReceipt;
pub(crate) use native::{
    ManagedSqliteShmTestUnmapNativeObservation, ManagedSqliteShmTestUnmapNativeOperation,
    ManagedSqliteShmTestUnmapNativeReceipt, ManagedSqliteShmTestUnmapNativeTiming,
};
pub(crate) use prestate::{
    ManagedSqliteShmTestUnmapDeletePrestate, ManagedSqliteShmTestUnmapDeletePrestateReceipt,
};

use super::{
    coordinator::ManagedSqliteShmCoordinator,
    types::{ManagedSqliteShmFailure, ManagedSqliteShmFailurePhase},
    ManagedSqliteDeleteOutcome,
};
use native::ManagedSqliteShmTestUnmapNativeControl;
use prestate::ManagedSqliteShmTestUnmapDeletePrestateControl;

pub(super) type ExactTarget = (u64, u64);
const MAX_UNMAP_ACTION_EVENTS: usize = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ManagedSqliteShmTestUnmapActionOutcome {
    Attempt,
    Success,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ManagedSqliteShmTestUnmapActionEvent {
    pub(crate) phase: ManagedSqliteShmFailurePhase,
    pub(crate) outcome: ManagedSqliteShmTestUnmapActionOutcome,
    pub(crate) ordinal: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ManagedSqliteShmTestUnmapReceipt {
    pub(crate) actions: Vec<ManagedSqliteShmTestUnmapActionEvent>,
    pub(crate) native: Option<ManagedSqliteShmTestUnmapNativeReceipt>,
    pub(crate) prestate: Option<ManagedSqliteShmTestUnmapDeletePrestateReceipt>,
    pub(crate) delete_outcome: Option<ManagedSqliteDeleteOutcome>,
    pub(crate) delete_authority: Option<ManagedSqliteShmTestUnmapDeleteAuthorityReceipt>,
    pub(crate) pending: usize,
    pub(crate) finished: bool,
}

#[derive(Default)]
pub(in crate::node_agent_managed_fs::sqlite_namespace::shm) struct ManagedSqliteShmTestUnmapController
{
    target: Option<ExactTarget>,
    actions: Vec<ManagedSqliteShmTestUnmapActionEvent>,
    native: ManagedSqliteShmTestUnmapNativeControl,
    prestate: ManagedSqliteShmTestUnmapDeletePrestateControl,
    delete_outcome: Option<ManagedSqliteDeleteOutcome>,
    delete_authority: Option<ManagedSqliteShmTestUnmapDeleteAuthorityReceipt>,
    finished: bool,
}

impl ManagedSqliteShmTestUnmapController {
    pub(super) fn observes_live_target(&self, target: ExactTarget) -> bool {
        self.target == Some(target) && !self.finished
    }

    pub(super) fn begin(&mut self, target: ExactTarget) -> Result<(), &'static str> {
        if target.0 == 0 || target.1 == 0 {
            return Err("NODE_MANAGED_SQLITE_SHM_TEST_UNMAP_TARGET_ZERO");
        }
        if self.target.is_some() {
            return Err("NODE_MANAGED_SQLITE_SHM_TEST_UNMAP_OBSERVATION_ALREADY_BEGUN");
        }
        self.target = Some(target);
        Ok(())
    }

    pub(super) fn install_native(
        &mut self,
        target: ExactTarget,
        operation: ManagedSqliteShmTestUnmapNativeOperation,
    ) -> Result<(), &'static str> {
        self.require_live_target(target)?;
        self.native.install(operation)
    }

    pub(super) fn install_prestate(
        &mut self,
        target: ExactTarget,
        prestate: ManagedSqliteShmTestUnmapDeletePrestate,
    ) -> Result<(), &'static str> {
        self.require_live_target(target)?;
        self.prestate.install(prestate)
    }

    pub(super) fn record_action(
        &mut self,
        target: ExactTarget,
        phase: ManagedSqliteShmFailurePhase,
        outcome: ManagedSqliteShmTestUnmapActionOutcome,
    ) -> Result<(), &'static str> {
        if self.target != Some(target) {
            return Ok(());
        }
        if self.finished {
            return Err("NODE_MANAGED_SQLITE_SHM_TEST_UNMAP_ACTION_AFTER_FINISH");
        }
        if self.actions.len() >= MAX_UNMAP_ACTION_EVENTS {
            return Err("NODE_MANAGED_SQLITE_SHM_TEST_UNMAP_ACTION_LEDGER_FULL");
        }
        let ordinal = self
            .actions
            .iter()
            .filter(|event| event.phase == phase && event.outcome == outcome)
            .count()
            .checked_add(1)
            .and_then(|value| u8::try_from(value).ok())
            .ok_or("NODE_MANAGED_SQLITE_SHM_TEST_UNMAP_ACTION_ORDINAL_INVALID")?;
        self.actions.push(ManagedSqliteShmTestUnmapActionEvent {
            phase,
            outcome,
            ordinal,
        });
        Ok(())
    }

    pub(super) fn select_native(
        &self,
        target: ExactTarget,
        phase: ManagedSqliteShmFailurePhase,
    ) -> Result<Option<ManagedSqliteShmTestUnmapNativeOperation>, &'static str> {
        if self.target != Some(target) {
            return Ok(None);
        }
        self.require_live_target(target)?;
        Ok(self.native.select_for_phase(phase))
    }

    pub(super) fn trigger_native(
        &mut self,
        target: ExactTarget,
        operation: ManagedSqliteShmTestUnmapNativeOperation,
    ) -> Result<(), &'static str> {
        self.require_live_target(target)?;
        self.native.trigger(operation)
    }

    pub(super) fn witness_native(
        &mut self,
        target: ExactTarget,
        operation: ManagedSqliteShmTestUnmapNativeOperation,
        observation: ManagedSqliteShmTestUnmapNativeObservation,
    ) -> Result<(), &'static str> {
        self.require_live_target(target)?;
        self.native.witness(operation, observation)
    }

    pub(super) fn take_authority_prestate(
        &mut self,
        target: ExactTarget,
    ) -> Result<Option<ManagedSqliteShmTestUnmapDeletePrestate>, &'static str> {
        if self.target != Some(target) {
            return Ok(None);
        }
        self.require_live_target(target)?;
        Ok(self.prestate.take_authority())
    }

    pub(super) fn take_not_found_prestate(
        &mut self,
        target: ExactTarget,
    ) -> Result<Option<ManagedSqliteShmTestUnmapDeletePrestate>, &'static str> {
        if self.target != Some(target) {
            return Ok(None);
        }
        self.require_live_target(target)?;
        Ok(self.prestate.take_not_found())
    }

    pub(super) fn mark_prestate_applied(
        &mut self,
        target: ExactTarget,
        prestate: ManagedSqliteShmTestUnmapDeletePrestate,
    ) -> Result<(), &'static str> {
        self.require_target(target)?;
        self.prestate.mark_applied(prestate);
        Ok(())
    }

    pub(super) fn record_prestate_setup_delete(
        &mut self,
        target: ExactTarget,
        prestate: ManagedSqliteShmTestUnmapDeletePrestate,
        outcome: ManagedSqliteDeleteOutcome,
    ) -> Result<(), &'static str> {
        self.require_live_target(target)?;
        self.prestate.record_setup_delete(prestate, outcome)
    }

    pub(super) fn record_delete_outcome(
        &mut self,
        target: ExactTarget,
        outcome: ManagedSqliteDeleteOutcome,
    ) -> Result<(), &'static str> {
        self.require_live_target(target)?;
        if self.delete_outcome.is_some()
            || !matches!(
                self.actions.last(),
                Some(event)
                    if event.phase == ManagedSqliteShmFailurePhase::ExactSiblingDelete
                        && event.outcome == ManagedSqliteShmTestUnmapActionOutcome::Attempt
            )
        {
            return Err("NODE_MANAGED_SQLITE_SHM_TEST_UNMAP_DELETE_OUTCOME_INVALID");
        }
        self.delete_outcome = Some(outcome);
        Ok(())
    }

    pub(super) fn record_delete_authority(
        &mut self,
        target: ExactTarget,
        receipt: ManagedSqliteShmTestUnmapDeleteAuthorityReceipt,
    ) -> Result<(), &'static str> {
        self.require_live_target(target)?;
        if self.delete_authority.is_some() {
            return Err("NODE_MANAGED_SQLITE_SHM_TEST_UNMAP_DELETE_AUTHORITY_DUPLICATE");
        }
        self.delete_authority = Some(receipt);
        Ok(())
    }

    pub(super) fn receipt(
        &self,
        target: ExactTarget,
    ) -> Result<ManagedSqliteShmTestUnmapReceipt, &'static str> {
        self.require_target(target)?;
        Ok(self.copy_receipt())
    }

    pub(super) fn finish(
        &mut self,
        target: ExactTarget,
    ) -> Result<ManagedSqliteShmTestUnmapReceipt, &'static str> {
        self.require_target(target)?;
        self.finished = true;
        Ok(self.copy_receipt())
    }

    fn copy_receipt(&self) -> ManagedSqliteShmTestUnmapReceipt {
        ManagedSqliteShmTestUnmapReceipt {
            actions: self.actions.clone(),
            native: self.native.receipt(),
            prestate: self.prestate.receipt(),
            delete_outcome: self.delete_outcome,
            delete_authority: self.delete_authority,
            pending: self.native.pending() + self.prestate.pending(),
            finished: self.finished,
        }
    }

    fn require_live_target(&self, target: ExactTarget) -> Result<(), &'static str> {
        self.require_target(target)?;
        if self.finished {
            return Err("NODE_MANAGED_SQLITE_SHM_TEST_UNMAP_OBSERVATION_FINISHED");
        }
        Ok(())
    }

    fn require_target(&self, target: ExactTarget) -> Result<(), &'static str> {
        if self.target != Some(target) {
            return Err("NODE_MANAGED_SQLITE_SHM_TEST_UNMAP_TARGET_MISMATCH");
        }
        Ok(())
    }
}

impl ManagedSqliteShmCoordinator {
    pub(super) fn observes_test_unmap_target(
        &self,
        connection_id: u64,
    ) -> Result<bool, ManagedSqliteShmFailure> {
        let target = (self.generation.get(), connection_id);
        self.test_unmap_runtime
            .lock()
            .map(|runtime| runtime.observes_live_target(target))
            .map_err(|_| {
                self.test_unmap_runtime_failure(
                    ManagedSqliteShmFailurePhase::DeleteAuthorization,
                    false,
                )
            })
    }

    pub(super) fn begin_test_unmap_action(
        &self,
        connection_id: u64,
        phase: ManagedSqliteShmFailurePhase,
        known_mutation: bool,
    ) -> Result<Option<ManagedSqliteShmTestUnmapNativeOperation>, ManagedSqliteShmFailure> {
        let target = (self.generation.get(), connection_id);
        let mut runtime = self
            .test_unmap_runtime
            .lock()
            .map_err(|_| self.test_unmap_runtime_failure(phase, known_mutation))?;
        runtime
            .record_action(
                target,
                phase,
                ManagedSqliteShmTestUnmapActionOutcome::Attempt,
            )
            .and_then(|()| runtime.select_native(target, phase))
            .map_err(|_| self.test_unmap_runtime_failure(phase, known_mutation))
    }

    pub(super) fn trigger_test_unmap_native(
        &self,
        connection_id: u64,
        operation: ManagedSqliteShmTestUnmapNativeOperation,
        known_mutation: bool,
    ) -> Result<(), ManagedSqliteShmFailure> {
        let phase = operation.phase();
        let target = (self.generation.get(), connection_id);
        self.test_unmap_runtime
            .lock()
            .map_err(|_| self.test_unmap_runtime_failure(phase, known_mutation))?
            .trigger_native(target, operation)
            .map_err(|_| self.test_unmap_runtime_failure(phase, known_mutation))
    }

    pub(super) fn witness_test_unmap_native(
        &self,
        connection_id: u64,
        operation: ManagedSqliteShmTestUnmapNativeOperation,
        observation: ManagedSqliteShmTestUnmapNativeObservation,
        known_mutation: bool,
    ) -> Result<(), ManagedSqliteShmFailure> {
        let phase = operation.phase();
        let target = (self.generation.get(), connection_id);
        self.test_unmap_runtime
            .lock()
            .map_err(|_| self.test_unmap_runtime_failure(phase, known_mutation))?
            .witness_native(target, operation, observation)
            .map_err(|_| self.test_unmap_runtime_failure(phase, known_mutation))
    }

    pub(super) fn finish_test_unmap_action(
        &self,
        connection_id: u64,
        phase: ManagedSqliteShmFailurePhase,
        known_mutation: bool,
    ) -> Result<(), ManagedSqliteShmFailure> {
        let target = (self.generation.get(), connection_id);
        self.test_unmap_runtime
            .lock()
            .map_err(|_| self.test_unmap_runtime_failure(phase, known_mutation))?
            .record_action(
                target,
                phase,
                ManagedSqliteShmTestUnmapActionOutcome::Success,
            )
            .map_err(|_| self.test_unmap_runtime_failure(phase, known_mutation))
    }

    pub(super) fn take_test_unmap_authority_prestate(
        &self,
        connection_id: u64,
    ) -> Result<Option<ManagedSqliteShmTestUnmapDeletePrestate>, ManagedSqliteShmFailure> {
        let target = (self.generation.get(), connection_id);
        self.test_unmap_runtime
            .lock()
            .map_err(|_| {
                self.test_unmap_runtime_failure(
                    ManagedSqliteShmFailurePhase::DeleteAuthorization,
                    false,
                )
            })?
            .take_authority_prestate(target)
            .map_err(|_| {
                self.test_unmap_runtime_failure(
                    ManagedSqliteShmFailurePhase::DeleteAuthorization,
                    false,
                )
            })
    }

    pub(super) fn take_test_unmap_not_found_prestate(
        &self,
        connection_id: u64,
    ) -> Result<Option<ManagedSqliteShmTestUnmapDeletePrestate>, ManagedSqliteShmFailure> {
        let target = (self.generation.get(), connection_id);
        self.test_unmap_runtime
            .lock()
            .map_err(|_| {
                self.test_unmap_runtime_failure(
                    ManagedSqliteShmFailurePhase::ExactSiblingDelete,
                    true,
                )
            })?
            .take_not_found_prestate(target)
            .map_err(|_| {
                self.test_unmap_runtime_failure(
                    ManagedSqliteShmFailurePhase::ExactSiblingDelete,
                    true,
                )
            })
    }

    pub(super) fn mark_test_unmap_prestate_applied(
        &self,
        connection_id: u64,
        prestate: ManagedSqliteShmTestUnmapDeletePrestate,
        phase: ManagedSqliteShmFailurePhase,
        known_mutation: bool,
    ) -> Result<(), ManagedSqliteShmFailure> {
        let target = (self.generation.get(), connection_id);
        self.test_unmap_runtime
            .lock()
            .map_err(|_| self.test_unmap_runtime_failure(phase, known_mutation))?
            .mark_prestate_applied(target, prestate)
            .map_err(|_| self.test_unmap_runtime_failure(phase, known_mutation))
    }

    pub(super) fn record_test_unmap_prestate_setup_delete(
        &self,
        connection_id: u64,
        prestate: ManagedSqliteShmTestUnmapDeletePrestate,
        outcome: ManagedSqliteDeleteOutcome,
    ) -> Result<(), ManagedSqliteShmFailure> {
        let phase = ManagedSqliteShmFailurePhase::ExactSiblingDelete;
        let target = (self.generation.get(), connection_id);
        self.test_unmap_runtime
            .lock()
            .map_err(|_| self.test_unmap_runtime_failure(phase, true))?
            .record_prestate_setup_delete(target, prestate, outcome)
            .map_err(|_| self.test_unmap_runtime_failure(phase, true))
    }

    pub(super) fn record_test_unmap_delete_outcome(
        &self,
        connection_id: u64,
        outcome: ManagedSqliteDeleteOutcome,
    ) -> Result<(), ManagedSqliteShmFailure> {
        let phase = ManagedSqliteShmFailurePhase::ExactSiblingDelete;
        let target = (self.generation.get(), connection_id);
        self.test_unmap_runtime
            .lock()
            .map_err(|_| self.test_unmap_runtime_failure(phase, true))?
            .record_delete_outcome(target, outcome)
            .map_err(|_| self.test_unmap_runtime_failure(phase, true))
    }

    pub(super) fn record_test_unmap_delete_authority(
        &self,
        connection_id: u64,
        receipt: ManagedSqliteShmTestUnmapDeleteAuthorityReceipt,
    ) -> Result<(), ManagedSqliteShmFailure> {
        let phase = ManagedSqliteShmFailurePhase::DeleteAuthorization;
        let target = (self.generation.get(), connection_id);
        self.test_unmap_runtime
            .lock()
            .map_err(|_| self.test_unmap_runtime_failure(phase, false))?
            .record_delete_authority(target, receipt)
            .map_err(|_| self.test_unmap_runtime_failure(phase, false))
    }

    fn test_unmap_runtime_failure(
        &self,
        phase: ManagedSqliteShmFailurePhase,
        known_mutation: bool,
    ) -> ManagedSqliteShmFailure {
        self.mark_domain_terminal();
        ManagedSqliteShmFailure::poisoned_code(
            phase,
            "NODE_MANAGED_SQLITE_SHM_TEST_UNMAP_RUNTIME_INVALID",
            known_mutation,
            false,
        )
    }
}
