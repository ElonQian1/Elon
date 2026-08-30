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
        ManagedSqliteRegistryProcessRouteRejection, ManagedSqliteRegistryRoutedCallbackLease,
    },
    types::{
        ManagedSqliteRegistryCallbackCompletionReceipt, ManagedSqliteRegistryFileLease,
        ManagedSqliteRegistryShmLease, ManagedSqliteRegistryTerminalReason,
    },
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

/// Exact physical receipt custody which must remain on the live Closing route until its already
/// admitted Close callback has produced a completion receipt. Drop fail-closes if control exits
/// before the callback result can be paired with this linear custody.
#[must_use = "deferred WAL-main terminal custody must be explicitly retained"]
pub(super) struct ManagedSqliteRegistryDeferredWalMainTerminal<Custody, NonceSource>
where
    Custody: ManagedSqliteRegistryCustody + 'static,
    NonceSource: ManagedSqliteRegistryNonceSource + 'static,
{
    owner: &'static ManagedSqliteRegistryProcessOwner<Custody, NonceSource>,
    route: ManagedSqliteRegistryRouteHandle,
    physical: Option<(
        ManagedSqliteWalMainCloseReceipt,
        ManagedSqliteRegistryFileLease,
        ManagedSqliteRegistryShmLease,
    )>,
}

impl<Custody, NonceSource> ManagedSqliteRegistryDeferredWalMainTerminal<Custody, NonceSource>
where
    Custody: ManagedSqliteRegistryCustody + 'static,
    NonceSource: ManagedSqliteRegistryNonceSource + 'static,
{
    fn new(
        owner: &'static ManagedSqliteRegistryProcessOwner<Custody, NonceSource>,
        route: ManagedSqliteRegistryRouteHandle,
        receipt: ManagedSqliteWalMainCloseReceipt,
        main: ManagedSqliteRegistryFileLease,
        shm: ManagedSqliteRegistryShmLease,
    ) -> Self {
        Self {
            owner,
            route,
            physical: Some((receipt, main, shm)),
        }
    }

    pub(super) fn retain(mut self) -> Result<(), ManagedSqliteRegistryProcessRouteRejection> {
        let physical = self.physical.take().expect("deferred WAL-main custody");
        self.owner.retain_terminal_wal_main_physical_custody(
            self.route,
            ManagedSqliteRegistryTerminalReason::FailureCustodyRetained,
            physical,
        )
    }

    pub(super) fn retain_with_callback_completion(
        mut self,
        callback: ManagedSqliteRegistryCallbackCompletionReceipt,
    ) -> Result<(), ManagedSqliteRegistryProcessRouteRejection> {
        let physical = self.physical.take().expect("deferred WAL-main custody");
        self.owner
            .retain_terminal_wal_main_with_callback_completion(self.route, physical, callback)
    }
}

impl<Custody, NonceSource> Drop
    for ManagedSqliteRegistryDeferredWalMainTerminal<Custody, NonceSource>
where
    Custody: ManagedSqliteRegistryCustody + 'static,
    NonceSource: ManagedSqliteRegistryNonceSource + 'static,
{
    fn drop(&mut self) {
        if let Some(physical) = self.physical.take() {
            let _ = self.owner.retain_terminal_wal_main_physical_custody(
                self.route,
                ManagedSqliteRegistryTerminalReason::FailureCustodyRetained,
                physical,
            );
        }
    }
}

pub(super) enum ManagedSqliteRegistryWalMainAfterPhysicalFailure<Custody, NonceSource>
where
    Custody: ManagedSqliteRegistryCustody + 'static,
    NonceSource: ManagedSqliteRegistryNonceSource + 'static,
{
    Rejected(ManagedSqliteRegistryPinnedFileCloseRejection),
    CompleteCallbackBeforeTerminal(
        ManagedSqliteRegistryDeferredWalMainTerminal<Custody, NonceSource>,
    ),
}

#[allow(clippy::too_many_arguments)]
pub(super) fn close_wal_main_after_physical<Custody, NonceSource>(
    owner: &'static ManagedSqliteRegistryProcessOwner<Custody, NonceSource>,
    route: ManagedSqliteRegistryRouteHandle,
    faults: Option<&Arc<dyn ManagedSqliteRegistryCloseLifecycleFaults>>,
    receipt: ManagedSqliteWalMainCloseReceipt,
    main: ManagedSqliteRegistryFileLease,
    shm: ManagedSqliteRegistryShmLease,
) -> Result<(), ManagedSqliteRegistryWalMainAfterPhysicalFailure<Custody, NonceSource>>
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
        return Err(ManagedSqliteRegistryWalMainAfterPhysicalFailure::Rejected(
            error,
        ));
    }
    if faults.is_some_and(|faults| {
        faults
            .before(ManagedSqliteRegistryCloseLifecyclePhase::RegistryWalMainClose)
            .unwrap_or(true)
    }) {
        return Err(
            ManagedSqliteRegistryWalMainAfterPhysicalFailure::CompleteCallbackBeforeTerminal(
                ManagedSqliteRegistryDeferredWalMainTerminal::new(owner, route, receipt, main, shm),
            ),
        );
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
        return Err(ManagedSqliteRegistryWalMainAfterPhysicalFailure::Rejected(
            error,
        ));
    }
    let arm_native = faults
        .map(|faults| faults.claim_registry_wal_main_native_uncertain())
        .transpose();
    match arm_native {
        Ok(Some(true)) => {
            if owner.arm_registry_wal_main_native_uncertain(route).is_err() {
                let _ = owner.retain_terminal_wal_main_physical_custody(
                    route,
                    crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::registry::types::ManagedSqliteRegistryTerminalReason::FailureCustodyRetained,
                    (receipt, main, shm),
                );
                return Err(ManagedSqliteRegistryWalMainAfterPhysicalFailure::Rejected(
                    ManagedSqliteRegistryPinnedFileCloseRejection::InjectedLifecycle,
                ));
            }
        }
        Ok(Some(false) | None) => {}
        Err(()) => {
            let _ = owner.retain_terminal_wal_main_physical_custody(
                route,
                crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::registry::types::ManagedSqliteRegistryTerminalReason::FailureCustodyRetained,
                (receipt, main, shm),
            );
            return Err(ManagedSqliteRegistryWalMainAfterPhysicalFailure::Rejected(
                ManagedSqliteRegistryPinnedFileCloseRejection::InjectedLifecycle,
            ));
        }
    }
    match owner.close_wal_main_after_direct_xclose(route, main, shm, receipt) {
        Err(rejection) => {
            if let Some(faults) = faults {
                faults
                    .native_failure(ManagedSqliteRegistryCloseLifecyclePhase::RegistryWalMainClose);
            }
            Err(ManagedSqliteRegistryWalMainAfterPhysicalFailure::Rejected(
                ManagedSqliteRegistryPinnedFileCloseRejection::Registry(rejection),
            ))
        }
        Ok(()) => {
            observe(
                faults,
                ManagedSqliteRegistryLifecycleStage::RegistryWalMainCloseSucceeded,
            )
            .map_err(ManagedSqliteRegistryWalMainAfterPhysicalFailure::Rejected)?;
            if faults.is_some_and(|faults| {
                faults
                    .after_success(ManagedSqliteRegistryCloseLifecyclePhase::RegistryWalMainClose)
                    .unwrap_or(true)
            }) {
                Err(ManagedSqliteRegistryWalMainAfterPhysicalFailure::Rejected(
                    ManagedSqliteRegistryPinnedFileCloseRejection::InjectedLifecycle,
                ))
            } else {
                Ok(())
            }
        }
    }
}
