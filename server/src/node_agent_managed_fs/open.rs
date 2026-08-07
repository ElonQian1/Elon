use std::{ffi::OsStr, sync::Arc};

use anyhow::anyhow;

use super::{
    identity_digest, managed_parent_identity_digest, platform, require_single_normal_component,
    validate_directory_identity, validate_regular_file_identity, ManagedDirectoryPrepareFailure,
    ManagedFileOpenFailure, ManagedObjectBinding, PinnedManagedDirectory, PinnedManagedFile,
    QuarantinedManagedFile,
};

impl PinnedManagedDirectory {
    pub(crate) fn create_new_directory_child(
        &self,
        name: &OsStr,
    ) -> std::result::Result<PinnedManagedDirectory, ManagedDirectoryPrepareFailure> {
        require_single_normal_component(name).map_err(ManagedDirectoryPrepareFailure::Unchanged)?;
        let parent = self.directory_handles.last().ok_or_else(|| {
            ManagedDirectoryPrepareFailure::Unchanged(anyhow!(
                "NODE_MANAGED_DIRECTORY_PARENT_HANDLE_MISSING"
            ))
        })?;
        let file = platform::create_new_directory_relative(parent.as_ref(), name)
            .map_err(|error| ManagedDirectoryPrepareFailure::Unchanged(error.into()))?;
        let file = Arc::new(file);
        let identity = platform::inspect(&file)
            .map_err(|error| ManagedDirectoryPrepareFailure::Mutated(error.into()))?;
        validate_directory_identity(identity, Some(self.root_volume_serial))
            .map_err(ManagedDirectoryPrepareFailure::Mutated)?;
        let binding = ManagedObjectBinding::directory(
            name,
            identity_digest(&self.root_identity_digest, None, identity),
            managed_parent_identity_digest(
                &self.root_identity_digest,
                parent.as_ref(),
                self.root_volume_serial,
            )
            .map_err(ManagedDirectoryPrepareFailure::Mutated)?,
        );
        let path = platform::canonical_path(&file)
            .map_err(|error| ManagedDirectoryPrepareFailure::Mutated(error.into()))?;
        let mut handles = self.directory_handles.clone();
        handles.push(file);
        Ok(PinnedManagedDirectory {
            path,
            root_volume_serial: self.root_volume_serial,
            root_identity_digest: self.root_identity_digest.clone(),
            directory_handles: handles,
            binding: Some(binding),
            filesystem_mutated: true,
        })
    }

    pub(crate) fn open_existing_read_write(
        self,
        name: &OsStr,
    ) -> std::result::Result<PinnedManagedFile, ManagedFileOpenFailure> {
        self.open_file(name, true, false, false)
    }

    pub(crate) fn create_new_read_write(
        self,
        name: &OsStr,
    ) -> std::result::Result<PinnedManagedFile, ManagedFileOpenFailure> {
        self.open_file(name, true, true, true)
    }

    /// Opens one existing child read-only with share-none semantics while retaining this pinned
    /// directory for subsequent siblings. Returned files share the parent handle chain through
    /// process-local `Arc`s rather than cloning operating-system handles per artifact.
    pub(crate) fn open_existing_read_only_child(
        &self,
        name: &OsStr,
    ) -> std::result::Result<PinnedManagedFile, ManagedFileOpenFailure> {
        self.open_file(name, false, false, false)
    }

    pub(super) fn open_file(
        &self,
        name: &OsStr,
        writable: bool,
        create_new: bool,
        deletable: bool,
    ) -> std::result::Result<PinnedManagedFile, ManagedFileOpenFailure> {
        if let Err(error) = require_single_normal_component(name) {
            return Err(ManagedFileOpenFailure::FileNotOpened {
                error: std::io::Error::new(std::io::ErrorKind::InvalidInput, error.to_string()),
                directory: self.shared_clone(),
            });
        }
        let parent = match self.directory_handles.last() {
            Some(parent) => parent,
            None => {
                return Err(ManagedFileOpenFailure::FileNotOpened {
                    error: std::io::Error::other("NODE_MANAGED_FILE_PARENT_HANDLE_MISSING"),
                    directory: self.shared_clone(),
                });
            }
        };
        let opened = if create_new {
            platform::create_new_file_relative(parent.as_ref(), name)
        } else if deletable {
            platform::open_existing_file_relative_deletable(parent.as_ref(), name)
        } else {
            platform::open_existing_file_relative(parent.as_ref(), name, writable)
        };
        let file = match opened {
            Ok(file) => file,
            Err(error) => {
                return Err(ManagedFileOpenFailure::FileNotOpened {
                    error,
                    directory: self.shared_clone(),
                });
            }
        };
        let identity = match platform::inspect(&file)
            .map_err(anyhow::Error::from)
            .and_then(|identity| {
                validate_regular_file_identity(identity, self.root_volume_serial)?;
                Ok(identity)
            }) {
            Ok(identity) => identity,
            Err(error) => {
                return Err(ManagedFileOpenFailure::Opened {
                    error,
                    file: QuarantinedManagedFile {
                        _file: file,
                        _directory_handles: self.directory_handles.clone(),
                        directory_filesystem_mutated: self.filesystem_mutated,
                    },
                });
            }
        };
        let identity_digest = identity_digest(&self.root_identity_digest, None, identity);
        let binding = ManagedObjectBinding::file(
            name,
            identity_digest.clone(),
            match managed_parent_identity_digest(
                &self.root_identity_digest,
                parent.as_ref(),
                self.root_volume_serial,
            ) {
                Ok(digest) => digest,
                Err(error) => {
                    return Err(ManagedFileOpenFailure::Opened {
                        error,
                        file: QuarantinedManagedFile {
                            _file: file,
                            _directory_handles: self.directory_handles.clone(),
                            directory_filesystem_mutated: self.filesystem_mutated,
                        },
                    });
                }
            },
        );
        Ok(PinnedManagedFile {
            file,
            _directory_handles: self.directory_handles.clone(),
            root_volume_serial: self.root_volume_serial,
            root_identity_digest: self.root_identity_digest.clone(),
            identity,
            identity_digest,
            binding,
            directory_filesystem_mutated: self.filesystem_mutated,
        })
    }

    fn shared_clone(&self) -> Self {
        Self {
            path: self.path.clone(),
            root_volume_serial: self.root_volume_serial,
            root_identity_digest: self.root_identity_digest.clone(),
            directory_handles: self.directory_handles.clone(),
            binding: self.binding.clone(),
            filesystem_mutated: self.filesystem_mutated,
        }
    }
}
