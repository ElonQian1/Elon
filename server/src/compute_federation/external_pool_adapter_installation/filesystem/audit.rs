use std::{
    collections::{BTreeMap, BTreeSet},
    fs::File,
    io::Read,
    path::{Path, PathBuf},
};

use sha2::{Digest, Sha256};

use super::{
    paths::{
        locate_paths, open_no_follow, pin_directory, require_safe_directory,
        require_safe_regular_file, require_single_link_handle,
    },
    ExternalPoolAdapterInstallationFsError,
};
use crate::compute_federation::external_pool_adapter_installation::{
    validate_external_pool_adapter_installation_binding, ExternalPoolAdapterInstallationBinding,
    PreparedExternalPoolAdapterInstallation,
};

pub(crate) fn audit_external_pool_adapter_installation(
    data_dir: &Path,
    binding: ExternalPoolAdapterInstallationBinding,
) -> Result<PreparedExternalPoolAdapterInstallation, ExternalPoolAdapterInstallationFsError> {
    validate_external_pool_adapter_installation_binding(&binding)
        .map_err(ExternalPoolAdapterInstallationFsError::Authority)?;
    let mut paths = locate_paths(data_dir, &binding.installation_content_digest)?;
    let final_metadata = std::fs::symlink_metadata(&paths.final_root).map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            ExternalPoolAdapterInstallationFsError::Missing
        } else {
            ExternalPoolAdapterInstallationFsError::Storage(error)
        }
    })?;
    require_safe_directory(&final_metadata, true)?;
    let canonical_root = std::fs::canonicalize(&paths.final_root)
        .map_err(ExternalPoolAdapterInstallationFsError::Storage)?;
    let canonical_shard = std::fs::canonicalize(&paths.shard)
        .map_err(ExternalPoolAdapterInstallationFsError::Storage)?;
    if canonical_root == canonical_shard || !canonical_root.starts_with(&canonical_shard) {
        return Err(ExternalPoolAdapterInstallationFsError::UnsafeTarget);
    }
    pin_directory(&paths.final_root, &mut paths.pinned_directories)?;

    let (observed, observed_directories) = enumerate_tree(&paths.final_root)?;
    let expected: BTreeSet<_> = binding
        .installed_files
        .iter()
        .map(|file| file.path.as_str())
        .collect();
    let expected_directories = expected_directories(&binding);
    if observed.keys().map(String::as_str).collect::<BTreeSet<_>>() != expected
        || observed_directories != expected_directories
    {
        return Err(ExternalPoolAdapterInstallationFsError::ContentDrift);
    }

    let mut reopened = Vec::with_capacity(binding.installed_files.len());
    for expected in &binding.installed_files {
        let path = observed
            .get(&expected.path)
            .ok_or(ExternalPoolAdapterInstallationFsError::ContentDrift)?;
        let mut file = open_no_follow(path)?;
        let before = file
            .metadata()
            .map_err(ExternalPoolAdapterInstallationFsError::Storage)?;
        require_safe_regular_file(&before)?;
        let mut digest = Sha256::new();
        let mut size = 0_u64;
        let mut buffer = [0_u8; 64 * 1024];
        loop {
            let read = file
                .read(&mut buffer)
                .map_err(ExternalPoolAdapterInstallationFsError::Storage)?;
            if read == 0 {
                break;
            }
            size = size
                .checked_add(read as u64)
                .ok_or(ExternalPoolAdapterInstallationFsError::ContentDrift)?;
            if size > expected.size_bytes {
                return Err(ExternalPoolAdapterInstallationFsError::ContentDrift);
            }
            digest.update(&buffer[..read]);
        }
        let after = file
            .metadata()
            .map_err(ExternalPoolAdapterInstallationFsError::Storage)?;
        require_safe_regular_file(&after)?;
        require_single_link_handle(&file)?;
        if size != expected.size_bytes
            || before.len() != after.len()
            || size != after.len()
            || hex::encode(digest.finalize()) != expected.sha256
            || !path_matches_handle(path, &file)?
        {
            return Err(ExternalPoolAdapterInstallationFsError::ContentDrift);
        }
        reopened.push(file);
    }
    Ok(PreparedExternalPoolAdapterInstallation {
        binding,
        _reopened_files: reopened,
        _pinned_directories: paths.pinned_directories,
        _final_root: paths.final_root,
    })
}

fn expected_directories(binding: &ExternalPoolAdapterInstallationBinding) -> BTreeSet<String> {
    let mut directories = BTreeSet::new();
    for file in &binding.installed_files {
        let components: Vec<_> = file.path.split('/').collect();
        for length in 1..components.len() {
            directories.insert(components[..length].join("/"));
        }
    }
    directories
}

fn enumerate_tree(
    root: &Path,
) -> Result<(BTreeMap<String, PathBuf>, BTreeSet<String>), ExternalPoolAdapterInstallationFsError> {
    let mut output = BTreeMap::new();
    let mut directories = BTreeSet::new();
    enumerate_directory(root, root, &mut output, &mut directories)?;
    Ok((output, directories))
}

fn enumerate_directory(
    root: &Path,
    directory: &Path,
    output: &mut BTreeMap<String, PathBuf>,
    directories: &mut BTreeSet<String>,
) -> Result<(), ExternalPoolAdapterInstallationFsError> {
    require_safe_directory(
        &std::fs::symlink_metadata(directory)
            .map_err(ExternalPoolAdapterInstallationFsError::Storage)?,
        true,
    )?;
    for entry in
        std::fs::read_dir(directory).map_err(ExternalPoolAdapterInstallationFsError::Storage)?
    {
        let entry = entry.map_err(ExternalPoolAdapterInstallationFsError::Storage)?;
        let path = entry.path();
        let metadata = std::fs::symlink_metadata(&path)
            .map_err(ExternalPoolAdapterInstallationFsError::Storage)?;
        if metadata.file_type().is_symlink() {
            return Err(ExternalPoolAdapterInstallationFsError::UnsafeTarget);
        }
        if metadata.is_dir() {
            let relative = canonical_relative(root, &path)?;
            if !directories.insert(relative) {
                return Err(ExternalPoolAdapterInstallationFsError::UnsafeTarget);
            }
            enumerate_directory(root, &path, output, directories)?;
            continue;
        }
        require_safe_regular_file(&metadata)?;
        let key = canonical_relative(root, &path)?;
        if output.insert(key, path).is_some() {
            return Err(ExternalPoolAdapterInstallationFsError::UnsafeTarget);
        }
    }
    Ok(())
}

fn canonical_relative(
    root: &Path,
    path: &Path,
) -> Result<String, ExternalPoolAdapterInstallationFsError> {
    let relative = path
        .strip_prefix(root)
        .map_err(|_| ExternalPoolAdapterInstallationFsError::UnsafeTarget)?;
    let components: Option<Vec<_>> = relative
        .components()
        .map(|component| match component {
            std::path::Component::Normal(value) => value.to_str(),
            _ => None,
        })
        .collect();
    Ok(components
        .ok_or(ExternalPoolAdapterInstallationFsError::UnsafeTarget)?
        .join("/"))
}

#[cfg(unix)]
fn path_matches_handle(
    path: &Path,
    file: &File,
) -> Result<bool, ExternalPoolAdapterInstallationFsError> {
    use std::os::unix::fs::MetadataExt;
    let path =
        std::fs::symlink_metadata(path).map_err(ExternalPoolAdapterInstallationFsError::Storage)?;
    let handle = file
        .metadata()
        .map_err(ExternalPoolAdapterInstallationFsError::Storage)?;
    Ok(path.dev() == handle.dev() && path.ino() == handle.ino())
}

#[cfg(windows)]
fn path_matches_handle(
    _path: &Path,
    _file: &File,
) -> Result<bool, ExternalPoolAdapterInstallationFsError> {
    // The retained handle excludes DELETE sharing, preventing replacement after no-follow open.
    Ok(true)
}
