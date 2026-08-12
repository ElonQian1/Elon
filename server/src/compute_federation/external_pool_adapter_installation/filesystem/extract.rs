use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    path::Path,
};

use anyhow::{bail, Context};
use sha2::{Digest, Sha256};
use zip::{CompressionMethod, ZipArchive};

use crate::compute_federation::{
    external_pool_adapter_adoption::validate_adoption_receipt,
    external_pool_adapter_artifact_package::{
        validate_artifact_package_receipt, ARTIFACT_PACKAGE_MANIFEST_PATH,
    },
    external_pool_adapter_artifact_source::CurrentQuarantinedExternalPoolAdapterArtifactBytes,
};

use super::{
    audit::audit_external_pool_adapter_installation,
    paths::{create_private_directory, ensure_child_directories, prepare_paths},
    with_storage_context, ExternalPoolAdapterInstallationFsError,
};
use crate::compute_federation::external_pool_adapter_installation::{
    binding_content_digest, ExternalPoolAdapterInstallationBinding,
    ExternalPoolAdapterInstallationTarget, InstalledExternalPoolAdapterFile,
    PreparedExternalPoolAdapterInstallation, INSTALLATION_STORAGE_NAMESPACE,
};

pub(crate) fn prepare_external_pool_adapter_installation(
    data_dir: &Path,
    mut artifact: CurrentQuarantinedExternalPoolAdapterArtifactBytes,
    target: ExternalPoolAdapterInstallationTarget,
) -> Result<PreparedExternalPoolAdapterInstallation, ExternalPoolAdapterInstallationFsError> {
    let binding = exact_binding(&target, &artifact)
        .map_err(ExternalPoolAdapterInstallationFsError::Authority)?;
    let paths = prepare_paths(data_dir, &binding.installation_content_digest).map_err(|error| {
        with_storage_context(
            error,
            format!("prepare installation paths under {}", data_dir.display()),
        )
    })?;
    match std::fs::symlink_metadata(&paths.final_root) {
        Ok(_) => return audit_external_pool_adapter_installation(data_dir, binding),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(ExternalPoolAdapterInstallationFsError::Storage(error)),
    }

    create_private_directory(&paths.staging_root).map_err(|error| {
        with_storage_context(
            error,
            format!("create staging directory {}", paths.staging_root.display()),
        )
    })?;
    extract_exact_archive(&mut artifact, &target, &paths.staging_root).map_err(|error| {
        with_storage_context(
            error,
            format!(
                "extract installation archive into {}",
                paths.staging_root.display()
            ),
        )
    })?;
    sync_tree(&paths.staging_root).map_err(|error| {
        with_storage_context(
            error,
            format!("sync staging tree {}", paths.staging_root.display()),
        )
    })?;

    match publish_no_replace(&paths.staging_root, &paths.final_root) {
        Ok(()) => {}
        // A losing immutable staging tree remains inert and unreferenced. Avoid recursive cleanup
        // here: its name is never authoritative, and deletion after directory substitution would
        // widen this boundary's destructive scope.
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
        Err(error) => {
            return Err(ExternalPoolAdapterInstallationFsError::Storage(
                std::io::Error::new(
                    error.kind(),
                    format!(
                        "publish installation tree {} to {}: {error}",
                        paths.staging_root.display(),
                        paths.final_root.display()
                    ),
                ),
            ))
        }
    }
    sync_directory(&paths.shard).map_err(|error| {
        with_storage_context(
            error,
            format!("sync installation shard {}", paths.shard.display()),
        )
    })?;
    audit_external_pool_adapter_installation(data_dir, binding).map_err(|error| {
        with_storage_context(
            error,
            format!("audit published installation under {}", data_dir.display()),
        )
    })
}

fn exact_binding(
    target: &ExternalPoolAdapterInstallationTarget,
    artifact: &CurrentQuarantinedExternalPoolAdapterArtifactBytes,
) -> anyhow::Result<ExternalPoolAdapterInstallationBinding> {
    validate_adoption_receipt(&target.adoption_receipt)?;
    validate_artifact_package_receipt(&target.package_receipt)?;
    let adoption = &target.adoption_receipt.adoption.binding;
    let package = &target.package_receipt.package;
    let manifest = &package.manifest;
    if package.admission_id != adoption.admission_id
        || package.admission_digest != adoption.admission_digest
        || manifest.adapter_id != adoption.adapter_id
        || manifest.release_version != adoption.adapter_release_version
        || manifest.capability_set_digest != adoption.capability_set_digest
        || package.archive_sha256 != adoption.declared_implementation_sha256
        || package.source_receipt_digest != target.source_receipt_digest
        || artifact.content_address_digest() != package.archive_sha256
        || artifact.artifact_size_bytes() != package.archive_size_bytes
    {
        bail!("installation roots do not bind one exact adopted package");
    }
    super::super::validation::identifier(&target.source_receipt_id, 200)?;
    super::super::validation::digest(&target.source_receipt_digest)?;
    let entrypoint = manifest
        .files
        .iter()
        .find(|file| file.path == manifest.runtime.entrypoint)
        .context("manifest entrypoint is missing")?;
    let installed_files = manifest
        .files
        .iter()
        .map(|file| InstalledExternalPoolAdapterFile {
            path: file.path.clone(),
            sha256: file.sha256.clone(),
            size_bytes: file.size_bytes,
            role: file.role.clone(),
        })
        .collect();
    let mut binding = ExternalPoolAdapterInstallationBinding {
        application_id: adoption.application_id.clone(),
        application_digest: adoption.application_digest.clone(),
        provider_id: adoption.provider_id.clone(),
        provider_owner_account_id: adoption.provider_owner_account_id.clone(),
        provider_policy_revision: adoption.provider_policy_revision,
        provider_digest: adoption.provider_digest.clone(),
        admission_id: adoption.admission_id.clone(),
        admission_digest: adoption.admission_digest.clone(),
        adapter_id: adoption.adapter_id.clone(),
        adapter_release_version: adoption.adapter_release_version.clone(),
        adapter_config_revision: adoption.adapter_config_revision,
        adapter_config_digest: adoption.adapter_config_digest.clone(),
        declared_implementation_sha256: adoption.declared_implementation_sha256.clone(),
        capability_set_digest: adoption.capability_set_digest.clone(),
        credential_locator_commitment: adoption.credential_locator_commitment.clone(),
        adoption_receipt_id: target.adoption_receipt.adoption_receipt_id.clone(),
        adoption_receipt_digest: target.adoption_receipt.adoption_receipt_digest.clone(),
        adoption_material_digest: target.adoption_receipt.adoption_material_digest.clone(),
        package_receipt_id: target.package_receipt.package_receipt_id.clone(),
        package_receipt_digest: target.package_receipt.package_receipt_digest.clone(),
        package_material_digest: target.package_receipt.package_material_digest.clone(),
        source_receipt_id: target.source_receipt_id.clone(),
        source_receipt_digest: target.source_receipt_digest.clone(),
        archive_sha256: package.archive_sha256.clone(),
        archive_size_bytes: package.archive_size_bytes,
        manifest_digest: package.manifest_digest.clone(),
        entry_inventory_digest: package.entry_inventory_digest.clone(),
        entry_count: package.entry_count,
        total_uncompressed_bytes: package.total_uncompressed_bytes,
        runtime_kind: manifest.runtime.kind.clone(),
        entrypoint_path: entrypoint.path.clone(),
        entrypoint_sha256: entrypoint.sha256.clone(),
        entrypoint_size_bytes: entrypoint.size_bytes,
        installation_content_digest: String::new(),
        storage_namespace: INSTALLATION_STORAGE_NAMESPACE.to_string(),
        installed_files,
    };
    binding.installation_content_digest = binding_content_digest(&binding)?;
    super::super::validation::validate_external_pool_adapter_installation_binding(&binding)?;
    Ok(binding)
}

fn extract_exact_archive(
    artifact: &mut CurrentQuarantinedExternalPoolAdapterArtifactBytes,
    target: &ExternalPoolAdapterInstallationTarget,
    staging_root: &Path,
) -> Result<(), ExternalPoolAdapterInstallationFsError> {
    artifact
        .reader()
        .seek(SeekFrom::Start(0))
        .map_err(ExternalPoolAdapterInstallationFsError::Storage)?;
    let mut archive = ZipArchive::new(artifact.reader())
        .context("open exact installation archive")
        .map_err(ExternalPoolAdapterInstallationFsError::Package)?;
    let package = &target.package_receipt.package;
    let expected: BTreeMap<_, _> = package
        .manifest
        .files
        .iter()
        .map(|file| (file.path.as_str(), file))
        .collect();
    let mut observed = BTreeSet::new();
    let mut saw_manifest = false;
    for index in 0..archive.len() {
        let mut entry = archive
            .by_index(index)
            .context("open installation archive entry")
            .map_err(ExternalPoolAdapterInstallationFsError::Package)?;
        let raw_name = std::str::from_utf8(entry.name_raw())
            .context("installation entry name is not UTF-8")
            .map_err(ExternalPoolAdapterInstallationFsError::Package)?
            .to_string();
        if raw_name != entry.name()
            || entry.enclosed_name().is_none()
            || entry.is_dir()
            || entry.is_symlink()
            || !entry.is_file()
            || entry.encrypted()
            || !entry.comment().is_empty()
            || !matches!(
                entry.compression(),
                CompressionMethod::Stored | CompressionMethod::Deflated
            )
            || !observed.insert(raw_name.clone())
        {
            return Err(ExternalPoolAdapterInstallationFsError::ContentDrift);
        }
        if raw_name == ARTIFACT_PACKAGE_MANIFEST_PATH {
            let mut bytes = Vec::new();
            entry
                .by_ref()
                .take(package.manifest_canonical_json.len() as u64 + 1)
                .read_to_end(&mut bytes)
                .map_err(ExternalPoolAdapterInstallationFsError::Storage)?;
            if saw_manifest || bytes != package.manifest_canonical_json.as_bytes() {
                return Err(ExternalPoolAdapterInstallationFsError::ContentDrift);
            }
            saw_manifest = true;
            continue;
        }
        let declared = expected
            .get(raw_name.as_str())
            .ok_or(ExternalPoolAdapterInstallationFsError::ContentDrift)?;
        let destination = staging_root.join(&raw_name);
        ensure_child_directories(
            staging_root,
            Path::new(&raw_name)
                .parent()
                .unwrap_or_else(|| Path::new("")),
        )?;
        write_exact_entry(
            &mut entry,
            &destination,
            declared.size_bytes,
            &declared.sha256,
        )?;
    }
    if !saw_manifest || observed.len() != expected.len() + 1 {
        return Err(ExternalPoolAdapterInstallationFsError::ContentDrift);
    }
    Ok(())
}

fn write_exact_entry<R: Read>(
    reader: &mut R,
    path: &Path,
    expected_size: u64,
    expected_sha256: &str,
) -> Result<(), ExternalPoolAdapterInstallationFsError> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        options.share_mode(0);
    }
    let mut output = options
        .open(path)
        .map_err(ExternalPoolAdapterInstallationFsError::Storage)?;
    let mut hash = Sha256::new();
    let mut size = 0_u64;
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let remaining = expected_size.saturating_add(1).saturating_sub(size) as usize;
        if remaining == 0 {
            break;
        }
        let chunk_len = buffer.len().min(remaining);
        let read = reader
            .read(&mut buffer[..chunk_len])
            .map_err(ExternalPoolAdapterInstallationFsError::Storage)?;
        if read == 0 {
            break;
        }
        size += read as u64;
        hash.update(&buffer[..read]);
        output
            .write_all(&buffer[..read])
            .map_err(ExternalPoolAdapterInstallationFsError::Storage)?;
    }
    if size != expected_size || hex::encode(hash.finalize()) != expected_sha256 {
        return Err(ExternalPoolAdapterInstallationFsError::ContentDrift);
    }
    output
        .sync_all()
        .map_err(ExternalPoolAdapterInstallationFsError::Storage)
}

fn sync_tree(root: &Path) -> Result<(), ExternalPoolAdapterInstallationFsError> {
    for entry in std::fs::read_dir(root).map_err(ExternalPoolAdapterInstallationFsError::Storage)? {
        let path = entry
            .map_err(ExternalPoolAdapterInstallationFsError::Storage)?
            .path();
        let metadata = std::fs::symlink_metadata(&path)
            .map_err(ExternalPoolAdapterInstallationFsError::Storage)?;
        if metadata.is_dir() {
            super::paths::require_safe_directory(&metadata, true)?;
            sync_tree(&path)?;
        } else {
            super::paths::require_safe_regular_file(&metadata)?;
        }
    }
    sync_directory(root)
}

#[cfg(unix)]
fn sync_directory(path: &Path) -> Result<(), ExternalPoolAdapterInstallationFsError> {
    File::open(path)
        .and_then(|file| file.sync_all())
        .map_err(ExternalPoolAdapterInstallationFsError::Storage)
}

#[cfg(not(unix))]
fn sync_directory(_path: &Path) -> Result<(), ExternalPoolAdapterInstallationFsError> {
    Ok(())
}

#[cfg(target_os = "linux")]
fn publish_no_replace(source: &Path, target: &Path) -> std::io::Result<()> {
    use std::{ffi::CString, os::unix::ffi::OsStrExt};
    const AT_FDCWD: i32 = -100;
    const RENAME_NOREPLACE: u32 = 1;
    unsafe extern "C" {
        fn renameat2(old_dir: i32, old: *const i8, new_dir: i32, new: *const i8, flags: u32)
            -> i32;
    }
    let source = CString::new(source.as_os_str().as_bytes())
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidInput, error))?;
    let target = CString::new(target.as_os_str().as_bytes())
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidInput, error))?;
    if unsafe {
        renameat2(
            AT_FDCWD,
            source.as_ptr(),
            AT_FDCWD,
            target.as_ptr(),
            RENAME_NOREPLACE,
        )
    } == 0
    {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}

#[cfg(windows)]
fn publish_no_replace(source: &Path, target: &Path) -> std::io::Result<()> {
    use windows_sys::Win32::Storage::FileSystem::MoveFileExW;
    let source = windows_extended_path(source)?;
    let target = windows_extended_path(target)?;
    const MOVEFILE_WRITE_THROUGH: u32 = 0x0000_0008;
    if unsafe { MoveFileExW(source.as_ptr(), target.as_ptr(), MOVEFILE_WRITE_THROUGH) } != 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}

#[cfg(windows)]
fn windows_extended_path(path: &Path) -> std::io::Result<Vec<u16>> {
    use std::os::windows::ffi::OsStrExt;

    let path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()?.join(path)
    };
    let raw: Vec<u16> = path.as_os_str().encode_wide().collect();
    let mut extended = Vec::with_capacity(raw.len() + 8);
    let slash = b'\\' as u16;
    let question = b'?' as u16;
    let dot = b'.' as u16;

    if raw.starts_with(&[slash, slash, question, slash])
        || raw.starts_with(&[slash, slash, dot, slash])
    {
        extended.extend_from_slice(&raw);
    } else if raw.starts_with(&[slash, slash]) {
        extended.extend("\\\\?\\UNC\\".encode_utf16());
        extended.extend_from_slice(&raw[2..]);
    } else if raw.get(1) == Some(&(b':' as u16))
        && matches!(raw.get(2), Some(value) if *value == slash || *value == b'/' as u16)
    {
        extended.extend("\\\\?\\".encode_utf16());
        extended.extend_from_slice(&raw);
    } else {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "installation publication requires an absolute Windows path",
        ));
    }
    extended.push(0);
    Ok(extended)
}

#[cfg(not(any(target_os = "linux", windows)))]
fn publish_no_replace(source: &Path, target: &Path) -> std::io::Result<()> {
    let _ = (source, target);
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "atomic no-replace directory publication is unsupported on this platform",
    ))
}
