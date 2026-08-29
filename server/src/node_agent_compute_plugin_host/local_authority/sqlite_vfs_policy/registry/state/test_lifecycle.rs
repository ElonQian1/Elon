use super::*;
use crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::registry::types::{
    ManagedSqliteRegistryCallbackCompletionReceipt, ManagedSqliteRegistryConnectionClosedReceipt,
};

impl ManagedSqliteRegistrySessionState {
    #[cfg(all(test, windows))]
    pub(super) fn allows_connection_observation_sidecar_shape(&self) -> bool {
        self.phase == ManagedSqliteRegistrySessionPhase::Closing
            && self.connection_owner
            && self.main_was_claimed
            && self.main_lease.is_none()
            && self.shm_lease.is_none()
            && self.callbacks_in_flight == 0
    }

    #[cfg(all(test, windows))]
    pub(in super::super) fn claim_connection_observation_sidecar(
        &mut self,
        role: ManagedSqliteLogicalFileRole,
    ) -> Result<ManagedSqliteRegistryFileLease, ManagedSqliteRegistryTransitionRejection> {
        self.ensure_shape()?;
        if self.phase != ManagedSqliteRegistrySessionPhase::Closing
            || !self.connection_owner
            || !self.main_was_claimed
            || self.main_lease.is_some()
            || self.shm_lease.is_some()
            || self.callbacks_in_flight != 0
            || self.sidecar_leases.iter().any(Option::is_some)
            || role != ManagedSqliteLogicalFileRole::Journal
        {
            self.enter_terminal(ManagedSqliteRegistryTerminalReason::StateInvariantViolated);
            return Err(ManagedSqliteRegistryTransitionRejection::StateInvariantViolated);
        }
        let ordinal = self.issue_ordinal()?;
        self.sidecar_leases[0] = Some(ManagedSqliteRegistryLeaseRecord { ordinal, role });
        debug_assert!(self.shape_is_valid());
        Ok(self.file_lease(ordinal, role))
    }

    pub(in super::super) fn arm_barrier_callback_completion_native_rejection(
        &mut self,
        lease: &ManagedSqliteRegistryCallbackLease,
    ) -> Result<(), ManagedSqliteRegistryTransitionRejection> {
        self.ensure_shape()?;
        if self.phase != ManagedSqliteRegistrySessionPhase::Active
            || lease.session_id != self.session_id
            || lease.kind != ManagedSqliteRegistryCallbackKind::Shm
            || self.callbacks_in_flight != 1
        {
            self.enter_terminal(ManagedSqliteRegistryTerminalReason::StateInvariantViolated);
            return Err(ManagedSqliteRegistryTransitionRejection::StateInvariantViolated);
        }
        self.enter_terminal(ManagedSqliteRegistryTerminalReason::FailureCustodyRetained);
        Ok(())
    }

    #[cfg(all(test, windows))]
    pub(in super::super) fn arm_close_callback_completion_native_rejection(
        &mut self,
        lease: &ManagedSqliteRegistryCallbackLease,
    ) -> Result<(), ManagedSqliteRegistryTransitionRejection> {
        self.ensure_shape()?;
        if self.phase != ManagedSqliteRegistrySessionPhase::Closing
            || lease.session_id != self.session_id
            || lease.kind != ManagedSqliteRegistryCallbackKind::Close
            || self.callbacks_in_flight != 1
            || self.main_lease.is_some()
            || self.shm_lease.is_some()
        {
            self.enter_terminal(ManagedSqliteRegistryTerminalReason::StateInvariantViolated);
            return Err(ManagedSqliteRegistryTransitionRejection::StateInvariantViolated);
        }
        self.enter_terminal(ManagedSqliteRegistryTerminalReason::FailureCustodyRetained);
        Ok(())
    }

    #[cfg(all(test, windows))]
    pub(in super::super) fn arm_route_retirement_native_rejection(
        &mut self,
        receipt: &ManagedSqliteRegistryConnectionClosedReceipt,
    ) -> Result<(), ManagedSqliteRegistryTransitionRejection> {
        self.validate_connection_closed_receipt(receipt)?;
        self.enter_terminal(ManagedSqliteRegistryTerminalReason::FailureCustodyRetained);
        Ok(())
    }

    pub(in super::super) fn finish_callback_with_receipt(
        &mut self,
        lease: &ManagedSqliteRegistryCallbackLease,
    ) -> Result<
        ManagedSqliteRegistryCallbackCompletionReceipt,
        ManagedSqliteRegistryTransitionRejection,
    > {
        if self.phase == ManagedSqliteRegistrySessionPhase::TerminalQuarantine {
            return Err(ManagedSqliteRegistryTransitionRejection::Terminal);
        }
        self.finish_callback(lease)?;
        Ok(ManagedSqliteRegistryCallbackCompletionReceipt::from_completed(lease))
    }

    pub(in super::super) fn observe_connection_closed_after_callback(
        &mut self,
        callback: &ManagedSqliteRegistryCallbackCompletionReceipt,
    ) -> Result<
        ManagedSqliteRegistryConnectionClosedReceipt,
        ManagedSqliteRegistryTransitionRejection,
    > {
        if callback.session_id() != self.session_id
            || callback.kind() != ManagedSqliteRegistryCallbackKind::Close
        {
            self.enter_terminal(ManagedSqliteRegistryTerminalReason::StateInvariantViolated);
            return Err(ManagedSqliteRegistryTransitionRejection::SessionIdentityMismatch);
        }
        self.observe_connection_closed()?;
        Ok(ManagedSqliteRegistryConnectionClosedReceipt::from_observed(
            self.session_id,
            self.route_epoch,
        ))
    }

    pub(in super::super) fn validate_connection_closed_receipt(
        &mut self,
        receipt: &ManagedSqliteRegistryConnectionClosedReceipt,
    ) -> Result<(), ManagedSqliteRegistryTransitionRejection> {
        self.ensure_shape()?;
        if self.phase != ManagedSqliteRegistrySessionPhase::AwaitingRouteRetirement
            || receipt.session_id() != self.session_id
            || receipt.route_epoch() != self.route_epoch
        {
            self.enter_terminal(ManagedSqliteRegistryTerminalReason::RouteIdentityMismatch);
            return Err(ManagedSqliteRegistryTransitionRejection::RouteRemovalUnproven);
        }
        Ok(())
    }
}
