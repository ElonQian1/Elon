//! Security boundary between an authorized Store claim and its resumable `.part` file.
//!
//! It binds the exact claim path to the installation-owned data root, pins every Windows path
//! component, reconciles the same-handle length with the committed cursor, and writes one bounded
//! segment through flush/fsync/identity revalidation. Trusted-time binding and Store commit remain
//! in the fetch contract.

use std::{ffi::OsStr, path::Path};

use anyhow::{anyhow, bail, Result};
use elon_pc_dev_runtime::NodeDataPaths;

use super::identity::ComputePluginInstallationIdentity;
use super::{
    fetch_contract::AuthorizedComputePluginDownloadSegment, manifest_validation::is_sha256,
};
use crate::{
    node_agent_data_root::{verify_root_marker_payload, ROOT_MARKER_FILE},
    node_agent_managed_fs::{
        ManagedFileOpenFailure, PinnedManagedDirectory, PinnedManagedFile, PinnedManagedRoot,
    },
};

mod types;
mod write;

pub(in crate::node_agent_compute_plugin_host) use types::{
    ComputePluginPartCursorDamage, ComputePluginPartCursorDamageKind,
    ComputePluginPartReconcileFailure, ComputePluginPartReconcileOutcome,
    ComputePluginPartReconcileResult, ComputePluginPinnedFileRecovery, PinnedComputePluginRoot,
    ReconciledComputePluginPartFile,
};
pub(in crate::node_agent_compute_plugin_host) use write::{
    write_compute_plugin_part_segment, ComputePluginSegmentWriteFailure,
    ComputePluginSegmentWritePhase, SyncedComputePluginPartFile,
};

const COMPUTE_PLUGIN_DIRECTORY: &str = "compute-plugin";
const CANDIDATES_DIRECTORY: &str = "candidates";
const DOWNLOADS_DIRECTORY: &str = "downloads";

pub(in crate::node_agent_compute_plugin_host) fn pin_compute_plugin_root(
    paths: &NodeDataPaths,
    installation: &ComputePluginInstallationIdentity,
) -> Result<PinnedComputePluginRoot> {
    if paths.compute_plugins() != paths.root().join(COMPUTE_PLUGIN_DIRECTORY) {
        bail!("COMPUTE_PLUGIN_PART_ROOT_BINDING_INVALID");
    }
    let root = PinnedManagedRoot::pin(paths.root(), installation.digest())?;
    verify_pinned_root_marker(&root, installation.install_id())?;
    root.prepare_directory(Path::new(COMPUTE_PLUGIN_DIRECTORY))
        .map_err(|failure| failure.into_error())?;
    Ok(PinnedComputePluginRoot {
        root,
        installation_id_digest: installation.digest().to_string(),
    })
}

pub(in crate::node_agent_compute_plugin_host) fn reconcile_compute_plugin_part_file(
    root: &PinnedComputePluginRoot,
    authorized: AuthorizedComputePluginDownloadSegment,
) -> ComputePluginPartReconcileResult {
    if let Err(error) = validate_file_binding(root, &authorized) {
        return Err(ComputePluginPartReconcileFailure::before(error, authorized));
    }

    let relative = Path::new(authorized.part_relative_path());
    let parent = match relative.parent() {
        Some(parent) => parent,
        None => {
            return Err(ComputePluginPartReconcileFailure::before(
                anyhow!("COMPUTE_PLUGIN_PART_PARENT_MISSING"),
                authorized,
            ));
        }
    };
    let file_name = match relative.file_name() {
        Some(file_name) => file_name.to_os_string(),
        None => {
            return Err(ComputePluginPartReconcileFailure::before(
                anyhow!("COMPUTE_PLUGIN_PART_FILE_NAME_MISSING"),
                authorized,
            ));
        }
    };
    let directory = match root.root.prepare_directory(parent) {
        Ok(directory) => directory,
        Err(failure) => {
            let filesystem_mutated = failure.filesystem_mutated();
            let error = failure.into_error();
            if filesystem_mutated {
                let recovery_key = authorized.into_recovery_key();
                return Err(ComputePluginPartReconcileFailure::recovery_without_file(
                    error,
                    recovery_key,
                ));
            }
            return Err(ComputePluginPartReconcileFailure::before(error, authorized));
        }
    };

    let offset = authorized.offset_bytes();
    let created = offset == 0;
    let file = if created {
        match directory.create_new_read_write(&file_name) {
            Ok(file) => file,
            Err(ManagedFileOpenFailure::NotOpened(error)) => {
                return Err(ComputePluginPartReconcileFailure::before(error, authorized));
            }
            Err(ManagedFileOpenFailure::FileNotOpened { error, directory })
                if error.kind() == std::io::ErrorKind::AlreadyExists =>
            {
                return reconcile_unexpected_zero_cursor_file(directory, &file_name, authorized);
            }
            Err(ManagedFileOpenFailure::FileNotOpened { error, directory }) => {
                if directory.filesystem_mutated() {
                    let recovery_key = authorized.into_recovery_key();
                    return Err(ComputePluginPartReconcileFailure::recovery_without_file(
                        error,
                        recovery_key,
                    ));
                }
                return Err(ComputePluginPartReconcileFailure::before(error, authorized));
            }
            Err(ManagedFileOpenFailure::Opened { error, file }) => {
                let recovery_key = authorized.into_recovery_key();
                return Err(ComputePluginPartReconcileFailure::quarantined_recovery(
                    error,
                    recovery_key,
                    file,
                ));
            }
        }
    } else {
        match directory.open_existing_read_write(&file_name) {
            Ok(file) => file,
            Err(ManagedFileOpenFailure::NotOpened(error))
                if error.kind() == std::io::ErrorKind::NotFound =>
            {
                return Ok(ComputePluginPartReconcileOutcome::CursorDamaged(
                    ComputePluginPartCursorDamage {
                        kind: ComputePluginPartCursorDamageKind::MissingCommittedFile,
                        authorized,
                        file: None,
                        observed_length_bytes: None,
                    },
                ));
            }
            Err(ManagedFileOpenFailure::NotOpened(error)) => {
                return Err(ComputePluginPartReconcileFailure::before(error, authorized));
            }
            Err(ManagedFileOpenFailure::FileNotOpened {
                error,
                directory: _,
            }) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(ComputePluginPartReconcileOutcome::CursorDamaged(
                    ComputePluginPartCursorDamage {
                        kind: ComputePluginPartCursorDamageKind::MissingCommittedFile,
                        authorized,
                        file: None,
                        observed_length_bytes: None,
                    },
                ));
            }
            Err(ManagedFileOpenFailure::FileNotOpened { error, directory }) => {
                if directory.filesystem_mutated() {
                    let recovery_key = authorized.into_recovery_key();
                    return Err(ComputePluginPartReconcileFailure::recovery_without_file(
                        error,
                        recovery_key,
                    ));
                }
                return Err(ComputePluginPartReconcileFailure::before(error, authorized));
            }
            Err(ManagedFileOpenFailure::Opened { error, file }) => {
                if file.directory_filesystem_mutated() {
                    let recovery_key = authorized.into_recovery_key();
                    return Err(ComputePluginPartReconcileFailure::quarantined_recovery(
                        error,
                        recovery_key,
                        file,
                    ));
                }
                return Err(ComputePluginPartReconcileFailure::opened_rejected(
                    error, authorized, file,
                ));
            }
        }
    };

    reconcile_open_file(authorized, file, created)
}

fn reconcile_open_file(
    authorized: AuthorizedComputePluginDownloadSegment,
    mut file: PinnedManagedFile,
    created: bool,
) -> ComputePluginPartReconcileResult {
    let observed = match i64::try_from(file.len_bytes()) {
        Ok(observed) => observed,
        Err(error) if created || file.directory_filesystem_mutated() => {
            let recovery_key = authorized.into_recovery_key();
            return Err(ComputePluginPartReconcileFailure::file_recovery(
                error,
                recovery_key,
                file,
            ));
        }
        Err(error) => {
            return Err(ComputePluginPartReconcileFailure::unreconciled(
                error, authorized, file,
            ));
        }
    };
    let committed = authorized.offset_bytes();
    if observed < committed {
        return Ok(ComputePluginPartReconcileOutcome::CursorDamaged(
            ComputePluginPartCursorDamage {
                kind: ComputePluginPartCursorDamageKind::ShorterThanCommittedCursor,
                authorized,
                file: Some(file),
                observed_length_bytes: Some(observed),
            },
        ));
    }
    if created && observed != 0 {
        let recovery_key = authorized.into_recovery_key();
        return Err(ComputePluginPartReconcileFailure::file_recovery(
            anyhow!("COMPUTE_PLUGIN_PART_CREATED_NON_EMPTY"),
            recovery_key,
            file,
        ));
    }

    let mut truncated_uncommitted_tail = false;
    if observed > committed {
        if let Err(error) = file.truncate_sync_and_revalidate(committed as u64) {
            let recovery_key = authorized.into_recovery_key();
            return Err(ComputePluginPartReconcileFailure::file_recovery(
                error,
                recovery_key,
                file,
            ));
        }
        truncated_uncommitted_tail = true;
    }
    Ok(ComputePluginPartReconcileOutcome::Ready(
        ReconciledComputePluginPartFile {
            authorized,
            file,
            truncated_uncommitted_tail,
        },
    ))
}

fn reconcile_unexpected_zero_cursor_file(
    directory: PinnedManagedDirectory,
    file_name: &OsStr,
    authorized: AuthorizedComputePluginDownloadSegment,
) -> ComputePluginPartReconcileResult {
    let recovery_key = authorized.into_recovery_key();
    let error = anyhow!("COMPUTE_PLUGIN_PART_ZERO_CURSOR_FILE_EXISTS");
    match directory.open_existing_read_write(file_name) {
        Ok(file) => Err(ComputePluginPartReconcileFailure::unexpected_existing(
            error,
            recovery_key,
            file,
        )),
        Err(ManagedFileOpenFailure::Opened { error, file }) => Err(
            ComputePluginPartReconcileFailure::quarantined_recovery(error, recovery_key, file),
        ),
        Err(ManagedFileOpenFailure::NotOpened(open_error)) => Err(
            ComputePluginPartReconcileFailure::recovery_without_file(open_error, recovery_key),
        ),
        Err(ManagedFileOpenFailure::FileNotOpened {
            error: open_error, ..
        }) => Err(ComputePluginPartReconcileFailure::recovery_without_file(
            open_error,
            recovery_key,
        )),
    }
}

fn verify_pinned_root_marker(root: &PinnedManagedRoot, install_id: &str) -> Result<()> {
    let mut marker = root
        .open_existing_read_only(Path::new(ROOT_MARKER_FILE))
        .map_err(|error| anyhow!("COMPUTE_PLUGIN_PART_ROOT_MARKER_OPEN: {error:?}"))?;
    let payload = marker.read_utf8_limited()?;
    verify_root_marker_payload(&payload, install_id)
}

fn validate_file_binding(
    root: &PinnedComputePluginRoot,
    authorized: &AuthorizedComputePluginDownloadSegment,
) -> Result<()> {
    if !is_sha256(authorized.installation_id_digest())
        || authorized.installation_id_digest() != root.installation_id_digest
        || !is_sha256(authorized.candidate_token_digest())
        || !is_sha256(authorized.artifact_digest())
        || authorized.ordinal() != authorized.download().ordinal
        || authorized.offset_bytes() < 0
        || authorized.length_bytes() <= 0
        || authorized.artifact_size_bytes() <= 0
    {
        bail!("COMPUTE_PLUGIN_PART_BINDING_INVALID");
    }
    let end = authorized
        .offset_bytes()
        .checked_add(authorized.length_bytes())
        .ok_or_else(|| anyhow!("COMPUTE_PLUGIN_PART_RANGE_OVERFLOW"))?;
    let expected = expected_part_relative_path(
        authorized.candidate_token_digest(),
        authorized.ordinal(),
        authorized.artifact_digest(),
    );
    if end != authorized.end_offset_bytes()
        || end > authorized.artifact_size_bytes()
        || authorized.part_relative_path() != expected
        || !relative_path_is_normal(Path::new(authorized.part_relative_path()))
    {
        bail!("COMPUTE_PLUGIN_PART_CLAIM_PATH_CHANGED");
    }
    Ok(())
}

fn expected_part_relative_path(
    candidate_token_digest: &str,
    ordinal: usize,
    artifact_digest: &str,
) -> String {
    format!(
        "{COMPUTE_PLUGIN_DIRECTORY}/{CANDIDATES_DIRECTORY}/{candidate_token_digest}/{DOWNLOADS_DIRECTORY}/{ordinal:04}-{artifact_digest}.part"
    )
}

fn relative_path_is_normal(path: &Path) -> bool {
    !path.as_os_str().is_empty()
        && path
            .components()
            .all(|component| matches!(component, std::path::Component::Normal(_)))
        && path.file_name().is_some_and(single_normal_component)
}

fn single_normal_component(value: &OsStr) -> bool {
    let mut components = Path::new(value).components();
    matches!(components.next(), Some(std::path::Component::Normal(_)))
        && components.next().is_none()
}
