//! Test-only installation into exact live WAL-main custody.
//!
//! The adapter receives only a validated script. Runtime generation, SHM connection identity and
//! the pinned WAL-main file remain private to this custody layer.

use std::sync::Arc;

use super::{
    ManagedSqliteRegistryCloseLifecycleFaults, ManagedSqliteRegistryCloseLifecyclePhase,
    ManagedSqliteRegistryPinnedFile, ManagedSqliteRegistryPinnedFileCloseRejection,
    ManagedSqliteRegistryPinnedFileCustody,
};
use crate::node_agent_managed_fs::{
    ManagedSqliteShmFailureClass, ManagedSqliteShmFailurePhase, ManagedSqliteShmTestFaultProbe,
    ManagedSqliteShmTestTargetObserver, ManagedSqliteWalMainCloseReceipt,
};

use super::super::{
    owner::{ManagedSqliteRegistryCustody, ManagedSqliteRegistryRouteHandle},
    process_owner::{ManagedSqliteRegistryNonceSource, ManagedSqliteRegistryProcessOwner},
    types::{
        ManagedSqliteRegistryFileLease, ManagedSqliteRegistryShmLease,
        ManagedSqliteRegistryTerminalReason,
    },
};

#[allow(clippy::too_many_arguments)]
pub(super) fn close_wal_main_after_physical<Custody, NonceSource>(
    owner: &'static ManagedSqliteRegistryProcessOwner<Custody, NonceSource>,
    route: ManagedSqliteRegistryRouteHandle,
    faults: Option<&Arc<dyn ManagedSqliteRegistryCloseLifecycleFaults>>,
    receipt: ManagedSqliteWalMainCloseReceipt,
    main: ManagedSqliteRegistryFileLease,
    shm: ManagedSqliteRegistryShmLease,
) -> Result<(), ManagedSqliteRegistryPinnedFileCloseRejection>
where
    Custody: ManagedSqliteRegistryCustody + 'static,
    NonceSource: ManagedSqliteRegistryNonceSource + 'static,
{
    if faults.is_some_and(|faults| {
        faults
            .before(ManagedSqliteRegistryCloseLifecyclePhase::RegistryWalMainClose)
            .unwrap_or(true)
    }) {
        let _retained_registry_close_evidence = Box::leak(Box::new((receipt, main, shm)));
        return Err(ManagedSqliteRegistryPinnedFileCloseRejection::InjectedLifecycle);
    }
    match owner.close_wal_main(route, main, shm, receipt) {
        Err(rejection) => {
            if let Some(faults) = faults {
                faults
                    .native_failure(ManagedSqliteRegistryCloseLifecyclePhase::RegistryWalMainClose);
            }
            Err(ManagedSqliteRegistryPinnedFileCloseRejection::Registry(
                rejection,
            ))
        }
        Ok(()) => {
            if faults.is_some_and(|faults| {
                faults
                    .after_success(ManagedSqliteRegistryCloseLifecyclePhase::RegistryWalMainClose)
                    .unwrap_or(true)
            }) {
                Err(ManagedSqliteRegistryPinnedFileCloseRejection::InjectedLifecycle)
            } else {
                Ok(())
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ManagedSqliteRegistryWalMainTestFaultRejection {
    CustodyMissing,
    NotWalMain,
    ScriptRejected(&'static str),
}

impl<Custody, NonceSource> ManagedSqliteRegistryPinnedFile<Custody, NonceSource>
where
    Custody: ManagedSqliteRegistryCustody + 'static,
    NonceSource: ManagedSqliteRegistryNonceSource + 'static,
{
    pub(super) fn exact_wal_main_shm_test_target_observer(
        &self,
    ) -> Result<ManagedSqliteShmTestTargetObserver, ManagedSqliteRegistryWalMainTestFaultRejection>
    {
        match self.custody.as_ref() {
            Some(ManagedSqliteRegistryPinnedFileCustody::WalMain { file, .. }) => file
                .test_shm_target_observer()
                .map_err(ManagedSqliteRegistryWalMainTestFaultRejection::ScriptRejected),
            Some(_) => Err(ManagedSqliteRegistryWalMainTestFaultRejection::NotWalMain),
            None => Err(ManagedSqliteRegistryWalMainTestFaultRejection::CustodyMissing),
        }
    }

    pub(super) fn install_exact_wal_main_shm_test_fault_script(
        &mut self,
        before_call: &[(ManagedSqliteShmFailurePhase, u32)],
        after_success: &[(
            ManagedSqliteShmFailurePhase,
            u32,
            ManagedSqliteShmFailureClass,
        )],
    ) -> Result<ManagedSqliteShmTestFaultProbe, ManagedSqliteRegistryWalMainTestFaultRejection>
    {
        let result = match self.custody.as_ref() {
            Some(ManagedSqliteRegistryPinnedFileCustody::WalMain { file, .. }) => file
                .install_shm_test_fault_script(before_call, after_success)
                .map_err(ManagedSqliteRegistryWalMainTestFaultRejection::ScriptRejected),
            Some(_) => Err(ManagedSqliteRegistryWalMainTestFaultRejection::NotWalMain),
            None => Err(ManagedSqliteRegistryWalMainTestFaultRejection::CustodyMissing),
        };
        if let Err(rejection) = result.as_ref() {
            let _ = self.owner.retain_terminal_custody(
                self.route,
                ManagedSqliteRegistryTerminalReason::FailureCustodyRetained,
                *rejection,
            );
        }
        result
    }
}
