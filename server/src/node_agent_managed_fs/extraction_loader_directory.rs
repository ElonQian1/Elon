//! Linear extraction-directory custody. Descendants are create-new only and are reached from an
//! already retained parent handle; no path/global-root reopen exists in this module.

#![allow(dead_code)]
use super::{
    identity_digest, managed_parent_identity_digest, platform, require_single_normal_component,
    same_file_identity, validate_directory_identity, ManagedFileOpenFailure, ManagedObjectBinding,
    PinnedManagedDirectory, PinnedManagedFile, PlatformFileIdentity,
};
use anyhow::{anyhow, Context, Error, Result};
use std::{error::Error as StdError, ffi::OsStr, fmt, fs::File, path::PathBuf, sync::Arc};
pub(super) struct PlatformExtractionLoaderDirectoryProbe {
    pub(super) file: File,
    pub(super) retained_delete_owner_granted_access: u32,
    pub(super) probe_granted_access: u32,
}
pub(super) struct PlatformExtractionLoaderDirectoryProbeFailure {
    pub(super) error: std::io::Error,
    pub(super) probe: Option<File>,
}
impl PlatformExtractionLoaderDirectoryProbeFailure {
    pub(super) fn before_probe(error: std::io::Error) -> Self {
        Self { error, probe: None }
    }
    pub(super) fn after_probe(error: std::io::Error, probe: File) -> Self {
        Self {
            error,
            probe: Some(probe),
        }
    }
}
#[must_use = "share-delete proof must remain with extraction loader custody"]
struct ManagedExtractionLoaderDirectoryShareReceipt {
    directory_identity: PlatformFileIdentity,
    retained_owner_canonical_path: PathBuf,
    probe_canonical_path: PathBuf,
    retained_delete_owner_granted_access: u32,
    probe_granted_access: u32,
}
#[must_use = "extraction loader directory custody must enter cleanup or loader ownership"]
pub(crate) struct PinnedManagedExtractionLoaderDirectory {
    directory: PinnedManagedDirectory,
    _share_delete_probe: File,
    _receipt: ManagedExtractionLoaderDirectoryShareReceipt,
}
#[must_use = "failed extraction loader custody must remain quarantined"]
pub(crate) struct ManagedExtractionLoaderDirectoryFailure {
    error: Error,
    _directory: PinnedManagedDirectory,
    _probe: Option<File>,
}
#[must_use = "failed descendant custody must be handled explicitly"]
pub(crate) enum ManagedExtractionLoaderDirectoryChildFailure {
    BeforeOwner(Error),
    OwnerRejected {
        _error: Error,
        _owner: File,
        _parent_handles: Vec<Arc<File>>,
    },
    Compatibility(ManagedExtractionLoaderDirectoryFailure),
}
#[must_use = "failed extraction file custody must be handled explicitly"]
pub(crate) struct ManagedExtractionLoaderFileFailure(ManagedFileOpenFailure);

impl PinnedManagedDirectory {
    pub(crate) fn into_extraction_loader_directory_custody(
        self,
    ) -> std::result::Result<
        PinnedManagedExtractionLoaderDirectory,
        ManagedExtractionLoaderDirectoryFailure,
    > {
        seal_extraction_loader_directory(self)
    }

    /// Creates exactly one new directory from this borrowed retained parent. AlreadyExists and
    /// every other non-success fail closed; there is deliberately no existing-object fallback.
    pub(crate) fn create_new_extraction_loader_directory_child(
        &self,
        name: &OsStr,
    ) -> std::result::Result<PinnedManagedDirectory, ManagedExtractionLoaderDirectoryChildFailure>
    {
        create_new_directory_child(self, name)
    }

    /// Creates one file from this borrowed retained parent without closing or reopening it.
    pub(crate) fn create_new_extraction_loader_file_child(
        &self,
        name: &OsStr,
    ) -> std::result::Result<PinnedManagedFile, ManagedExtractionLoaderFileFailure> {
        clone_directory(self)
            .create_new_read_write(name)
            .map_err(ManagedExtractionLoaderFileFailure)
    }
}

impl PinnedManagedExtractionLoaderDirectory {
    pub(crate) fn create_new_directory_child(
        &self,
        name: &OsStr,
    ) -> std::result::Result<PinnedManagedDirectory, ManagedExtractionLoaderDirectoryChildFailure>
    {
        self.directory
            .create_new_extraction_loader_directory_child(name)
    }

    pub(crate) fn create_new_file_child(
        &self,
        name: &OsStr,
    ) -> std::result::Result<PinnedManagedFile, ManagedExtractionLoaderFileFailure> {
        self.directory.create_new_extraction_loader_file_child(name)
    }

    /// The live probe and handle-derived receipt cannot be split from the typed owner.
    pub(crate) fn into_loader_parts(self) -> Self {
        self
    }

    pub(crate) fn into_cleanup_directory(self) -> PinnedManagedDirectory {
        self.directory
    }
}

fn create_new_directory_child(
    parent: &PinnedManagedDirectory,
    name: &OsStr,
) -> std::result::Result<PinnedManagedDirectory, ManagedExtractionLoaderDirectoryChildFailure> {
    require_single_normal_component(name)
        .map_err(ManagedExtractionLoaderDirectoryChildFailure::BeforeOwner)?;
    let parent_handle = parent.directory_handles.last().ok_or_else(|| {
        ManagedExtractionLoaderDirectoryChildFailure::BeforeOwner(anyhow!(
            "NODE_MANAGED_EXTRACTION_PARENT_MISSING"
        ))
    })?;
    let owner = platform::create_new_directory_relative(parent_handle, name)
        .map_err(|error| ManagedExtractionLoaderDirectoryChildFailure::BeforeOwner(error.into()))?;
    let (path, binding) = match inspect_child(parent, name, &owner) {
        Ok(result) => result,
        Err(error) => {
            return Err(
                ManagedExtractionLoaderDirectoryChildFailure::OwnerRejected {
                    _error: error,
                    _owner: owner,
                    _parent_handles: parent.directory_handles.clone(),
                },
            );
        }
    };
    let mut handles = parent.directory_handles.clone();
    handles.push(Arc::new(owner));
    let child = PinnedManagedDirectory {
        path,
        root_volume_serial: parent.root_volume_serial,
        root_identity_digest: parent.root_identity_digest.clone(),
        directory_handles: handles,
        binding: Some(binding),
        filesystem_mutated: true,
    };
    seal_extraction_loader_directory(child)
        .map(|custody| custody.directory)
        .map_err(ManagedExtractionLoaderDirectoryChildFailure::Compatibility)
}

fn inspect_child(
    parent: &PinnedManagedDirectory,
    name: &OsStr,
    owner: &File,
) -> Result<(PathBuf, ManagedObjectBinding)> {
    let identity = platform::inspect(owner).context("NODE_MANAGED_EXTRACTION_CHILD_INSPECT")?;
    validate_directory_identity(identity, Some(parent.root_volume_serial))?;
    let parent_handle = parent
        .directory_handles
        .last()
        .ok_or_else(|| anyhow!("NODE_MANAGED_EXTRACTION_PARENT_MISSING"))?;
    let binding = ManagedObjectBinding::directory(
        name,
        identity_digest(&parent.root_identity_digest, None, identity),
        managed_parent_identity_digest(
            &parent.root_identity_digest,
            parent_handle,
            parent.root_volume_serial,
        )?,
    );
    let path =
        platform::canonical_path(owner).context("NODE_MANAGED_EXTRACTION_CHILD_CANONICAL_PATH")?;
    Ok((path, binding))
}

fn seal_extraction_loader_directory(
    directory: PinnedManagedDirectory,
) -> std::result::Result<
    PinnedManagedExtractionLoaderDirectory,
    ManagedExtractionLoaderDirectoryFailure,
> {
    match try_seal(&directory) {
        Ok((probe, receipt)) => Ok(PinnedManagedExtractionLoaderDirectory {
            directory,
            _share_delete_probe: probe,
            _receipt: receipt,
        }),
        Err(SealFailure { error, probe }) => Err(ManagedExtractionLoaderDirectoryFailure {
            error,
            _directory: directory,
            _probe: probe,
        }),
    }
}

struct SealFailure {
    error: Error,
    probe: Option<File>,
}

fn try_seal(
    directory: &PinnedManagedDirectory,
) -> std::result::Result<(File, ManagedExtractionLoaderDirectoryShareReceipt), SealFailure> {
    let (owner, parent, name, owner_identity, owner_path) =
        validate_retained_owner(directory).map_err(|error| SealFailure { error, probe: None })?;
    let platform_probe = platform::probe_extraction_loader_directory_relative(owner, parent, name)
        .map_err(|failure| SealFailure {
            error: failure.error.into(),
            probe: failure.probe,
        })?;
    let probe = platform_probe.file;
    let probe_identity =
        match platform::inspect(&probe).context("NODE_MANAGED_EXTRACTION_LOADER_PROBE_INSPECT") {
            Ok(identity) => identity,
            Err(error) => {
                return Err(SealFailure {
                    error,
                    probe: Some(probe),
                })
            }
        };
    if let Err(error) =
        validate_directory_identity(probe_identity, Some(directory.root_volume_serial))
    {
        return Err(SealFailure {
            error,
            probe: Some(probe),
        });
    }
    if !same_file_identity(owner_identity, probe_identity) {
        return Err(SealFailure {
            error: anyhow!("NODE_MANAGED_EXTRACTION_LOADER_FILE_ID_CHANGED"),
            probe: Some(probe),
        });
    }
    let probe_path = match platform::canonical_path(&probe)
        .context("NODE_MANAGED_EXTRACTION_LOADER_PROBE_PATH")
    {
        Ok(path) => path,
        Err(error) => {
            return Err(SealFailure {
                error,
                probe: Some(probe),
            })
        }
    };
    if probe_path != owner_path {
        return Err(SealFailure {
            error: anyhow!("NODE_MANAGED_EXTRACTION_LOADER_PATH_CHANGED"),
            probe: Some(probe),
        });
    }
    Ok((
        probe,
        ManagedExtractionLoaderDirectoryShareReceipt {
            directory_identity: owner_identity,
            retained_owner_canonical_path: owner_path,
            probe_canonical_path: probe_path,
            retained_delete_owner_granted_access: platform_probe
                .retained_delete_owner_granted_access,
            probe_granted_access: platform_probe.probe_granted_access,
        },
    ))
}

fn validate_retained_owner(
    directory: &PinnedManagedDirectory,
) -> Result<(&File, &File, &OsStr, PlatformFileIdentity, PathBuf)> {
    let binding = directory
        .binding
        .as_ref()
        .filter(|binding| binding.is_directory())
        .ok_or_else(|| anyhow!("NODE_MANAGED_EXTRACTION_LOADER_BINDING_INVALID"))?;
    require_single_normal_component(binding.relative_name())?;
    let owner = directory
        .directory_handles
        .last()
        .ok_or_else(|| anyhow!("NODE_MANAGED_EXTRACTION_LOADER_OWNER_MISSING"))?;
    let parent = directory
        .directory_handles
        .get(
            directory
                .directory_handles
                .len()
                .checked_sub(2)
                .ok_or_else(|| anyhow!("NODE_MANAGED_EXTRACTION_LOADER_PARENT_MISSING"))?,
        )
        .ok_or_else(|| anyhow!("NODE_MANAGED_EXTRACTION_LOADER_PARENT_MISSING"))?;
    let identity =
        platform::inspect(owner).context("NODE_MANAGED_EXTRACTION_LOADER_OWNER_INSPECT")?;
    validate_directory_identity(identity, Some(directory.root_volume_serial))?;
    let digest = identity_digest(&directory.root_identity_digest, None, identity);
    let parent_digest = managed_parent_identity_digest(
        &directory.root_identity_digest,
        parent,
        directory.root_volume_serial,
    )?;
    if digest != binding.identity_digest() || parent_digest != binding.parent_identity_digest() {
        return Err(anyhow!("NODE_MANAGED_EXTRACTION_LOADER_BINDING_CHANGED"));
    }
    let path =
        platform::canonical_path(owner).context("NODE_MANAGED_EXTRACTION_LOADER_OWNER_PATH")?;
    if path != directory.path {
        return Err(anyhow!("NODE_MANAGED_EXTRACTION_LOADER_OWNER_PATH_CHANGED"));
    }
    Ok((owner, parent, binding.relative_name(), identity, path))
}

fn clone_directory(directory: &PinnedManagedDirectory) -> PinnedManagedDirectory {
    PinnedManagedDirectory {
        path: directory.path.clone(),
        root_volume_serial: directory.root_volume_serial,
        root_identity_digest: directory.root_identity_digest.clone(),
        directory_handles: directory.directory_handles.clone(),
        binding: directory.binding.clone(),
        filesystem_mutated: directory.filesystem_mutated,
    }
}

impl fmt::Debug for PinnedManagedExtractionLoaderDirectory {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PinnedManagedExtractionLoaderDirectory")
            .field("directory", &"<retained-delete-owner>")
            .field("share_delete_probe", &"<retained>")
            .field("receipt", &"<handle-derived>")
            .finish()
    }
}

impl fmt::Debug for ManagedExtractionLoaderDirectoryFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple("ManagedExtractionLoaderDirectoryFailure")
            .field(&"<retained>")
            .finish()
    }
}

impl fmt::Debug for ManagedExtractionLoaderDirectoryChildFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple("ManagedExtractionLoaderDirectoryChildFailure")
            .field(&"<retained-if-opened>")
            .finish()
    }
}

impl fmt::Debug for ManagedExtractionLoaderFileFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple("ManagedExtractionLoaderFileFailure")
            .field(&"<retained>")
            .finish()
    }
}

macro_rules! opaque_custody_error {
    ($kind:ty, $message:literal) => {
        impl fmt::Display for $kind {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str($message)
            }
        }
        impl StdError for $kind {}
    };
}

opaque_custody_error!(
    ManagedExtractionLoaderDirectoryFailure,
    "NODE_MANAGED_EXTRACTION_LOADER_DIRECTORY_FAILED"
);
opaque_custody_error!(
    ManagedExtractionLoaderDirectoryChildFailure,
    "NODE_MANAGED_EXTRACTION_LOADER_CHILD_FAILED"
);
opaque_custody_error!(
    ManagedExtractionLoaderFileFailure,
    "NODE_MANAGED_EXTRACTION_LOADER_FILE_FAILED"
);
