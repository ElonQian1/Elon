//! Source-only linear acquisition contract for recursive Windows system-image waves.
//!
//! Receipt `A0` binds the GrantReady/base request range and produces the first recursive parse
//! frontier. Receipt `Ak` binds producer wave `k`; the final `AN` binds the last producer and an
//! empty frontier. Parse receipts retain only the producer acquisition ordinal, so this receipt
//! chain can commit their digests without introducing a digest cycle.

mod custody;
mod digest;
mod failure;
mod plan;
mod plan_digest;
mod plan_forwarder_validation;
mod plan_owner_validation;
mod plan_projection;
mod plan_validation;
mod validation;

use std::convert::Infallible;

use anyhow::Result;

use super::super::super::launch_path_discovery::AuthenticatedWindowsRecursiveResolutionPolicy;
use super::super::{
    SealedWindowsLoaderNamespacePrerequisite, SealedWindowsLoaderResolutionAuthority,
};
use super::SealedWindowsRecursiveResolutionClosure;

pub(in crate::node_agent_compute_plugin_host::runtime_loader_load_set) use custody::{
    DispatchReadyWindowsRecursiveWaveGrantCustody, TerminalWindowsRecursiveResolutionCustody,
    WindowsRecursiveResolutionAccumulatedCustody, WindowsRecursiveWaveCandidateAcquisitionCustody,
    WindowsRecursiveWaveCompletedCustody, WindowsRecursiveWaveLeaseAcquisitionCustody,
    WindowsRecursiveWaveRequestCustody, WindowsRecursiveWaveResolvedPlanCustody,
    WindowsRecursiveWaveSameOwnerParseCustody,
};
pub(in crate::node_agent_compute_plugin_host::runtime_loader_load_set) use failure::{
    WindowsRecursiveWaveAdvanceFailureClass, WindowsRecursiveWaveAdvanceFailureCustody,
};
pub(in crate::node_agent_compute_plugin_host::runtime_loader_load_set) use plan::{
    AuthenticatedWindowsRecursiveWaveResolutionPlan, WindowsRecursiveWaveRequestPlan,
};

/// One source-only acquisition receipt in the unified `A0..AN` chain.
///
/// `target_parse_wave_ordinal` is `Some(producer + 1)` exactly when the next frontier is nonempty.
/// The terminal receipt uses `None`. The receipt owns no live handle; live grants, candidates,
/// leases and parse owners remain in the by-value typestate graph until a future whole-chain
/// sealer consumes them.
#[must_use = "wave acquisition receipt must remain ordered in its sealed acquisition chain"]
pub(in crate::node_agent_compute_plugin_host::runtime_loader_load_set) struct WindowsRecursiveWaveAcquisitionReceipt
{
    acquisition_receipt_ordinal: usize,
    previous_acquisition_receipt_digest: Option<String>,
    authenticated_recursive_policy_digest: String,
    parser_policy_digest: String,
    producer_wave_ordinal: usize,
    target_parse_wave_ordinal: Option<usize>,
    source_frontier_parse_receipt_ordinals: Vec<usize>,
    first_module_request_ordinal: usize,
    module_request_count: usize,
    first_searched_name_ordinal: usize,
    searched_name_count: usize,
    first_system_image_request_ordinal: usize,
    system_image_request_count: usize,
    input_custody_digest: String,
    base_parsed_image_owner_set_digest: String,
    retained_forwarder_chain_set_digest: String,
    source_request_plan_digest: String,
    resolved_plan_digest: String,
    pre_dispatch_plan_evidence: WindowsRecursiveAcquisitionPlanEvidence,
    pre_dispatch_plan_evidence_digest: String,
    searched_name_grant_set_digest: String,
    filesystem_candidate_set_digest: String,
    immutable_content_lease_set_digest: String,
    same_owner_parse_set_digest: String,
    next_frontier_parse_receipt_ordinals: Vec<usize>,
    output_custody_digest: String,
    receipt_digest: String,
}

/// Retained, canonical pre-dispatch evidence. `A0` reuses the already typed GrantReady plan;
/// recursive `Ak` receipts retain every independently recomputable typed-plan commitment needed
/// for final graph cross-binding without retaining or cloning a live owner.
enum WindowsRecursiveAcquisitionPlanEvidence {
    BaseGrantReady {
        grant_ready_resolution_plan_digest: String,
    },
    RecursiveWave {
        plan: plan::WindowsRecursiveWaveDispatchPlanEvidence,
    },
}

/// Independently authenticated policy plus all `A0..AN` acquisition receipts.
///
/// The policy is retained by value and cannot be detached from the receipt chain. The uninhabited
/// sealer prevents source-shaped receipts from being promoted into final recursive authority.
#[must_use = "authenticated recursive policy and all acquisition receipts are one linear owner"]
pub(in crate::node_agent_compute_plugin_host::runtime_loader_load_set) struct SealedWindowsRecursiveResolutionAcquisitionChain
{
    policy: AuthenticatedWindowsRecursiveResolutionPolicy,
    parser_policy_digest: String,
    receipts: Vec<WindowsRecursiveWaveAcquisitionReceipt>,
    receipt_set_digest: String,
    acquisition_chain_digest: String,
    _recursive_acquisition_chain_sealer_unavailable: Infallible,
}

impl SealedWindowsRecursiveResolutionAcquisitionChain {
    pub(in crate::node_agent_compute_plugin_host::runtime_loader_load_set) fn policy(
        &self,
    ) -> &AuthenticatedWindowsRecursiveResolutionPolicy {
        &self.policy
    }

    pub(in crate::node_agent_compute_plugin_host::runtime_loader_load_set) fn receipts(
        &self,
    ) -> &[WindowsRecursiveWaveAcquisitionReceipt] {
        &self.receipts
    }

    pub(in crate::node_agent_compute_plugin_host::runtime_loader_load_set) fn digest(
        &self,
    ) -> &str {
        &self.acquisition_chain_digest
    }

    pub(in crate::node_agent_compute_plugin_host::runtime_loader_load_set) fn validate_projection_against(
        &self,
        closure: &SealedWindowsRecursiveResolutionClosure,
        resolution: &SealedWindowsLoaderResolutionAuthority,
    ) -> Result<()> {
        validation::validate_projection_against(self, closure, resolution)
    }

    /// Recomputes each receipt's grant commitment from the final live grant owners. This is kept
    /// separate from projection validation because the sealed resolution intentionally contains
    /// no duplicate grant handles.
    pub(in crate::node_agent_compute_plugin_host::runtime_loader_load_set) fn validate_namespace_grants_against(
        &self,
        closure: &SealedWindowsRecursiveResolutionClosure,
        namespace: &SealedWindowsLoaderNamespacePrerequisite,
        resolution: &SealedWindowsLoaderResolutionAuthority,
    ) -> Result<()> {
        validation::validate_namespace_grants_against(self, closure, namespace, resolution)
    }
}
