use super::*;

impl<Custody: ManagedSqliteRegistryCustody> ManagedSqliteRegistryOwner<Custody> {
    pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::registry) fn claim_main(
        &mut self,
        handle: ManagedSqliteRegistryRouteHandle,
    ) -> Result<ManagedSqliteRegistryFileLease, ManagedSqliteRegistryRouteRejection> {
        self.exact_entry_mut(handle)?
            .state
            .claim_main()
            .map_err(ManagedSqliteRegistryRouteRejection::State)
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::registry) fn claim_sidecar(
        &mut self,
        handle: ManagedSqliteRegistryRouteHandle,
        role: ManagedSqliteLogicalFileRole,
    ) -> Result<ManagedSqliteRegistryFileLease, ManagedSqliteRegistryRouteRejection> {
        self.exact_entry_mut(handle)?
            .state
            .claim_sidecar(role)
            .map_err(ManagedSqliteRegistryRouteRejection::State)
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::registry) fn claim_shm(
        &mut self,
        handle: ManagedSqliteRegistryRouteHandle,
    ) -> Result<ManagedSqliteRegistryShmLease, ManagedSqliteRegistryRouteRejection> {
        self.exact_entry_mut(handle)?
            .state
            .claim_shm()
            .map_err(ManagedSqliteRegistryRouteRejection::State)
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::registry) fn activate_connection(
        &mut self,
        handle: ManagedSqliteRegistryRouteHandle,
    ) -> Result<(), ManagedSqliteRegistryRouteRejection> {
        self.exact_entry_mut(handle)?
            .state
            .activate_connection()
            .map_err(ManagedSqliteRegistryRouteRejection::State)
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::registry) fn close_file(
        &mut self,
        handle: ManagedSqliteRegistryRouteHandle,
        lease: ManagedSqliteRegistryFileLease,
        outcome: ManagedSqliteRegistryCloseOutcome,
    ) -> Result<(), ManagedSqliteRegistryRouteRejection> {
        self.exact_entry_mut(handle)?
            .state
            .close_file(lease, outcome)
            .map_err(ManagedSqliteRegistryRouteRejection::State)
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::registry) fn close_wal_main(
        &mut self,
        handle: ManagedSqliteRegistryRouteHandle,
        main: ManagedSqliteRegistryFileLease,
        shm: ManagedSqliteRegistryShmLease,
        proofs: Result<
            ManagedSqliteRegistryWalMainCloseProofs,
            ManagedSqliteRegistryTerminalReason,
        >,
    ) -> Result<(), ManagedSqliteRegistryRouteRejection> {
        let state = &mut self.exact_entry_mut(handle)?.state;
        let (main_outcome, shm_outcome) = match proofs {
            Ok(proofs) => {
                let (main, shm) = proofs.into_parts();
                (
                    ManagedSqliteRegistryCloseOutcome::Proven(main),
                    ManagedSqliteRegistryCloseOutcome::Proven(shm),
                )
            }
            Err(reason) => (
                ManagedSqliteRegistryCloseOutcome::Unproven(reason),
                ManagedSqliteRegistryCloseOutcome::Unproven(reason),
            ),
        };
        state
            .close_shm(shm, shm_outcome)
            .and_then(|()| state.close_file(main, main_outcome))
            .map_err(ManagedSqliteRegistryRouteRejection::State)
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::registry) fn connection_close_failed(
        &mut self,
        handle: ManagedSqliteRegistryRouteHandle,
        reason: ManagedSqliteRegistryTerminalReason,
    ) -> Result<(), ManagedSqliteRegistryRouteRejection> {
        self.exact_entry_mut(handle)?
            .state
            .connection_close_failed(reason)
            .map_err(ManagedSqliteRegistryRouteRejection::State)
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::registry) fn observe_connection_closed(
        &mut self,
        handle: ManagedSqliteRegistryRouteHandle,
    ) -> Result<(), ManagedSqliteRegistryRouteRejection> {
        self.exact_entry_mut(handle)?
            .state
            .observe_connection_closed()
            .map_err(ManagedSqliteRegistryRouteRejection::State)
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::registry) fn retire_closed(
        &mut self,
        handle: ManagedSqliteRegistryRouteHandle,
    ) -> Result<ManagedSqliteRegistryRetirementReceipt, ManagedSqliteRegistryRouteRejection> {
        self.exact_entry(handle)?;
        let mut entry = self
            .routes
            .remove(&handle.token)
            .expect("validated route must remain present under exclusive owner access");
        let proof = ManagedSqliteRegistryRouteRemovalProof::from_removed_route(
            entry.state.session_id(),
            entry.state.route_epoch(),
        );
        match entry.state.retire_after_route_removed(proof) {
            Ok(receipt) => Ok(receipt),
            Err(rejection) => {
                entry
                    .state
                    .quarantine(ManagedSqliteRegistryTerminalReason::FailureCustodyRetained);
                Self::retain_terminal(entry);
                Err(ManagedSqliteRegistryRouteRejection::State(rejection))
            }
        }
    }
}
