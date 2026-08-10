use super::*;
use crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::registry::types::{
    ManagedSqliteRegistryCallbackCompletionReceipt, ManagedSqliteRegistryConnectionClosedReceipt,
};

impl ManagedSqliteRegistrySessionState {
    pub(super) fn finish_callback_with_receipt(
        &mut self,
        lease: &ManagedSqliteRegistryCallbackLease,
    ) -> Result<
        ManagedSqliteRegistryCallbackCompletionReceipt,
        ManagedSqliteRegistryTransitionRejection,
    > {
        self.finish_callback(lease)?;
        Ok(ManagedSqliteRegistryCallbackCompletionReceipt::from_completed(lease))
    }

    pub(super) fn observe_connection_closed_after_callback(
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

    pub(super) fn validate_connection_closed_receipt(
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
