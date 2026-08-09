//! Linear promotion of an already-routed main file into WAL-main + SHM custody.

use super::{
    operations::ManagedSqliteRegistryPinnedFileOperationRejection, ManagedSqliteRegistryPinnedFile,
    ManagedSqliteRegistryPinnedFileCustody,
};
use crate::{
    node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::registry::{
        owner::ManagedSqliteRegistryCustody,
        process_owner::ManagedSqliteRegistryNonceSource,
        types::{ManagedSqliteRegistryCallbackKind, ManagedSqliteRegistryTerminalReason},
    },
    node_agent_managed_fs::PinnedManagedSqliteWalRuntime,
};

impl<Custody, NonceSource> ManagedSqliteRegistryPinnedFile<Custody, NonceSource>
where
    Custody: ManagedSqliteRegistryCustody + 'static,
    NonceSource: ManagedSqliteRegistryNonceSource + 'static,
{
    pub(super) fn promote_main_to_wal(
        &mut self,
        runtime: &PinnedManagedSqliteWalRuntime,
    ) -> Result<(), ManagedSqliteRegistryPinnedFileOperationRejection> {
        let callback = self
            .owner
            .begin_callback(self.route, ManagedSqliteRegistryCallbackKind::Shm)
            .map_err(ManagedSqliteRegistryPinnedFileOperationRejection::Registry)?;
        let result = self.promote_main_custody(runtime);
        match (result, callback.complete()) {
            (Err(rejection), _) => Err(rejection),
            (Ok(()), Err(rejection)) => Err(
                ManagedSqliteRegistryPinnedFileOperationRejection::Registry(rejection),
            ),
            (Ok(()), Ok(())) => Ok(()),
        }
    }

    fn promote_main_custody(
        &mut self,
        runtime: &PinnedManagedSqliteWalRuntime,
    ) -> Result<(), ManagedSqliteRegistryPinnedFileOperationRejection> {
        let mut custody = self
            .custody
            .take()
            .expect("live WAL promotion must retain exact main custody");
        let ManagedSqliteRegistryPinnedFileCustody::Main { file, lease } = custody else {
            let result = match &mut custody {
                ManagedSqliteRegistryPinnedFileCustody::WalMain { file, .. } => {
                    if file.shm_mut().is_some() {
                        Ok(())
                    } else {
                        Err(ManagedSqliteRegistryPinnedFileOperationRejection::ShmDetached)
                    }
                }
                ManagedSqliteRegistryPinnedFileCustody::Sidecar { .. } => {
                    Err(ManagedSqliteRegistryPinnedFileOperationRejection::UnsupportedFileRole)
                }
                ManagedSqliteRegistryPinnedFileCustody::Main { .. } => unreachable!(),
            };
            self.custody = Some(custody);
            return result;
        };
        let shm = match self.owner.claim_shm(self.route) {
            Ok(shm) => shm,
            Err(rejection) => {
                self.custody = Some(ManagedSqliteRegistryPinnedFileCustody::Main { file, lease });
                return Err(ManagedSqliteRegistryPinnedFileOperationRejection::Registry(
                    rejection,
                ));
            }
        };
        match runtime.bind_main_file(file) {
            Ok(file) => {
                self.custody = Some(ManagedSqliteRegistryPinnedFileCustody::WalMain {
                    file,
                    main: lease,
                    shm,
                });
                Ok(())
            }
            Err(failure) => {
                let (failure, file) = failure.into_parts();
                let retention = self.owner.retain_terminal_custody(
                    self.route,
                    ManagedSqliteRegistryTerminalReason::FailureCustodyRetained,
                    (file, lease, shm),
                );
                match retention {
                    Ok(()) => Err(ManagedSqliteRegistryPinnedFileOperationRejection::Shm(
                        failure,
                    )),
                    Err(rejection) => Err(
                        ManagedSqliteRegistryPinnedFileOperationRejection::Registry(rejection),
                    ),
                }
            }
        }
    }
}
