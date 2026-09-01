//! Test-only installation into exact live WAL-main custody.
//!
//! The adapter receives only a validated script. Runtime generation, SHM connection identity and
//! the pinned WAL-main file remain private to this custody layer.

use std::sync::Arc;

#[cfg(windows)]
use super::ManagedSqliteRegistryLifecycleStage;
use super::{
    ManagedSqliteRegistryPinnedFile, ManagedSqliteRegistryPinnedFileCloseRejection,
    ManagedSqliteRegistryPinnedFileCustody, ManagedSqliteRegistryUnmapRuntimeEvent,
};
use crate::node_agent_managed_fs::{
    ManagedSqliteShmFailureClass, ManagedSqliteShmFailurePhase, ManagedSqliteShmTestFaultProbe,
    ManagedSqliteShmTestTargetObserver, ManagedSqliteWalMainCloseReceipt,
};
#[cfg(windows)]
use crate::node_agent_managed_fs::{
    ManagedSqliteShmLockAttempt, ManagedSqliteShmLockRequest, PinnedManagedSqliteFile,
};

use super::super::{
    owner::{ManagedSqliteRegistryCustody, ManagedSqliteRegistryRouteHandle},
    process_owner::{ManagedSqliteRegistryNonceSource, ManagedSqliteRegistryProcessOwner},
    types::{
        ManagedSqliteRegistryFileLease, ManagedSqliteRegistryRetirementReceipt,
        ManagedSqliteRegistryShmLease, ManagedSqliteRegistryTerminalReason,
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in super::super::super) enum ManagedSqliteRegistryCloseLifecyclePhase {
    BarrierCallbackCompletion,
    UnmapCallbackCompletion,
    RegistryWalMainClose,
    CallbackCompletion,
    ConnectionObservation,
    RouteRetirement,
}

/// Ordered, redacted proof emitted only by the exact test-only unsafe-retention preemption path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in super::super::super) struct ManagedSqliteRegistryUnsafeShmRoutePreemptionReceipt {
    preemption_retained: bool,
    unsafe_retention_route_unknown: bool,
    callback_completion_route_unknown: bool,
}

impl ManagedSqliteRegistryUnsafeShmRoutePreemptionReceipt {
    pub(in super::super::super) const fn new(
        preemption_retained: bool,
        unsafe_retention_route_unknown: bool,
        callback_completion_route_unknown: bool,
    ) -> Self {
        Self {
            preemption_retained,
            unsafe_retention_route_unknown,
            callback_completion_route_unknown,
        }
    }

    pub(in super::super::super) const fn ordered_values(self) -> [u64; 3] {
        [
            self.preemption_retained as u64,
            self.unsafe_retention_route_unknown as u64,
            self.callback_completion_route_unknown as u64,
        ]
    }
}

/// Ordered proof that an exact ordinary Lock result preceded test-only terminal preemption.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in super::super::super) struct ManagedSqliteRegistryOrdinaryShmLockRoutePreemptionReceipt {
    request_matched: bool,
    lower_outcome_matched: bool,
    preemption_retained: bool,
    callback_completion_route_unknown: bool,
}

impl ManagedSqliteRegistryOrdinaryShmLockRoutePreemptionReceipt {
    pub(in super::super::super) const fn new(
        request_matched: bool,
        lower_outcome_matched: bool,
        preemption_retained: bool,
        callback_completion_route_unknown: bool,
    ) -> Self {
        Self {
            request_matched,
            lower_outcome_matched,
            preemption_retained,
            callback_completion_route_unknown,
        }
    }

    pub(in super::super::super) const fn ordered_values(self) -> [u64; 4] {
        [
            self.request_matched as u64,
            self.lower_outcome_matched as u64,
            self.preemption_retained as u64,
            self.callback_completion_route_unknown as u64,
        ]
    }
}

pub(in super::super::super) trait ManagedSqliteRegistryCloseLifecycleFaults:
    Send + Sync + 'static
{
    fn before(&self, phase: ManagedSqliteRegistryCloseLifecyclePhase) -> Result<bool, ()>;
    fn after_success(&self, phase: ManagedSqliteRegistryCloseLifecyclePhase) -> Result<bool, ()>;
    fn native_failure(&self, phase: ManagedSqliteRegistryCloseLifecyclePhase);
    fn observe_unmap_runtime_event(
        &self,
        event: ManagedSqliteRegistryUnmapRuntimeEvent,
    ) -> Result<(), ()>;
    fn unmap_runtime_observation_enabled(&self) -> Result<bool, ()>;
    fn claim_native_failure_gate(
        &self,
        phase: ManagedSqliteRegistryCloseLifecyclePhase,
    ) -> Result<bool, ()>;
    fn publish_retirement(&self, receipt: ManagedSqliteRegistryRetirementReceipt)
        -> Result<(), ()>;
    fn retain_retirement_failure(&self, receipt: ManagedSqliteRegistryRetirementReceipt);

    #[cfg(windows)]
    fn claim_unsafe_shm_route_preemption(&self) -> Result<bool, ()> {
        Ok(false)
    }

    #[cfg(windows)]
    fn record_unsafe_shm_route_preemption_receipt(
        &self,
        receipt: ManagedSqliteRegistryUnsafeShmRoutePreemptionReceipt,
    ) -> Result<(), ()> {
        let _ = receipt;
        Ok(())
    }

    #[cfg(windows)]
    fn claim_ordinary_shm_lock_route_preemption(
        &self,
        request: ManagedSqliteShmLockRequest,
        outcome: ManagedSqliteShmLockAttempt,
    ) -> Result<bool, ()> {
        let _ = (request, outcome);
        Ok(false)
    }

    #[cfg(windows)]
    fn record_ordinary_shm_lock_route_preemption_receipt(
        &self,
        receipt: ManagedSqliteRegistryOrdinaryShmLockRoutePreemptionReceipt,
    ) -> Result<(), ()> {
        let _ = receipt;
        Ok(())
    }

    #[cfg(windows)]
    fn take_connection_observation_sidecar(&self) -> Result<Option<PinnedManagedSqliteFile>, ()>;

    #[cfg(windows)]
    fn observe_registry_lifecycle_stage(
        &self,
        stage: ManagedSqliteRegistryLifecycleStage,
    ) -> Result<(), ()>;

    #[cfg(windows)]
    fn claim_physical_success_handoff(&self) -> Result<bool, ()> {
        Ok(false)
    }

    #[cfg(windows)]
    fn claim_registry_wal_main_native_uncertain(&self) -> Result<bool, ()> {
        Ok(false)
    }

    #[cfg(windows)]
    fn claim_close_callback_admission_rejection(&self) -> Result<bool, ()> {
        Ok(false)
    }

    #[cfg(windows)]
    fn claim_begin_connection_close_rejection(&self) -> Result<bool, ()> {
        Ok(false)
    }
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
