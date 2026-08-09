use super::*;

impl<Custody, NonceSource> ManagedSqliteRegistryProcessOwner<Custody, NonceSource>
where
    Custody: ManagedSqliteRegistryCustody + 'static,
    NonceSource: ManagedSqliteRegistryNonceSource + 'static,
{
    /// Permanently retains physical and registry custody before removing the exact route into
    /// terminal quarantine. Retention happens first so owner poisoning or a stale route can never
    /// make an uncertain handle fall through ordinary Rust destruction.
    pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::registry) fn retain_terminal_custody<
        Retained: 'static,
    >(
        &self,
        route: ManagedSqliteRegistryRouteHandle,
        reason: ManagedSqliteRegistryTerminalReason,
        custody: Retained,
    ) -> Result<(), ManagedSqliteRegistryProcessRouteRejection> {
        let _permanent_physical_custody = Box::leak(Box::new(custody));
        self.apply_route(route, |routes| routes.quarantine(route, reason))
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::registry) fn claim_main(
        &self,
        route: ManagedSqliteRegistryRouteHandle,
    ) -> Result<ManagedSqliteRegistryFileLease, ManagedSqliteRegistryProcessRouteRejection> {
        self.apply_route(route, |routes| routes.claim_main(route))
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::registry) fn claim_sidecar(
        &self,
        route: ManagedSqliteRegistryRouteHandle,
        role: ManagedSqliteLogicalFileRole,
    ) -> Result<ManagedSqliteRegistryFileLease, ManagedSqliteRegistryProcessRouteRejection> {
        self.apply_route(route, |routes| routes.claim_sidecar(route, role))
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::registry) fn claim_shm(
        &self,
        route: ManagedSqliteRegistryRouteHandle,
    ) -> Result<ManagedSqliteRegistryShmLease, ManagedSqliteRegistryProcessRouteRejection> {
        self.apply_route(route, |routes| routes.claim_shm(route))
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::registry) fn activate_connection(
        &self,
        route: ManagedSqliteRegistryRouteHandle,
    ) -> Result<(), ManagedSqliteRegistryProcessRouteRejection> {
        self.apply_route(route, |routes| routes.activate_connection(route))
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::registry) fn close_sidecar(
        &self,
        route: ManagedSqliteRegistryRouteHandle,
        lease: ManagedSqliteRegistryFileLease,
        receipt: ManagedSqliteFileCloseReceipt,
    ) -> Result<(), ManagedSqliteRegistryProcessRouteRejection> {
        let outcome = lease.close_with_file_receipt(receipt);
        self.apply_route_retaining_failure(route, (lease, outcome), |routes, evidence| {
            routes.close_file(route, &evidence.0, &evidence.1)
        })
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::registry) fn close_main(
        &self,
        route: ManagedSqliteRegistryRouteHandle,
        lease: ManagedSqliteRegistryFileLease,
        receipt: ManagedSqliteMainFileCloseReceipt,
    ) -> Result<(), ManagedSqliteRegistryProcessRouteRejection> {
        let outcome = lease.close_with_main_receipt(receipt);
        self.apply_route_retaining_failure(route, (lease, outcome), |routes, evidence| {
            routes.close_file(route, &evidence.0, &evidence.1)
        })
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::registry) fn close_wal_main(
        &self,
        route: ManagedSqliteRegistryRouteHandle,
        main: ManagedSqliteRegistryFileLease,
        shm: ManagedSqliteRegistryShmLease,
        receipt: ManagedSqliteWalMainCloseReceipt,
    ) -> Result<(), ManagedSqliteRegistryProcessRouteRejection> {
        let proofs = ManagedSqliteRegistryWalMainCloseProofs::from_receipt(&main, &shm, receipt);
        let (main_outcome, shm_outcome) = match proofs {
            Ok(proofs) => {
                let (main_proof, shm_proof) = proofs.into_parts();
                (
                    ManagedSqliteRegistryCloseOutcome::Proven(main_proof),
                    ManagedSqliteRegistryCloseOutcome::Proven(shm_proof),
                )
            }
            Err(reason) => (
                ManagedSqliteRegistryCloseOutcome::Unproven(reason),
                ManagedSqliteRegistryCloseOutcome::Unproven(reason),
            ),
        };
        self.apply_route_retaining_failure(
            route,
            (main, shm, main_outcome, shm_outcome),
            |routes, evidence| {
                routes.close_wal_main(route, &evidence.0, &evidence.1, &evidence.2, &evidence.3)
            },
        )
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::registry) fn connection_close_failed(
        &self,
        route: ManagedSqliteRegistryRouteHandle,
        reason: ManagedSqliteRegistryTerminalReason,
    ) -> Result<(), ManagedSqliteRegistryProcessRouteRejection> {
        self.apply_route(route, |routes| {
            routes.connection_close_failed(route, reason)
        })
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::registry) fn observe_connection_closed(
        &self,
        route: ManagedSqliteRegistryRouteHandle,
    ) -> Result<(), ManagedSqliteRegistryProcessRouteRejection> {
        self.apply_route(route, |routes| routes.observe_connection_closed(route))
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::registry) fn retire_closed(
        &self,
        route: ManagedSqliteRegistryRouteHandle,
    ) -> Result<ManagedSqliteRegistryRetirementReceipt, ManagedSqliteRegistryProcessRouteRejection>
    {
        self.apply_route(route, |routes| routes.retire_closed(route))
    }
}
