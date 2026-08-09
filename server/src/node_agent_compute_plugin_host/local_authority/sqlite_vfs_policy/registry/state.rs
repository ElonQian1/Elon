use std::num::NonZeroU64;

use super::types::{
    ManagedSqliteRegistryCallbackKind, ManagedSqliteRegistryCallbackLease,
    ManagedSqliteRegistryCloseOutcome, ManagedSqliteRegistryFileLease,
    ManagedSqliteRegistryLeaseRecord, ManagedSqliteRegistryRetirementReceipt,
    ManagedSqliteRegistryRouteRemovalProof, ManagedSqliteRegistrySessionId,
    ManagedSqliteRegistrySessionPhase, ManagedSqliteRegistryShmLease,
    ManagedSqliteRegistryTerminalReason, ManagedSqliteRegistryTransitionRejection,
};
use crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::types::ManagedSqliteLogicalFileRole;

/// Pure, currently unconstructible lifecycle state for one future one-shot registry session.
/// A future owner must quarantine complete custody whenever a session is abandoned before retire.
#[must_use = "a non-retired session must remain under routing or terminal custody"]
pub(super) struct ManagedSqliteRegistrySessionState {
    session_id: ManagedSqliteRegistrySessionId,
    route_epoch: NonZeroU64,
    phase: ManagedSqliteRegistrySessionPhase,
    next_lease_ordinal: u64,
    connection_owner: bool,
    main_was_claimed: bool,
    main_lease: Option<ManagedSqliteRegistryLeaseRecord>,
    sidecar_leases: [Option<ManagedSqliteRegistryLeaseRecord>; 4],
    shm_lease: Option<NonZeroU64>,
    callbacks_in_flight: u32,
    terminal_reason: Option<ManagedSqliteRegistryTerminalReason>,
}

impl ManagedSqliteRegistrySessionState {
    pub(super) fn phase(&self) -> ManagedSqliteRegistrySessionPhase {
        self.phase
    }

    pub(super) fn terminal_reason(&self) -> Option<ManagedSqliteRegistryTerminalReason> {
        self.terminal_reason
    }

    pub(super) fn begin_open_attempt(
        &mut self,
    ) -> Result<(), ManagedSqliteRegistryTransitionRejection> {
        self.ensure_shape()?;
        if self.phase == ManagedSqliteRegistrySessionPhase::TerminalQuarantine {
            return Err(ManagedSqliteRegistryTransitionRejection::Terminal);
        }
        if self.phase != ManagedSqliteRegistrySessionPhase::PendingMain {
            return Err(ManagedSqliteRegistryTransitionRejection::WrongPhase);
        }
        self.connection_owner = true;
        self.phase = ManagedSqliteRegistrySessionPhase::Opening;
        Ok(())
    }

    pub(super) fn begin_callback(
        &mut self,
        kind: ManagedSqliteRegistryCallbackKind,
    ) -> Result<ManagedSqliteRegistryCallbackLease, ManagedSqliteRegistryTransitionRejection> {
        self.ensure_shape()?;
        if !self.callback_allowed(kind) {
            return Err(
                if self.phase == ManagedSqliteRegistrySessionPhase::TerminalQuarantine {
                    ManagedSqliteRegistryTransitionRejection::Terminal
                } else {
                    ManagedSqliteRegistryTransitionRejection::WrongPhase
                },
            );
        }
        let Some(next) = self.callbacks_in_flight.checked_add(1) else {
            self.enter_terminal(ManagedSqliteRegistryTerminalReason::CallbackCounterOverflow);
            return Err(ManagedSqliteRegistryTransitionRejection::CounterOverflow);
        };
        self.callbacks_in_flight = next;
        Ok(ManagedSqliteRegistryCallbackLease {
            session_id: self.session_id,
            kind,
        })
    }

    pub(super) fn finish_callback(
        &mut self,
        lease: ManagedSqliteRegistryCallbackLease,
    ) -> Result<(), ManagedSqliteRegistryTransitionRejection> {
        self.ensure_shape()?;
        if lease.session_id != self.session_id || self.callbacks_in_flight == 0 {
            self.enter_terminal(ManagedSqliteRegistryTerminalReason::StateInvariantViolated);
            return Err(ManagedSqliteRegistryTransitionRejection::SessionIdentityMismatch);
        }
        let _completed_kind = lease.kind;
        self.callbacks_in_flight -= 1;
        Ok(())
    }

    pub(super) fn claim_main(
        &mut self,
    ) -> Result<ManagedSqliteRegistryFileLease, ManagedSqliteRegistryTransitionRejection> {
        self.ensure_shape()?;
        if self.phase != ManagedSqliteRegistrySessionPhase::Opening {
            return Err(self.phase_rejection());
        }
        if self.main_was_claimed || self.main_lease.is_some() {
            return Err(ManagedSqliteRegistryTransitionRejection::MainAlreadyClaimed);
        }
        let ordinal = self.issue_ordinal()?;
        let role = ManagedSqliteLogicalFileRole::Main;
        self.main_was_claimed = true;
        self.main_lease = Some(ManagedSqliteRegistryLeaseRecord { ordinal, role });
        Ok(self.file_lease(ordinal, role))
    }

    pub(super) fn claim_sidecar(
        &mut self,
        role: ManagedSqliteLogicalFileRole,
    ) -> Result<ManagedSqliteRegistryFileLease, ManagedSqliteRegistryTransitionRejection> {
        self.ensure_shape()?;
        if !matches!(
            self.phase,
            ManagedSqliteRegistrySessionPhase::Opening | ManagedSqliteRegistrySessionPhase::Active
        ) {
            return Err(self.phase_rejection());
        }
        if !matches!(
            role,
            ManagedSqliteLogicalFileRole::Journal | ManagedSqliteLogicalFileRole::Wal
        ) {
            return Err(ManagedSqliteRegistryTransitionRejection::InvalidSidecarRole);
        }
        if !self.main_was_claimed || self.main_lease.is_none() {
            return Err(ManagedSqliteRegistryTransitionRejection::MainNotClaimed);
        }
        let slot = self
            .sidecar_leases
            .iter()
            .position(Option::is_none)
            .ok_or(ManagedSqliteRegistryTransitionRejection::LeaseCapacityExhausted)?;
        let ordinal = self.issue_ordinal()?;
        self.sidecar_leases[slot] = Some(ManagedSqliteRegistryLeaseRecord { ordinal, role });
        Ok(self.file_lease(ordinal, role))
    }

    pub(super) fn claim_shm(
        &mut self,
    ) -> Result<ManagedSqliteRegistryShmLease, ManagedSqliteRegistryTransitionRejection> {
        self.ensure_shape()?;
        if self.phase != ManagedSqliteRegistrySessionPhase::Active {
            return Err(self.phase_rejection());
        }
        if !self.main_was_claimed || self.main_lease.is_none() {
            return Err(ManagedSqliteRegistryTransitionRejection::MainNotClaimed);
        }
        if self.shm_lease.is_some() {
            return Err(ManagedSqliteRegistryTransitionRejection::LeaseCapacityExhausted);
        }
        let ordinal = self.issue_ordinal()?;
        self.shm_lease = Some(ordinal);
        Ok(ManagedSqliteRegistryShmLease {
            session_id: self.session_id,
            ordinal,
        })
    }

    pub(super) fn activate_connection(
        &mut self,
    ) -> Result<(), ManagedSqliteRegistryTransitionRejection> {
        self.ensure_shape()?;
        if self.phase != ManagedSqliteRegistrySessionPhase::Opening {
            return Err(self.phase_rejection());
        }
        if !self.connection_owner || !self.main_was_claimed || self.main_lease.is_none() {
            return Err(ManagedSqliteRegistryTransitionRejection::MainNotClaimed);
        }
        if self.callbacks_in_flight != 0 {
            return Err(ManagedSqliteRegistryTransitionRejection::OutstandingCallbacks);
        }
        self.phase = ManagedSqliteRegistrySessionPhase::Active;
        Ok(())
    }

    pub(super) fn begin_connection_close(
        &mut self,
    ) -> Result<(), ManagedSqliteRegistryTransitionRejection> {
        self.ensure_shape()?;
        if !matches!(
            self.phase,
            ManagedSqliteRegistrySessionPhase::Opening | ManagedSqliteRegistrySessionPhase::Active
        ) || !self.connection_owner
        {
            return Err(self.phase_rejection());
        }
        if self.callbacks_in_flight != 0 {
            return Err(ManagedSqliteRegistryTransitionRejection::OutstandingCallbacks);
        }
        self.phase = ManagedSqliteRegistrySessionPhase::Closing;
        Ok(())
    }

    pub(super) fn close_file(
        &mut self,
        lease: ManagedSqliteRegistryFileLease,
        outcome: ManagedSqliteRegistryCloseOutcome,
    ) -> Result<(), ManagedSqliteRegistryTransitionRejection> {
        self.ensure_shape()?;
        self.ensure_matching_session(lease.session_id)?;
        if self.phase == ManagedSqliteRegistrySessionPhase::TerminalQuarantine {
            return Err(ManagedSqliteRegistryTransitionRejection::Terminal);
        }
        if matches!(
            self.phase,
            ManagedSqliteRegistrySessionPhase::PendingMain
                | ManagedSqliteRegistrySessionPhase::AwaitingRouteRetirement
                | ManagedSqliteRegistrySessionPhase::Retired
        ) {
            self.enter_terminal(ManagedSqliteRegistryTerminalReason::LeaseIdentityMismatch);
            return Err(ManagedSqliteRegistryTransitionRejection::LeaseIdentityMismatch);
        }
        self.validate_close_outcome(outcome, lease.ordinal)?;
        match lease.role {
            ManagedSqliteLogicalFileRole::Main => {
                if !matches!(
                    self.phase,
                    ManagedSqliteRegistrySessionPhase::Opening
                        | ManagedSqliteRegistrySessionPhase::Closing
                ) {
                    self.enter_terminal(ManagedSqliteRegistryTerminalReason::LeaseIdentityMismatch);
                    return Err(ManagedSqliteRegistryTransitionRejection::WrongPhase);
                }
                if self.sidecar_leases.iter().any(Option::is_some)
                    || self.shm_lease.is_some()
                    || self.main_lease.is_none_or(|record| {
                        record.ordinal != lease.ordinal || record.role != lease.role
                    })
                {
                    self.enter_terminal(ManagedSqliteRegistryTerminalReason::LeaseIdentityMismatch);
                    return Err(ManagedSqliteRegistryTransitionRejection::LeaseIdentityMismatch);
                }
                self.main_lease = None;
            }
            ManagedSqliteLogicalFileRole::Journal | ManagedSqliteLogicalFileRole::Wal => {
                let Some(slot) = self.sidecar_leases.iter().position(|record| {
                    record.is_some_and(|record| {
                        record.ordinal == lease.ordinal && record.role == lease.role
                    })
                }) else {
                    self.enter_terminal(ManagedSqliteRegistryTerminalReason::LeaseIdentityMismatch);
                    return Err(ManagedSqliteRegistryTransitionRejection::LeaseIdentityMismatch);
                };
                self.sidecar_leases[slot] = None;
            }
        }
        Ok(())
    }

    pub(super) fn close_shm(
        &mut self,
        lease: ManagedSqliteRegistryShmLease,
        outcome: ManagedSqliteRegistryCloseOutcome,
    ) -> Result<(), ManagedSqliteRegistryTransitionRejection> {
        self.ensure_shape()?;
        self.ensure_matching_session(lease.session_id)?;
        if self.phase == ManagedSqliteRegistrySessionPhase::TerminalQuarantine {
            return Err(ManagedSqliteRegistryTransitionRejection::Terminal);
        }
        self.validate_close_outcome(outcome, lease.ordinal)?;
        if self.shm_lease != Some(lease.ordinal) {
            self.enter_terminal(ManagedSqliteRegistryTerminalReason::LeaseIdentityMismatch);
            return Err(ManagedSqliteRegistryTransitionRejection::LeaseIdentityMismatch);
        }
        self.shm_lease = None;
        Ok(())
    }

    pub(super) fn connection_close_failed(
        &mut self,
        reason: ManagedSqliteRegistryTerminalReason,
    ) -> Result<(), ManagedSqliteRegistryTransitionRejection> {
        if self.phase != ManagedSqliteRegistrySessionPhase::Closing {
            self.enter_terminal(ManagedSqliteRegistryTerminalReason::StateInvariantViolated);
            return Err(ManagedSqliteRegistryTransitionRejection::WrongPhase);
        }
        self.enter_terminal(reason);
        Err(ManagedSqliteRegistryTransitionRejection::Terminal)
    }

    pub(super) fn observe_connection_closed(
        &mut self,
    ) -> Result<(), ManagedSqliteRegistryTransitionRejection> {
        self.ensure_shape()?;
        if self.phase != ManagedSqliteRegistrySessionPhase::Closing || !self.connection_owner {
            self.enter_terminal(ManagedSqliteRegistryTerminalReason::StateInvariantViolated);
            return Err(ManagedSqliteRegistryTransitionRejection::WrongPhase);
        }
        if self.callbacks_in_flight != 0
            || self.main_lease.is_some()
            || self.sidecar_leases.iter().any(Option::is_some)
            || self.shm_lease.is_some()
        {
            self.enter_terminal(ManagedSqliteRegistryTerminalReason::ConnectionCloseUnproven);
            return Err(ManagedSqliteRegistryTransitionRejection::OutstandingHandles);
        }
        self.connection_owner = false;
        self.phase = ManagedSqliteRegistrySessionPhase::AwaitingRouteRetirement;
        Ok(())
    }

    pub(super) fn retire_after_route_removed(
        &mut self,
        proof: ManagedSqliteRegistryRouteRemovalProof,
    ) -> Result<ManagedSqliteRegistryRetirementReceipt, ManagedSqliteRegistryTransitionRejection>
    {
        self.ensure_shape()?;
        if self.phase != ManagedSqliteRegistrySessionPhase::AwaitingRouteRetirement {
            self.enter_terminal(ManagedSqliteRegistryTerminalReason::RouteIdentityMismatch);
            return Err(ManagedSqliteRegistryTransitionRejection::RouteRemovalUnproven);
        }
        self.finish_retirement(proof)
    }

    pub(super) fn cancel_pending_after_route_removed(
        &mut self,
        proof: ManagedSqliteRegistryRouteRemovalProof,
    ) -> Result<ManagedSqliteRegistryRetirementReceipt, ManagedSqliteRegistryTransitionRejection>
    {
        self.ensure_shape()?;
        if self.phase != ManagedSqliteRegistrySessionPhase::PendingMain {
            self.enter_terminal(ManagedSqliteRegistryTerminalReason::RouteIdentityMismatch);
            return Err(ManagedSqliteRegistryTransitionRejection::RouteRemovalUnproven);
        }
        self.finish_retirement(proof)
    }

    pub(super) fn quarantine(&mut self, reason: ManagedSqliteRegistryTerminalReason) {
        self.enter_terminal(reason);
    }

    fn finish_retirement(
        &mut self,
        proof: ManagedSqliteRegistryRouteRemovalProof,
    ) -> Result<ManagedSqliteRegistryRetirementReceipt, ManagedSqliteRegistryTransitionRejection>
    {
        if proof.session_id() != self.session_id || proof.route_epoch() != self.route_epoch {
            self.enter_terminal(ManagedSqliteRegistryTerminalReason::RouteIdentityMismatch);
            return Err(ManagedSqliteRegistryTransitionRejection::RouteRemovalUnproven);
        }
        let receipt = ManagedSqliteRegistryRetirementReceipt {
            session_id: self.session_id,
            route_epoch: self.route_epoch,
            main_was_claimed: self.main_was_claimed,
        };
        self.phase = ManagedSqliteRegistrySessionPhase::Retired;
        Ok(receipt)
    }

    fn callback_allowed(&self, kind: ManagedSqliteRegistryCallbackKind) -> bool {
        match self.phase {
            ManagedSqliteRegistrySessionPhase::Opening
            | ManagedSqliteRegistrySessionPhase::Active => true,
            ManagedSqliteRegistrySessionPhase::Closing => matches!(
                kind,
                ManagedSqliteRegistryCallbackKind::Delete
                    | ManagedSqliteRegistryCallbackKind::Io
                    | ManagedSqliteRegistryCallbackKind::Close
                    | ManagedSqliteRegistryCallbackKind::Shm
            ),
            ManagedSqliteRegistrySessionPhase::PendingMain
            | ManagedSqliteRegistrySessionPhase::AwaitingRouteRetirement
            | ManagedSqliteRegistrySessionPhase::Retired
            | ManagedSqliteRegistrySessionPhase::TerminalQuarantine => false,
        }
    }

    fn file_lease(
        &self,
        ordinal: NonZeroU64,
        role: ManagedSqliteLogicalFileRole,
    ) -> ManagedSqliteRegistryFileLease {
        ManagedSqliteRegistryFileLease {
            session_id: self.session_id,
            ordinal,
            role,
        }
    }

    fn issue_ordinal(&mut self) -> Result<NonZeroU64, ManagedSqliteRegistryTransitionRejection> {
        let Some(next) = self.next_lease_ordinal.checked_add(1) else {
            self.enter_terminal(ManagedSqliteRegistryTerminalReason::LeaseCounterOverflow);
            return Err(ManagedSqliteRegistryTransitionRejection::CounterOverflow);
        };
        let Some(ordinal) = NonZeroU64::new(next) else {
            self.enter_terminal(ManagedSqliteRegistryTerminalReason::LeaseCounterOverflow);
            return Err(ManagedSqliteRegistryTransitionRejection::CounterOverflow);
        };
        self.next_lease_ordinal = next;
        Ok(ordinal)
    }

    fn ensure_matching_session(
        &mut self,
        session_id: ManagedSqliteRegistrySessionId,
    ) -> Result<(), ManagedSqliteRegistryTransitionRejection> {
        if session_id == self.session_id {
            return Ok(());
        }
        self.enter_terminal(ManagedSqliteRegistryTerminalReason::LeaseIdentityMismatch);
        Err(ManagedSqliteRegistryTransitionRejection::SessionIdentityMismatch)
    }

    fn validate_close_outcome(
        &mut self,
        outcome: ManagedSqliteRegistryCloseOutcome,
        expected_ordinal: NonZeroU64,
    ) -> Result<(), ManagedSqliteRegistryTransitionRejection> {
        let proof = match outcome {
            ManagedSqliteRegistryCloseOutcome::Proven(proof) => proof,
            ManagedSqliteRegistryCloseOutcome::Unproven(reason) => {
                self.enter_terminal(reason);
                return Err(ManagedSqliteRegistryTransitionRejection::Terminal);
            }
        };
        if proof.session_id() != self.session_id || proof.lease_ordinal() != expected_ordinal {
            self.enter_terminal(ManagedSqliteRegistryTerminalReason::LeaseIdentityMismatch);
            return Err(ManagedSqliteRegistryTransitionRejection::LeaseIdentityMismatch);
        }
        Ok(())
    }

    fn ensure_shape(&mut self) -> Result<(), ManagedSqliteRegistryTransitionRejection> {
        if self.shape_is_valid() {
            return Ok(());
        }
        self.enter_terminal(ManagedSqliteRegistryTerminalReason::StateInvariantViolated);
        Err(ManagedSqliteRegistryTransitionRejection::StateInvariantViolated)
    }

    fn shape_is_valid(&self) -> bool {
        if self.phase == ManagedSqliteRegistrySessionPhase::TerminalQuarantine {
            return self.terminal_reason.is_some();
        }
        if self.terminal_reason.is_some()
            || self.main_lease.is_some_and(|record| {
                !self.main_was_claimed
                    || record.role != ManagedSqliteLogicalFileRole::Main
                    || record.ordinal.get() > self.next_lease_ordinal
            })
            || self.sidecar_leases.iter().flatten().any(|record| {
                !self.main_was_claimed
                    || self.main_lease.is_none()
                    || record.role == ManagedSqliteLogicalFileRole::Main
                    || record.ordinal.get() > self.next_lease_ordinal
            })
            || self.shm_lease.is_some_and(|ordinal| {
                !self.main_was_claimed
                    || self.main_lease.is_none()
                    || ordinal.get() > self.next_lease_ordinal
            })
        {
            return false;
        }
        match self.phase {
            ManagedSqliteRegistrySessionPhase::PendingMain => {
                !self.connection_owner
                    && !self.main_was_claimed
                    && self.next_lease_ordinal == 0
                    && self.no_live_resources()
            }
            ManagedSqliteRegistrySessionPhase::Opening => self.connection_owner,
            ManagedSqliteRegistrySessionPhase::Active => {
                self.connection_owner && self.main_was_claimed && self.main_lease.is_some()
            }
            ManagedSqliteRegistrySessionPhase::Closing => self.connection_owner,
            ManagedSqliteRegistrySessionPhase::AwaitingRouteRetirement
            | ManagedSqliteRegistrySessionPhase::Retired => {
                !self.connection_owner && self.no_live_resources()
            }
            ManagedSqliteRegistrySessionPhase::TerminalQuarantine => false,
        }
    }

    fn no_live_resources(&self) -> bool {
        self.callbacks_in_flight == 0
            && self.main_lease.is_none()
            && self.sidecar_leases.iter().all(Option::is_none)
            && self.shm_lease.is_none()
    }

    fn phase_rejection(&self) -> ManagedSqliteRegistryTransitionRejection {
        if self.phase == ManagedSqliteRegistrySessionPhase::TerminalQuarantine {
            ManagedSqliteRegistryTransitionRejection::Terminal
        } else {
            ManagedSqliteRegistryTransitionRejection::WrongPhase
        }
    }

    fn enter_terminal(&mut self, reason: ManagedSqliteRegistryTerminalReason) {
        if self.terminal_reason.is_none() {
            self.terminal_reason = Some(reason);
        }
        self.phase = ManagedSqliteRegistrySessionPhase::TerminalQuarantine;
    }
}
