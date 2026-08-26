use std::{ffi::OsStr, path::Path};

use anyhow::{anyhow, Error, Result};

use super::{PinnedComputePluginRoot, CANDIDATES_DIRECTORY, COMPUTE_PLUGIN_DIRECTORY};
use crate::{
    node_agent_compute_plugin_host::manifest_validation::is_sha256,
    node_agent_managed_fs::{
        PinnedManagedDirectory, PinnedManagedExtractionLoaderDirectory, PinnedManagedFile,
    },
};

const STAGING_DIRECTORY: &str = "staging";
pub(in crate::node_agent_compute_plugin_host) const COMPUTE_PLUGIN_STAGING_SEAL_FILE: &str =
    ".elon-staging-seal.json";

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

/// A create-new staging namespace below one pinned candidate root. Descendants are created only
/// from the retained package root or an exact plan-parent handle; full-path root traversal is not
/// an extraction authority.
pub(in crate::node_agent_compute_plugin_host) struct PreparedComputePluginCandidateStaging<'root> {
    root: &'root PinnedComputePluginRoot,
    directory: PinnedManagedExtractionLoaderDirectory,
    relative_root: String,
    staging_run_digest: String,
}

/// Complete staging ownership split for the loader transition. The package-root directory moves
/// into loader namespace custody while the root borrow and binding scalars remain in the authority
/// residue; neither side can be reconstructed from paths alone.
pub(in crate::node_agent_compute_plugin_host) struct PreparedComputePluginStagingLoaderParts<'root>
{
    pub(in crate::node_agent_compute_plugin_host) root: &'root PinnedComputePluginRoot,
    pub(in crate::node_agent_compute_plugin_host) package_root:
        PinnedManagedExtractionLoaderDirectory,
    pub(in crate::node_agent_compute_plugin_host) relative_root: String,
    pub(in crate::node_agent_compute_plugin_host) staging_run_digest: String,
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

    /// Purpose-specific borrow for retained-handle launch-path discovery. This does not expose a
    /// path, raw handle, component grant, selected working directory, or loader authority.
    pub(in crate::node_agent_compute_plugin_host) fn loader_launch_path_package_root(
        &self,
    ) -> &PinnedManagedExtractionLoaderDirectory {
        &self.directory
    }

    pub(in crate::node_agent_compute_plugin_host) fn create_new_directory_child(
        &self,
        name: &OsStr,
    ) -> Result<PinnedManagedDirectory> {
        self.directory
            .create_new_directory_child(name)
            .map_err(Error::new)
    }

    pub(in crate::node_agent_compute_plugin_host) fn create_new_file_child(
        &self,
        name: &OsStr,
    ) -> Result<PinnedManagedFile> {
        self.directory
            .create_new_file_child(name)
            .map_err(Error::new)
    }

    pub(in crate::node_agent_compute_plugin_host) fn create_new_seal_file(
        &self,
    ) -> Result<PinnedManagedFile> {
        self.create_new_file_child(OsStr::new(COMPUTE_PLUGIN_STAGING_SEAL_FILE))
    }

    pub(in crate::node_agent_compute_plugin_host) fn pin_cleanup_ancestors(
        &self,
        candidate_token_digest: &str,
    ) -> Result<(
        PinnedManagedDirectory,
        PinnedManagedDirectory,
        PinnedManagedDirectory,
    )> {
        if !is_sha256(candidate_token_digest) {
            return Err(anyhow!("COMPUTE_PLUGIN_CLEANUP_CANDIDATE_DIGEST_INVALID"));
        }
        let candidate_relative =
            format!("{COMPUTE_PLUGIN_DIRECTORY}/{CANDIDATES_DIRECTORY}/{candidate_token_digest}");
        let candidate_parent_relative =
            format!("{COMPUTE_PLUGIN_DIRECTORY}/{CANDIDATES_DIRECTORY}");
        let staging_relative = format!("{candidate_relative}/{STAGING_DIRECTORY}");
        let expected_run = format!("{staging_relative}/{}", self.staging_run_digest);
        if self.relative_root != expected_run {
            return Err(anyhow!("COMPUTE_PLUGIN_CLEANUP_STAGING_BINDING_CHANGED"));
        }
        let candidate_parent = self
            .root
            .root
            .pin_existing_directory_for_cleanup(Path::new(&candidate_parent_relative))?;
        let candidate = self
            .root
            .root
            .pin_existing_directory_for_cleanup(Path::new(&candidate_relative))?;
        let staging = self
            .root
            .root
            .pin_existing_directory_for_cleanup(Path::new(&staging_relative))?;
        Ok((candidate_parent, candidate, staging))
    }

    pub(in crate::node_agent_compute_plugin_host) fn into_cleanup_directory(
        self,
    ) -> PinnedManagedDirectory {
        self.directory.into_cleanup_directory()
    }
}

impl<'root> PreparedComputePluginCandidateStaging<'root> {
    pub(in crate::node_agent_compute_plugin_host) fn into_loader_transition_parts(
        self,
    ) -> PreparedComputePluginStagingLoaderParts<'root> {
        PreparedComputePluginStagingLoaderParts {
            root: self.root,
            package_root: self.directory.into_loader_parts(),
            relative_root: self.relative_root,
            staging_run_digest: self.staging_run_digest,
        }
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
        })?
        .into_extraction_loader_directory_custody()
        .map_err(|failure| ComputePluginStagingPrepareFailure {
            error: failure.into(),
            filesystem_mutated: true,
        })?;
    Ok(PreparedComputePluginCandidateStaging {
        root,
        directory,
        relative_root: format!("{parent_relative}/{staging_run_digest}"),
        staging_run_digest: staging_run_digest.to_string(),
    })
}
