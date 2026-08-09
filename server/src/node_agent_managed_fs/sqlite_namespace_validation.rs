use anyhow::{anyhow, bail, Result};

use super::super::{managed_parent_identity_digest, namespace::PlatformParentRelativeObservation};
use super::{
    identity_digest, platform, validate_directory_identity, ManagedSqliteDeleteFailure,
    ManagedSqliteDeleteFailurePhase, ManagedSqliteFileKind, ManagedSqliteFileOpenFailure,
    ManagedSqliteFileOpenFailurePhase, ManagedSqliteOpenMode, PinnedManagedDirectory,
    PinnedManagedSqliteNamespace, FILE_CREATED, FILE_OPENED,
};

pub(super) fn reserved_shm_open_failure() -> ManagedSqliteFileOpenFailure {
    ManagedSqliteFileOpenFailure::not_opened(
        ManagedSqliteFileOpenFailurePhase::PlatformOpen,
        std::io::Error::other("NODE_MANAGED_SQLITE_SHM_REQUIRES_WAL_COORDINATOR"),
    )
}

pub(super) fn reserved_shm_delete_failure() -> ManagedSqliteDeleteFailure {
    ManagedSqliteDeleteFailure::new(
        ManagedSqliteDeleteFailurePhase::PlatformOpen,
        std::io::Error::other("NODE_MANAGED_SQLITE_SHM_REQUIRES_WAL_COORDINATOR"),
        None,
    )
}

pub(super) fn validate_namespace_directory(directory: &PinnedManagedDirectory) -> Result<()> {
    let binding = directory
        .binding
        .as_ref()
        .ok_or_else(|| anyhow!("NODE_MANAGED_SQLITE_NAMESPACE_BINDING_MISSING"))?;
    if !binding.is_directory() {
        bail!("NODE_MANAGED_SQLITE_NAMESPACE_BINDING_NOT_DIRECTORY");
    }
    let handle = directory
        .directory_handles
        .last()
        .ok_or_else(|| anyhow!("NODE_MANAGED_SQLITE_NAMESPACE_PARENT_HANDLE_MISSING"))?;
    let identity = platform::inspect(handle)?;
    validate_directory_identity(identity, Some(directory.root_volume_serial))?;
    if identity_digest(&directory.root_identity_digest, None, identity) != binding.identity_digest()
    {
        bail!("NODE_MANAGED_SQLITE_NAMESPACE_BINDING_CHANGED");
    }
    let parent = directory
        .directory_handles
        .get(
            directory
                .directory_handles
                .len()
                .checked_sub(2)
                .ok_or_else(|| anyhow!("NODE_MANAGED_SQLITE_NAMESPACE_BINDING_PARENT_MISSING"))?,
        )
        .ok_or_else(|| anyhow!("NODE_MANAGED_SQLITE_NAMESPACE_BINDING_PARENT_MISSING"))?;
    if managed_parent_identity_digest(
        &directory.root_identity_digest,
        parent,
        directory.root_volume_serial,
    )? != binding.parent_identity_digest()
    {
        bail!("NODE_MANAGED_SQLITE_NAMESPACE_PARENT_BINDING_CHANGED");
    }
    Ok(())
}

pub(super) fn validate_open_completion(
    call_status: i32,
    completion_status: i32,
    information: usize,
    mode: ManagedSqliteOpenMode,
) -> Result<()> {
    if call_status != 0 || completion_status != 0 {
        bail!("NODE_MANAGED_SQLITE_OPEN_COMPLETION_FAILED");
    }
    let valid = match mode {
        ManagedSqliteOpenMode::Existing => information == FILE_OPENED,
        ManagedSqliteOpenMode::OpenOrCreate => {
            information == FILE_OPENED || information == FILE_CREATED
        }
    };
    if !valid {
        bail!("NODE_MANAGED_SQLITE_OPEN_DISPOSITION_INVALID");
    }
    Ok(())
}

impl PinnedManagedSqliteNamespace {
    pub(super) fn observe_absent(
        &self,
        kind: ManagedSqliteFileKind,
        phase: ManagedSqliteDeleteFailurePhase,
    ) -> std::result::Result<(), ManagedSqliteDeleteFailure> {
        let post_barrier = phase == ManagedSqliteDeleteFailurePhase::PostBarrierObservation;
        let parent_phase = if post_barrier {
            ManagedSqliteDeleteFailurePhase::PostBarrierParentValidation
        } else {
            ManagedSqliteDeleteFailurePhase::PostDispositionParentValidation
        };
        let close_phase = if post_barrier {
            ManagedSqliteDeleteFailurePhase::PostBarrierObservationHandleClose
        } else {
            ManagedSqliteDeleteFailurePhase::PreBarrierObservationHandleClose
        };
        match platform::observe_child_relative(
            self.parent().map_err(|error| {
                ManagedSqliteDeleteFailure::new(parent_phase, super::io_error(error), None)
            })?,
            kind.name(),
        ) {
            Ok(PlatformParentRelativeObservation::Absent) => Ok(()),
            Ok(PlatformParentRelativeObservation::Present(file)) => {
                match self.quarantine(kind, file).close() {
                    Ok(_) => Err(ManagedSqliteDeleteFailure::new(
                        phase,
                        std::io::Error::other("NODE_MANAGED_SQLITE_DELETE_NAME_STILL_PRESENT"),
                        None,
                    )),
                    Err(failure) => Err(ManagedSqliteDeleteFailure::close_failed(
                        close_phase,
                        failure,
                    )),
                }
            }
            Err(error) => Err(ManagedSqliteDeleteFailure::new(phase, error, None)),
        }
    }
}
