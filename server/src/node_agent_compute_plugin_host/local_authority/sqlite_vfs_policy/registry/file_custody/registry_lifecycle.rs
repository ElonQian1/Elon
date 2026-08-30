//! Redacted stage vocabulary and typed close-callback helpers for lifecycle evidence tests.

use std::sync::Arc;

use super::{
    ManagedSqliteRegistryCloseLifecycleFaults, ManagedSqliteRegistryCloseLifecyclePhase,
    ManagedSqliteRegistryPinnedFileCloseRejection,
};
use crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::registry::{
    owner::{ManagedSqliteRegistryCustody, ManagedSqliteRegistryRouteHandle},
    process_owner::{
        ManagedSqliteRegistryNonceSource, ManagedSqliteRegistryProcessOwner,
        ManagedSqliteRegistryRoutedCallbackLease,
    },
    types::{ManagedSqliteRegistryFileLease, ManagedSqliteRegistryShmLease},
};
use crate::node_agent_managed_fs::ManagedSqliteWalMainCloseReceipt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in super::super::super) enum ManagedSqliteRegistryLifecycleStage {
    OutstandingSidecarRetained,
    RawCloseEntered,
    CallbackBegin,
    PhysicalCloseSucceeded,
    RegistryWalMainCloseAttempt,
    RegistryWalMainCloseSucceeded,
    CallbackCompletionAttempt,
    CallbackCompletionSucceeded,
    ConnectionObservationAttempt,
    ConnectionObservationSucceeded,
    RouteRetirementAttempt,
    RouteRetirementSucceeded,
    RetirementPublishAttempt,
    RetirementPublishSucceeded,
    RetirementClaimAttempt,
    RetirementClaimSucceeded,
    LogicalRemovalAttempt,
    LogicalRemovalSucceeded { removed_names: u8 },
}

impl ManagedSqliteRegistryLifecycleStage {
    pub(in super::super::super) const fn order(self) -> u8 {
        match self {
            Self::RawCloseEntered => 1,
            Self::CallbackBegin => 2,
            Self::PhysicalCloseSucceeded => 3,
            Self::RegistryWalMainCloseAttempt => 4,
            Self::RegistryWalMainCloseSucceeded => 5,
            Self::CallbackCompletionAttempt => 6,
            Self::CallbackCompletionSucceeded => 7,
            Self::OutstandingSidecarRetained => 8,
            Self::ConnectionObservationAttempt => 9,
            Self::ConnectionObservationSucceeded => 10,
            Self::RouteRetirementAttempt => 11,
            Self::RouteRetirementSucceeded => 12,
            Self::RetirementPublishAttempt => 13,
            Self::RetirementPublishSucceeded => 14,
            Self::RetirementClaimAttempt => 15,
            Self::RetirementClaimSucceeded => 16,
            Self::LogicalRemovalAttempt => 17,
            Self::LogicalRemovalSucceeded { .. } => 18,
        }
    }
}

pub(super) fn observe(
    faults: Option<&Arc<dyn ManagedSqliteRegistryCloseLifecycleFaults>>,
    stage: ManagedSqliteRegistryLifecycleStage,
) -> Result<(), ManagedSqliteRegistryPinnedFileCloseRejection> {
    if faults.is_some_and(|faults| faults.observe_registry_lifecycle_stage(stage).is_err()) {
        Err(ManagedSqliteRegistryPinnedFileCloseRejection::InjectedLifecycle)
    } else {
        Ok(())
    }
}

pub(super) fn arm_close_completion_native<Custody, NonceSource>(
    faults: Option<&Arc<dyn ManagedSqliteRegistryCloseLifecycleFaults>>,
    callback: &mut ManagedSqliteRegistryRoutedCallbackLease<Custody, NonceSource>,
) -> Result<(), ManagedSqliteRegistryPinnedFileCloseRejection>
where
    Custody: ManagedSqliteRegistryCustody + 'static,
    NonceSource: ManagedSqliteRegistryNonceSource + 'static,
{
    let arm = match faults {
        Some(faults) => faults
            .claim_native_failure_gate(ManagedSqliteRegistryCloseLifecyclePhase::CallbackCompletion)
            .map_err(|()| ManagedSqliteRegistryPinnedFileCloseRejection::InjectedLifecycle)?,
        None => false,
    };
    if arm {
        callback
            .arm_close_callback_completion_native_rejection()
            .map_err(ManagedSqliteRegistryPinnedFileCloseRejection::Registry)?;
    }
    Ok(())
}

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
    if let Err(error) = observe(
        faults,
        ManagedSqliteRegistryLifecycleStage::PhysicalCloseSucceeded,
    ) {
        let _ = owner.retain_terminal_wal_main_physical_custody(
            route,
            crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::registry::types::ManagedSqliteRegistryTerminalReason::FailureCustodyRetained,
            (receipt, main, shm),
        );
        return Err(error);
    }
    if faults.is_some_and(|faults| {
        faults
            .before(ManagedSqliteRegistryCloseLifecyclePhase::RegistryWalMainClose)
            .unwrap_or(true)
    }) {
        let _ = owner.retain_terminal_wal_main_physical_custody(
            route,
            crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::registry::types::ManagedSqliteRegistryTerminalReason::FailureCustodyRetained,
            (receipt, main, shm),
        );
        return Err(ManagedSqliteRegistryPinnedFileCloseRejection::InjectedLifecycle);
    }
    if let Err(error) = observe(
        faults,
        ManagedSqliteRegistryLifecycleStage::RegistryWalMainCloseAttempt,
    ) {
        let _ = owner.retain_terminal_wal_main_physical_custody(
            route,
            crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::registry::types::ManagedSqliteRegistryTerminalReason::FailureCustodyRetained,
            (receipt, main, shm),
        );
        return Err(error);
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
            observe(
                faults,
                ManagedSqliteRegistryLifecycleStage::RegistryWalMainCloseSucceeded,
            )?;
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
