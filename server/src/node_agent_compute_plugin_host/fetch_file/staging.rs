use std::{
    ffi::OsStr,
    path::{Component, Path},
};

use anyhow::{anyhow, Error, Result};

use super::{PinnedComputePluginRoot, CANDIDATES_DIRECTORY, COMPUTE_PLUGIN_DIRECTORY};
use crate::{
    node_agent_compute_plugin_host::manifest_validation::is_sha256,
    node_agent_managed_fs::{
        ManagedDirectoryPrepareFailure, ManagedFileOpenFailure, PinnedManagedDirectory,
        PinnedManagedFile,
    },
};

const STAGING_DIRECTORY: &str = "staging";

pub(in crate::node_agent_compute_plugin_host) struct ComputePluginStagingPrepareFailure {
    error: Error,
    filesystem_mutated: bool,
}

impl ComputePluginStagingPrepareFailure {
    pub(in crate::node_agent_compute_plugin_host) fn filesystem_mutated(&self) -> bool {
        self.filesystem_mutated
    }

    pub(in crate::node_agent_compute_plugin_host) fn into_error(self) -> Error {
        self.error
    }
}

impl std::fmt::Debug for ComputePluginStagingPrepareFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ComputePluginStagingPrepareFailure")
            .field("filesystem_mutated", &self.filesystem_mutated)
            .finish()
    }
}

/// A create-new staging namespace below one pinned candidate root. All descendant lookups continue
/// to use the original pinned root and normalized relative components.
pub(in crate::node_agent_compute_plugin_host) struct PreparedComputePluginCandidateStaging<'root> {
    root: &'root PinnedComputePluginRoot,
    directory: PinnedManagedDirectory,
    relative_root: String,
    staging_run_digest: String,
}

impl PreparedComputePluginCandidateStaging<'_> {
    pub(in crate::node_agent_compute_plugin_host) fn staging_run_digest(&self) -> &str {
        &self.staging_run_digest
    }

    pub(in crate::node_agent_compute_plugin_host) fn relative_root(&self) -> &str {
        &self.relative_root
    }

    pub(in crate::node_agent_compute_plugin_host) fn root_identity_digest(&self) -> &str {
        self.root.root_identity_digest()
    }

    pub(in crate::node_agent_compute_plugin_host) fn prepare_directory(
        &self,
        relative: &str,
    ) -> Result<PinnedManagedDirectory> {
        self.root
            .root
            .prepare_directory(&self.descendant_path(relative)?)
            .map_err(ManagedDirectoryPrepareFailure::into_error)
    }

    pub(in crate::node_agent_compute_plugin_host) fn create_new_file(
        &self,
        relative: &str,
    ) -> Result<PinnedManagedFile> {
        let full = self.descendant_path(relative)?;
        let parent = full
            .parent()
            .ok_or_else(|| anyhow!("COMPUTE_PLUGIN_STAGING_FILE_PARENT_MISSING"))?;
        let name = full
            .file_name()
            .ok_or_else(|| anyhow!("COMPUTE_PLUGIN_STAGING_FILE_NAME_MISSING"))?;
        let directory = self.root.root.pin_existing_directory(parent)?;
        directory.create_new_read_write(name).map_err(open_error)
    }

    fn descendant_path(&self, relative: &str) -> Result<std::path::PathBuf> {
        let path = Path::new(relative);
        if relative.is_empty()
            || relative.contains('\\')
            || relative.starts_with('/')
            || relative.ends_with('/')
            || relative.contains("//")
            || path
                .components()
                .any(|component| !matches!(component, Component::Normal(_)))
        {
            return Err(anyhow!("COMPUTE_PLUGIN_STAGING_RELATIVE_PATH_INVALID"));
        }
        Ok(Path::new(&self.relative_root).join(path))
    }
}

impl std::fmt::Debug for PreparedComputePluginCandidateStaging<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("PreparedComputePluginCandidateStaging")
            .field("relative_root", &"<redacted>")
            .field("staging_run_digest", &"<redacted>")
            .field("directory", &self.directory)
            .finish()
    }
}

pub(in crate::node_agent_compute_plugin_host) fn prepare_compute_plugin_candidate_staging<'root>(
    root: &'root PinnedComputePluginRoot,
    candidate_token_digest: &str,
    staging_run_digest: &str,
) -> std::result::Result<
    PreparedComputePluginCandidateStaging<'root>,
    ComputePluginStagingPrepareFailure,
> {
    if !is_sha256(candidate_token_digest)
        || !is_sha256(staging_run_digest)
        || !is_sha256(root.installation_id_digest())
        || !is_sha256(root.root_identity_digest())
    {
        return Err(ComputePluginStagingPrepareFailure {
            error: anyhow!("COMPUTE_PLUGIN_STAGING_BINDING_INVALID"),
            filesystem_mutated: false,
        });
    }
    let parent_relative = format!(
        "{COMPUTE_PLUGIN_DIRECTORY}/{CANDIDATES_DIRECTORY}/{candidate_token_digest}/{STAGING_DIRECTORY}"
    );
    let parent = root
        .root
        .prepare_directory(Path::new(&parent_relative))
        .map_err(|failure| ComputePluginStagingPrepareFailure {
            filesystem_mutated: failure.filesystem_mutated(),
            error: failure.into_error(),
        })?;
    let parent_mutated = parent.filesystem_mutated();
    let directory = parent
        .create_new_directory_child(OsStr::new(staging_run_digest))
        .map_err(|failure| ComputePluginStagingPrepareFailure {
            filesystem_mutated: parent_mutated || failure.filesystem_mutated(),
            error: failure.into_error(),
        })?;
    Ok(PreparedComputePluginCandidateStaging {
        root,
        directory,
        relative_root: format!("{parent_relative}/{staging_run_digest}"),
        staging_run_digest: staging_run_digest.to_string(),
    })
}

fn open_error(failure: ManagedFileOpenFailure) -> Error {
    match failure {
        ManagedFileOpenFailure::NotOpened(error)
        | ManagedFileOpenFailure::FileNotOpened { error, .. } => error.into(),
        ManagedFileOpenFailure::Opened { error, .. } => error,
    }
}
