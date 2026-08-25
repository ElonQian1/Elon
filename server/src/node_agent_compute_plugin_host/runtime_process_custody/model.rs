use std::{fmt, fs::File, path::PathBuf};

use anyhow::{bail, Result};

use crate::node_agent_compute_plugin_host::{
    identity::ComputePluginReleaseRef, manifest_validation::is_sha256,
    work_admission_contract::DurableWorkAdmittedPluginSlot,
};

#[cfg(windows)]
use super::launch_security::SealedWindowsRunnerLaunchSecurity;
use super::policy::WindowsRunnerProcessPolicy;

/// Locked loader load-set custody reserved for a future managed-filesystem owned transition.
///
/// There is intentionally no constructor in this source slice. A future producer must consume the
/// complete share-none admission custody, anchor identity across reopen, return distinct
/// NotTransitioned/OutcomeUncertain failure custody, re-hash the loader-compatible replacement,
/// and pin its dependency/path namespace. A path, digest, receipt, PID, caller-opened `File`, or
/// short borrow must never be accepted as a substitute.
pub(super) struct SealedComputePluginRunnerImage {
    pub(super) executable: File,
    pub(super) working_directory: File,
    pub(super) loader_dependency_files: Vec<File>,
    pub(super) loader_namespace_directories: Vec<File>,
    pub(super) absolute_path: PathBuf,
    pub(super) working_directory_path: PathBuf,
    pub(super) installation_id_digest: String,
    pub(super) root_identity_digest: String,
    pub(super) working_directory_identity_digest: String,
    pub(super) plugin_id: String,
    pub(super) slot_ref: String,
    pub(super) release: ComputePluginReleaseRef,
    pub(super) relative_path: String,
    pub(super) digest: String,
    pub(super) size_bytes: u64,
    pub(super) file_identity_digest: String,
    pub(super) loader_dependency_closure_digest: String,
    pub(super) path_namespace_lock_digest: String,
}

/// Exact work-admission plus sealed image material prepared before any process is created.
/// Construction stays private until the retained-runner launch bridge exists.
#[must_use = "validated Runner preparation must be consumed by suspended process custody"]
pub(super) struct ValidatedWindowsRunnerProcessPreparation<'root> {
    pub(super) admitted: DurableWorkAdmittedPluginSlot<'root>,
    pub(super) image: SealedComputePluginRunnerImage,
    #[cfg(windows)]
    pub(super) launch_security: SealedWindowsRunnerLaunchSecurity,
    pub(super) policy: WindowsRunnerProcessPolicy,
}

#[cfg(windows)]
impl<'root> ValidatedWindowsRunnerProcessPreparation<'root> {
    pub(super) fn from_sealed_authorities(
        admitted: DurableWorkAdmittedPluginSlot<'root>,
        image: SealedComputePluginRunnerImage,
        launch_security: SealedWindowsRunnerLaunchSecurity,
    ) -> Result<Self> {
        admitted.receipts().validate()?;
        let source = admitted.receipts().source().source();
        let profile = source.launch_profile();
        profile.validate()?;
        let expected_size = u64::try_from(profile.runner_file_size_bytes())?;
        if !image.absolute_path.is_absolute()
            || !image.working_directory_path.is_absolute()
            || !is_sha256(&image.installation_id_digest)
            || !is_sha256(&image.root_identity_digest)
            || !is_sha256(&image.working_directory_identity_digest)
            || !is_sha256(&image.digest)
            || !is_sha256(&image.file_identity_digest)
            || !is_sha256(&image.loader_dependency_closure_digest)
            || !is_sha256(&image.path_namespace_lock_digest)
            || image.installation_id_digest != source.installation_id_digest()
            || image.plugin_id != source.plugin_id()
            || image.slot_ref != source.slot_ref()
            || &image.release != source.release()
            || image.relative_path != profile.runner_relative_path()
            || image.digest != profile.runner_file_digest()
            || image.size_bytes != expected_size
            || !profile.runner_file_executable()
            || image
                .executable
                .metadata()
                .map(|metadata| metadata.len())
                .ok()
                != Some(expected_size)
            || !image.working_directory.metadata()?.is_dir()
            || image.loader_dependency_files.iter().any(|file| {
                file.metadata()
                    .map(|metadata| !metadata.is_file())
                    .unwrap_or(true)
            })
            || image.loader_namespace_directories.iter().any(|file| {
                file.metadata()
                    .map(|metadata| !metadata.is_dir())
                    .unwrap_or(true)
            })
        {
            bail!("COMPUTE_PLUGIN_RUNNER_IMAGE_BINDING_CHANGED");
        }
        launch_security.validate()?;
        let policy = WindowsRunnerProcessPolicy::from_sources(&admitted, &image, &launch_security)?;
        Ok(Self {
            admitted,
            image,
            launch_security,
            policy,
        })
    }
}

#[cfg(windows)]
use std::os::windows::io::OwnedHandle;

#[cfg(windows)]
pub(super) struct ComputePluginRunnerProcessIdentity {
    pub(super) process_id: u32,
    pub(super) primary_thread_id: u32,
    pub(super) creation_filetime: u64,
}

/// Process-local custody of one inert Runner process. The primary thread remains suspended for the
/// entire lifetime of this value; no resume method exists in this source slice.
#[cfg(windows)]
#[must_use = "dropping Runner custody terminates its Job Object"]
pub(in crate::node_agent_compute_plugin_host) struct PreparedComputePluginRunnerProcess<'root> {
    pub(super) job: OwnedHandle,
    pub(super) process: OwnedHandle,
    pub(super) primary_thread: OwnedHandle,
    pub(super) launch_security: SealedWindowsRunnerLaunchSecurity,
    pub(super) runner_image: SealedComputePluginRunnerImage,
    pub(super) admitted: DurableWorkAdmittedPluginSlot<'root>,
    pub(super) identity: ComputePluginRunnerProcessIdentity,
    pub(super) start_material_digest: String,
}

#[cfg(windows)]
impl fmt::Debug for PreparedComputePluginRunnerProcess<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PreparedComputePluginRunnerProcess")
            .field("process_id", &self.identity.process_id)
            .field("primary_thread_id", &self.identity.primary_thread_id)
            .field("creation_filetime", &self.identity.creation_filetime)
            .field("primary_thread", &"<suspended-owned-handle>")
            .field("job", &"<kill-on-close-owned-handle>")
            .field("runner_image", &"<retained-loader-compatible-handle>")
            .field("launch_security", &"<restricted-token-empty-dacl-owner>")
            .field("start_material_digest", &"<redacted>")
            .field("resume_authority", &"absent")
            .finish()
    }
}

impl fmt::Debug for SealedComputePluginRunnerImage {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SealedComputePluginRunnerImage")
            .field("executable", &"<retained-loader-compatible-handle>")
            .field("working_directory", &"<retained-directory-handle>")
            .field("absolute_path", &"<redacted-handle-derived>")
            .field("file_identity_digest", &"<redacted>")
            .field("root_identity_digest", &"<redacted>")
            .field("working_directory_identity_digest", &"<redacted>")
            .field("loader_dependency_closure_digest", &"<redacted>")
            .field("path_namespace_lock_digest", &"<redacted>")
            .finish()
    }
}

#[cfg(windows)]
impl fmt::Debug for ValidatedWindowsRunnerProcessPreparation<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ValidatedWindowsRunnerProcessPreparation")
            .field("admitted", &self.admitted)
            .field("image", &self.image)
            .field("launch_security", &self.launch_security)
            .field("policy", &self.policy)
            .finish()
    }
}
