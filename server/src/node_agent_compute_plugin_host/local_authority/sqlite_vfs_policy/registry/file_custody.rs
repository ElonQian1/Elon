//! Linear pairing between managed-fs handles and exact registry leases.
//!
//! The private ABI adapter can own one of these values and route callbacks through its controlled
//! operations, but production xOpen still has no constructor for that state. Physical close
//! receipts are the only path that retires a lease. Any abandoned state or failed physical close
//! is retained for process lifetime before its exact route is quarantined.

#[cfg(test)]
use super::types::ManagedSqliteRegistryCallbackCompletionReceipt;
use super::{
    owner::{ManagedSqliteRegistryCustody, ManagedSqliteRegistryRouteHandle},
    process_owner::{
        ManagedSqliteRegistryNonceSource, ManagedSqliteRegistryProcessOwner,
        ManagedSqliteRegistryProcessRouteRejection,
    },
    types::{
        ManagedSqliteRegistryFileLease, ManagedSqliteRegistryShmLease,
        ManagedSqliteRegistryTerminalReason,
    },
};
use crate::{
    node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::ManagedSqliteLogicalFileRole,
    node_agent_managed_fs::{
        ManagedSqliteFileKind, ManagedSqliteWalMainCloseFailurePhase, PinnedManagedSqliteFile,
        PinnedManagedSqliteMainFile, PinnedManagedSqliteWalMainFile,
    },
};
#[cfg(test)]
use std::sync::Arc;

mod abi;
mod operations;
mod promotion;
#[cfg(all(test, windows))]
mod registry_lifecycle;
#[cfg(test)]
mod test_faults;

#[cfg(all(test, windows))]
pub(in super::super) use registry_lifecycle::ManagedSqliteRegistryLifecycleStage;

pub(in crate::node_agent_compute_plugin_host::local_authority) use abi::{
    ComputePluginHandleBoundSqliteAbiFile, HandleBoundSqliteAbiAttempt, HandleBoundSqliteAbiFile,
    HandleBoundSqliteAbiLockLevel, HandleBoundSqliteAbiShmLockAction, HandleBoundSqliteAbiShmMap,
    HandleBoundSqliteAbiUnlockLevel,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ManagedSqliteRegistryFilePairRejection {
    SidecarRoleMismatch,
    MainRoleMismatch,
}

#[derive(Debug)]
pub(super) enum ManagedSqliteRegistryPinnedFileCloseRejection {
    Physical {
        reason: ManagedSqliteRegistryTerminalReason,
        quarantine: Option<ManagedSqliteRegistryProcessRouteRejection>,
    },
    Registry(ManagedSqliteRegistryProcessRouteRejection),
    #[cfg(test)]
    InjectedLifecycle,
}

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in super::super) enum ManagedSqliteRegistryCloseLifecyclePhase {
    BarrierCallbackCompletion,
    RegistryWalMainClose,
    CallbackCompletion,
    ConnectionObservation,
    RouteRetirement,
}

#[cfg(test)]
pub(in super::super) trait ManagedSqliteRegistryCloseLifecycleFaults:
    Send + Sync + 'static
{
    fn before(&self, phase: ManagedSqliteRegistryCloseLifecyclePhase) -> Result<bool, ()>;
    fn after_success(&self, phase: ManagedSqliteRegistryCloseLifecyclePhase) -> Result<bool, ()>;
    fn native_failure(&self, phase: ManagedSqliteRegistryCloseLifecyclePhase);

    fn claim_native_failure_gate(
        &self,
        phase: ManagedSqliteRegistryCloseLifecyclePhase,
    ) -> Result<bool, ()>;

    fn publish_retirement(
        &self,
        receipt: super::types::ManagedSqliteRegistryRetirementReceipt,
    ) -> Result<(), ()>;

    fn retain_retirement_failure(
        &self,
        receipt: super::types::ManagedSqliteRegistryRetirementReceipt,
    );

    #[cfg(windows)]
    fn take_connection_observation_sidecar(&self) -> Result<Option<PinnedManagedSqliteFile>, ()>;

    #[cfg(windows)]
    fn observe_registry_lifecycle_stage(
        &self,
        stage: ManagedSqliteRegistryLifecycleStage,
    ) -> Result<(), ()>;
}

struct ManagedSqliteRegistryPinnedFileCloseSuccess {
    #[cfg(test)]
    callback: ManagedSqliteRegistryCallbackCompletionReceipt,
}

enum ManagedSqliteRegistryPinnedFileCustody {
    Sidecar {
        file: PinnedManagedSqliteFile,
        lease: ManagedSqliteRegistryFileLease,
    },
    Main {
        file: PinnedManagedSqliteMainFile,
        lease: ManagedSqliteRegistryFileLease,
    },
    WalMain {
        file: PinnedManagedSqliteWalMainFile,
        main: ManagedSqliteRegistryFileLease,
        shm: ManagedSqliteRegistryShmLease,
    },
}

/// The only future Rust state allowed behind one handle-bound `sqlite3_file.state` pointer.
///
/// The owner and exact route cannot be separated from physical handle and lease custody. Callers
/// must consume this state with `close`; dropping it fail-closes by retaining every component.
#[must_use = "pinned SQLite file state must be consumed by explicit close"]
pub(super) struct ManagedSqliteRegistryPinnedFile<Custody, NonceSource>
where
    Custody: ManagedSqliteRegistryCustody + 'static,
    NonceSource: ManagedSqliteRegistryNonceSource + 'static,
{
    owner: &'static ManagedSqliteRegistryProcessOwner<Custody, NonceSource>,
    route: ManagedSqliteRegistryRouteHandle,
    custody: Option<ManagedSqliteRegistryPinnedFileCustody>,
    #[cfg(test)]
    close_faults: Option<Arc<dyn ManagedSqliteRegistryCloseLifecycleFaults>>,
}

impl<Custody, NonceSource> ManagedSqliteRegistryPinnedFile<Custody, NonceSource>
where
    Custody: ManagedSqliteRegistryCustody + 'static,
    NonceSource: ManagedSqliteRegistryNonceSource + 'static,
{
    pub(super) fn bind_sidecar(
        owner: &'static ManagedSqliteRegistryProcessOwner<Custody, NonceSource>,
        route: ManagedSqliteRegistryRouteHandle,
        file: PinnedManagedSqliteFile,
        lease: ManagedSqliteRegistryFileLease,
    ) -> Result<Self, ManagedSqliteRegistryFilePairRejection> {
        let expected = match lease.role() {
            ManagedSqliteLogicalFileRole::Journal => ManagedSqliteFileKind::Journal,
            ManagedSqliteLogicalFileRole::Wal => ManagedSqliteFileKind::Wal,
            ManagedSqliteLogicalFileRole::Main => {
                let _ = owner.retain_terminal_custody(
                    route,
                    ManagedSqliteRegistryTerminalReason::LeaseIdentityMismatch,
                    (file, lease),
                );
                return Err(ManagedSqliteRegistryFilePairRejection::SidecarRoleMismatch);
            }
        };
        if file.kind() != expected {
            let _ = owner.retain_terminal_custody(
                route,
                ManagedSqliteRegistryTerminalReason::LeaseIdentityMismatch,
                (file, lease),
            );
            return Err(ManagedSqliteRegistryFilePairRejection::SidecarRoleMismatch);
        }
        Ok(Self {
            owner,
            route,
            custody: Some(ManagedSqliteRegistryPinnedFileCustody::Sidecar { file, lease }),
            #[cfg(test)]
            close_faults: None,
        })
    }

    pub(super) fn bind_main(
        owner: &'static ManagedSqliteRegistryProcessOwner<Custody, NonceSource>,
        route: ManagedSqliteRegistryRouteHandle,
        file: PinnedManagedSqliteMainFile,
        lease: ManagedSqliteRegistryFileLease,
    ) -> Result<Self, ManagedSqliteRegistryFilePairRejection> {
        if lease.role() != ManagedSqliteLogicalFileRole::Main {
            let _ = owner.retain_terminal_custody(
                route,
                ManagedSqliteRegistryTerminalReason::LeaseIdentityMismatch,
                (file, lease),
            );
            return Err(ManagedSqliteRegistryFilePairRejection::MainRoleMismatch);
        }
        Ok(Self {
            owner,
            route,
            custody: Some(ManagedSqliteRegistryPinnedFileCustody::Main { file, lease }),
            #[cfg(test)]
            close_faults: None,
        })
    }

    pub(super) fn bind_wal_main(
        owner: &'static ManagedSqliteRegistryProcessOwner<Custody, NonceSource>,
        route: ManagedSqliteRegistryRouteHandle,
        file: PinnedManagedSqliteWalMainFile,
        main: ManagedSqliteRegistryFileLease,
        shm: ManagedSqliteRegistryShmLease,
    ) -> Result<Self, ManagedSqliteRegistryFilePairRejection> {
        if main.role() != ManagedSqliteLogicalFileRole::Main {
            let _ = owner.retain_terminal_custody(
                route,
                ManagedSqliteRegistryTerminalReason::LeaseIdentityMismatch,
                (file, main, shm),
            );
            return Err(ManagedSqliteRegistryFilePairRejection::MainRoleMismatch);
        }
        Ok(Self {
            owner,
            route,
            custody: Some(ManagedSqliteRegistryPinnedFileCustody::WalMain { file, main, shm }),
            #[cfg(test)]
            close_faults: None,
        })
    }

    #[cfg(test)]
    pub(super) fn install_close_lifecycle_faults(
        &mut self,
        faults: Arc<dyn ManagedSqliteRegistryCloseLifecycleFaults>,
    ) -> Result<(), ()> {
        if self.close_faults.is_some() {
            return Err(());
        }
        self.close_faults = Some(faults);
        Ok(())
    }

    pub(super) fn close(self) -> Result<(), ManagedSqliteRegistryPinnedFileCloseRejection> {
        self.close_inner().map(drop)
    }

    #[cfg(test)]
    pub(super) fn close_with_callback_receipt(
        self,
    ) -> Result<
        ManagedSqliteRegistryCallbackCompletionReceipt,
        ManagedSqliteRegistryPinnedFileCloseRejection,
    > {
        self.close_inner().map(|success| success.callback)
    }

    fn close_inner(
        mut self,
    ) -> Result<
        ManagedSqliteRegistryPinnedFileCloseSuccess,
        ManagedSqliteRegistryPinnedFileCloseRejection,
    > {
        let callback = self
            .owner
            .begin_callback(
                self.route,
                super::types::ManagedSqliteRegistryCallbackKind::Close,
            )
            .map_err(ManagedSqliteRegistryPinnedFileCloseRejection::Registry)?;
        #[cfg(all(test, windows))]
        registry_lifecycle::observe(
            self.close_faults.as_ref(),
            ManagedSqliteRegistryLifecycleStage::CallbackBegin,
        )?;
        let custody = self
            .custody
            .take()
            .expect("live pinned file state must retain exact custody");
        let close = match custody {
            ManagedSqliteRegistryPinnedFileCustody::Sidecar { file, lease } => match file.close() {
                Ok(receipt) => self
                    .owner
                    .close_sidecar(self.route, lease, receipt)
                    .map_err(ManagedSqliteRegistryPinnedFileCloseRejection::Registry),
                Err(failure) => Err(self.retain_physical_failure(
                    ManagedSqliteRegistryTerminalReason::HandleCloseUnproven,
                    (failure, lease),
                )),
            },
            ManagedSqliteRegistryPinnedFileCustody::Main { file, lease } => match file.close() {
                Ok(receipt) => self
                    .owner
                    .close_main(self.route, lease, receipt)
                    .map_err(ManagedSqliteRegistryPinnedFileCloseRejection::Registry),
                Err(failure) => Err(self.retain_physical_failure(
                    ManagedSqliteRegistryTerminalReason::HandleCloseUnproven,
                    (failure, lease),
                )),
            },
            ManagedSqliteRegistryPinnedFileCustody::WalMain { file, main, shm } => {
                match file.close() {
                    Ok(receipt) => {
                        #[cfg(all(test, windows))]
                        {
                            registry_lifecycle::close_wal_main_after_physical(
                                self.owner,
                                self.route,
                                self.close_faults.as_ref(),
                                receipt,
                                main,
                                shm,
                            )
                        }
                        #[cfg(all(test, not(windows)))]
                        {
                            test_faults::close_wal_main_after_physical(
                                self.owner,
                                self.route,
                                self.close_faults.as_ref(),
                                receipt,
                                main,
                                shm,
                            )
                        }
                        #[cfg(not(test))]
                        {
                            self.owner
                                .close_wal_main(self.route, main, shm, receipt)
                                .map_err(ManagedSqliteRegistryPinnedFileCloseRejection::Registry)
                        }
                    }
                    Err(failure) => {
                        let reason = match failure.phase() {
                            ManagedSqliteWalMainCloseFailurePhase::ShmUnmap => {
                                ManagedSqliteRegistryTerminalReason::ShmTeardownUnproven
                            }
                            ManagedSqliteWalMainCloseFailurePhase::MainClose => {
                                ManagedSqliteRegistryTerminalReason::HandleCloseUnproven
                            }
                        };
                        Err(self.retain_physical_failure(reason, (failure, main, shm)))
                    }
                }
            }
        };
        #[cfg(test)]
        if self.close_faults.as_ref().is_some_and(|faults| {
            faults
                .before(ManagedSqliteRegistryCloseLifecyclePhase::CallbackCompletion)
                .unwrap_or(true)
        }) {
            drop(callback);
            return match close {
                Err(rejection) => Err(rejection),
                Ok(()) => Err(ManagedSqliteRegistryPinnedFileCloseRejection::InjectedLifecycle),
            };
        }
        #[cfg(all(test, windows))]
        let callback_complete = {
            let mut callback = callback;
            if let Err(rejection) = registry_lifecycle::arm_close_completion_native(
                self.close_faults.as_ref(),
                &mut callback,
            ) {
                let _ = self.owner.retain_terminal_custody(
                    self.route,
                    ManagedSqliteRegistryTerminalReason::FailureCustodyRetained,
                    callback,
                );
                return Err(rejection);
            }
            if let Err(rejection) = registry_lifecycle::observe(
                self.close_faults.as_ref(),
                ManagedSqliteRegistryLifecycleStage::CallbackCompletionAttempt,
            ) {
                let _ = self.owner.retain_terminal_custody(
                    self.route,
                    ManagedSqliteRegistryTerminalReason::FailureCustodyRetained,
                    callback,
                );
                return Err(rejection);
            }
            callback.complete_with_receipt()
        };
        #[cfg(all(test, not(windows)))]
        let callback_complete = callback.complete_with_receipt();
        #[cfg(not(test))]
        let callback_complete = callback.complete();
        #[cfg(not(test))]
        return match (close, callback_complete) {
            (Err(rejection), _) => Err(rejection),
            (Ok(()), Err(rejection)) => Err(
                ManagedSqliteRegistryPinnedFileCloseRejection::Registry(rejection),
            ),
            (Ok(()), Ok(())) => Ok(ManagedSqliteRegistryPinnedFileCloseSuccess {}),
        };
        #[cfg(test)]
        if callback_complete.is_err() {
            if let Some(faults) = self.close_faults.as_ref() {
                faults.native_failure(ManagedSqliteRegistryCloseLifecyclePhase::CallbackCompletion);
            }
        }
        #[cfg(test)]
        match (close, callback_complete) {
            (Err(rejection), Ok(receipt)) => {
                let _ = self.owner.retain_terminal_custody(
                    self.route,
                    ManagedSqliteRegistryTerminalReason::FailureCustodyRetained,
                    receipt,
                );
                Err(rejection)
            }
            (Err(rejection), Err(_)) => Err(rejection),
            (Ok(()), Err(rejection)) => Err(
                ManagedSqliteRegistryPinnedFileCloseRejection::Registry(rejection),
            ),
            (Ok(()), Ok(receipt)) => {
                #[cfg(windows)]
                {
                    if let Err(rejection) = registry_lifecycle::observe(
                        self.close_faults.as_ref(),
                        ManagedSqliteRegistryLifecycleStage::CallbackCompletionSucceeded,
                    ) {
                        let _ = self.owner.retain_terminal_custody(
                            self.route,
                            ManagedSqliteRegistryTerminalReason::FailureCustodyRetained,
                            receipt,
                        );
                        return Err(rejection);
                    }
                }
                if self.close_faults.as_ref().is_some_and(|faults| {
                    faults
                        .after_success(ManagedSqliteRegistryCloseLifecyclePhase::CallbackCompletion)
                        .unwrap_or(true)
                }) {
                    let _ = self.owner.retain_terminal_custody(
                        self.route,
                        ManagedSqliteRegistryTerminalReason::FailureCustodyRetained,
                        receipt,
                    );
                    Err(ManagedSqliteRegistryPinnedFileCloseRejection::InjectedLifecycle)
                } else {
                    Ok(ManagedSqliteRegistryPinnedFileCloseSuccess { callback: receipt })
                }
            }
        }
    }

    fn retain_physical_failure<Retained: 'static>(
        &self,
        reason: ManagedSqliteRegistryTerminalReason,
        custody: Retained,
    ) -> ManagedSqliteRegistryPinnedFileCloseRejection {
        let quarantine = self
            .owner
            .retain_terminal_custody(self.route, reason, custody)
            .err();
        ManagedSqliteRegistryPinnedFileCloseRejection::Physical { reason, quarantine }
    }
}

impl<Custody, NonceSource> Drop for ManagedSqliteRegistryPinnedFile<Custody, NonceSource>
where
    Custody: ManagedSqliteRegistryCustody + 'static,
    NonceSource: ManagedSqliteRegistryNonceSource + 'static,
{
    fn drop(&mut self) {
        if let Some(custody) = self.custody.take() {
            #[cfg(test)]
            {
                let _ = match custody {
                    custody @ ManagedSqliteRegistryPinnedFileCustody::WalMain { .. } => {
                        self.owner.retain_terminal_wal_main_physical_custody(
                            self.route,
                            ManagedSqliteRegistryTerminalReason::FailureCustodyRetained,
                            custody,
                        )
                    }
                    custody => self.owner.retain_terminal_custody(
                        self.route,
                        ManagedSqliteRegistryTerminalReason::FailureCustodyRetained,
                        custody,
                    ),
                };
            }
            #[cfg(not(test))]
            let _ = self.owner.retain_terminal_custody(
                self.route,
                ManagedSqliteRegistryTerminalReason::FailureCustodyRetained,
                custody,
            );
        }
    }
}

#[cfg(all(test, windows))]
mod tests;
