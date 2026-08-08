use std::{error::Error as StdError, ffi::OsStr, fmt, path::Path};

use anyhow::{anyhow, Context, Result};

use super::{
    identity_digest, managed_parent_identity_digest, normal_relative_components, platform,
    validate_directory_identity, ManagedFileOpenFailure, ManagedObjectBinding,
    PinnedManagedDirectory, PinnedManagedFile, PinnedManagedRoot,
};

mod disposition;

/// Legacy process-local evidence used only by the unadapted cleanup executor. It does not retain
/// parent custody and cannot authorize absence, durability, completion, or recovery.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ManagedObjectDeleteEvidence {
    identity_digest: Option<String>,
    object_kind: ManagedObjectKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ManagedObjectKind {
    File,
    Directory,
}

impl ManagedObjectDeleteEvidence {
    pub(crate) fn identity_digest(&self) -> Option<&str> {
        self.identity_digest.as_deref()
    }

    pub(crate) fn is_directory(&self) -> bool {
        self.object_kind == ManagedObjectKind::Directory
    }
}

#[must_use = "failed file deletion retains the exact pinned file for retry or quarantine"]
pub(crate) struct ManagedFileDeleteFailure {
    error: std::io::Error,
    file: PinnedManagedFile,
}

#[must_use = "failed directory deletion retains the exact pinned directory for retry or quarantine"]
pub(crate) struct ManagedDirectoryDeleteFailure {
    error: std::io::Error,
    directory: PinnedManagedDirectory,
}

impl PinnedManagedRoot {
    /// Pins one existing directory with DELETE access only on the final path component. Managed
    /// prefixes retain their flush-capable access but never gain DELETE or delete sharing.
    pub(crate) fn pin_existing_directory_for_cleanup(
        &self,
        relative: &Path,
    ) -> Result<PinnedManagedDirectory> {
        let components = normal_relative_components(relative, false)?;
        let mut path = self.root_path.clone();
        let mut handles = self.root_handles.clone();
        let last_index = components.len() - 1;
        let mut binding = None;
        for (index, component) in components.into_iter().enumerate() {
            path.push(&component);
            let parent = handles
                .last()
                .ok_or_else(|| anyhow!("NODE_MANAGED_CLEANUP_DIRECTORY_PARENT_HANDLE_MISSING"))?;
            let file = if index == last_index {
                platform::open_directory_relative_deletable(parent.as_ref(), &component)
            } else {
                platform::open_managed_directory_relative(parent.as_ref(), &component)
            }
            .with_context(|| format!("NODE_MANAGED_CLEANUP_DIRECTORY_OPEN {}", path.display()))?;
            let identity = platform::inspect(&file).with_context(|| {
                format!("NODE_MANAGED_CLEANUP_DIRECTORY_INSPECT {}", path.display())
            })?;
            validate_directory_identity(identity, Some(self.root_volume_serial))?;
            binding = Some(ManagedObjectBinding::directory(
                &component,
                identity_digest(&self.root_identity_digest, None, identity),
                managed_parent_identity_digest(
                    &self.root_identity_digest,
                    parent.as_ref(),
                    self.root_volume_serial,
                )?,
            ));
            path = platform::canonical_path(&file)
                .context("NODE_MANAGED_CLEANUP_DIRECTORY_CANONICAL_PATH")?;
            handles.push(std::sync::Arc::new(file));
        }
        Ok(PinnedManagedDirectory {
            path,
            root_volume_serial: self.root_volume_serial,
            root_identity_digest: self.root_identity_digest.clone(),
            directory_handles: handles,
            binding,
            filesystem_mutated: false,
        })
    }
}

impl PinnedManagedDirectory {
    pub(crate) fn open_existing_read_only_cleanup_child(
        &self,
        name: &OsStr,
    ) -> std::result::Result<PinnedManagedFile, ManagedFileOpenFailure> {
        self.open_file(name, false, false, true)
    }

    pub(crate) fn delete_exact(
        self,
    ) -> std::result::Result<ManagedObjectDeleteEvidence, ManagedDirectoryDeleteFailure> {
        let handle = match self.directory_handles.last() {
            Some(handle) => handle,
            None => {
                return Err(ManagedDirectoryDeleteFailure {
                    error: std::io::Error::other("NODE_MANAGED_DELETE_DIRECTORY_HANDLE_MISSING"),
                    directory: self,
                });
            }
        };
        if let Err(error) = platform::delete_by_handle(handle.as_ref()) {
            return Err(ManagedDirectoryDeleteFailure {
                error,
                directory: self,
            });
        }
        let identity_digest = self
            .binding
            .as_ref()
            .map(|binding| binding.identity_digest().to_string());
        drop(self);
        Ok(ManagedObjectDeleteEvidence {
            identity_digest,
            object_kind: ManagedObjectKind::Directory,
        })
    }
}

impl PinnedManagedFile {
    pub(crate) fn delete_exact(
        self,
    ) -> std::result::Result<ManagedObjectDeleteEvidence, ManagedFileDeleteFailure> {
        if let Err(error) = platform::delete_by_handle(&self.file) {
            return Err(ManagedFileDeleteFailure { error, file: self });
        }
        let identity_digest = self.identity_digest.clone();
        drop(self);
        Ok(ManagedObjectDeleteEvidence {
            identity_digest: Some(identity_digest),
            object_kind: ManagedObjectKind::File,
        })
    }
}

impl ManagedFileDeleteFailure {
    pub(crate) fn into_parts(self) -> (std::io::Error, PinnedManagedFile) {
        (self.error, self.file)
    }
}

impl ManagedDirectoryDeleteFailure {
    pub(crate) fn into_parts(self) -> (std::io::Error, PinnedManagedDirectory) {
        (self.error, self.directory)
    }
}

macro_rules! impl_delete_failure {
    ($type:ident, $retained:literal) => {
        impl fmt::Debug for $type {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter
                    .debug_struct(stringify!($type))
                    .field("error_kind", &self.error.kind())
                    .field("raw_os_error", &self.error.raw_os_error())
                    .field("retained", &$retained)
                    .finish()
            }
        }

        impl fmt::Display for $type {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(formatter, "NODE_MANAGED_DELETE_FAILED: {}", self.error)
            }
        }

        impl StdError for $type {
            fn source(&self) -> Option<&(dyn StdError + 'static)> {
                Some(&self.error)
            }
        }
    };
}

impl_delete_failure!(ManagedFileDeleteFailure, "file-handle");
impl_delete_failure!(ManagedDirectoryDeleteFailure, "directory-handle");

#[cfg(all(test, windows))]
mod tests {
    use std::{ffi::OsStr, fs, path::Path};

    use uuid::Uuid;

    use super::*;

    fn test_root() -> (std::path::PathBuf, PinnedManagedRoot) {
        let path =
            std::env::temp_dir().join(format!("elon-managed-delete-{}", Uuid::new_v4().simple()));
        fs::create_dir_all(&path).expect("create test root");
        let root = PinnedManagedRoot::pin(&path, &"a".repeat(64)).expect("pin test root");
        (path, root)
    }

    #[test]
    fn managed_delete_removes_exact_file_then_directory() {
        let (path, root) = test_root();
        let directory = root
            .prepare_directory(Path::new("candidate"))
            .expect("prepare candidate");
        let file = directory
            .create_new_read_write(OsStr::new("artifact.bin"))
            .expect("create artifact");
        let evidence = file.delete_exact().expect("delete artifact");
        assert!(!evidence.is_directory());
        assert!(evidence.identity_digest().is_some());
        drop(evidence);

        let directory = root
            .pin_existing_directory_for_cleanup(Path::new("candidate"))
            .expect("pin candidate for cleanup");
        assert!(directory
            .delete_exact()
            .expect("delete candidate")
            .is_directory());
        assert!(!path.join("candidate").exists());
        drop(root);
        fs::remove_dir(path).expect("remove test root");
    }

    #[test]
    fn managed_delete_failure_retains_directory_for_retry() {
        let (path, root) = test_root();
        fs::create_dir(path.join("candidate")).expect("create candidate");
        fs::write(path.join("candidate/artifact.bin"), b"retained").expect("write artifact");
        let cleanup = root
            .pin_existing_directory_for_cleanup(Path::new("candidate"))
            .expect("pin candidate for cleanup");
        let file = cleanup
            .open_existing_read_only_cleanup_child(OsStr::new("artifact.bin"))
            .expect("pin artifact for cleanup");
        let (_, retained) = cleanup
            .delete_exact()
            .expect_err("non-empty directory must fail")
            .into_parts();

        file.delete_exact().expect("delete child");
        retained.delete_exact().expect("retry retained directory");
        assert!(!path.join("candidate").exists());
        drop(root);
        fs::remove_dir(path).expect("remove test root");
    }

    #[test]
    fn managed_delete_reopens_existing_file_with_delete_custody() {
        let (path, root) = test_root();
        let directory = root
            .prepare_directory(Path::new("candidate"))
            .expect("prepare candidate");
        drop(
            directory
                .create_new_read_write(OsStr::new("artifact.bin"))
                .expect("create artifact"),
        );

        let directory = root
            .pin_existing_directory_for_cleanup(Path::new("candidate"))
            .expect("pin candidate for cleanup");
        directory
            .open_existing_read_only_cleanup_child(OsStr::new("artifact.bin"))
            .expect("pin existing artifact for cleanup")
            .delete_exact()
            .expect("delete existing artifact");
        directory.delete_exact().expect("delete candidate");

        assert!(!path.join("candidate").exists());
        drop(root);
        fs::remove_dir(path).expect("remove test root");
    }
}
