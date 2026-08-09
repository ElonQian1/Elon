//! Linear pairing between managed-fs handles and exact registry leases.
//!
//! A future `sqlite3_file` may own one of these values, but the current inert ABI has no
//! constructor or callback path to it. Physical close receipts are the only path that retires a
//! lease. Any abandoned state or failed physical close is retained for process lifetime before its
//! exact route is quarantined.

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
}

pub(super) type ComputePluginHandleBoundSqlitePinnedFile = ManagedSqliteRegistryPinnedFile<
    crate::node_agent_compute_plugin_host::local_authority::ComputePluginHandleBoundAuthorityOpenIntent,
    super::process_owner::ManagedSqliteRegistrySystemNonceSource,
>;

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
        })
    }

    pub(super) fn close(mut self) -> Result<(), ManagedSqliteRegistryPinnedFileCloseRejection> {
        let custody = self
            .custody
            .take()
            .expect("live pinned file state must retain exact custody");
        match custody {
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
                    Ok(receipt) => self
                        .owner
                        .close_wal_main(self.route, main, shm, receipt)
                        .map_err(ManagedSqliteRegistryPinnedFileCloseRejection::Registry),
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
