//! Independently signed policy for bounded recursive Windows startup-import resolution.
//!
//! Launch-context payload V1 predates recursive parser limits. This source-only authority uses a
//! separate signature domain and binds one already-authenticated context plus its preliminary
//! request/parser inputs. It therefore neither mutates the V1 launch-context hash domain nor lets
//! a closure self-assert unauthenticated limit values. No signature verifier produces this owner.

use std::convert::Infallible;

use anyhow::{bail, Result};

use crate::node_agent_compute_plugin_host::manifest_validation::is_sha256;

use super::{digest::PlanDigest, PreliminaryWindowsRunnerResolutionRequestPlanView};

const RECURSIVE_DYNAMIC_LOAD_SCOPE: &str = "startup_import_closure_only_v1";

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

/// Control-signed recursive policy bound one-way to an authenticated launch-context V1 intent.
///
/// Structural permissions are hard V1 invariants rather than opt-in switches: ambient PATH,
/// nested API-set host redirection, positive Shadow resolution and post-start dynamic loads all
/// remain outside this policy. The `Infallible` field keeps even correctly shaped source data from
/// becoming authenticated evidence without a future signature/key-currentness backend.
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
    signed_policy_payload_digest: String,
    verified_policy_payload_digest: String,
    signed_policy_envelope_digest: String,
    signature_verification_receipt_digest: String,
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

    /// Borrow-only gate required before the first side-effecting dispatch of either A0 or a
    /// recursive producer wave. A future caller must derive these cumulative projections from its
    /// whole typed plan; passing this gate creates no grant, candidate, lease or retry authority.
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
        if self.launch_context_intent_digest != expected_launch_context_intent_digest
            || self.preliminary_request_plan_digest != expected_preliminary_request_plan_digest
            || self.parser_policy_digest != expected_parser_policy_digest
            || self.authenticated_preloaded_module_set_digest
                != expected_authenticated_preloaded_module_set_digest
            || self.inherited_resolution_route_order != expected_resolution_route_order
            || self.ambient_path_allowed
            || self.nested_api_set_host_redirection_allowed
            || self.positive_shadow_disposition_allowed
            || self.dynamic_module_load_scope != RECURSIVE_DYNAMIC_LOAD_SCOPE
            || self.control_key_id.trim().is_empty()
            || self.control_keyring_generation == 0
            || [
                &self.launch_context_intent_digest,
                &self.preliminary_request_plan_digest,
                &self.parser_policy_digest,
                &self.authenticated_preloaded_module_set_digest,
                &self.policy_payload_digest,
                &self.signed_policy_payload_digest,
                &self.verified_policy_payload_digest,
                &self.signed_policy_envelope_digest,
                &self.signature_verification_receipt_digest,
                &self.authenticated_recursive_policy_digest,
            ]
            .into_iter()
            .any(|value| !is_sha256(value))
            || self.recompute_payload_digest() != self.policy_payload_digest
            || self.signed_policy_payload_digest != self.policy_payload_digest
            || self.verified_policy_payload_digest != self.policy_payload_digest
            || self.recompute_authenticated_binding_digest()
                != self.authenticated_recursive_policy_digest
        {
            bail!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_POLICY_SOURCE_CHANGED");
        }
        Ok(())
    }

    fn recompute_payload_digest(&self) -> String {
        let mut digest = PlanDigest::new(b"ELON_WINDOWS_RECURSIVE_RESOLUTION_POLICY_PAYLOAD_V1");
        for value in [
            &self.launch_context_intent_digest,
            &self.preliminary_request_plan_digest,
            &self.parser_policy_digest,
            &self.authenticated_preloaded_module_set_digest,
        ] {
            digest.text(value);
        }
        for route in &self.inherited_resolution_route_order {
            digest.text(route);
        }
        for limit in [
            self.limits.max_wave_count,
            self.limits.max_parsed_image_count,
            self.limits.max_module_request_count,
            self.limits.max_searched_name_count,
            self.limits.max_system_image_request_count,
            self.limits.max_forwarder_hop_count,
        ] {
            digest.u64(limit);
        }
        digest.boolean(self.ambient_path_allowed);
        digest.boolean(self.nested_api_set_host_redirection_allowed);
        digest.boolean(self.positive_shadow_disposition_allowed);
        digest.text(&self.dynamic_module_load_scope);
        digest.text(&self.control_key_id);
        digest.u64(self.control_keyring_generation);
        digest.finish()
    }

    fn recompute_authenticated_binding_digest(&self) -> String {
        let mut digest = PlanDigest::new(b"ELON_WINDOWS_AUTHENTICATED_RECURSIVE_POLICY_BINDING_V1");
        for value in [
            &self.policy_payload_digest,
            &self.signed_policy_payload_digest,
            &self.verified_policy_payload_digest,
            &self.signed_policy_envelope_digest,
            &self.signature_verification_receipt_digest,
        ] {
            digest.text(value);
        }
        digest.finish()
    }
}
