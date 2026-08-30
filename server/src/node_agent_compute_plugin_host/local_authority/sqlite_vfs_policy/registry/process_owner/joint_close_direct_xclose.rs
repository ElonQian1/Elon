//! Exact process-owner seam for Registry consumption after a real direct main `xClose`.

use super::lifecycle::ManagedSqliteRegistryTerminalCustodyTestRetentionKind;
use super::*;

impl<Custody, NonceSource> ManagedSqliteRegistryProcessOwner<Custody, NonceSource>
where
    Custody: ManagedSqliteRegistryCustody + 'static,
    NonceSource: ManagedSqliteRegistryNonceSource + 'static,
{
    /// WindowsDynamic-only registry consumption for the exact direct-main-xClose topology. The
    /// ordinary production transition remains strict about closing every sidecar before main.
    pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::registry) fn close_wal_main_after_direct_xclose(
        &self,
        route: ManagedSqliteRegistryRouteHandle,
        main: ManagedSqliteRegistryFileLease,
        shm: ManagedSqliteRegistryShmLease,
        receipt: ManagedSqliteWalMainCloseReceipt,
    ) -> Result<(), ManagedSqliteRegistryProcessRouteRejection> {
        match self.claim_registry_wal_main_native_uncertain(route) {
            Ok(true) => {
                let _ = self.retain_terminal_wal_main_physical_custody(
                    route,
                    ManagedSqliteRegistryTerminalReason::FailureCustodyRetained,
                    (receipt, main, shm),
                );
                return Err(
                    ManagedSqliteRegistryProcessRouteRejection::RegistryWalMainNativeUncertain,
                );
            }
            Ok(false) => {}
            Err(rejection) => {
                let _ = self.retain_terminal_wal_main_physical_custody(
                    route,
                    ManagedSqliteRegistryTerminalReason::FailureCustodyRetained,
                    (receipt, main, shm),
                );
                return Err(rejection);
            }
        }
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
            ManagedSqliteRegistryTerminalCustodyTestRetentionKind::CompletionEvidence,
            |routes, evidence| {
                routes.close_wal_main_after_direct_xclose(
                    route,
                    &evidence.0,
                    &evidence.1,
                    &evidence.2,
                    &evidence.3,
                )
            },
        )
    }
}
