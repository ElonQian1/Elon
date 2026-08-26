//! Validation for the authenticated, currently unproducible launch-context intent.

use std::collections::HashSet;

use anyhow::{bail, Result};

use crate::node_agent_compute_plugin_host::{
    manifest_validation::is_sha256,
    work_admission_contract::ComputePluginWorkAdmissionLaunchProfile,
};

use super::{
    digest::PlanDigest, AuthenticatedWindowsDllSearchPolicy,
    AuthenticatedWindowsProcessCreationPolicy,
    AuthenticatedWindowsProcessMachineContextExpectation,
    AuthenticatedWindowsRunnerLaunchContextIntent,
    AuthenticatedWindowsRunnerLaunchSecurityExpectation,
    WindowsRunnerLaunchContextPreCreateProjection,
};

const REQUIRED_PROCESS_CREATION_FLAGS: &[&str] = &[
    "create_no_window",
    "create_suspended",
    "create_unicode_environment",
    "extended_startupinfo_present",
];
const EMPTY_ENVIRONMENT_POLICY: &str = "explicit_empty_unicode_environment_block_v1";
const REQUIRED_RESOLUTION_ROUTES: &[&str] = &[
    "preloaded",
    "api_set",
    "known_dll",
    "side_by_side",
    "filesystem",
];

impl AuthenticatedWindowsRunnerLaunchContextIntent {
    pub(super) fn validate_binding(
        &self,
        admission_source_digest: &str,
        admission_receipt_digest: &str,
        profile: &ComputePluginWorkAdmissionLaunchProfile,
    ) -> Result<()> {
        self.machine_context.validate(&self.target_architecture)?;
        self.dll_search_policy.validate()?;
        self.process_creation_policy.validate()?;
        self.launch_security_expectation.validate()?;
        if self.admission_source_digest != admission_source_digest
            || self.admission_receipt_digest != admission_receipt_digest
            || self.manifest_digest != profile.manifest_digest()
            || self.signed_manifest_envelope_digest != profile.signed_manifest_envelope_digest()
            || self.grant_digest != profile.grant_digest()
            || self.target_id != profile.target_id()
            || self.target_operating_system != "windows"
            || self.target_operating_system != profile.target().operating_system
            || self.target_architecture != profile.target().architecture
            || self.runner_relative_path != profile.runner_relative_path()
            || self.entrypoint_arguments_digest != profile.entrypoint_arguments_digest()
            || self.control_key_id.is_empty()
            || self.control_keyring_generation == 0
            || [
                &self.selection_payload_digest,
                &self.signed_selection_payload_digest,
                &self.verified_selection_payload_digest,
                &self.signed_selection_envelope_digest,
                &self.signature_verification_receipt_digest,
                &self.context_intent_digest,
            ]
            .into_iter()
            .any(|value| !is_sha256(value))
            || self.recompute_payload_digest() != self.selection_payload_digest
            || self.signed_selection_payload_digest != self.selection_payload_digest
            || self.verified_selection_payload_digest != self.selection_payload_digest
            || self.recompute_authenticated_binding_digest() != self.context_intent_digest
        {
            bail!("COMPUTE_PLUGIN_WINDOWS_LAUNCH_CONTEXT_SOURCE_CHANGED");
        }
        Ok(())
    }

    /// The signed payload never includes its envelope, verification receipt or final authenticated
    /// binding digest. This one-way layering prevents a future signature schema from reintroducing
    /// a digest fixed point.
    fn recompute_payload_digest(&self) -> String {
        let mut digest = PlanDigest::new(b"ELON_WINDOWS_RUNNER_LAUNCH_CONTEXT_PAYLOAD_V1");
        for value in [
            &self.admission_source_digest,
            &self.admission_receipt_digest,
            &self.manifest_digest,
            &self.signed_manifest_envelope_digest,
            &self.grant_digest,
            &self.target_id,
            &self.target_operating_system,
            &self.target_architecture,
            &self.runner_relative_path,
            &self.entrypoint_arguments_digest,
            &self.machine_context.context_policy_digest,
            &self.dll_search_policy.policy_digest,
            &self.process_creation_policy.policy_digest,
            &self.launch_security_expectation.token_profile_policy_digest,
            &self.control_key_id,
        ] {
            digest.text(value);
        }
        digest.u64(self.control_keyring_generation);
        digest.selector(&self.working_directory_selector);
        digest.finish()
    }

    fn recompute_authenticated_binding_digest(&self) -> String {
        let mut digest =
            PlanDigest::new(b"ELON_WINDOWS_RUNNER_AUTHENTICATED_LAUNCH_CONTEXT_BINDING_V1");
        for value in [
            &self.selection_payload_digest,
            &self.signed_selection_payload_digest,
            &self.verified_selection_payload_digest,
            &self.signed_selection_envelope_digest,
            &self.signature_verification_receipt_digest,
        ] {
            digest.text(value);
        }
        digest.finish()
    }

    pub(super) fn validate_process_projection(
        &self,
        selected_context_selector_digest: &str,
        selected_working_directory_identity_digest: &str,
        profile: &ComputePluginWorkAdmissionLaunchProfile,
        expected: &WindowsRunnerLaunchContextPreCreateProjection<'_>,
    ) -> Result<()> {
        self.validate_binding(
            &self.admission_source_digest,
            &self.admission_receipt_digest,
            profile,
        )?;
        let provided_flags = expected
            .process_creation_flags
            .iter()
            .copied()
            .collect::<Vec<_>>();
        let required_flags = self
            .process_creation_policy
            .creation_flags
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>();
        if expected.launch_context_selector_digest != selected_context_selector_digest
            || expected.process_machine_context_digest != self.machine_context.context_policy_digest
            || expected.working_directory_identity_digest
                != selected_working_directory_identity_digest
            || expected.runner_relative_path != self.runner_relative_path
            || expected.entrypoint_arguments_digest != self.entrypoint_arguments_digest
            || expected.restricted_token
                != self.launch_security_expectation.restricted_token_required
            || expected.app_container != self.launch_security_expectation.app_container_required
            || expected.inherited_handles != self.process_creation_policy.inherited_handles
            || expected.environment_policy != self.process_creation_policy.environment_policy
            || provided_flags != required_flags
            || !is_sha256(expected.startup_import_resolution_profile_digest)
        {
            bail!("COMPUTE_PLUGIN_WINDOWS_AUTHENTICATED_LAUNCH_CONTEXT_PROJECTION_CHANGED");
        }
        Ok(())
    }
}

impl AuthenticatedWindowsProcessMachineContextExpectation {
    fn validate(&self, target_architecture: &str) -> Result<()> {
        if self.target_architecture != target_architecture
            || !matches!(self.wow64_mode.as_str(), "native" | "wow64")
            || !is_sha256(&self.context_policy_digest)
            || self.recompute_digest() != self.context_policy_digest
        {
            bail!("COMPUTE_PLUGIN_WINDOWS_MACHINE_CONTEXT_EXPECTATION_INVALID");
        }
        Ok(())
    }

    fn recompute_digest(&self) -> String {
        let mut digest = PlanDigest::new(b"ELON_WINDOWS_PROCESS_MACHINE_EXPECTATION_V1");
        digest.text(&self.target_architecture);
        digest.text(&self.wow64_mode);
        digest.finish()
    }
}

impl AuthenticatedWindowsDllSearchPolicy {
    fn validate(&self) -> Result<()> {
        let mut roles = HashSet::new();
        let routes = self
            .resolution_route_order
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>();
        let phases = self
            .search_order
            .iter()
            .map(|role| role.policy_phase())
            .collect::<Vec<_>>();
        if self.search_order.is_empty()
            || self.ambient_path_allowed
            || !is_sha256(&self.policy_digest)
            || self
                .search_order
                .iter()
                .any(|role| !roles.insert(role.unique_key()))
            || routes.as_slice() != REQUIRED_RESOLUTION_ROUTES
            || !matches!(
                self.search_order[0],
                super::WindowsPreliminarySearchDirectoryRole::ApplicationDirectory
            )
            || !matches!(
                self.search_order[self.search_order.len() - 1],
                super::WindowsPreliminarySearchDirectoryRole::CurrentDirectory
            )
            || phases.windows(2).any(|pair| pair[0] > pair[1])
            || ![2_u8, 3, 4]
                .into_iter()
                .all(|required| phases.contains(&required))
            || self.recompute_digest() != self.policy_digest
        {
            bail!("COMPUTE_PLUGIN_WINDOWS_DLL_SEARCH_POLICY_INVALID");
        }
        Ok(())
    }

    fn recompute_digest(&self) -> String {
        let mut digest = PlanDigest::new(b"ELON_WINDOWS_DLL_SEARCH_POLICY_V1");
        for role in &self.search_order {
            digest.text(&role.unique_key());
        }
        for route in &self.resolution_route_order {
            digest.text(route);
        }
        digest.boolean(self.ambient_path_allowed);
        digest.finish()
    }
}

impl AuthenticatedWindowsProcessCreationPolicy {
    fn validate(&self) -> Result<()> {
        let flags = self
            .creation_flags
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>();
        if self.inherited_handles
            || self.environment_policy != EMPTY_ENVIRONMENT_POLICY
            || flags.as_slice() != REQUIRED_PROCESS_CREATION_FLAGS
            || !is_sha256(&self.policy_digest)
            || self.recompute_digest() != self.policy_digest
        {
            bail!("COMPUTE_PLUGIN_WINDOWS_PROCESS_CREATION_POLICY_INVALID");
        }
        Ok(())
    }

    fn recompute_digest(&self) -> String {
        let mut digest = PlanDigest::new(b"ELON_WINDOWS_PROCESS_CREATION_POLICY_V1");
        digest.boolean(self.inherited_handles);
        digest.text(&self.environment_policy);
        for flag in &self.creation_flags {
            digest.text(flag);
        }
        digest.finish()
    }
}

impl AuthenticatedWindowsRunnerLaunchSecurityExpectation {
    fn validate(&self) -> Result<()> {
        if (!self.restricted_token_required && !self.app_container_required)
            || !is_sha256(&self.token_profile_policy_digest)
            || self.recompute_digest() != self.token_profile_policy_digest
        {
            bail!("COMPUTE_PLUGIN_WINDOWS_LAUNCH_SECURITY_EXPECTATION_INVALID");
        }
        Ok(())
    }

    fn recompute_digest(&self) -> String {
        let mut digest = PlanDigest::new(b"ELON_WINDOWS_LAUNCH_SECURITY_EXPECTATION_V1");
        digest.boolean(self.restricted_token_required);
        digest.boolean(self.app_container_required);
        digest.finish()
    }
}
