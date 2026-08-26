use std::{error::Error as StdError, fmt, mem::ManuallyDrop, path::PathBuf};

use anyhow::{bail, Error, Result};

use crate::node_agent_compute_plugin_host::{
    manifest_validation::is_sha256,
    runtime_loader_load_set::{
        LoaderLockedWorkAdmittedPluginSlot, WindowsRunnerLaunchContextPreCreateProjection,
    },
};

#[cfg(windows)]
use super::launch_security::SealedWindowsRunnerLaunchSecurity;
use super::policy::{WindowsRunnerProcessPolicy, EMPTY_ENVIRONMENT_POLICY, PROCESS_CREATION_FLAGS};

/// Exact work-admission plus sealed image material prepared before any process is created.
/// The loader-locked successor already replaced the share-none admission; keeping both would be
/// physically impossible because the Runner is one of the admission-owned package handles.
#[must_use = "validated Runner preparation must be consumed by suspended process custody"]
pub(super) struct ValidatedWindowsRunnerProcessPreparation<'root> {
    pub(super) loader_locked: LoaderLockedWorkAdmittedPluginSlot<'root>,
    #[cfg(windows)]
    pub(super) launch_security: SealedWindowsRunnerLaunchSecurity,
    pub(super) policy: WindowsRunnerProcessPolicy,
    pub(super) application_path: PathBuf,
    pub(super) working_directory_path: PathBuf,
}

/// Borrow-only validation failure retains both exact input owners. It cannot collapse the
/// loader-locked admission or launch-security handle set into a scalar error.
pub(super) struct WindowsRunnerProcessPreparationValidationFailure<'root> {
    error: Error,
    _loader_locked: LoaderLockedWorkAdmittedPluginSlot<'root>,
    _launch_security: SealedWindowsRunnerLaunchSecurity,
}

#[cfg(windows)]
impl<'root> ValidatedWindowsRunnerProcessPreparation<'root> {
    pub(super) fn from_sealed_authorities(
        loader_locked: LoaderLockedWorkAdmittedPluginSlot<'root>,
        launch_security: SealedWindowsRunnerLaunchSecurity,
    ) -> std::result::Result<Self, WindowsRunnerProcessPreparationValidationFailure<'root>> {
        let validated = validate_sealed_authorities(&loader_locked, &launch_security);
        let (policy, application_path, working_directory_path) = match validated {
            Ok(value) => value,
            Err(error) => {
                return Err(WindowsRunnerProcessPreparationValidationFailure {
                    error,
                    _loader_locked: loader_locked,
                    _launch_security: launch_security,
                })
            }
        };
        Ok(Self {
            loader_locked,
            launch_security,
            policy,
            application_path,
            working_directory_path,
        })
    }
}

#[cfg(windows)]
fn validate_sealed_authorities(
    loader_locked: &LoaderLockedWorkAdmittedPluginSlot<'_>,
    launch_security: &SealedWindowsRunnerLaunchSecurity,
) -> Result<(WindowsRunnerProcessPolicy, PathBuf, PathBuf)> {
    loader_locked.receipts().validate()?;
    let loader_binding = loader_locked.validate_internal_binding()?;
    let source = loader_locked.receipts().source().source();
    let profile = source.launch_profile();
    profile.validate()?;
    let expected_size = u64::try_from(profile.runner_file_size_bytes())?;
    let image = loader_locked.image();
    if !is_sha256(image.installation_id_digest())
        || !is_sha256(image.root_identity_digest())
        || !is_sha256(image.working_directory_identity_digest())
        || !is_sha256(image.digest())
        || !is_sha256(image.file_identity_digest())
        || !is_sha256(image.startup_import_resolution_profile_digest())
        || !is_sha256(image.startup_import_namespace_authority_digest())
        || image.installation_id_digest() != source.installation_id_digest()
        || image.plugin_id() != source.plugin_id()
        || image.slot_ref() != source.slot_ref()
        || image.release() != source.release()
        || image.relative_path() != profile.runner_relative_path()
        || image.digest() != profile.runner_file_digest()
        || image.size_bytes() != expected_size
        || !profile.runner_file_executable()
        || image.package_file_count() == 0
        || !image.retained_runner_matches()
        || !image.retained_working_directory_matches()
    {
        bail!("COMPUTE_PLUGIN_RUNNER_IMAGE_BINDING_CHANGED");
    }
    launch_security.validate()?;
    if launch_security.launch_context_selector_digest() != image.launch_context_selector_digest()
        || launch_security.startup_import_resolution_profile_digest()
            != image.startup_import_resolution_profile_digest()
    {
        bail!("COMPUTE_PLUGIN_WINDOWS_LAUNCH_SECURITY_LOADER_CONTEXT_CHANGED");
    }
    let precreate_launch_context = WindowsRunnerLaunchContextPreCreateProjection::new(
        image.launch_context_selector_digest(),
        image.process_machine_context_digest(),
        image.startup_import_resolution_profile_digest(),
        image.working_directory_identity_digest(),
        profile.runner_relative_path(),
        profile.entrypoint_arguments_digest(),
        launch_security.restricted_token_expected(),
        launch_security.app_container_expected(),
        false,
        EMPTY_ENVIRONMENT_POLICY,
        PROCESS_CREATION_FLAGS,
    );
    loader_locked.validate_authenticated_launch_context_projection(&precreate_launch_context)?;
    let policy = WindowsRunnerProcessPolicy::from_sources(&loader_locked, &launch_security)?;
    Ok((
        policy,
        loader_binding.application_path().to_path_buf(),
        loader_binding.working_directory_path().to_path_buf(),
    ))
}

#[cfg(windows)]
use std::os::windows::io::OwnedHandle;

#[cfg(windows)]
pub(super) struct ComputePluginRunnerProcessIdentity {
    pub(super) process_id: u32,
    pub(super) primary_thread_id: u32,
    pub(super) creation_filetime: u64,
}

/// One indivisible owner graph for the suspended child and every authority that makes its image
/// immutable. `PreparedComputePluginRunnerProcess::drop` may destroy this graph only after
/// termination is confirmed; otherwise the surrounding `ManuallyDrop` deliberately parks all of
/// it for a future process-level recovery service.
#[cfg(windows)]
pub(super) struct WindowsRunnerLiveProcessCustody<'root> {
    pub(super) job: OwnedHandle,
    pub(super) process: OwnedHandle,
    pub(super) primary_thread: OwnedHandle,
    pub(super) launch_security: SealedWindowsRunnerLaunchSecurity,
    pub(super) loader_locked: LoaderLockedWorkAdmittedPluginSlot<'root>,
    pub(super) identity: ComputePluginRunnerProcessIdentity,
    pub(super) loader_currentness: super::namespace_query::WindowsRunnerPreCreateLoaderCurrentness,
}

/// Process-local custody of one inert Runner process. The primary thread remains suspended for the
/// entire lifetime of this value; no resume method exists in this source slice.
#[cfg(windows)]
#[must_use = "dropping Runner custody terminates its Job Object"]
pub(in crate::node_agent_compute_plugin_host) struct PreparedComputePluginRunnerProcess<'root> {
    pub(super) custody: ManuallyDrop<WindowsRunnerLiveProcessCustody<'root>>,
}

#[cfg(windows)]
impl fmt::Debug for PreparedComputePluginRunnerProcess<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let custody = &*self.custody;
        formatter
            .debug_struct("PreparedComputePluginRunnerProcess")
            .field("process_id", &custody.identity.process_id)
            .field("primary_thread_id", &custody.identity.primary_thread_id)
            .field("creation_filetime", &custody.identity.creation_filetime)
            .field("primary_thread", &"<suspended-owned-handle>")
            .field("job", &"<kill-on-close-owned-handle>")
            .field(
                "loader_locked",
                &"<successor-authority-and-full-package-custody>",
            )
            .field("launch_security", &"<restricted-token-empty-dacl-owner>")
            .field("start_material_digest", &"<redacted>")
            .field("resume_authority", &"absent")
            .finish()
    }
}

#[cfg(windows)]
impl fmt::Debug for ValidatedWindowsRunnerProcessPreparation<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ValidatedWindowsRunnerProcessPreparation")
            .field("loader_locked", &self.loader_locked)
            .field("launch_security", &self.launch_security)
            .field("policy", &self.policy)
            .field("application_path", &"<retained-handle-derived>")
            .field("working_directory_path", &"<retained-handle-derived>")
            .finish()
    }
}

#[cfg(windows)]
impl fmt::Debug for WindowsRunnerProcessPreparationValidationFailure<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("WindowsRunnerProcessPreparationValidationFailure")
            .field("error", &self.error)
            .field("loader_locked", &"<retained>")
            .field("launch_security", &"<retained>")
            .finish()
    }
}

#[cfg(windows)]
impl fmt::Display for WindowsRunnerProcessPreparationValidationFailure<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{:#}", self.error)
    }
}

#[cfg(windows)]
impl StdError for WindowsRunnerProcessPreparationValidationFailure<'_> {}
