//! By-value typestate for one recursive request -> grant -> candidate -> lease -> parse wave.

use std::convert::Infallible;

use anyhow::Result;

use crate::node_agent_managed_fs::{
    ManagedLoaderSystemImageContentLeasePositiveOutcomeCustody,
    PinnedWindowsLoaderResolvedSystemImageCandidate,
};

use super::super::super::super::launch_path_discovery::{
    AuthenticatedWindowsRecursiveResolutionPolicy, WindowsRecursivePolicyDispatchAuthorization,
};
use super::super::super::{
    PreFinalWindowsLoaderNamespaceGrantSet, WindowsLoaderPackageContentLeaseCustody,
    WindowsLoaderSearchedNameFenceCustody,
};
use super::super::{
    WindowsPostLeaseSystemImageParseReceipt, WindowsRecursiveWaveAcquisitionReceipt,
};
use super::{
    plan::{
        WindowsRecursiveBaseParsedImageOwnerPlanEntry,
        WindowsRecursiveRetainedForwarderChainPlanEntry,
        WindowsRecursiveRetainedSearchDirectoryAuthorityRef,
        WindowsRecursiveWaveDispatchPlanEvidence,
    },
    plan_digest, plan_validation, AuthenticatedWindowsRecursiveWaveResolutionPlan,
    WindowsRecursiveWaveRequestPlan,
};

/// Exact earlier graph, authenticated policy and live owners retained across every wave.
///
/// Package leases and filesystem positive outcomes each occur once in this owner. Later wave
/// records refer to already-retained package/preloaded/KnownDLL owners by typed ordinal instead of
/// cloning a handle or inventing a filesystem candidate for them.
#[must_use = "whole recursive graph must remain in the active wave or failure custody"]
pub(in crate::node_agent_compute_plugin_host::runtime_loader_load_set) struct WindowsRecursiveResolutionAccumulatedCustody<
    'root,
> {
    pub(super) root_namespace: PreFinalWindowsLoaderNamespaceGrantSet<'root>,
    pub(super) authenticated_policy: AuthenticatedWindowsRecursiveResolutionPolicy,
    pub(super) base_parsed_image_owners: Vec<WindowsRecursiveBaseParsedImageOwnerPlanEntry>,
    pub(super) retained_forwarder_chains: Vec<WindowsRecursiveRetainedForwarderChainPlanEntry>,
    pub(super) retained_search_directories:
        Vec<WindowsRecursiveRetainedSearchDirectoryAuthorityRef>,
    pub(super) base_package_content_leases: Vec<WindowsLoaderPackageContentLeaseCustody>,
    pub(super) retained_filesystem_content_leases:
        Vec<ManagedLoaderSystemImageContentLeasePositiveOutcomeCustody>,
    pub(super) completed_parse_receipts: Vec<WindowsPostLeaseSystemImageParseReceipt>,
    pub(super) completed_acquisition_receipts: Vec<WindowsRecursiveWaveAcquisitionReceipt>,
    pub(super) whole_state_digest: String,
}

struct WindowsRecursivePendingSearchedNameGrantRef {
    searched_name_ordinal: usize,
    search_directory_ordinal: usize,
    grant_request_digest: String,
}

struct WindowsRecursivePendingFilesystemCandidateRef {
    resolution_request_ordinal: usize,
    candidate_plan_digest: String,
}

struct WindowsRecursivePendingFilesystemLeaseRef {
    resolution_request_ordinal: usize,
    candidate_binding_digest: String,
    lease_request_digest: String,
}

struct WindowsRecursivePendingSameOwnerParseRef {
    parse_receipt_ordinal: usize,
    producer_module_request_ordinal: usize,
    source_owner_binding_digest: String,
}

/// Retained package lease is centralized in accumulated custody; this value is only its exact
/// typed parse reference.
struct WindowsRecursivePackageContentLeaseSourceRef {
    package_file_ordinal: usize,
    package_content_lease_binding_digest: String,
}

/// Exact authenticated bootstrap-section source; no filesystem candidate is involved.
struct WindowsRecursiveAuthenticatedPreloadedSourceRef {
    preloaded_module_ordinal: usize,
    component_identity_digest: String,
    immutable_section_identity_digest: String,
    authenticated_evidence_digest: String,
}

/// Exact KnownDLL section/mapping authority; no parent-relative file candidate is involved.
struct WindowsRecursiveKnownDllSectionSourceRef {
    known_dll_authority_record_ordinal: usize,
    section_identity_digest: String,
    immutable_section_identity_digest: String,
    section_image_mapping_receipt_digest: String,
}

/// Only a filesystem-backed ordinary or side-by-side route owns a retained candidate.
#[must_use = "filesystem candidate must enter one lease attempt or failure custody"]
struct WindowsRecursiveFilesystemCandidateCustody {
    resolution_request_ordinal: usize,
    candidate: PinnedWindowsLoaderResolvedSystemImageCandidate,
}

/// Positive lease response and same retained image owner used by the PE parser.
#[must_use = "filesystem lease outcome must remain through same-owner parse and final custody"]
struct WindowsRecursiveFilesystemLeaseSourceCustody {
    resolution_request_ordinal: usize,
    outcome: ManagedLoaderSystemImageContentLeasePositiveOutcomeCustody,
}

/// Route-specific post-resolution owner. Three routes reuse typed immutable authority; only the
/// filesystem variant carries a candidate file into lease acquisition.
enum WindowsRecursiveRouteCandidateCustody {
    PackageContentLease(WindowsRecursivePackageContentLeaseSourceRef),
    AuthenticatedPreloadedModule(WindowsRecursiveAuthenticatedPreloadedSourceRef),
    KnownDllSection(WindowsRecursiveKnownDllSectionSourceRef),
    ResolvedFilesystemSystemImage(WindowsRecursiveFilesystemCandidateCustody),
}

/// Exact immutable owner from which one target image must be parsed. The enum prevents package,
/// preloaded or KnownDLL targets from being coerced through the filesystem lease route.
enum WindowsRecursiveSameOwnerParseSourceCustody {
    PackageContentLease(WindowsRecursivePackageContentLeaseSourceRef),
    AuthenticatedPreloadedModule(WindowsRecursiveAuthenticatedPreloadedSourceRef),
    KnownDllSection(WindowsRecursiveKnownDllSectionSourceRef),
    ResolvedFilesystemSystemImage(WindowsRecursiveFilesystemLeaseSourceCustody),
}

/// Whole graph plus canonical outgoing request plan, before the first grant dispatch.
#[must_use = "wave request custody must move whole into grant acquisition or remain intact"]
pub(in crate::node_agent_compute_plugin_host::runtime_loader_load_set) struct WindowsRecursiveWaveRequestCustody<
    'root,
> {
    accumulated: WindowsRecursiveResolutionAccumulatedCustody<'root>,
    request_plan: WindowsRecursiveWaveRequestPlan,
    _wave_request_dispatch_producer_unavailable: Infallible,
}

/// Authenticated resolver output before the complete borrow-only plan gate. It has no grant
/// attempt or backend capability and cannot skip the policy-currentness pending state.
#[must_use = "unvalidated recursive resolution must validate whole or remain intact"]
pub(in crate::node_agent_compute_plugin_host::runtime_loader_load_set) struct WindowsRecursiveWaveResolvedPlanCustody<
    'root,
> {
    accumulated: WindowsRecursiveResolutionAccumulatedCustody<'root>,
    request_plan: WindowsRecursiveWaveRequestPlan,
    resolved_plan: AuthenticatedWindowsRecursiveWaveResolutionPlan,
    _resolved_plan_validator_transition_unavailable: Infallible,
}

/// The complete typed Ak plan has passed its borrow-only structural and cumulative-limit gate,
/// but no searched-name request may be dispatched until a fresh policy-currentness authorization
/// is consumed into the next state. The plan evidence remains by value so the authorization is
/// bound to the exact evidence that will accompany every later acquisition stage.
#[must_use = "validated wave must obtain fresh policy currentness before first dispatch"]
pub(in crate::node_agent_compute_plugin_host::runtime_loader_load_set) struct PolicyCurrentnessPendingWindowsRecursiveWaveGrantCustody<
    'root,
> {
    accumulated: WindowsRecursiveResolutionAccumulatedCustody<'root>,
    validated_plan_evidence: WindowsRecursiveWaveDispatchPlanEvidence,
    _policy_currentness_authorization_producer_unavailable: Infallible,
}

impl PolicyCurrentnessPendingWindowsRecursiveWaveGrantCustody<'_> {
    /// Borrow-only Ak gate for the authorization that a future consuming transition would move
    /// into `DispatchReadyWindowsRecursiveWaveGrantCustody` beside this exact plan evidence.
    pub(super) fn validate_authorization_before_first_dispatch(
        &self,
        authorization: &WindowsRecursivePolicyDispatchAuthorization,
    ) -> Result<()> {
        let producer_wave_ordinal = self.validated_plan_evidence.producer_wave_ordinal;
        let pre_dispatch_plan_evidence_digest =
            plan_digest::recursive_pre_dispatch_plan_evidence_digest(
                &self.validated_plan_evidence,
            )?;
        authorization.validate_against(
            &self.accumulated.authenticated_policy,
            producer_wave_ordinal,
            producer_wave_ordinal,
            &self.validated_plan_evidence.input_custody_digest,
            &pre_dispatch_plan_evidence_digest,
        )
    }
}

/// The only Ak state a future searched-name dispatcher may consume. Its typed plan evidence and
/// fresh policy-currentness authorization remain by value through every later acquisition stage.
#[must_use = "dispatch-ready wave grants must advance whole or enter grant failure custody"]
pub(in crate::node_agent_compute_plugin_host::runtime_loader_load_set) struct DispatchReadyWindowsRecursiveWaveGrantCustody<
    'root,
> {
    accumulated: WindowsRecursiveResolutionAccumulatedCustody<'root>,
    validated_plan_evidence: WindowsRecursiveWaveDispatchPlanEvidence,
    policy_dispatch_authorization: WindowsRecursivePolicyDispatchAuthorization,
    acquired_searched_name_grants: Vec<WindowsLoaderSearchedNameFenceCustody>,
    pending_searched_name_grants: Vec<WindowsRecursivePendingSearchedNameGrantRef>,
    searched_name_grant_set_digest: String,
    _dispatch_ready_grant_advancer_unavailable: Infallible,
}

pub(super) type WindowsRecursiveWaveGrantAcquisitionCustody<'root> =
    DispatchReadyWindowsRecursiveWaveGrantCustody<'root>;

impl WindowsRecursiveWaveResolvedPlanCustody<'_> {
    /// Complete typed-plan gate. A future validator transition may consume this intact state into
    /// `PolicyCurrentnessPendingWindowsRecursiveWaveGrantCustody` only after this returns success;
    /// no such producer exists in the source-only architecture slice.
    pub(super) fn validate_whole_before_first_dispatch(&self) -> Result<()> {
        let projection = plan_validation::validate_whole_before_first_dispatch(
            &self.accumulated,
            &self.request_plan,
            &self.resolved_plan,
        )?;
        self.accumulated
            .authenticated_policy
            .validate_projected_totals_before_dispatch(
                projection.recursive_wave_count,
                projection.parsed_image_count,
                projection.module_request_count,
                projection.searched_name_count,
                projection.system_image_request_count,
                projection.forwarder_hop_depth,
            )
    }
}

/// Route selection after all required searched-name grants. Filesystem candidates first become
/// real owners in this post-grant state; other terminal routes retain purpose-specific refs.
#[must_use = "route candidates must advance whole into required leases or failure custody"]
pub(in crate::node_agent_compute_plugin_host::runtime_loader_load_set) struct WindowsRecursiveWaveCandidateAcquisitionCustody<
    'root,
> {
    accumulated: WindowsRecursiveResolutionAccumulatedCustody<'root>,
    validated_plan_evidence: WindowsRecursiveWaveDispatchPlanEvidence,
    policy_dispatch_authorization: WindowsRecursivePolicyDispatchAuthorization,
    acquired_searched_name_grants: Vec<WindowsLoaderSearchedNameFenceCustody>,
    acquired_route_candidates: Vec<WindowsRecursiveRouteCandidateCustody>,
    pending_filesystem_candidates: Vec<WindowsRecursivePendingFilesystemCandidateRef>,
    filesystem_candidate_set_digest: String,
    _wave_candidate_advancer_unavailable: Infallible,
}

/// Lease acquisition owns every acquired non-filesystem source and every still-linear filesystem
/// candidate. An active lease attempt moves one candidate out and must be parked in failure
/// custody if no valid positive transition is available.
#[must_use = "partial wave leases must advance whole or enter lease failure custody"]
pub(in crate::node_agent_compute_plugin_host::runtime_loader_load_set) struct WindowsRecursiveWaveLeaseAcquisitionCustody<
    'root,
> {
    accumulated: WindowsRecursiveResolutionAccumulatedCustody<'root>,
    validated_plan_evidence: WindowsRecursiveWaveDispatchPlanEvidence,
    policy_dispatch_authorization: WindowsRecursivePolicyDispatchAuthorization,
    acquired_searched_name_grants: Vec<WindowsLoaderSearchedNameFenceCustody>,
    acquired_parse_sources: Vec<WindowsRecursiveSameOwnerParseSourceCustody>,
    pending_filesystem_candidates: Vec<WindowsRecursiveFilesystemCandidateCustody>,
    pending_filesystem_leases: Vec<WindowsRecursivePendingFilesystemLeaseRef>,
    immutable_content_lease_set_digest: String,
    _wave_lease_advancer_unavailable: Infallible,
}

/// Same-owner parse stage. Every completed receipt remains beside its exact immutable source;
/// pending sources cannot be reduced to a path, digest or retry scalar.
#[must_use = "same-owner parse custody must complete whole or enter parse failure custody"]
pub(in crate::node_agent_compute_plugin_host::runtime_loader_load_set) struct WindowsRecursiveWaveSameOwnerParseCustody<
    'root,
> {
    accumulated: WindowsRecursiveResolutionAccumulatedCustody<'root>,
    validated_plan_evidence: WindowsRecursiveWaveDispatchPlanEvidence,
    policy_dispatch_authorization: WindowsRecursivePolicyDispatchAuthorization,
    acquired_searched_name_grants: Vec<WindowsLoaderSearchedNameFenceCustody>,
    completed_parse_sources: Vec<WindowsRecursiveSameOwnerParseSourceCustody>,
    completed_parse_receipts: Vec<WindowsPostLeaseSystemImageParseReceipt>,
    pending_parse_sources: Vec<WindowsRecursiveSameOwnerParseSourceCustody>,
    pending_parse_refs: Vec<WindowsRecursivePendingSameOwnerParseRef>,
    same_owner_parse_set_digest: String,
    _same_owner_parser_advancer_unavailable: Infallible,
}

/// One completed producer acquisition, still retaining the exact sources and whole predecessor
/// graph. The next wave may be formed only by a future consuming sealer.
#[must_use = "completed wave must enter the next wave or terminal whole-chain custody"]
pub(in crate::node_agent_compute_plugin_host::runtime_loader_load_set) struct WindowsRecursiveWaveCompletedCustody<
    'root,
> {
    accumulated: WindowsRecursiveResolutionAccumulatedCustody<'root>,
    acquired_searched_name_grants: Vec<WindowsLoaderSearchedNameFenceCustody>,
    completed_parse_sources: Vec<WindowsRecursiveSameOwnerParseSourceCustody>,
    completed_parse_receipts: Vec<WindowsPostLeaseSystemImageParseReceipt>,
    acquisition_receipt: WindowsRecursiveWaveAcquisitionReceipt,
    _completed_wave_sealer_unavailable: Infallible,
}

/// A base-only closure reaches the terminal state directly after `A0`; otherwise the last
/// recursive completed wave owns `AN`. This explicit sum avoids manufacturing an empty recursive
/// wave merely to represent the zero-wave case.
enum WindowsRecursiveTerminalPredecessorCustody<'root> {
    BaseOnly(WindowsRecursiveResolutionAccumulatedCustody<'root>),
    CompletedRecursiveWave(WindowsRecursiveWaveCompletedCustody<'root>),
}

/// Terminal `AN` custody. It is reachable only with an empty next frontier and retains either the
/// base-only accumulated graph or the final completed recursive wave by value.
#[must_use = "terminal recursive custody must move whole into the final closure sealer"]
pub(in crate::node_agent_compute_plugin_host::runtime_loader_load_set) struct TerminalWindowsRecursiveResolutionCustody<
    'root,
> {
    predecessor: WindowsRecursiveTerminalPredecessorCustody<'root>,
    terminal_empty_frontier_receipt_digest: String,
    _terminal_recursive_resolution_sealer_unavailable: Infallible,
}
