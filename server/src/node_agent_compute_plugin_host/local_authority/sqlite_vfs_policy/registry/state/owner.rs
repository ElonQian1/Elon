use super::*;

impl ManagedSqliteRegistrySessionState {
    pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::registry) fn new_pending(
        session_id: ManagedSqliteRegistrySessionId,
        route_epoch: NonZeroU64,
    ) -> Self {
        Self {
            session_id,
            route_epoch,
            phase: ManagedSqliteRegistrySessionPhase::PendingMain,
            next_lease_ordinal: 0,
            connection_owner: false,
            main_was_claimed: false,
            main_lease: None,
            sidecar_leases: [None; 4],
            shm_lease: None,
            callbacks_in_flight: 0,
            terminal_reason: None,
        }
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::registry) fn session_id(
        &self,
    ) -> ManagedSqliteRegistrySessionId {
        self.session_id
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::registry) fn route_epoch(
        &self,
    ) -> NonZeroU64 {
        self.route_epoch
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::registry) fn phase(
        &self,
    ) -> ManagedSqliteRegistrySessionPhase {
        self.phase
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::registry) fn terminal_reason(
        &self,
    ) -> Option<ManagedSqliteRegistryTerminalReason> {
        self.terminal_reason
    }
}
