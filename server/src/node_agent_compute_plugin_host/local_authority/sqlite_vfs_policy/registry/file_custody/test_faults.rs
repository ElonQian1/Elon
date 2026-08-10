//! Test-only installation into exact live WAL-main custody.
//!
//! The adapter receives only a validated script. Runtime generation, SHM connection identity and
//! the pinned WAL-main file remain private to this custody layer.

use super::{ManagedSqliteRegistryPinnedFile, ManagedSqliteRegistryPinnedFileCustody};
use crate::node_agent_managed_fs::{
    ManagedSqliteShmFailureClass, ManagedSqliteShmFailurePhase, ManagedSqliteShmTestFaultProbe,
};

use super::super::{
    owner::ManagedSqliteRegistryCustody, process_owner::ManagedSqliteRegistryNonceSource,
    types::ManagedSqliteRegistryTerminalReason,
};

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
