//! Test-only access to the ordered outer runtime receipt used by direct xClose.

use super::*;

impl<Custody, NonceSource> ManagedSqliteRegistryPinnedFile<Custody, NonceSource>
where
    Custody: ManagedSqliteRegistryCustody + 'static,
    NonceSource: ManagedSqliteRegistryNonceSource + 'static,
{
    pub(super) fn joint_close_runtime_observation_enabled(
        &self,
    ) -> Result<bool, ManagedSqliteRegistryPinnedFileCloseRejection> {
        match self.close_faults.as_ref() {
            Some(faults) => faults
                .unmap_runtime_observation_enabled()
                .map_err(|()| ManagedSqliteRegistryPinnedFileCloseRejection::InjectedLifecycle),
            None => Ok(false),
        }
    }

    pub(super) fn observe_joint_close_runtime_event(
        &self,
        event: ManagedSqliteRegistryUnmapRuntimeEvent,
    ) -> Result<(), ManagedSqliteRegistryPinnedFileCloseRejection> {
        match self.close_faults.as_ref() {
            Some(faults) => faults
                .observe_unmap_runtime_event(event)
                .map_err(|()| ManagedSqliteRegistryPinnedFileCloseRejection::InjectedLifecycle),
            None => Ok(()),
        }
    }
}
