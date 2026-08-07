use std::sync::Arc;

use super::super::{
    identity_digest, managed_parent_identity_digest, platform, validate_directory_identity,
    validate_regular_file_identity, ManagedDeleteDisposition, ManagedObjectBinding,
    PinnedManagedDirectory, PinnedManagedFile,
};
use super::{ManagedDirectoryDeleteFailure, ManagedFileDeleteFailure};

impl PinnedManagedDirectory {
    /// Strong successor to `delete_exact`: validates the handle-derived binding, requires unique
    /// custody of the target directory handle, closes it after disposition, and retains the exact
    /// parent handle for a separate parent-relative observation. The legacy executor is not yet
    /// adapted to call this method.
    pub(crate) fn set_delete_disposition_exact(
        self,
    ) -> std::result::Result<ManagedDeleteDisposition, ManagedDirectoryDeleteFailure> {
        if let Err(error) = validate_directory_delete_binding(&self) {
            return Err(ManagedDirectoryDeleteFailure {
                error,
                directory: self,
            });
        }
        let binding = match self.binding.as_ref() {
            Some(binding) => binding.clone(),
            None => {
                return Err(ManagedDirectoryDeleteFailure {
                    error: std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "NODE_MANAGED_DELETE_DIRECTORY_BINDING_LOST",
                    ),
                    directory: self,
                });
            }
        };
        let PinnedManagedDirectory {
            path,
            root_volume_serial,
            root_identity_digest,
            mut directory_handles,
            binding: _,
            filesystem_mutated,
        } = self;
        let target = match directory_handles.pop() {
            Some(target) => target,
            None => {
                return Err(ManagedDirectoryDeleteFailure {
                    error: std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "NODE_MANAGED_DELETE_DIRECTORY_TARGET_LOST",
                    ),
                    directory: PinnedManagedDirectory {
                        path,
                        root_volume_serial,
                        root_identity_digest,
                        directory_handles,
                        binding: Some(binding),
                        filesystem_mutated,
                    },
                });
            }
        };
        let target = match Arc::try_unwrap(target) {
            Ok(target) => target,
            Err(target) => {
                directory_handles.push(target);
                return Err(ManagedDirectoryDeleteFailure {
                    error: std::io::Error::new(
                        std::io::ErrorKind::WouldBlock,
                        "NODE_MANAGED_DELETE_DIRECTORY_HANDLE_ALIASED",
                    ),
                    directory: PinnedManagedDirectory {
                        path,
                        root_volume_serial,
                        root_identity_digest,
                        directory_handles,
                        binding: Some(binding),
                        filesystem_mutated,
                    },
                });
            }
        };
        if let Err(error) = platform::delete_by_handle(&target) {
            directory_handles.push(Arc::new(target));
            return Err(ManagedDirectoryDeleteFailure {
                error,
                directory: PinnedManagedDirectory {
                    path,
                    root_volume_serial,
                    root_identity_digest,
                    directory_handles,
                    binding: Some(binding),
                    filesystem_mutated,
                },
            });
        }
        drop(target);
        Ok(ManagedDeleteDisposition::new(
            binding,
            root_volume_serial,
            root_identity_digest,
            directory_handles,
        ))
    }
}

impl PinnedManagedFile {
    /// Strong successor to `delete_exact`; the returned non-Clone capability retains the exact
    /// parent handle and does not claim that the original name is absent.
    pub(crate) fn set_delete_disposition_exact(
        self,
    ) -> std::result::Result<ManagedDeleteDisposition, ManagedFileDeleteFailure> {
        if let Err(error) = validate_file_delete_binding(&self) {
            return Err(ManagedFileDeleteFailure { error, file: self });
        }
        if let Err(error) = platform::delete_by_handle(&self.file) {
            return Err(ManagedFileDeleteFailure { error, file: self });
        }
        let PinnedManagedFile {
            file,
            _directory_handles,
            root_volume_serial,
            root_identity_digest,
            identity: _,
            identity_digest: _,
            binding,
            directory_filesystem_mutated: _,
        } = self;
        drop(file);
        Ok(ManagedDeleteDisposition::new(
            binding,
            root_volume_serial,
            root_identity_digest,
            _directory_handles,
        ))
    }
}

fn validate_directory_delete_binding(directory: &PinnedManagedDirectory) -> std::io::Result<()> {
    let binding = directory.binding.as_ref().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "NODE_MANAGED_DELETE_DIRECTORY_BINDING_MISSING",
        )
    })?;
    if !binding.is_directory() || directory.directory_handles.len() < 2 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "NODE_MANAGED_DELETE_DIRECTORY_BINDING_INVALID",
        ));
    }
    let target = directory
        .directory_handles
        .last()
        .ok_or_else(|| std::io::Error::other("NODE_MANAGED_DELETE_DIRECTORY_HANDLE_MISSING"))?;
    let actual_identity = platform::inspect(target.as_ref())?;
    validate_directory_identity(actual_identity, Some(directory.root_volume_serial))
        .map_err(|error| std::io::Error::other(error.to_string()))?;
    let actual_identity_digest =
        identity_digest(&directory.root_identity_digest, None, actual_identity);
    let parent = directory
        .directory_handles
        .get(directory.directory_handles.len() - 2)
        .ok_or_else(|| std::io::Error::other("NODE_MANAGED_DELETE_PARENT_HANDLE_MISSING"))?;
    validate_delete_binding(
        binding,
        &actual_identity_digest,
        &directory.root_identity_digest,
        parent.as_ref(),
        directory.root_volume_serial,
    )
}

fn validate_file_delete_binding(file: &PinnedManagedFile) -> std::io::Result<()> {
    if file.binding.is_directory() || file.binding.identity_digest() != file.identity_digest {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "NODE_MANAGED_DELETE_FILE_BINDING_INVALID",
        ));
    }
    let actual_identity = platform::inspect(&file.file)?;
    validate_regular_file_identity(actual_identity, file.root_volume_serial)
        .map_err(|error| std::io::Error::other(error.to_string()))?;
    let actual_identity_digest = identity_digest(&file.root_identity_digest, None, actual_identity);
    let parent = file
        ._directory_handles
        .last()
        .ok_or_else(|| std::io::Error::other("NODE_MANAGED_DELETE_PARENT_HANDLE_MISSING"))?;
    validate_delete_binding(
        &file.binding,
        &actual_identity_digest,
        &file.root_identity_digest,
        parent.as_ref(),
        file.root_volume_serial,
    )
}

fn validate_delete_binding(
    binding: &ManagedObjectBinding,
    actual_identity_digest: &str,
    root_identity_digest: &str,
    parent: &std::fs::File,
    root_volume_serial: u64,
) -> std::io::Result<()> {
    let actual_parent_identity_digest =
        managed_parent_identity_digest(root_identity_digest, parent, root_volume_serial)
            .map_err(|error| std::io::Error::other(error.to_string()))?;
    if actual_identity_digest != binding.identity_digest()
        || actual_parent_identity_digest != binding.parent_identity_digest()
    {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "NODE_MANAGED_DELETE_BINDING_CHANGED",
        ));
    }
    Ok(())
}

#[cfg(all(test, windows))]
mod tests {
    use std::{ffi::OsStr, fs, path::Path};

    use uuid::Uuid;

    use super::*;
    use crate::node_agent_managed_fs::{ManagedParentRelativeObservation, PinnedManagedRoot};

    fn test_root() -> (std::path::PathBuf, PinnedManagedRoot) {
        let path = std::env::temp_dir().join(format!(
            "elon-managed-disposition-{}",
            Uuid::new_v4().simple()
        ));
        fs::create_dir_all(&path).expect("create test root");
        let root = PinnedManagedRoot::pin(&path, &"a".repeat(64)).expect("pin test root");
        (path, root)
    }

    #[test]
    fn cleanup_strong_disposition_requires_parent_relative_absence() {
        let (path, root) = test_root();
        let directory = root
            .prepare_directory(Path::new("candidate"))
            .expect("prepare candidate");
        let file = directory
            .create_new_read_write(OsStr::new("artifact.bin"))
            .expect("create artifact");
        let expected_identity = file.identity_digest().to_string();

        let disposition = file
            .set_delete_disposition_exact()
            .expect("set file disposition");
        assert_eq!(
            disposition.identity_digest(),
            Some(expected_identity.as_str())
        );
        let absence = match disposition
            .observe_parent_relative()
            .expect("observe file namespace")
        {
            ManagedParentRelativeObservation::Absent(absence) => absence,
            _ => panic!("disposed file name must be absent"),
        };
        assert_eq!(
            absence.object_binding().identity_digest(),
            expected_identity
        );
        drop(absence);

        let directory = root
            .pin_existing_directory_for_cleanup(Path::new("candidate"))
            .expect("pin candidate for cleanup");
        let directory_disposition = directory
            .set_delete_disposition_exact()
            .expect("set directory disposition");
        assert!(matches!(
            directory_disposition
                .observe_parent_relative()
                .expect("observe directory namespace"),
            ManagedParentRelativeObservation::Absent(_)
        ));
        drop(root);
        fs::remove_dir(path).expect("remove test root");
    }

    #[test]
    fn cleanup_strong_disposition_failure_retains_exact_directory_for_retry() {
        let (path, root) = test_root();
        fs::create_dir(path.join("candidate")).expect("create candidate");
        fs::write(path.join("candidate/artifact.bin"), b"").expect("create artifact");
        let directory = root
            .pin_existing_directory_for_cleanup(Path::new("candidate"))
            .expect("pin candidate for cleanup");
        let file = directory
            .open_existing_read_only_cleanup_child(OsStr::new("artifact.bin"))
            .expect("pin child for cleanup");
        let (error, retained_directory) = directory
            .set_delete_disposition_exact()
            .expect_err("shared child parent custody must block directory disposition")
            .into_parts();
        assert_eq!(error.kind(), std::io::ErrorKind::WouldBlock);

        let file_absence = match file
            .set_delete_disposition_exact()
            .expect("set child disposition")
            .observe_parent_relative()
            .expect("observe child absence")
        {
            ManagedParentRelativeObservation::Absent(absence) => absence,
            _ => panic!("disposed child name must be absent"),
        };
        drop(file_absence);

        assert!(matches!(
            retained_directory
                .set_delete_disposition_exact()
                .expect("retry exact retained directory")
                .observe_parent_relative()
                .expect("observe retried directory"),
            ManagedParentRelativeObservation::Absent(_)
        ));
        drop(root);
        fs::remove_dir(path).expect("remove test root");
    }
}
