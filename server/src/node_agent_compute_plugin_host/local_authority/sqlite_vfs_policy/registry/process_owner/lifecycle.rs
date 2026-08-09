use super::*;

impl<Custody, NonceSource> ManagedSqliteRegistryProcessOwner<Custody, NonceSource>
where
    Custody: ManagedSqliteRegistryCustody + 'static,
    NonceSource: ManagedSqliteRegistryNonceSource + 'static,
{
    pub(super) fn claim_main(
        &self,
        route: ManagedSqliteRegistryRouteHandle,
    ) -> Result<ManagedSqliteRegistryFileLease, ManagedSqliteRegistryProcessRouteRejection> {
        self.apply_route(route, |routes| routes.claim_main(route))
    }

    pub(super) fn claim_sidecar(
        &self,
        route: ManagedSqliteRegistryRouteHandle,
        role: ManagedSqliteLogicalFileRole,
    ) -> Result<ManagedSqliteRegistryFileLease, ManagedSqliteRegistryProcessRouteRejection> {
        self.apply_route(route, |routes| routes.claim_sidecar(route, role))
    }

    pub(super) fn claim_shm(
        &self,
        route: ManagedSqliteRegistryRouteHandle,
    ) -> Result<ManagedSqliteRegistryShmLease, ManagedSqliteRegistryProcessRouteRejection> {
        self.apply_route(route, |routes| routes.claim_shm(route))
    }

    pub(super) fn activate_connection(
        &self,
        route: ManagedSqliteRegistryRouteHandle,
    ) -> Result<(), ManagedSqliteRegistryProcessRouteRejection> {
        self.apply_route(route, |routes| routes.activate_connection(route))
    }

    pub(super) fn close_sidecar(
        &self,
        route: ManagedSqliteRegistryRouteHandle,
        lease: ManagedSqliteRegistryFileLease,
        receipt: ManagedSqliteFileCloseReceipt,
    ) -> Result<(), ManagedSqliteRegistryProcessRouteRejection> {
        let outcome = lease.close_with_file_receipt(receipt);
        self.apply_route(route, |routes| routes.close_file(route, lease, outcome))
    }

    pub(super) fn close_main(
        &self,
        route: ManagedSqliteRegistryRouteHandle,
        lease: ManagedSqliteRegistryFileLease,
        receipt: ManagedSqliteMainFileCloseReceipt,
    ) -> Result<(), ManagedSqliteRegistryProcessRouteRejection> {
        let outcome = lease.close_with_main_receipt(receipt);
        self.apply_route(route, |routes| routes.close_file(route, lease, outcome))
    }

    pub(super) fn close_wal_main(
        &self,
        route: ManagedSqliteRegistryRouteHandle,
        main: ManagedSqliteRegistryFileLease,
        shm: ManagedSqliteRegistryShmLease,
        receipt: ManagedSqliteWalMainCloseReceipt,
    ) -> Result<(), ManagedSqliteRegistryProcessRouteRejection> {
        let proofs = ManagedSqliteRegistryWalMainCloseProofs::from_receipt(&main, &shm, receipt);
        self.apply_route(route, |routes| {
            routes.close_wal_main(route, main, shm, proofs)
        })
    }

    pub(super) fn connection_close_failed(
        &self,
        route: ManagedSqliteRegistryRouteHandle,
        reason: ManagedSqliteRegistryTerminalReason,
    ) -> Result<(), ManagedSqliteRegistryProcessRouteRejection> {
        self.apply_route(route, |routes| {
            routes.connection_close_failed(route, reason)
        })
    }

    pub(super) fn observe_connection_closed(
        &self,
        route: ManagedSqliteRegistryRouteHandle,
    ) -> Result<(), ManagedSqliteRegistryProcessRouteRejection> {
        self.apply_route(route, |routes| routes.observe_connection_closed(route))
    }

    pub(super) fn retire_closed(
        &self,
        route: ManagedSqliteRegistryRouteHandle,
    ) -> Result<ManagedSqliteRegistryRetirementReceipt, ManagedSqliteRegistryProcessRouteRejection>
    {
        self.apply_route(route, |routes| routes.retire_closed(route))
    }
}
