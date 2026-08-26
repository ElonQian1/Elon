//! Policy-current A0 authority required before the first base name-grant dispatch.
//!
//! The whole GrantReady owner, authenticated recursive policy and one exact dispatch
//! authorization remain linear and inseparable. No currentness or dispatch producer exists in
//! this source-only architecture slice.

use std::convert::Infallible;

use anyhow::Result;

use super::super::super::launch_path_discovery::{
    AuthenticatedWindowsRecursiveResolutionPolicy, WindowsRecursivePolicyDispatchAuthorization,
};
use super::super::system_closure::base_pre_dispatch_plan_evidence_digest;
use super::GrantReadyWindowsRunnerResolutionPrerequisite;

/// The only GrantReady shape a future base searched-name dispatcher may consume.
///
/// Authorization coordinates are fixed to acquisition receipt `0`, producer wave `0`, and the
/// exact GrantReady plan digest as input custody. The uninhabited field prevents a source-shaped
/// policy or authorization from making A0 dispatch reachable.
#[must_use = "policy-current GrantReady custody must enter A0 dispatch or remain whole"]
pub(in crate::node_agent_compute_plugin_host::runtime_loader_load_set) struct PolicyCurrentGrantReadyWindowsRunnerResolutionPrerequisite<
    'root,
> {
    grant_ready: GrantReadyWindowsRunnerResolutionPrerequisite<'root>,
    authenticated_recursive_policy: AuthenticatedWindowsRecursiveResolutionPolicy,
    policy_dispatch_authorization: WindowsRecursivePolicyDispatchAuthorization,
    _policy_currentness_transition_producer_unavailable: Infallible,
}

impl PolicyCurrentGrantReadyWindowsRunnerResolutionPrerequisite<'_> {
    /// Borrow-only gate immediately before the first base name-grant side effect.
    pub(in crate::node_agent_compute_plugin_host::runtime_loader_load_set) fn validate_whole_before_first_base_dispatch(
        &self,
    ) -> Result<()> {
        self.grant_ready.validate_whole()?;
        self.authenticated_recursive_policy
            .validate_against(&self.grant_ready.borrow_preliminary_requests())?;
        let input_custody_digest = self.grant_ready.plan.digest();
        let pre_dispatch_plan_evidence_digest =
            base_pre_dispatch_plan_evidence_digest(input_custody_digest)?;
        self.policy_dispatch_authorization.validate_against(
            &self.authenticated_recursive_policy,
            0,
            0,
            input_custody_digest,
            &pre_dispatch_plan_evidence_digest,
        )
    }
}
