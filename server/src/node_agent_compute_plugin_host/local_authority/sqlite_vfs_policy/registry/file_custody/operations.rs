//! Registry-routed operations over exact managed-fs file custody.
//!
//! Every operation acquires one callback lease before touching physical state. This module does
//! not expose raw file handles or SHM addresses; a future ABI adapter must remain above this gate.

use std::num::NonZeroU32;

use super::{ManagedSqliteRegistryPinnedFile, ManagedSqliteRegistryPinnedFileCustody};
use crate::node_agent_managed_fs::{
    ManagedSqliteLockAttempt, ManagedSqliteLockFailure, ManagedSqliteObservedLock,
    ManagedSqliteRequestedLock, ManagedSqliteShmFailure, ManagedSqliteShmLockAttempt,
    ManagedSqliteShmLockRequest, ManagedSqliteShmMapMode, ManagedSqliteShmMapOutcome,
    ManagedSqliteShmUnmapMode, ManagedSqliteUnlockTarget,
};

use super::super::{
    owner::ManagedSqliteRegistryCustody,
    process_owner::{ManagedSqliteRegistryNonceSource, ManagedSqliteRegistryProcessRouteRejection},
    types::ManagedSqliteRegistryCallbackKind,
};

#[derive(Debug)]
pub(super) enum ManagedSqliteRegistryPinnedFileOperationRejection {
    Registry(ManagedSqliteRegistryProcessRouteRejection),
    Io(anyhow::Error),
    Lock(ManagedSqliteLockFailure),
    Shm(ManagedSqliteShmFailure),
    UnsupportedFileRole,
    ShmDetached,
}

impl<Custody, NonceSource> ManagedSqliteRegistryPinnedFile<Custody, NonceSource>
where
    Custody: ManagedSqliteRegistryCustody + 'static,
    NonceSource: ManagedSqliteRegistryNonceSource + 'static,
{
    pub(super) fn read_at_zero_filled(
        &mut self,
        offset: u64,
        buffer: &mut [u8],
    ) -> Result<usize, ManagedSqliteRegistryPinnedFileOperationRejection> {
        self.with_callback(ManagedSqliteRegistryCallbackKind::Io, |custody| {
            match custody {
                ManagedSqliteRegistryPinnedFileCustody::Sidecar { file, .. } => {
                    file.read_at_zero_filled(offset, buffer)
                }
                ManagedSqliteRegistryPinnedFileCustody::Main { file, .. } => {
                    file.read_at_zero_filled(offset, buffer)
                }
                ManagedSqliteRegistryPinnedFileCustody::WalMain { file, .. } => {
                    file.main_mut().read_at_zero_filled(offset, buffer)
                }
            }
            .map_err(ManagedSqliteRegistryPinnedFileOperationRejection::Io)
        })
    }

    pub(super) fn write_all_at(
        &mut self,
        offset: u64,
        bytes: &[u8],
    ) -> Result<(), ManagedSqliteRegistryPinnedFileOperationRejection> {
        self.with_callback(ManagedSqliteRegistryCallbackKind::Io, |custody| {
            match custody {
                ManagedSqliteRegistryPinnedFileCustody::Sidecar { file, .. } => {
                    file.write_all_at(offset, bytes)
                }
                ManagedSqliteRegistryPinnedFileCustody::Main { file, .. } => {
                    file.write_all_at(offset, bytes)
                }
                ManagedSqliteRegistryPinnedFileCustody::WalMain { file, .. } => {
                    file.main_mut().write_all_at(offset, bytes)
                }
            }
            .map_err(ManagedSqliteRegistryPinnedFileOperationRejection::Io)
        })
    }

    pub(super) fn truncate(
        &mut self,
        size: u64,
    ) -> Result<(), ManagedSqliteRegistryPinnedFileOperationRejection> {
        self.with_callback(ManagedSqliteRegistryCallbackKind::Io, |custody| {
            match custody {
                ManagedSqliteRegistryPinnedFileCustody::Sidecar { file, .. } => file.truncate(size),
                ManagedSqliteRegistryPinnedFileCustody::Main { file, .. } => file.truncate(size),
                ManagedSqliteRegistryPinnedFileCustody::WalMain { file, .. } => {
                    file.main_mut().truncate(size)
                }
            }
            .map_err(ManagedSqliteRegistryPinnedFileOperationRejection::Io)
        })
    }

    pub(super) fn size(
        &mut self,
    ) -> Result<u64, ManagedSqliteRegistryPinnedFileOperationRejection> {
        self.with_callback(ManagedSqliteRegistryCallbackKind::Io, |custody| {
            match custody {
                ManagedSqliteRegistryPinnedFileCustody::Sidecar { file, .. } => file.size(),
                ManagedSqliteRegistryPinnedFileCustody::Main { file, .. } => file.size(),
                ManagedSqliteRegistryPinnedFileCustody::WalMain { file, .. } => {
                    file.main_mut().size()
                }
            }
            .map_err(ManagedSqliteRegistryPinnedFileOperationRejection::Io)
        })
    }

    pub(super) fn full_sync(
        &mut self,
    ) -> Result<(), ManagedSqliteRegistryPinnedFileOperationRejection> {
        self.with_callback(ManagedSqliteRegistryCallbackKind::Io, |custody| {
            match custody {
                ManagedSqliteRegistryPinnedFileCustody::Sidecar { file, .. } => file.full_sync(),
                ManagedSqliteRegistryPinnedFileCustody::Main { file, .. } => file.full_sync(),
                ManagedSqliteRegistryPinnedFileCustody::WalMain { file, .. } => {
                    file.main_mut().full_sync()
                }
            }
            .map_err(ManagedSqliteRegistryPinnedFileOperationRejection::Io)
        })
    }

    pub(super) fn revalidate(
        &mut self,
    ) -> Result<(), ManagedSqliteRegistryPinnedFileOperationRejection> {
        self.with_callback(ManagedSqliteRegistryCallbackKind::Io, |custody| {
            match custody {
                ManagedSqliteRegistryPinnedFileCustody::Sidecar { file, .. } => file.revalidate(),
                ManagedSqliteRegistryPinnedFileCustody::Main { file, .. } => file.revalidate(),
                ManagedSqliteRegistryPinnedFileCustody::WalMain { file, .. } => {
                    file.main_mut().revalidate()
                }
            }
            .map_err(ManagedSqliteRegistryPinnedFileOperationRejection::Io)
        })
    }

    pub(super) fn lock_level(
        &mut self,
    ) -> Result<ManagedSqliteObservedLock, ManagedSqliteRegistryPinnedFileOperationRejection> {
        self.with_main(|file| file.lock_level())
    }

    pub(super) fn lock_to(
        &mut self,
        requested: ManagedSqliteRequestedLock,
    ) -> Result<ManagedSqliteLockAttempt, ManagedSqliteRegistryPinnedFileOperationRejection> {
        self.with_main(|file| file.lock_to(requested))
    }

    pub(super) fn unlock_to(
        &mut self,
        target: ManagedSqliteUnlockTarget,
    ) -> Result<(), ManagedSqliteRegistryPinnedFileOperationRejection> {
        self.with_main(|file| file.unlock_to(target))
    }

    pub(super) fn check_reserved_lock(
        &mut self,
    ) -> Result<bool, ManagedSqliteRegistryPinnedFileOperationRejection> {
        self.with_main(|file| file.check_reserved_lock())
    }

    pub(super) fn shm_map(
        &mut self,
        region: u32,
        region_size: NonZeroU32,
        mode: ManagedSqliteShmMapMode,
    ) -> Result<ManagedSqliteShmMapOutcome, ManagedSqliteRegistryPinnedFileOperationRejection> {
        self.with_shm(|shm| shm.map(region, region_size, mode))
    }

    pub(super) fn shm_lock(
        &mut self,
        request: ManagedSqliteShmLockRequest,
    ) -> Result<ManagedSqliteShmLockAttempt, ManagedSqliteRegistryPinnedFileOperationRejection>
    {
        self.with_shm(|shm| shm.lock(request))
    }

    pub(super) fn shm_barrier(
        &mut self,
    ) -> Result<(), ManagedSqliteRegistryPinnedFileOperationRejection> {
        self.with_callback(ManagedSqliteRegistryCallbackKind::Shm, |custody| {
            let ManagedSqliteRegistryPinnedFileCustody::WalMain { file, .. } = custody else {
                return Err(ManagedSqliteRegistryPinnedFileOperationRejection::UnsupportedFileRole);
            };
            let Some(shm) = file.shm_mut() else {
                return Err(ManagedSqliteRegistryPinnedFileOperationRejection::ShmDetached);
            };
            shm.barrier();
            Ok(())
        })
    }

    pub(super) fn shm_unmap(
        &mut self,
        mode: ManagedSqliteShmUnmapMode,
    ) -> Result<(), ManagedSqliteRegistryPinnedFileOperationRejection> {
        let callback = self
            .owner
            .begin_callback(self.route, ManagedSqliteRegistryCallbackKind::Shm)
            .map_err(ManagedSqliteRegistryPinnedFileOperationRejection::Registry)?;
        let result = self.unmap_shm_custody(mode);
        match (result, callback.complete()) {
            (Err(rejection), _) => Err(rejection),
            (Ok(()), Err(rejection)) => Err(
                ManagedSqliteRegistryPinnedFileOperationRejection::Registry(rejection),
            ),
            (Ok(()), Ok(())) => Ok(()),
        }
    }

    fn with_main<T>(
        &mut self,
        operation: impl FnOnce(
            &mut crate::node_agent_managed_fs::PinnedManagedSqliteMainFile,
        ) -> Result<T, ManagedSqliteLockFailure>,
    ) -> Result<T, ManagedSqliteRegistryPinnedFileOperationRejection> {
        self.with_callback(ManagedSqliteRegistryCallbackKind::Io, |custody| {
            let file = match custody {
                ManagedSqliteRegistryPinnedFileCustody::Main { file, .. } => file,
                ManagedSqliteRegistryPinnedFileCustody::WalMain { file, .. } => file.main_mut(),
                ManagedSqliteRegistryPinnedFileCustody::Sidecar { .. } => {
                    return Err(
                        ManagedSqliteRegistryPinnedFileOperationRejection::UnsupportedFileRole,
                    );
                }
            };
            operation(file).map_err(ManagedSqliteRegistryPinnedFileOperationRejection::Lock)
        })
    }

    fn with_shm<T>(
        &mut self,
        operation: impl FnOnce(
            &mut crate::node_agent_managed_fs::PinnedManagedSqliteShmConnection,
        ) -> Result<T, ManagedSqliteShmFailure>,
    ) -> Result<T, ManagedSqliteRegistryPinnedFileOperationRejection> {
        self.with_callback(ManagedSqliteRegistryCallbackKind::Shm, |custody| {
            let ManagedSqliteRegistryPinnedFileCustody::WalMain { file, .. } = custody else {
                return Err(ManagedSqliteRegistryPinnedFileOperationRejection::UnsupportedFileRole);
            };
            let Some(shm) = file.shm_mut() else {
                return Err(ManagedSqliteRegistryPinnedFileOperationRejection::ShmDetached);
            };
            operation(shm).map_err(ManagedSqliteRegistryPinnedFileOperationRejection::Shm)
        })
    }

    fn with_callback<T>(
        &mut self,
        kind: ManagedSqliteRegistryCallbackKind,
        operation: impl FnOnce(
            &mut ManagedSqliteRegistryPinnedFileCustody,
        ) -> Result<T, ManagedSqliteRegistryPinnedFileOperationRejection>,
    ) -> Result<T, ManagedSqliteRegistryPinnedFileOperationRejection> {
        let callback = self
            .owner
            .begin_callback(self.route, kind)
            .map_err(ManagedSqliteRegistryPinnedFileOperationRejection::Registry)?;
        let result = operation(
            self.custody
                .as_mut()
                .expect("live pinned file operation must retain exact custody"),
        );
        match (result, callback.complete()) {
            (Err(rejection), _) => Err(rejection),
            (Ok(value), Err(rejection)) => Err(
                ManagedSqliteRegistryPinnedFileOperationRejection::Registry(rejection),
            ),
            (Ok(value), Ok(())) => Ok(value),
        }
    }

    fn unmap_shm_custody(
        &mut self,
        mode: ManagedSqliteShmUnmapMode,
    ) -> Result<(), ManagedSqliteRegistryPinnedFileOperationRejection> {
        let custody = self
            .custody
            .take()
            .expect("live SHM unmap must retain exact custody");
        let ManagedSqliteRegistryPinnedFileCustody::WalMain {
            mut file,
            main,
            shm,
        } = custody
        else {
            self.custody = Some(custody);
            return Err(ManagedSqliteRegistryPinnedFileOperationRejection::UnsupportedFileRole);
        };
        if file.shm_mut().is_none() {
            self.custody =
                Some(ManagedSqliteRegistryPinnedFileCustody::WalMain { file, main, shm });
            return Err(ManagedSqliteRegistryPinnedFileOperationRejection::ShmDetached);
        }
        match file.unmap_shm(mode) {
            Ok(file) => {
                self.custody =
                    Some(ManagedSqliteRegistryPinnedFileCustody::WalMain { file, main, shm });
                Ok(())
            }
            Err(failure) => {
                let (failure, file) = failure.into_parts();
                self.custody =
                    Some(ManagedSqliteRegistryPinnedFileCustody::WalMain { file, main, shm });
                Err(ManagedSqliteRegistryPinnedFileOperationRejection::Shm(
                    failure,
                ))
            }
        }
    }
}
