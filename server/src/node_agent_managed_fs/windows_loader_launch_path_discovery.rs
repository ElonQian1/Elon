use std::{
    fs::File,
    mem::{size_of, MaybeUninit},
    os::windows::io::AsRawHandle,
    path::{Component, Path},
    sync::Arc,
};

use super::ntstatus_error;
use crate::node_agent_managed_fs::{
    identity_digest,
    loader_launch_path_discovery::{
        seal_discovered_path, ManagedLoaderLaunchPathComponentDiscovery,
        ManagedLoaderLaunchPathDiscoveryReceipt, DIRECTORY_DISCOVERY_CLASS_PROVENANCE,
        FILE_DISCOVERY_CLASS_PROVENANCE,
    },
    validate_directory_identity, validate_regular_file_identity, ManagedObjectBinding,
    PinnedManagedDirectory, PinnedManagedFile, PlatformFileIdentity,
};
use crate::node_agent_managed_fs::loader_launch_path_discovery::ManagedLoaderLaunchPathObjectKind::{
    Directory, File as FileKind,
};
use anyhow::{anyhow, bail, Context, Result};
use windows_sys::{
    Wdk::Storage::FileSystem::{
        FileAccessInformation, NtQueryInformationFile, FILE_ACCESS_INFORMATION,
    },
    Win32::{
        Foundation::{HANDLE, STATUS_SUCCESS},
        Storage::FileSystem::{FILE_READ_ATTRIBUTES, FILE_READ_DATA, FILE_TRAVERSE, SYNCHRONIZE},
        System::IO::IO_STATUS_BLOCK,
    },
};

pub(in crate::node_agent_managed_fs) fn discover_loader_directory_launch_path(
    directory: &PinnedManagedDirectory,
) -> Result<ManagedLoaderLaunchPathDiscoveryReceipt> {
    let handles = directory
        .directory_handles
        .iter()
        .map(Arc::as_ref)
        .collect::<Vec<_>>();
    let binding = directory
        .binding
        .as_ref()
        .ok_or_else(|| anyhow!("NODE_MANAGED_LOADER_LAUNCH_PATH_DIRECTORY_BINDING_MISSING"))?;
    discover_path(
        &directory.root_identity_digest,
        directory.root_volume_serial,
        &handles,
        false,
        binding,
    )
}

pub(in crate::node_agent_managed_fs) fn discover_loader_file_launch_path(
    file: &PinnedManagedFile,
) -> Result<ManagedLoaderLaunchPathDiscoveryReceipt> {
    let mut handles = file
        ._directory_handles
        .iter()
        .map(Arc::as_ref)
        .collect::<Vec<_>>();
    handles.push(&file.file);
    discover_path(
        &file.root_identity_digest,
        file.root_volume_serial,
        &handles,
        true,
        &file.binding,
    )
}

fn discover_path(
    root_identity_digest: &str,
    volume: u64,
    handles: &[&File],
    final_is_file: bool,
    final_binding: &ManagedObjectBinding,
) -> Result<ManagedLoaderLaunchPathDiscoveryReceipt> {
    if handles.len() < 2 {
        bail!("NODE_MANAGED_LOADER_LAUNCH_PATH_CHAIN_TOO_SHORT");
    }
    let anchor_identity = inspect_owner(handles[0], volume, false)?;
    let anchor_access = granted_access(handles[0], false)?;
    let mut parent_path =
        super::canonical_path(handles[0]).context("NODE_MANAGED_LOADER_LAUNCH_PATH_ANCHOR_PATH")?;
    let mut parent_digest = identity_digest(root_identity_digest, None, anchor_identity);
    let mut components = Vec::with_capacity(handles.len() - 1);
    for (index, handle) in handles.iter().enumerate().skip(1) {
        let is_file = final_is_file && index + 1 == handles.len();
        let identity = inspect_owner(handle, volume, is_file)?;
        let (object_kind, discovery_class_provenance) = if is_file {
            (FileKind, FILE_DISCOVERY_CLASS_PROVENANCE)
        } else {
            (Directory, DIRECTORY_DISCOVERY_CLASS_PROVENANCE)
        };
        let canonical = super::canonical_path(handle)
            .context("NODE_MANAGED_LOADER_LAUNCH_PATH_COMPONENT_PATH")?;
        let normalized_component = single_child_component(&parent_path, &canonical)?;
        let object_identity_digest = identity_digest(root_identity_digest, None, identity);
        components.push(ManagedLoaderLaunchPathComponentDiscovery {
            ordinal: index - 1,
            parent_identity_digest: parent_digest,
            normalized_component,
            object_identity_digest: object_identity_digest.clone(),
            object_kind,
            granted_access: granted_access(handle, is_file)?,
            discovery_class_provenance,
        });
        parent_digest = object_identity_digest;
        parent_path = canonical;
    }
    seal_discovered_path(
        root_identity_digest,
        parent_digest,
        &parent_path,
        anchor_access,
        final_is_file,
        final_binding,
        components,
    )
}

fn inspect_owner(file: &File, volume: u64, is_file: bool) -> Result<PlatformFileIdentity> {
    let identity = super::inspect(file).context("NODE_MANAGED_LOADER_LAUNCH_PATH_INSPECT")?;
    if is_file {
        validate_regular_file_identity(identity, volume)?;
    } else {
        validate_directory_identity(identity, Some(volume))?;
    }
    Ok(identity)
}

fn granted_access(file: &File, is_file: bool) -> Result<u32> {
    let access = query_granted_access(file)?;
    let required = FILE_READ_ATTRIBUTES
        | SYNCHRONIZE
        | if is_file {
            FILE_READ_DATA
        } else {
            FILE_TRAVERSE
        };
    if access & required != required {
        bail!("NODE_MANAGED_LOADER_LAUNCH_PATH_ACCESS_CHANGED");
    }
    Ok(access)
}

fn single_child_component(parent: &Path, child: &Path) -> Result<String> {
    let relative = child
        .strip_prefix(parent)
        .map_err(|_| anyhow!("NODE_MANAGED_LOADER_LAUNCH_PATH_PARENT_CHANGED"))?;
    let mut components = relative.components();
    let Some(Component::Normal(name)) = components.next() else {
        bail!("NODE_MANAGED_LOADER_LAUNCH_PATH_COMPONENT_INVALID");
    };
    if components.next().is_some() {
        bail!("NODE_MANAGED_LOADER_LAUNCH_PATH_COMPONENT_NOT_SINGLE");
    }
    name.to_str()
        .filter(|value| !value.is_empty() && !value.contains('\0'))
        .map(str::to_owned)
        .ok_or_else(|| anyhow!("NODE_MANAGED_LOADER_LAUNCH_PATH_COMPONENT_NOT_UTF8"))
}

fn query_granted_access(file: &File) -> std::io::Result<u32> {
    let mut access = MaybeUninit::<FILE_ACCESS_INFORMATION>::uninit();
    let mut io_status = MaybeUninit::<IO_STATUS_BLOCK>::uninit();
    // SAFETY: both fixed-size outputs remain live for this synchronous query.
    let status = unsafe {
        NtQueryInformationFile(
            file.as_raw_handle() as HANDLE,
            io_status.as_mut_ptr(),
            access.as_mut_ptr().cast(),
            size_of::<FILE_ACCESS_INFORMATION>() as u32,
            FileAccessInformation,
        )
    };
    if status != STATUS_SUCCESS {
        return Err(ntstatus_error(status));
    }
    // SAFETY: STATUS_SUCCESS initialized the synchronous IO status block.
    let completion = unsafe { io_status.assume_init().Anonymous.Status };
    if completion != STATUS_SUCCESS {
        return Err(ntstatus_error(completion));
    }
    // SAFETY: both NT and completion statuses succeeded.
    Ok(unsafe { access.assume_init() }.AccessFlags)
}
