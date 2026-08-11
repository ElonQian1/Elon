use std::{
    fs::{File, Metadata},
    path::{Path, PathBuf},
};

use crate::compute_federation::external_pool_adapter_artifact_source::ExternalPoolAdapterArtifactSourceFsError;

const QUARANTINE_COMPONENTS: [&str; 6] = [
    "compute-federation",
    "external-pool-adapter-artifacts",
    "v1",
    "quarantine",
    "blobs",
    "sha256",
];
const QUARANTINE_ROOT_COMPONENTS: usize = 4;

pub(crate) struct ArtifactPaths {
    shard_dir: PathBuf,
    final_path: PathBuf,
    quarantine_root: PathBuf,
    digest: String,
    pinned_directories: Vec<File>,
}

impl ArtifactPaths {
    pub(crate) fn temporary_path(&self) -> PathBuf {
        self.shard_dir.join(format!(
            ".{}.{}.part",
            self.digest,
            uuid::Uuid::new_v4().simple()
        ))
    }

    pub(crate) fn final_path(&self) -> &Path {
        &self.final_path
    }

    pub(super) fn quarantine_root(&self) -> &Path {
        &self.quarantine_root
    }

    pub(super) fn into_pinned_directories(self) -> Vec<File> {
        self.pinned_directories
    }
}

pub(crate) async fn prepare_paths(
    data_dir: &Path,
    digest: &str,
) -> Result<ArtifactPaths, ExternalPoolAdapterArtifactSourceFsError> {
    artifact_paths(data_dir, digest, true).await
}

pub(crate) async fn locate_paths(
    data_dir: &Path,
    digest: &str,
) -> Result<ArtifactPaths, ExternalPoolAdapterArtifactSourceFsError> {
    artifact_paths(data_dir, digest, false).await
}

async fn artifact_paths(
    data_dir: &Path,
    digest: &str,
    create_missing: bool,
) -> Result<ArtifactPaths, ExternalPoolAdapterArtifactSourceFsError> {
    let data_dir = data_dir.to_path_buf();
    let quarantine_root = QUARANTINE_COMPONENTS[..QUARANTINE_ROOT_COMPONENTS]
        .iter()
        .fold(data_dir.clone(), |path, component| path.join(component));
    let digest = digest.to_string();
    let task_digest = digest.clone();
    let (shard_dir, pinned_directories) = tokio::task::spawn_blocking(move || {
        ensure_quarantine_directory(&data_dir, &task_digest[..2], create_missing)
    })
    .await
    .map_err(ExternalPoolAdapterArtifactSourceFsError::Task)??;
    let final_path = shard_dir.join(format!("{digest}.blob"));
    Ok(ArtifactPaths {
        shard_dir,
        final_path,
        quarantine_root,
        digest,
        pinned_directories,
    })
}

fn ensure_quarantine_directory(
    data_dir: &Path,
    shard: &str,
    create_missing: bool,
) -> Result<(PathBuf, Vec<File>), ExternalPoolAdapterArtifactSourceFsError> {
    if create_missing {
        std::fs::create_dir_all(data_dir)
            .map_err(ExternalPoolAdapterArtifactSourceFsError::Storage)?;
    }
    ensure_directory(data_dir, false, create_missing)?;
    let mut pinned_directories = Vec::new();
    pin_directory(data_dir, &mut pinned_directories)?;
    let mut current = data_dir.to_path_buf();
    for component in QUARANTINE_COMPONENTS
        .into_iter()
        .chain(std::iter::once(shard))
    {
        current.push(component);
        ensure_directory(&current, true, create_missing)?;
        pin_directory(&current, &mut pinned_directories)?;
    }
    Ok((current, pinned_directories))
}

fn ensure_directory(
    path: &Path,
    private_when_created: bool,
    create_missing: bool,
) -> Result<(), ExternalPoolAdapterArtifactSourceFsError> {
    match std::fs::symlink_metadata(path) {
        Ok(metadata) => require_safe_directory(&metadata, private_when_created),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            if !create_missing {
                return Err(ExternalPoolAdapterArtifactSourceFsError::BlobMissing);
            }
            let created = match std::fs::create_dir(path) {
                Ok(()) => true,
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => false,
                Err(error) => {
                    return Err(ExternalPoolAdapterArtifactSourceFsError::Storage(error));
                }
            };
            if created && private_when_created {
                set_private_directory_permissions(path)?;
            }
            let metadata = std::fs::symlink_metadata(path)
                .map_err(ExternalPoolAdapterArtifactSourceFsError::Storage)?;
            require_safe_directory(&metadata, private_when_created)
        }
        Err(error) => Err(ExternalPoolAdapterArtifactSourceFsError::Storage(error)),
    }
}

#[cfg(unix)]
fn set_private_directory_permissions(
    path: &Path,
) -> Result<(), ExternalPoolAdapterArtifactSourceFsError> {
    use std::os::unix::fs::PermissionsExt;

    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700))
        .map_err(ExternalPoolAdapterArtifactSourceFsError::Storage)
}

#[cfg(not(unix))]
fn set_private_directory_permissions(
    _path: &Path,
) -> Result<(), ExternalPoolAdapterArtifactSourceFsError> {
    Ok(())
}

fn require_safe_directory(
    metadata: &Metadata,
    private_namespace: bool,
) -> Result<(), ExternalPoolAdapterArtifactSourceFsError> {
    if unsafe_file_type(metadata) || !metadata.is_dir() {
        return Err(ExternalPoolAdapterArtifactSourceFsError::UnsafeTarget);
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;

        let mode = metadata.mode() & 0o777;
        if (private_namespace && mode != 0o700) || (!private_namespace && mode & 0o022 != 0) {
            return Err(ExternalPoolAdapterArtifactSourceFsError::UnsafeTarget);
        }
    }
    Ok(())
}

#[cfg(windows)]
fn pin_directory(
    path: &Path,
    pinned_directories: &mut Vec<File>,
) -> Result<(), ExternalPoolAdapterArtifactSourceFsError> {
    use std::fs::OpenOptions;
    use std::os::windows::fs::OpenOptionsExt;

    const FILE_SHARE_READ: u32 = 0x0000_0001;
    const FILE_SHARE_WRITE: u32 = 0x0000_0002;
    const FILE_FLAG_BACKUP_SEMANTICS: u32 = 0x0200_0000;
    const FILE_FLAG_OPEN_REPARSE_POINT: u32 = 0x0020_0000;

    // The quarantine writer needs directory write sharing for create-new files and hard-link
    // installation. DELETE sharing remains intentionally excluded so no pinned directory can be
    // renamed or replaced while its path and leaf identity are being verified.
    let directory = OpenOptions::new()
        .read(true)
        .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE)
        .custom_flags(FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT)
        .open(path)
        .map_err(ExternalPoolAdapterArtifactSourceFsError::Storage)?;
    let metadata = directory
        .metadata()
        .map_err(ExternalPoolAdapterArtifactSourceFsError::Storage)?;
    require_safe_directory(&metadata, false)?;
    pinned_directories.push(directory);
    Ok(())
}

#[cfg(not(windows))]
fn pin_directory(
    _path: &Path,
    _pinned_directories: &mut Vec<File>,
) -> Result<(), ExternalPoolAdapterArtifactSourceFsError> {
    Ok(())
}

pub(super) fn unsafe_file_type(metadata: &Metadata) -> bool {
    if metadata.file_type().is_symlink() {
        return true;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0000_0400;
        return metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0;
    }
    #[cfg(not(windows))]
    false
}

pub(crate) fn validate_sha256(value: &str) -> Result<(), ExternalPoolAdapterArtifactSourceFsError> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(ExternalPoolAdapterArtifactSourceFsError::InvalidContentAddress);
    }
    Ok(())
}
