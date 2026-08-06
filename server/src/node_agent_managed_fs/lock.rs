use std::ffi::OsStr;

use super::{
    identity_digest, platform, require_single_normal_component, validate_regular_file_identity,
    ManagedExclusiveFileLockFailure, PinnedManagedDirectory, PinnedManagedExclusiveFileLock,
    QuarantinedManagedFile,
};

impl PinnedManagedDirectory {
    /// Acquires one persistent lock file below this exact pinned directory. Windows opens are
    /// parent-handle relative and share-none. An existing file is preferred; only a definite
    /// NotFound permits create-new, and one AlreadyExists race permits a final existing-only open.
    /// Every other race or uncertain result fails closed without deleting the lock file.
    pub(crate) fn acquire_exclusive_file_lock(
        self,
        name: &OsStr,
    ) -> std::result::Result<PinnedManagedExclusiveFileLock, ManagedExclusiveFileLockFailure> {
        if let Err(error) = require_single_normal_component(name) {
            return Err(ManagedExclusiveFileLockFailure::not_acquired(
                std::io::Error::new(std::io::ErrorKind::InvalidInput, error.to_string()),
                self,
            ));
        }
        let Some(parent) = self.directory_handles.last() else {
            return Err(ManagedExclusiveFileLockFailure::not_acquired(
                std::io::Error::other("NODE_MANAGED_FILE_PARENT_HANDLE_MISSING"),
                self,
            ));
        };
        let (file, lock_file_created) =
            match platform::open_existing_file_relative(parent.as_ref(), name, true) {
                Ok(file) => (file, false),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    match platform::create_new_file_relative(parent.as_ref(), name) {
                        Ok(file) => (file, true),
                        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                            match platform::open_existing_file_relative(parent.as_ref(), name, true)
                            {
                                Ok(file) => (file, false),
                                Err(error) => {
                                    return Err(ManagedExclusiveFileLockFailure::not_acquired(
                                        error, self,
                                    ));
                                }
                            }
                        }
                        Err(error) => {
                            return Err(ManagedExclusiveFileLockFailure::not_acquired(error, self));
                        }
                    }
                }
                Err(error) => {
                    return Err(ManagedExclusiveFileLockFailure::not_acquired(error, self));
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
                return Err(ManagedExclusiveFileLockFailure::opened_rejected(
                    error,
                    QuarantinedManagedFile {
                        _file: file,
                        _directory_handles: self.directory_handles,
                        directory_filesystem_mutated: self.filesystem_mutated || lock_file_created,
                    },
                ));
            }
        };
        Ok(PinnedManagedExclusiveFileLock {
            _file: file,
            _directory_handles: self.directory_handles,
            identity_digest: identity_digest(&self.root_identity_digest, None, identity),
        })
    }
}
