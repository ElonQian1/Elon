use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::registry) struct ManagedSqliteRegistrySessionTestSnapshot
{
    phase: ManagedSqliteRegistrySessionPhase,
    connection_owner: bool,
    main_file_lock_owner_lease: bool,
    shm_lease: bool,
    callbacks_in_flight: u32,
    access_callback_allowed: bool,
}

impl ManagedSqliteRegistrySessionTestSnapshot {
    pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::registry) fn phase(
        self,
    ) -> ManagedSqliteRegistrySessionPhase {
        self.phase
    }
    pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::registry) fn connection_owner(
        self,
    ) -> bool {
        self.connection_owner
    }
    pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::registry) fn main_file_lock_owner_lease(
        self,
    ) -> bool {
        self.main_file_lock_owner_lease
    }
    pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::registry) fn shm_lease(
        self,
    ) -> bool {
        self.shm_lease
    }
    pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::registry) fn callbacks_in_flight(
        self,
    ) -> u32 {
        self.callbacks_in_flight
    }
    pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::registry) fn access_callback_allowed(
        self,
    ) -> bool {
        self.access_callback_allowed
    }
}

impl ManagedSqliteRegistrySessionState {
    pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::registry) fn registration_shutdown_test_snapshot(
        &self,
    ) -> Result<ManagedSqliteRegistrySessionTestSnapshot, &'static str> {
        if !self.shape_is_valid() {
            return Err("registry session shape is invalid during registration shutdown");
        }
        Ok(ManagedSqliteRegistrySessionTestSnapshot {
            phase: self.phase,
            connection_owner: self.connection_owner,
            main_file_lock_owner_lease: self.main_lease.is_some_and(|lease| {
                self.main_was_claimed && lease.role == ManagedSqliteLogicalFileRole::Main
            }),
            shm_lease: self.shm_lease.is_some(),
            callbacks_in_flight: self.callbacks_in_flight,
            access_callback_allowed: self
                .callback_allowed(ManagedSqliteRegistryCallbackKind::Access),
        })
    }
}
