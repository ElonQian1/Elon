//! Handle-pinned filesystem primitives for security-sensitive NodeAgent content. Windows child
//! lookups are parent-handle relative; other platforms fail closed until they provide a native
//! beneath/no-follow implementation.

use std::{
    ffi::{OsStr, OsString},
    io::{Read, Seek, SeekFrom},
    path::{Component, Path, PathBuf},
    sync::Arc,
};

use anyhow::{anyhow, bail, Context, Result};
use sha2::{Digest, Sha256};

mod copy;
mod delete;
mod hash;
mod lock;
mod open;
#[cfg(not(windows))]
#[path = "node_agent_managed_fs/unsupported.rs"]
mod platform;
mod read;
mod types;
mod write;

pub(crate) use copy::ManagedFileCopyResult;
pub(crate) use delete::{
    ManagedDirectoryDeleteFailure, ManagedFileDeleteFailure, ManagedObjectDeleteEvidence,
};
pub(crate) use hash::{ManagedFileHashFailure, ManagedFileHashPhase, ManagedFileHashResult};
pub(crate) use read::ManagedFileReadCursor;
pub(crate) use types::{
    ManagedDirectoryPrepareFailure, ManagedExclusiveFileLockFailure, ManagedFileOpenFailure,
    PinnedManagedDirectory, PinnedManagedExclusiveFileLock, PinnedManagedFile, PinnedManagedRoot,
    QuarantinedManagedFile,
};
pub(crate) use write::{ManagedFileSegmentWriteFailure, ManagedFileSegmentWritePhase};
#[cfg(windows)]
#[path = "node_agent_managed_fs/windows.rs"]
mod platform;

const MANAGED_FILE_ID_DOMAIN: &[u8] = b"ELON_NODE_MANAGED_FILE_ID_V1";
const MAX_PINNED_MARKER_BYTES: usize = 16 * 1024;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) struct PlatformFileIdentity {
    pub(super) volume_serial: u64,
    pub(super) file_id: [u8; 16],
    pub(super) number_of_links: u32,
    pub(super) file_size: u64,
    pub(super) is_directory: bool,
    pub(super) is_reparse_point: bool,
}

impl PinnedManagedRoot {
    pub(crate) fn pin(root: &Path, installation_binding_digest: &str) -> Result<Self> {
        require_sha256(installation_binding_digest)?;
        let (initial_path, components) = absolute_root_parts(root)?;
        let initial_file = platform::open_initial_directory(&initial_path)
            .with_context(|| format!("NODE_MANAGED_ROOT_VOLUME_OPEN {}", initial_path.display()))?;
        let initial_identity =
            platform::inspect(&initial_file).context("NODE_MANAGED_ROOT_VOLUME_INSPECT")?;
        validate_directory_identity(initial_identity, None)?;
        let mut current_path = platform::canonical_path(&initial_file)
            .context("NODE_MANAGED_ROOT_VOLUME_CANONICAL")?;
        let expected_volume = initial_identity.volume_serial;
        let mut root_identity = initial_identity;
        let mut root_handles = Vec::with_capacity(components.len() + 1);
        root_handles.push(Arc::new(initial_file));
        for component in components {
            current_path.push(&component);
            let parent = root_handles
                .last()
                .ok_or_else(|| anyhow!("NODE_MANAGED_ROOT_PARENT_HANDLE_MISSING"))?;
            let file = Arc::new(
                platform::open_directory_relative(parent.as_ref(), &component).with_context(
                    || format!("NODE_MANAGED_ROOT_PREFIX_OPEN {}", current_path.display()),
                )?,
            );
            let identity = platform::inspect(&file).with_context(|| {
                format!(
                    "NODE_MANAGED_ROOT_PREFIX_INSPECT {}",
                    current_path.display()
                )
            })?;
            validate_directory_identity(identity, Some(expected_volume))?;
            current_path = platform::canonical_path(&file).with_context(|| {
                format!(
                    "NODE_MANAGED_ROOT_PREFIX_CANONICAL {}",
                    current_path.display()
                )
            })?;
            root_identity = identity;
            root_handles.push(file);
        }
        Ok(Self {
            root_path: current_path,
            root_volume_serial: root_identity.volume_serial,
            installation_binding_digest: installation_binding_digest.to_string(),
            root_identity_digest: identity_digest(installation_binding_digest, None, root_identity),
            root_handles,
        })
    }

    pub(crate) fn installation_binding_digest(&self) -> &str {
        &self.installation_binding_digest
    }

    pub(crate) fn root_identity_digest(&self) -> &str {
        &self.root_identity_digest
    }

    pub(crate) fn open_existing_read_only(
        &self,
        relative: &Path,
    ) -> std::result::Result<PinnedManagedFile, ManagedFileOpenFailure> {
        let (parent, name) = split_relative_file(relative).map_err(io_invalid_input)?;
        let directory = self.pin_existing_directory(&parent).map_err(io_other)?;
        directory.open_file(&name, false, false, false)
    }

    pub(crate) fn prepare_directory(
        &self,
        relative: &Path,
    ) -> std::result::Result<PinnedManagedDirectory, ManagedDirectoryPrepareFailure> {
        let components = normal_relative_components(relative, true)
            .map_err(ManagedDirectoryPrepareFailure::Unchanged)?;
        let mut path = self.root_path.clone();
        let mut handles = self.root_handles.clone();
        let mut filesystem_mutated = false;
        for component in components {
            path.push(&component);
            let parent = match handles.last() {
                Some(parent) => parent,
                None => {
                    return Err(directory_prepare_failure(
                        anyhow!("NODE_MANAGED_DIRECTORY_PARENT_HANDLE_MISSING"),
                        filesystem_mutated,
                    ));
                }
            };
            let file = match platform::open_directory_relative(parent.as_ref(), &component) {
                Ok(file) => file,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    match platform::create_new_directory_relative(parent.as_ref(), &component) {
                        Ok(file) => {
                            filesystem_mutated = true;
                            file
                        }
                        Err(create_error)
                            if create_error.kind() == std::io::ErrorKind::AlreadyExists =>
                        {
                            filesystem_mutated = true;
                            match platform::open_directory_relative(parent.as_ref(), &component) {
                                Ok(file) => file,
                                Err(error) => {
                                    return Err(directory_prepare_failure(
                                        anyhow::Error::new(error).context(format!(
                                            "NODE_MANAGED_DIRECTORY_OPEN_RACED {}",
                                            path.display()
                                        )),
                                        filesystem_mutated,
                                    ));
                                }
                            }
                        }
                        Err(create_error) => {
                            return Err(directory_prepare_failure(
                                anyhow::Error::new(create_error).context(format!(
                                    "NODE_MANAGED_DIRECTORY_CREATE {}",
                                    path.display()
                                )),
                                filesystem_mutated,
                            ));
                        }
                    }
                }
                Err(error) => {
                    return Err(directory_prepare_failure(
                        anyhow::Error::new(error)
                            .context(format!("NODE_MANAGED_DIRECTORY_OPEN {}", path.display())),
                        filesystem_mutated,
                    ));
                }
            };
            let file = Arc::new(file);
            let identity = match platform::inspect(&file) {
                Ok(identity) => identity,
                Err(error) => {
                    return Err(directory_prepare_failure(
                        anyhow::Error::new(error)
                            .context(format!("NODE_MANAGED_DIRECTORY_INSPECT {}", path.display())),
                        filesystem_mutated,
                    ));
                }
            };
            if let Err(error) = validate_directory_identity(identity, Some(self.root_volume_serial))
            {
                return Err(directory_prepare_failure(error, filesystem_mutated));
            }
            path = match platform::canonical_path(&file) {
                Ok(path) => path,
                Err(error) => {
                    return Err(directory_prepare_failure(
                        anyhow::Error::new(error).context("NODE_MANAGED_DIRECTORY_CANONICAL_PATH"),
                        filesystem_mutated,
                    ));
                }
            };
            handles.push(file);
        }
        Ok(PinnedManagedDirectory {
            path,
            root_volume_serial: self.root_volume_serial,
            root_identity_digest: self.root_identity_digest.clone(),
            directory_handles: handles,
            filesystem_mutated,
        })
    }

    pub(crate) fn pin_existing_directory(&self, relative: &Path) -> Result<PinnedManagedDirectory> {
        let components = normal_relative_components(relative, true)?;
        let mut path = self.root_path.clone();
        let mut handles = self.root_handles.clone();
        for component in components {
            path.push(&component);
            let parent = handles
                .last()
                .ok_or_else(|| anyhow!("NODE_MANAGED_DIRECTORY_PARENT_HANDLE_MISSING"))?;
            let file = Arc::new(
                platform::open_directory_relative(parent.as_ref(), &component)
                    .with_context(|| format!("NODE_MANAGED_DIRECTORY_OPEN {}", path.display()))?,
            );
            let identity = platform::inspect(&file)
                .with_context(|| format!("NODE_MANAGED_DIRECTORY_INSPECT {}", path.display()))?;
            validate_directory_identity(identity, Some(self.root_volume_serial))?;
            path =
                platform::canonical_path(&file).context("NODE_MANAGED_DIRECTORY_CANONICAL_PATH")?;
            handles.push(file);
        }
        Ok(PinnedManagedDirectory {
            path,
            root_volume_serial: self.root_volume_serial,
            root_identity_digest: self.root_identity_digest.clone(),
            directory_handles: handles,
            filesystem_mutated: false,
        })
    }
}

impl PinnedManagedFile {
    pub(crate) fn len_bytes(&self) -> u64 {
        self.identity.file_size
    }

    pub(crate) fn identity_digest(&self) -> &str {
        &self.identity_digest
    }

    pub(crate) fn directory_filesystem_mutated(&self) -> bool {
        self.directory_filesystem_mutated
    }

    pub(crate) fn read_utf8_limited(&mut self) -> Result<String> {
        self.file.seek(SeekFrom::Start(0))?;
        let mut bytes = Vec::new();
        self.file
            .by_ref()
            .take((MAX_PINNED_MARKER_BYTES + 1) as u64)
            .read_to_end(&mut bytes)?;
        if bytes.len() > MAX_PINNED_MARKER_BYTES {
            bail!("NODE_MANAGED_FILE_READ_LIMIT");
        }
        String::from_utf8(bytes).map_err(|_| anyhow!("NODE_MANAGED_FILE_NOT_UTF8"))
    }
}

fn absolute_root_parts(root: &Path) -> Result<(PathBuf, Vec<OsString>)> {
    if !root.is_absolute() {
        bail!("NODE_MANAGED_ROOT_NOT_ABSOLUTE");
    }
    let mut base = PathBuf::new();
    let mut components = Vec::new();
    for component in root.components() {
        match component {
            Component::Prefix(_) | Component::RootDir if components.is_empty() => {
                base.push(component.as_os_str());
            }
            Component::Normal(value) => components.push(value.to_os_string()),
            Component::CurDir | Component::ParentDir => bail!("NODE_MANAGED_ROOT_NOT_NORMALIZED"),
            Component::Prefix(_) | Component::RootDir => bail!("NODE_MANAGED_ROOT_INVALID"),
        }
    }
    if !base.is_absolute() || components.is_empty() {
        bail!("NODE_MANAGED_ROOT_IS_FILESYSTEM_ROOT");
    }
    Ok((base, components))
}

fn normal_relative_components(path: &Path, allow_empty: bool) -> Result<Vec<OsString>> {
    let mut components = Vec::new();
    for component in path.components() {
        match component {
            Component::Normal(value) => components.push(value.to_os_string()),
            _ => bail!("NODE_MANAGED_RELATIVE_PATH_INVALID"),
        }
    }
    if components.is_empty() && !allow_empty {
        bail!("NODE_MANAGED_RELATIVE_PATH_EMPTY");
    }
    Ok(components)
}

fn split_relative_file(path: &Path) -> Result<(PathBuf, OsString)> {
    let mut components = normal_relative_components(path, false)?;
    let name = components
        .pop()
        .ok_or_else(|| anyhow!("NODE_MANAGED_FILE_NAME_MISSING"))?;
    let mut parent = PathBuf::new();
    for component in components {
        parent.push(component);
    }
    Ok((parent, name))
}

fn require_single_normal_component(name: &OsStr) -> Result<()> {
    let path = Path::new(name);
    let components = normal_relative_components(path, false)?;
    if components.len() != 1 {
        bail!("NODE_MANAGED_FILE_NAME_INVALID");
    }
    Ok(())
}

fn validate_directory_identity(
    identity: PlatformFileIdentity,
    expected_volume: Option<u64>,
) -> Result<()> {
    if !identity.is_directory || identity.is_reparse_point {
        bail!("NODE_MANAGED_DIRECTORY_IDENTITY_INVALID");
    }
    if expected_volume.is_some_and(|volume| volume != identity.volume_serial) {
        bail!("NODE_MANAGED_DIRECTORY_VOLUME_CHANGED");
    }
    Ok(())
}

fn validate_regular_file_identity(
    identity: PlatformFileIdentity,
    expected_volume: u64,
) -> Result<()> {
    if identity.is_directory
        || identity.is_reparse_point
        || identity.number_of_links != 1
        || identity.volume_serial != expected_volume
    {
        bail!("NODE_MANAGED_REGULAR_FILE_IDENTITY_INVALID");
    }
    Ok(())
}

fn same_file_identity(left: PlatformFileIdentity, right: PlatformFileIdentity) -> bool {
    left.volume_serial == right.volume_serial && left.file_id == right.file_id
}

fn identity_digest(
    binding_digest: &str,
    root_identity_digest: Option<&str>,
    identity: PlatformFileIdentity,
) -> String {
    let mut digest = Sha256::new();
    digest.update(MANAGED_FILE_ID_DOMAIN);
    digest.update([0]);
    digest.update(binding_digest.as_bytes());
    digest.update([0]);
    if let Some(root_identity_digest) = root_identity_digest {
        digest.update(root_identity_digest.as_bytes());
    }
    digest.update([0]);
    digest.update(identity.volume_serial.to_le_bytes());
    digest.update(identity.file_id);
    hex::encode(digest.finalize())
}

fn require_sha256(value: &str) -> Result<()> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|value| value.is_ascii_digit() || (b'a'..=b'f').contains(&value))
    {
        bail!("NODE_MANAGED_INSTALLATION_BINDING_INVALID");
    }
    Ok(())
}

fn io_invalid_input(error: anyhow::Error) -> ManagedFileOpenFailure {
    ManagedFileOpenFailure::NotOpened(std::io::Error::new(
        std::io::ErrorKind::InvalidInput,
        error.to_string(),
    ))
}

fn io_other(error: anyhow::Error) -> ManagedFileOpenFailure {
    ManagedFileOpenFailure::NotOpened(std::io::Error::other(error.to_string()))
}

fn directory_prepare_failure(
    error: anyhow::Error,
    filesystem_mutated: bool,
) -> ManagedDirectoryPrepareFailure {
    if filesystem_mutated {
        ManagedDirectoryPrepareFailure::Mutated(error)
    } else {
        ManagedDirectoryPrepareFailure::Unchanged(error)
    }
}
