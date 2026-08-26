//! Independently signed policy for bounded recursive Windows startup-import resolution.
//!
//! Launch-context payload V1 predates recursive parser limits. The immutable policy payload stays
//! in its original hash domain, while signer-envelope and typed verification evidence are bound by
//! authenticated-policy binding V2. A separate point-of-use currentness owner must authorize one
//! exact A0/Ak dispatch. No signature verifier or currentness backend produces either owner.

mod currentness;
mod digest;
mod signature;
mod validation;

use std::convert::Infallible;

use anyhow::{bail, Result};

use super::PreliminaryWindowsRunnerResolutionRequestPlanView;
use signature::{
    SignedWindowsRecursiveResolutionPolicyEnvelope,
    WindowsRecursivePolicySignatureVerificationReceipt,
};

pub(in crate::node_agent_compute_plugin_host::runtime_loader_load_set) use currentness::WindowsRecursivePolicyDispatchAuthorization;

pub(super) const RECURSIVE_DYNAMIC_LOAD_SCOPE: &str = "startup_import_closure_only_v1";

#[derive(Clone, Copy, PartialEq, Eq)]
pub(in crate::node_agent_compute_plugin_host::runtime_loader_load_set) struct AuthenticatedWindowsRecursiveResolutionPolicyLimits
{
    pub(in crate::node_agent_compute_plugin_host::runtime_loader_load_set) max_wave_count: u64,
    pub(in crate::node_agent_compute_plugin_host::runtime_loader_load_set) max_parsed_image_count:
        u64,
    pub(in crate::node_agent_compute_plugin_host::runtime_loader_load_set) max_module_request_count:
        u64,
    pub(in crate::node_agent_compute_plugin_host::runtime_loader_load_set) max_searched_name_count:
        u64,
    pub(in crate::node_agent_compute_plugin_host::runtime_loader_load_set) max_system_image_request_count:
        u64,
    /// Maximum forwarder-chain depth, not the total number of forwarder edges.
    pub(in crate::node_agent_compute_plugin_host::runtime_loader_load_set) max_forwarder_hop_count:
        u64,
}

/// One signed recursive policy after exact typed signature verification.
///
/// The payload is immutable V1 material. The signed envelope adds authority scope, generation and
/// validity bounds without mutating that payload domain. Signature verification evidence is kept
/// by value rather than reduced to caller-supplied digest strings. Point-of-use currentness remains
/// a separate one-dispatch linear owner so a previously verified policy cannot authorize a later
/// wave after signer revocation, policy supersession or expiry.
#[must_use = "authenticated recursive policy must remain with the whole recursive custody chain"]
pub(in crate::node_agent_compute_plugin_host::runtime_loader_load_set) struct AuthenticatedWindowsRecursiveResolutionPolicy
{
    launch_context_intent_digest: String,
    preliminary_request_plan_digest: String,
    parser_policy_digest: String,
    authenticated_preloaded_module_set_digest: String,
    inherited_resolution_route_order: Vec<String>,
    limits: AuthenticatedWindowsRecursiveResolutionPolicyLimits,
    ambient_path_allowed: bool,
    nested_api_set_host_redirection_allowed: bool,
    positive_shadow_disposition_allowed: bool,
    dynamic_module_load_scope: String,
    control_key_id: String,
    control_keyring_generation: u64,
    policy_payload_digest: String,
    signed_envelope: SignedWindowsRecursiveResolutionPolicyEnvelope,
    signature_verification: WindowsRecursivePolicySignatureVerificationReceipt,
    authenticated_recursive_policy_digest: String,
    _authenticated_recursive_policy_source_producer_unavailable: Infallible,
}

impl AuthenticatedWindowsRecursiveResolutionPolicy {
    pub(in crate::node_agent_compute_plugin_host::runtime_loader_load_set) fn digest(
        &self,
    ) -> &str {
        &self.authenticated_recursive_policy_digest
    }

    pub(in crate::node_agent_compute_plugin_host::runtime_loader_load_set) fn limits(
        &self,
    ) -> AuthenticatedWindowsRecursiveResolutionPolicyLimits {
        self.limits
    }

    pub(in crate::node_agent_compute_plugin_host::runtime_loader_load_set) fn parser_policy_digest(
        &self,
    ) -> &str {
        &self.parser_policy_digest
    }

    pub(in crate::node_agent_compute_plugin_host::runtime_loader_load_set) fn signature_verification_receipt_digest(
        &self,
    ) -> &str {
        &self
            .signature_verification
            .signature_verification_receipt_digest
    }

    /// Borrow-only limit gate. A caller must derive every cumulative projection from its whole
    /// typed plan. This check is necessary but not sufficient for dispatch: the same exact A0/Ak
    /// owner also needs a current `WindowsRecursivePolicyDispatchAuthorization`.
    #[allow(clippy::too_many_arguments)]
    pub(in crate::node_agent_compute_plugin_host::runtime_loader_load_set) fn validate_projected_totals_before_dispatch(
        &self,
        projected_recursive_wave_count: usize,
        projected_parsed_image_count: usize,
        projected_module_request_count: usize,
        projected_searched_name_count: usize,
        projected_system_image_request_count: usize,
        projected_forwarder_hop_depth: usize,
    ) -> Result<()> {
        let project = |value| {
            u64::try_from(value).map_err(|_| {
                anyhow::anyhow!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_POLICY_PROJECTION_OVERFLOW")
            })
        };
        let wave_count = project(projected_recursive_wave_count)?;
        let parsed_image_count = project(projected_parsed_image_count)?;
        let module_request_count = project(projected_module_request_count)?;
        let searched_name_count = project(projected_searched_name_count)?;
        let system_image_request_count = project(projected_system_image_request_count)?;
        let forwarder_hop_depth = project(projected_forwarder_hop_depth)?;
        if wave_count > self.limits.max_wave_count
            || parsed_image_count > self.limits.max_parsed_image_count
            || module_request_count > self.limits.max_module_request_count
            || searched_name_count > self.limits.max_searched_name_count
            || system_image_request_count > self.limits.max_system_image_request_count
            || forwarder_hop_depth > self.limits.max_forwarder_hop_count
        {
            bail!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_POLICY_LIMIT_EXCEEDED");
        }
        Ok(())
    }

    pub(in crate::node_agent_compute_plugin_host::runtime_loader_load_set) fn validate_against(
        &self,
        preliminary: &PreliminaryWindowsRunnerResolutionRequestPlanView<'_>,
    ) -> Result<()> {
        self.validate_expected_binding(
            preliminary.selected_context.context_intent_digest,
            preliminary.preliminary_request_plan_digest,
            preliminary.parser_policy_digest,
            preliminary.authenticated_preloaded_module_set_digest,
            preliminary.resolution_route_order,
        )
    }

    pub(in crate::node_agent_compute_plugin_host::runtime_loader_load_set) fn validate_expected_binding(
        &self,
        expected_launch_context_intent_digest: &str,
        expected_preliminary_request_plan_digest: &str,
        expected_parser_policy_digest: &str,
        expected_authenticated_preloaded_module_set_digest: &str,
        expected_resolution_route_order: &[String],
    ) -> Result<()> {
        validation::validate_policy_against(
            self,
            expected_launch_context_intent_digest,
            expected_preliminary_request_plan_digest,
            expected_parser_policy_digest,
            expected_authenticated_preloaded_module_set_digest,
            expected_resolution_route_order,
        )
    }
}
