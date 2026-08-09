use std::{fs::File, sync::Arc};

use anyhow::{anyhow, bail, Result};

use super::{
    identity_digest, managed_parent_identity_digest, namespace::PlatformParentRelativeObservation,
    platform, same_file_identity, validate_directory_identity, validate_regular_file_identity,
    PinnedManagedDirectory,
};

#[path = "sqlite_namespace_io.rs"]
mod io;
#[path = "sqlite_namespace_types.rs"]
mod types;

use types::ManagedSqliteNamespaceInner;
pub(crate) use types::{
    ManagedSqliteAccess, ManagedSqliteDeleteFailure, ManagedSqliteDeleteFailurePhase,
    ManagedSqliteDeleteOutcome, ManagedSqliteDirectoryBarrierFailureKind, ManagedSqliteFileKind,
    ManagedSqliteFileOpenFailure, ManagedSqliteFileOpenFailurePhase,
    ManagedSqliteNamespaceBindFailure, ManagedSqliteOpenMode, PinnedManagedSqliteFile,
    PinnedManagedSqliteNamespace, QuarantinedManagedSqliteFile,
};

const FILE_OPENED: usize = 1;
const FILE_CREATED: usize = 2;

impl PinnedManagedDirectory {
    /// Consumes one already pinned, unchanged directory into the sealed four-name SQLite namespace.
    /// No string path or raw handle is exposed by the resulting capability.
    pub(crate) fn into_sqlite_namespace(
        self,
    ) -> std::result::Result<PinnedManagedSqliteNamespace, ManagedSqliteNamespaceBindFailure> {
        if self.filesystem_mutated {
            return Err(ManagedSqliteNamespaceBindFailure::new(
                std::io::Error::other("NODE_MANAGED_SQLITE_NAMESPACE_DIRECTORY_MUTATED"),
                self,
            ));
        }
        if let Err(error) = validate_namespace_directory(&self) {
            return Err(ManagedSqliteNamespaceBindFailure::new(
                std::io::Error::other(error.to_string()),
                self,
            ));
        }
        let directory_identity = match self
            .directory_handles
            .last()
            .ok_or_else(|| anyhow!("NODE_MANAGED_SQLITE_NAMESPACE_PARENT_HANDLE_MISSING"))
            .and_then(|directory| platform::inspect(directory).map_err(anyhow::Error::new))
        {
            Ok(identity) => identity,
            Err(error) => {
                return Err(ManagedSqliteNamespaceBindFailure::new(
                    std::io::Error::other(error.to_string()),
                    self,
                ));
            }
        };
        let directory_binding = match self.binding.as_ref() {
            Some(binding) => binding.clone(),
            None => {
                return Err(ManagedSqliteNamespaceBindFailure::new(
                    std::io::Error::other("NODE_MANAGED_SQLITE_NAMESPACE_BINDING_MISSING"),
                    self,
                ));
            }
        };
        Ok(PinnedManagedSqliteNamespace {
            inner: Arc::new(ManagedSqliteNamespaceInner {
                root_volume_serial: self.root_volume_serial,
                root_identity_digest: self.root_identity_digest,
                directory_identity,
                directory_binding,
                directory_handles: self.directory_handles,
            }),
        })
    }
}

impl PinnedManagedSqliteNamespace {
    pub(crate) fn open(
        &self,
        kind: ManagedSqliteFileKind,
        access: ManagedSqliteAccess,
        mode: ManagedSqliteOpenMode,
    ) -> std::result::Result<PinnedManagedSqliteFile, ManagedSqliteFileOpenFailure> {
        if mode == ManagedSqliteOpenMode::OpenOrCreate && access != ManagedSqliteAccess::ReadWrite {
            return Err(ManagedSqliteFileOpenFailure::not_opened(
                ManagedSqliteFileOpenFailurePhase::PlatformOpen,
                std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "NODE_MANAGED_SQLITE_CREATE_REQUIRES_WRITE",
                ),
            ));
        }
        self.validate_parent().map_err(|error| {
            ManagedSqliteFileOpenFailure::not_opened(
                ManagedSqliteFileOpenFailurePhase::ParentValidation,
                io_error(error),
            )
        })?;
        let parent = self.parent().map_err(|error| {
            ManagedSqliteFileOpenFailure::not_opened(
                ManagedSqliteFileOpenFailurePhase::ParentValidation,
                io_error(error),
            )
        })?;
        let opened =
            platform::open_sqlite_file_relative(parent, kind, access, mode).map_err(|error| {
                ManagedSqliteFileOpenFailure::not_opened(
                    ManagedSqliteFileOpenFailurePhase::PlatformOpen,
                    error,
                )
            })?;
        let file = opened.file;
        if let Err(error) = validate_open_completion(
            opened.call_status,
            opened.completion_status,
            opened.information,
            mode,
        ) {
            return Err(self.rejected_open(
                kind,
                file,
                ManagedSqliteFileOpenFailurePhase::OpenCompletion,
                error,
            ));
        }
        let identity = match self.validate_file(&file) {
            Ok(identity) => identity,
            Err(error) => {
                return Err(self.rejected_open(
                    kind,
                    file,
                    ManagedSqliteFileOpenFailurePhase::FileValidation,
                    error,
                ));
            }
        };
        if let Err(error) = self.validate_parent() {
            return Err(self.rejected_open(
                kind,
                file,
                ManagedSqliteFileOpenFailurePhase::ParentValidation,
                error,
            ));
        }
        Ok(PinnedManagedSqliteFile {
            file,
            namespace: Arc::clone(&self.inner),
            kind,
            access,
            identity,
            identity_digest: identity_digest(&self.inner.root_identity_digest, None, identity),
            created: opened.information == FILE_CREATED,
        })
    }

    pub(crate) fn access(
        &self,
        kind: ManagedSqliteFileKind,
        access: ManagedSqliteAccess,
    ) -> std::result::Result<bool, ManagedSqliteFileOpenFailure> {
        self.validate_parent().map_err(|error| {
            ManagedSqliteFileOpenFailure::not_opened(
                ManagedSqliteFileOpenFailurePhase::ParentValidation,
                io_error(error),
            )
        })?;
        let parent = self.parent().map_err(|error| {
            ManagedSqliteFileOpenFailure::not_opened(
                ManagedSqliteFileOpenFailurePhase::ParentValidation,
                io_error(error),
            )
        })?;
        let opened = match platform::open_sqlite_file_for_access_relative(parent, kind, access) {
            Ok(opened) => opened,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                self.validate_parent().map_err(|error| {
                    ManagedSqliteFileOpenFailure::not_opened(
                        ManagedSqliteFileOpenFailurePhase::ParentValidation,
                        io_error(error),
                    )
                })?;
                return Ok(false);
            }
            Err(error) => {
                return Err(ManagedSqliteFileOpenFailure::not_opened(
                    ManagedSqliteFileOpenFailurePhase::PlatformOpen,
                    error,
                ));
            }
        };
        let file = self.validate_observation_open(kind, opened)?;
        if let Err(error) = self.validate_parent() {
            return Err(self.rejected_open(
                kind,
                file,
                ManagedSqliteFileOpenFailurePhase::ParentValidation,
                error,
            ));
        }
        drop(file);
        Ok(true)
    }

    pub(crate) fn delete(
        &self,
        kind: ManagedSqliteFileKind,
        sync_parent: bool,
    ) -> std::result::Result<ManagedSqliteDeleteOutcome, ManagedSqliteDeleteFailure> {
        self.validate_parent().map_err(|error| {
            ManagedSqliteDeleteFailure::new(
                ManagedSqliteDeleteFailurePhase::ParentValidation,
                io_error(error),
                None,
            )
        })?;
        let parent = self.parent().map_err(|error| {
            ManagedSqliteDeleteFailure::new(
                ManagedSqliteDeleteFailurePhase::ParentValidation,
                io_error(error),
                None,
            )
        })?;
        let opened = match platform::open_sqlite_file_for_delete_relative(parent, kind) {
            Ok(opened) => opened,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                self.validate_parent().map_err(|error| {
                    ManagedSqliteDeleteFailure::new(
                        ManagedSqliteDeleteFailurePhase::ParentValidation,
                        io_error(error),
                        None,
                    )
                })?;
                return Ok(ManagedSqliteDeleteOutcome::NotFound);
            }
            Err(error) => {
                return Err(ManagedSqliteDeleteFailure::new(
                    ManagedSqliteDeleteFailurePhase::PlatformOpen,
                    error,
                    None,
                ));
            }
        };
        let file = opened.file;
        if let Err(error) = validate_open_completion(
            opened.call_status,
            opened.completion_status,
            opened.information,
            ManagedSqliteOpenMode::Existing,
        ) {
            return Err(self.delete_with_custody(
                kind,
                file,
                ManagedSqliteDeleteFailurePhase::OpenCompletion,
                error,
            ));
        }
        if let Err(error) = self.validate_file(&file) {
            return Err(self.delete_with_custody(
                kind,
                file,
                ManagedSqliteDeleteFailurePhase::FileValidation,
                error,
            ));
        }
        if let Err(error) = self.validate_parent() {
            return Err(self.delete_with_custody(
                kind,
                file,
                ManagedSqliteDeleteFailurePhase::ParentValidation,
                error,
            ));
        }
        if let Err(error) = platform::delete_by_handle(&file) {
            return Err(self.delete_with_custody(
                kind,
                file,
                ManagedSqliteDeleteFailurePhase::Disposition,
                error.into(),
            ));
        }
        drop(file);
        self.observe_absent(kind, ManagedSqliteDeleteFailurePhase::PreBarrierObservation)?;
        if sync_parent {
            let receipt = platform::flush_namespace_directory(parent).map_err(|failure| {
                let (error, failure_kind) = failure.into_parts();
                ManagedSqliteDeleteFailure::barrier(error, failure_kind)
            })?;
            let _filesystem_kind = receipt.filesystem_kind();
            self.observe_absent(
                kind,
                ManagedSqliteDeleteFailurePhase::PostBarrierObservation,
            )?;
        }
        let final_parent_phase = if sync_parent {
            ManagedSqliteDeleteFailurePhase::PostBarrierParentValidation
        } else {
            ManagedSqliteDeleteFailurePhase::PostDispositionParentValidation
        };
        self.validate_parent().map_err(|error| {
            ManagedSqliteDeleteFailure::new(final_parent_phase, io_error(error), None)
        })?;
        Ok(ManagedSqliteDeleteOutcome::Deleted)
    }

    fn parent(&self) -> Result<&File> {
        self.inner
            .directory_handles
            .last()
            .map(AsRef::as_ref)
            .ok_or_else(|| anyhow!("NODE_MANAGED_SQLITE_NAMESPACE_PARENT_HANDLE_MISSING"))
    }

    fn validate_parent(&self) -> Result<()> {
        let actual = platform::inspect(self.parent()?)?;
        validate_directory_identity(actual, Some(self.inner.root_volume_serial))?;
        if !same_file_identity(actual, self.inner.directory_identity)
            || identity_digest(&self.inner.root_identity_digest, None, actual)
                != self.inner.directory_binding.identity_digest()
        {
            bail!("NODE_MANAGED_SQLITE_NAMESPACE_PARENT_IDENTITY_CHANGED");
        }
        Ok(())
    }

    fn validate_file(&self, file: &File) -> Result<super::PlatformFileIdentity> {
        let identity = platform::inspect(file)?;
        validate_regular_file_identity(identity, self.inner.root_volume_serial)?;
        Ok(identity)
    }

    fn validate_observation_open(
        &self,
        kind: ManagedSqliteFileKind,
        opened: platform::PlatformManagedSqliteOpen,
    ) -> std::result::Result<File, ManagedSqliteFileOpenFailure> {
        let file = opened.file;
        if let Err(error) = validate_open_completion(
            opened.call_status,
            opened.completion_status,
            opened.information,
            ManagedSqliteOpenMode::Existing,
        ) {
            return Err(self.rejected_open(
                kind,
                file,
                ManagedSqliteFileOpenFailurePhase::OpenCompletion,
                error,
            ));
        }
        if let Err(error) = self.validate_file(&file) {
            return Err(self.rejected_open(
                kind,
                file,
                ManagedSqliteFileOpenFailurePhase::FileValidation,
                error,
            ));
        }
        Ok(file)
    }

    fn rejected_open(
        &self,
        kind: ManagedSqliteFileKind,
        file: File,
        phase: ManagedSqliteFileOpenFailurePhase,
        error: anyhow::Error,
    ) -> ManagedSqliteFileOpenFailure {
        ManagedSqliteFileOpenFailure::opened_rejected(
            phase,
            io_error(error),
            self.quarantine(kind, file),
        )
    }

    fn delete_with_custody(
        &self,
        kind: ManagedSqliteFileKind,
        file: File,
        phase: ManagedSqliteDeleteFailurePhase,
        error: anyhow::Error,
    ) -> ManagedSqliteDeleteFailure {
        ManagedSqliteDeleteFailure::new(phase, io_error(error), Some(self.quarantine(kind, file)))
    }

    fn quarantine(&self, kind: ManagedSqliteFileKind, file: File) -> QuarantinedManagedSqliteFile {
        QuarantinedManagedSqliteFile {
            _file: file,
            _namespace: Arc::clone(&self.inner),
            kind,
        }
    }

    fn observe_absent(
        &self,
        kind: ManagedSqliteFileKind,
        phase: ManagedSqliteDeleteFailurePhase,
    ) -> std::result::Result<(), ManagedSqliteDeleteFailure> {
        let parent_phase = if phase == ManagedSqliteDeleteFailurePhase::PostBarrierObservation {
            ManagedSqliteDeleteFailurePhase::PostBarrierParentValidation
        } else {
            ManagedSqliteDeleteFailurePhase::PostDispositionParentValidation
        };
        match platform::observe_child_relative(
            self.parent().map_err(|error| {
                ManagedSqliteDeleteFailure::new(parent_phase, io_error(error), None)
            })?,
            kind.name(),
        ) {
            Ok(PlatformParentRelativeObservation::Absent) => Ok(()),
            Ok(PlatformParentRelativeObservation::Present(file)) => {
                Err(ManagedSqliteDeleteFailure::new(
                    phase,
                    std::io::Error::other("NODE_MANAGED_SQLITE_DELETE_NAME_STILL_PRESENT"),
                    Some(self.quarantine(kind, file)),
                ))
            }
            Err(error) => Err(ManagedSqliteDeleteFailure::new(phase, error, None)),
        }
    }
}

fn validate_namespace_directory(directory: &PinnedManagedDirectory) -> Result<()> {
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

fn validate_open_completion(
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

fn io_error(error: anyhow::Error) -> std::io::Error {
    std::io::Error::other(error.to_string())
}
